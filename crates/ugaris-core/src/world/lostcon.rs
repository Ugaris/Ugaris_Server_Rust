//! Lost-connection linger (`src/module/lostcon.c`, `CDR_LOSTCON`).
//!
//! C's `kick_player` (`src/system/player.c:174`) does not despawn a
//! disconnecting player's character immediately: it detaches the character
//! from the socket, switches `ch[cn].driver` to `CDR_LOSTCON`, and arms
//! `dat->timeout = ticker + lagout_time`. The character stays fully live
//! (attackable, on the map) until either the player reconnects (`tick_login`
//! reclaims it in place, clearing the driver back to `0`) or the timeout
//! expires, at which point `lostcon_driver` calls `exit_char` to save and
//! despawn it. This module owns the `World`-side half of that state
//! machine; the session-teardown/reconnect wiring and the save I/O live in
//! `ugaris-server`.
//!
//! The self-defense AI cascade `lostcon_driver` runs each tick while
//! lingering: the message loop (`process_lostcon_messages`) and the
//! visible-enemy attack/invisible-follow cascade
//! (`World::process_lostcon_attack_action_with_random`, `npc_fight.rs`)
//! are ported; this module additionally ports the rest of
//! `lostcon_driver`'s body - the early-exit gauntlet
//! (`lostcon_early_exit_characters`, rest-area/arena/karma), the low-hp-
//! heal/low-mana-potion/low-magicshield pre-cascade
//! (`process_lostcon_self_care_precascade`) that runs *before* the attack
//! cascade, the bless/magicshield/heal fallback
//! (`process_lostcon_self_care_postcascade`) that runs *after* it, and the
//! `do_idle` tail (`queue_lostcon_idle`) for a tick where nothing else
//! happened.

use super::*;

/// C `lostcon_ppd`'s six self-care toggles consumed directly by
/// `lostcon_driver`'s own body (`src/module/lostcon.c:164-220`), as
/// opposed to `FightDriverSuppressions`'s ten toggles consumed by
/// `fight_driver_attack_enemy`. `PlayerRuntime::lostcon_self_care_
/// suppressions` (`ugaris-core::player`) builds one of these from the
/// lingering session's stashed `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LostconSelfCareSuppressions {
    pub noheal: bool,
    pub noshield: bool,
    pub nobless: bool,
    pub nolife: bool,
    pub nomana: bool,
    pub nocombo: bool,
}

impl World {
    /// C `kick_player`: `ch[cn].driver = CDR_LOSTCON` plus the
    /// `char_driver(driver, CDT_DEAD, cn, 0, 0)` reset that arms
    /// `dat->timeout = ticker + lagout_time`. Returns `false` if the
    /// character does not exist.
    pub fn enter_lostcon(&mut self, character_id: CharacterId, deadline_tick: u64) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        character.driver = CDR_LOSTCON;
        character.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
            deadline: deadline_tick,
        }));
        true
    }

    /// C `tick_login()`/`read_login()`: `ch[n].driver = 0` once the
    /// lingering character is reclaimed by a reconnecting session.
    pub fn reclaim_lostcon(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if character.driver != CDR_LOSTCON {
            return false;
        }
        character.driver = 0;
        character.driver_state = None;
        true
    }

    /// Whether `character_id` is currently lingering under `CDR_LOSTCON`.
    pub fn is_lostcon(&self, character_id: CharacterId) -> bool {
        self.characters
            .get(&character_id)
            .is_some_and(|character| character.driver == CDR_LOSTCON)
    }

    /// C `lostcon_driver`'s `!ch[cn].player && ticker > dat->timeout`
    /// branch: characters whose lagout linger has expired and were never
    /// reclaimed. Callers are responsible for saving and calling
    /// `remove_character` (the C `exit_char`/`kick_char` tail).
    pub fn expired_lostcon_characters(&self, current_tick: u64) -> Vec<CharacterId> {
        self.characters
            .iter()
            .filter_map(|(&character_id, character)| {
                let deadline = match &character.driver_state {
                    Some(CharacterDriverState::Lostcon(data))
                        if character.driver == CDR_LOSTCON =>
                    {
                        data.deadline
                    }
                    _ => return None,
                };
                (current_tick >= deadline).then_some(character_id)
            })
            .collect()
    }

    /// C `lostcon_driver`'s early-exit gauntlet (`src/module/lostcon.c:87-
    /// 104`), covering every immediate-exit condition besides the ordinary
    /// lagout timeout (`expired_lostcon_characters`): leaving a rest-area
    /// tile or an arena tile at once, and the karma cutoff (`karma <= -12`,
    /// or `karma <= -5` when not `CF_PAID`). Callers should treat a
    /// character in the returned list exactly like an entry from
    /// `expired_lostcon_characters` - queue a save+despawn instead of
    /// running the rest of the self-defense cascade this tick.
    ///
    /// Deliberately not ported: C's own second arena check ("leave after
    /// 10s if lagging in an arena", `lostcon.c:96-99`, which also guards a
    /// `kick_player` call behind `if (ch[cn].player)`). `kick_player`
    /// (`src/system/player.c:187`) unconditionally clears `ch[cn].player =
    /// 0` in the same statement that sets `ch[cn].driver = CDR_LOSTCON`,
    /// so every character this function (and all of `lostcon_driver`) ever
    /// runs for already has `ch[cn].player == 0` - the immediate arena
    /// check directly above it (`lostcon.c:92-95`, ported below) already
    /// unconditionally fires first for every arena tile outside area 34,
    /// making the 10s-lag check and its `kick_player` guard permanently
    /// unreachable dead code in the C oracle itself.
    pub fn lostcon_early_exit_characters(&self, area_id: u16) -> Vec<CharacterId> {
        self.characters
            .iter()
            .filter_map(|(&character_id, character)| {
                if character.driver != CDR_LOSTCON {
                    return None;
                }
                if let Some(tile) = self
                    .map
                    .tile(usize::from(character.x), usize::from(character.y))
                {
                    if tile.flags.contains(MapFlags::RESTAREA) {
                        return Some(character_id);
                    }
                    if tile.flags.contains(MapFlags::ARENA) && area_id != 34 {
                        return Some(character_id);
                    }
                }
                if character.karma <= -12
                    || (!character.flags.contains(CharacterFlags::PAID) && character.karma <= -5)
                {
                    return Some(character_id);
                }
                None
            })
            .collect()
    }

    /// C `lostcon_driver`'s per-message loop (`src/module/lostcon.c:117-
    /// 141`): drains the lingering character's driver-message queue.
    /// `NT_GOTHIT` is the only message type that does anything -
    /// `fight_driver_note_hit(cn)` plus (when `msg->dat1` names an
    /// attacker) `fight_driver_add_enemy(cn, co, 1, 1)` (hurtme=1,
    /// visible=1, unconditionally - no `can_attack` gate at this call
    /// site, matching C exactly). `NT_CHAR`'s would-be aggro-on-sight is
    /// commented out in C itself (`// if (co && can_attack(cn,co))
    /// fight_driver_add_enemy(cn,co,1,1);`) and `NT_TEXT` is a no-op
    /// comment - neither is ported, matching C's real (non-commented)
    /// behavior. No-op if `character_id` is not currently `CDR_LOSTCON`.
    pub fn process_lostcon_messages(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        if character.driver != CDR_LOSTCON {
            return;
        }
        let messages = std::mem::take(&mut character.driver_messages);
        if messages.is_empty() {
            return;
        }
        let current_tick = self.tick.0 as i32;

        for message in messages {
            if message.message_type != NT_GOTHIT {
                continue;
            }
            let Some(character) = self.characters.get_mut(&character_id) else {
                return;
            };
            character
                .fight_driver
                .get_or_insert_with(FightDriverData::default)
                .last_hit = current_tick;

            if message.dat1 <= 0 {
                continue;
            }
            let target_id = CharacterId(message.dat1 as u32);
            let Some(target) = self.characters.get(&target_id).cloned() else {
                continue;
            };
            let Some(character) = self.characters.get_mut(&character_id) else {
                return;
            };
            add_simple_baddy_enemy_unchecked(character, target_id, 1, current_tick);
            if let Some(data) = character.fight_driver.as_mut() {
                if let Some(enemy) = data
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.target_id == target_id)
                {
                    enemy.visible = true;
                    enemy.last_x = target.x;
                    enemy.last_y = target.y;
                }
            }
        }
    }

    /// C `lostcon_driver`'s low-hp-heal/low-mana-potion/low-magicshield
    /// pre-cascade (`src/module/lostcon.c:164-197`), which runs *before*
    /// `fight_driver_update`/`fight_driver_attack_visible`/
    /// `fight_driver_follow_invisible`. Returns `true` only for the two
    /// sub-checks that C itself `return`s early from (heal, magicshield) -
    /// the mana-potion drink in between never returns early in C (only
    /// `break`s its own search loop), so a `false` return here does not
    /// mean nothing happened, only that the caller should still proceed to
    /// the attack cascade (which self-gates on `action != 0`, so a
    /// successful heal/magicshield here already prevents it from doing
    /// anything, exactly like C's `return`). No-op (returns `false`
    /// without touching state) if `character_id` is not currently
    /// `CDR_LOSTCON`, already mid-action, or dead.
    pub fn process_lostcon_self_care_precascade(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        suppressions: LostconSelfCareSuppressions,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_LOSTCON
            || character.action != 0
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }
        let weather_movement_percent = self.settings.weather_movement_percent;

        // low on hp? try heal (`lostcon.c:164-170`).
        if character.hp
            < character_value_present(&character, CharacterValue::Hp) * POWERSCALE * 3 / 4
            && !suppressions.noheal
            && character.mana
                > character_value_present(&character, CharacterValue::Mana) * POWERSCALE / 2
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_heal(
                    caster,
                    &character,
                    None,
                    &self.map,
                    weather_movement_percent,
                )
                .is_ok()
                {
                    return true;
                }
            }
        }

        // low on mana? use an inventory potion (`lostcon.c:172-189`). Does
        // not return early in C - falls through to the magicshield check
        // regardless of whether a potion was actually found/used.
        if character.mana
            < character_value_present(&character, CharacterValue::Mana) * POWERSCALE / 4
            && (!suppressions.nolife || !suppressions.nocombo)
        {
            if let Some(item_id) = find_lostcon_mana_potion(&character, &self.items, suppressions) {
                self.execute_item_driver_request(
                    ItemDriverRequest::Driver {
                        driver: IDR_POTION,
                        item_id,
                        character_id,
                        spec: 0,
                    },
                    area_id,
                );
            }
        }

        // low on magic shield? try respell (`lostcon.c:191-197`).
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.lifeshield
            < character_value_present(&character, CharacterValue::MagicShield) * POWERSCALE / 4
            && !suppressions.noshield
            && character.mana
                > character_value_present(&character, CharacterValue::Mana) * POWERSCALE / 2
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_magicshield(caster, &self.map, weather_movement_percent).is_ok() {
                    return true;
                }
            }
        }

        false
    }

    /// C `lostcon_driver`'s bless/magicshield/heal fallback
    /// (`src/module/lostcon.c:207-218`), which runs after both
    /// `fight_driver_attack_visible` and `fight_driver_follow_invisible`
    /// returned false (nothing to fight). Tries each spell in C's exact
    /// order and stops at the first one that succeeds, matching C's
    /// `if (...) return;` chain. No-op if `character_id` is not currently
    /// `CDR_LOSTCON`, already mid-action (including from a successful
    /// precascade step or attack cascade this same tick), or dead.
    pub fn process_lostcon_self_care_postcascade(
        &mut self,
        character_id: CharacterId,
        suppressions: LostconSelfCareSuppressions,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_LOSTCON
            || character.action != 0
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }
        let weather_movement_percent = self.settings.weather_movement_percent;
        let current_tick = self.tick.0 as u32;

        if character_value_base(&character, CharacterValue::Bless) != 0
            && character.mana >= BLESS_COST
            && !suppressions.nobless
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_bless(
                    caster,
                    &character,
                    &self.items,
                    current_tick,
                    None,
                    &self.map,
                    weather_movement_percent,
                )
                .is_ok()
                {
                    return true;
                }
            }
        }

        if character_value_base(&character, CharacterValue::MagicShield) * POWERSCALE
            > character.lifeshield
            && character.mana >= POWERSCALE * 3
            && !suppressions.noshield
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_magicshield(caster, &self.map, weather_movement_percent).is_ok() {
                    return true;
                }
            }
        }

        if character_value_base(&character, CharacterValue::Heal) != 0
            && character.hp < character_value_base(&character, CharacterValue::Hp) * POWERSCALE / 2
            && character.mana >= POWERSCALE * 3
            && !suppressions.noheal
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_heal(
                    caster,
                    &character,
                    None,
                    &self.map,
                    weather_movement_percent,
                )
                .is_ok()
                {
                    return true;
                }
            }
        }

        false
    }

    /// C `lostcon_driver`'s tail `do_idle(cn, TICKS)` (`lostcon.c:220`):
    /// the fallback when nothing else this tick (message loop excepted)
    /// took any action. No-op if `character_id` is not currently
    /// `CDR_LOSTCON`, already mid-action, or dead.
    pub fn queue_lostcon_idle(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if character.driver != CDR_LOSTCON || character.flags.contains(CharacterFlags::DEAD) {
            return false;
        }
        do_idle(character, TICKS_PER_SECOND as i32).is_ok()
    }
}

/// C `lostcon_driver`'s low-mana potion search (`lostcon.c:176-187`):
/// scans inventory slots 30.. in order for the first `IDR_POTION` item
/// with a mana component (`drdata[2]`); a combo potion (`drdata[1]` also
/// set) is gated by `nocombo`, a pure mana potion by `nomana`. Matches
/// C's outer gate (`!nolife || !nocombo`) verbatim even though it does not
/// mention `nomana` at all - callers apply that outer gate themselves
/// before calling this.
fn find_lostcon_mana_potion(
    character: &Character,
    items: &HashMap<ItemId, Item>,
    suppressions: LostconSelfCareSuppressions,
) -> Option<ItemId> {
    character
        .inventory
        .get(30..INVENTORY_SIZE)
        .unwrap_or_default()
        .iter()
        .flatten()
        .find_map(|item_id| {
            let item = items.get(item_id)?;
            if item.driver != IDR_POTION || crate::item_driver::drdata(item, 2) == 0 {
                return None;
            }
            if crate::item_driver::drdata(item, 1) != 0 {
                (!suppressions.nocombo).then_some(*item_id)
            } else {
                (!suppressions.nomana).then_some(*item_id)
            }
        })
}
