//! Gladiator student NPC (`CDR_GLADIATOR`), the disposable opponent
//! `fight_student` (`world::npc::area37::fiona`) spawns from a
//! `"Gladiator_<1..=10>"` template each time a player says "enter" during
//! Fiona's student-challenge chain.
//!
//! Ports `src/area/37/arkhata.c::gladiator_driver` (`:1081-1163`) and
//! `gladiator_dead` (`:1176-1178`). Like `world::npc::area25::warpfighter`,
//! this reproduces `standard_message_driver(cn, msg, 1, 1)`'s three
//! outcomes (`NT_CHAR` auto-aggro, `NT_SEEHIT` ally-help, `NT_GOTHIT`
//! self-defense) directly, since `CDR_GLADIATOR` is not a
//! `CharacterDriverState::SimpleBaddy`.
//!
//! `gladiator_dead` is a `notify_area` broadcast, not a `LegacyHurtEvent`-
//! keyed death hook (unlike most of this codebase's `*_dead` family) - it
//! needs no killer-lookup gate beyond "is the killer a player", so it is
//! ported as a plain `World` method called directly from the death-hurt
//! pipeline the same place other `notify_area`-based death reactions are,
//! rather than through `world_events::death_hooks`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `if (ticker - dat->last_talk > TICKS*60*3)` 3-minute self-destruct
//!   (`:1136-1150`) also teleports every player standing inside the arena
//!   bounding box back to `(15, 235)` before destroying itself - ported
//!   verbatim via a plain linear scan (no sector index in this port, same
//!   precedent as every other bounding-box scan in this codebase).
//! - `misc` (declared on C's shared `struct std_npc_driver_data`) is never
//!   read or written by `gladiator_driver` itself - no field for it here,
//!   same "only port fields the driver actually uses" precedent as
//!   `world::npc::area37::rammy`'s `RammyDriverData` doc comment.

use crate::world::*;

/// C `TICKS * 60 * 3` (`arkhata.c:1136`): the 3-minute self-destruct
/// timeout.
const GLADIATOR_SELF_DESTRUCT_TICKS: u64 = TICKS_PER_SECOND * 60 * 3;
/// C `for (x=9;x<=24;x++) for(y=238;y<=252;y++)` (`arkhata.c:1139-1140`):
/// the same Fighting School arena bounds `fight_student`'s own busy-check
/// scans (`world::npc::area37::fiona`).
const GLADIATOR_ARENA_X: std::ops::RangeInclusive<u16> = 9..=24;
const GLADIATOR_ARENA_Y: std::ops::RangeInclusive<u16> = 238..=252;
/// C `notify_area(15, 232, ...)` (`gladiator_dead`, `arkhata.c:1177`).
const GLADIATOR_DEAD_NOTIFY_X: u16 = 15;
const GLADIATOR_DEAD_NOTIFY_Y: u16 = 232;

impl World {
    /// C `ch_driver`'s `CDR_GLADIATOR` dispatch (`arkhata.c:4567-4569`).
    pub fn process_gladiator_actions(&mut self, area_id: u16) -> usize {
        let gladiator_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GLADIATOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for gladiator_id in gladiator_ids {
            let mut seed = self.legacy_random_seed;
            let did_act = {
                let mut random = |below: u32| legacy_random_below_from_seed(&mut seed, below);
                self.process_gladiator_tick(gladiator_id, area_id, &mut random)
            };
            self.legacy_random_seed = seed;
            if did_act {
                acted += 1;
            }
        }
        acted
    }

    /// C `gladiator_driver`'s per-tick body (`arkhata.c:1081-1163`).
    fn process_gladiator_tick(
        &mut self,
        gladiator_id: CharacterId,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&gladiator_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Gladiator(data)) => data,
            _ => GladiatorDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&gladiator_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `case NT_CREATE: dat->last_talk = ticker;`
                // (`arkhata.c:1096-1097`).
                NT_CREATE => {
                    data.last_talk = self.tick.0;
                }
                // C `case NT_TEXT: co = msg->dat3; tabunga(cn, co,
                // (char*)msg->dat2);` (`arkhata.c:1103-1105`).
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(gladiator_id, speaker_id, text);
                    }
                }
                // C `case NT_DEAD: co = msg->dat1; cc = msg->dat2; if
                // (cc==cn && (ch[co].flags & CF_PLAYER)) {
                // remove_destroy_char(cn); return; }` (`arkhata.c:1114-
                // 1119`): a witnessed death (not self) is a no-op here -
                // this driver's own self-destruct-on-death is handled by
                // its `gladiator_dead` death hook, not this branch.
                NT_DEAD => {
                    let victim_id = CharacterId(message.dat1.max(0) as u32);
                    let killer_id = CharacterId(message.dat2.max(0) as u32);
                    if killer_id == gladiator_id
                        && self
                            .characters
                            .get(&victim_id)
                            .is_some_and(|victim| victim.flags.contains(CharacterFlags::PLAYER))
                    {
                        self.remove_character(gladiator_id);
                        return true;
                    }
                }
                // C `standard_message_driver(cn, msg, 1, 1)`'s `NT_CHAR`
                // branch (`drvlib.c:2470-2476`).
                NT_CHAR if message.dat1 > 0 => {
                    let target_id = CharacterId(message.dat1 as u32);
                    self.gladiator_add_standard_enemy(gladiator_id, target_id, 0, true, false);
                }
                // C `standard_message_driver`'s `NT_SEEHIT` branch
                // (`drvlib.c:2478-2510`).
                NT_SEEHIT if message.dat1 > 0 && message.dat2 > 0 => {
                    self.gladiator_handle_seehit(gladiator_id, message);
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`).
                NT_GOTHIT if message.dat1 > 0 => {
                    let tick = self.tick.0 as i32;
                    if let Some(character) = self.characters.get_mut(&gladiator_id) {
                        character
                            .fight_driver
                            .get_or_insert_with(FightDriverData::default)
                            .last_hit = tick;
                    }
                    self.gladiator_handle_gothit(gladiator_id, message);
                }
                _ => {}
            }
        }

        if let Some(gladiator) = self.characters.get_mut(&gladiator_id) {
            gladiator.driver_state = Some(CharacterDriverState::Gladiator(data));
        }

        // C `if (ticker - dat->last_talk > TICKS*60*3) { ...
        // remove_destroy_char(cn); return; }` (`arkhata.c:1136-1150`).
        let last_talk = match self
            .characters
            .get(&gladiator_id)
            .and_then(|character| character.driver_state.as_ref())
        {
            Some(CharacterDriverState::Gladiator(data)) => data.last_talk,
            _ => 0,
        };
        if self.tick.0.saturating_sub(last_talk) > GLADIATOR_SELF_DESTRUCT_TICKS {
            self.npc_say(gladiator_id, "That's all folks!");
            let player_ids: Vec<CharacterId> = self
                .characters
                .values()
                .filter(|character| {
                    character.flags.contains(CharacterFlags::PLAYER)
                        && GLADIATOR_ARENA_X.contains(&character.x)
                        && GLADIATOR_ARENA_Y.contains(&character.y)
                })
                .map(|character| character.id)
                .collect();
            for player_id in player_ids {
                self.teleport_char_driver(player_id, 15, 235);
            }
            self.remove_character(gladiator_id);
            return true;
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`arkhata.c:1152-1158`).
        let Some(attacker) = self.characters.get(&gladiator_id).cloned() else {
            return false;
        };
        if self.fight_driver_attack_visible_and_follow(
            gladiator_id,
            &attacker,
            area_id,
            FightDriverSuppressions::default(),
            true,
            random,
        ) {
            return true;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`arkhata.c:1160-1162`).
        if self.regenerate_simple_baddy(gladiator_id) {
            return true;
        }
        if self.spell_self_simple_baddy(gladiator_id) {
            return true;
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT,
        // ret, lastact)` (`arkhata.c:1164-1166`): the gladiator's post
        // position (C's `tmpx`/`tmpy`, set to `(14, 244)` at creation)
        // reuses `rest_x`/`rest_y`, the same substitution every other
        // stationary NPC in this codebase uses.
        let (post_x, post_y) = self
            .characters
            .get(&gladiator_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            gladiator_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`arkhata.c:1168`).
        self.idle_simple_baddy(gladiator_id)
    }

    /// C `standard_message_driver`'s `NT_SEEHIT` branch (`drvlib.c:2478-
    /// 2510`): help a friend being attacked, or help a friend attacking.
    fn gladiator_handle_seehit(
        &mut self,
        gladiator_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        let victim_id = CharacterId(message.dat2.max(0) as u32);
        let Some(gladiator) = self.characters.get(&gladiator_id).cloned() else {
            return;
        };

        // C `if (co != cn && ch[co].group == ch[cn].group) { ...
        // fight_driver_add_enemy(cn, cc, 1, 1); break; }` (victim is our
        // friend: help against the attacker).
        if victim_id != gladiator_id
            && self
                .characters
                .get(&victim_id)
                .is_some_and(|victim| victim.group == gladiator.group)
        {
            self.gladiator_add_standard_enemy(gladiator_id, attacker_id, 1, true, true);
            return;
        }
        // C `if (cc != cn && ch[cc].group == ch[cn].group) { ...
        // fight_driver_add_enemy(cn, co, 0, 1); break; }` (attacker is our
        // friend: help against the victim).
        if attacker_id != gladiator_id
            && self
                .characters
                .get(&attacker_id)
                .is_some_and(|attacker| attacker.group == gladiator.group)
        {
            self.gladiator_add_standard_enemy(gladiator_id, victim_id, 0, true, false);
        }
    }

    /// C `standard_message_driver`'s `NT_GOTHIT` self-defense half
    /// (`drvlib.c:2523-2537`, `fight_driver_note_hit` already applied by
    /// the caller). C's own `char_see_char` check here only decides the
    /// *stored* initial `visible` flag (`0`/`1`), which `fight_driver_
    /// update`'s unconditional recompute overwrites again before it is
    /// ever read this same tick - so, like `world::npc::area25::
    /// warpfighter`'s identical `NT_GOTHIT` port, `require_visible` is
    /// simply `false` here (the enemy is added regardless of current
    /// line of sight).
    fn gladiator_handle_gothit(
        &mut self,
        gladiator_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        let Some(gladiator) = self.characters.get(&gladiator_id).cloned() else {
            return;
        };
        let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
            return;
        };
        if gladiator.group == attacker.group {
            return;
        }
        if !can_attack(&gladiator, &attacker, &self.map) {
            return;
        }
        self.gladiator_add_standard_enemy(gladiator_id, attacker_id, 1, false, true);
    }

    /// C `standard_message_driver`'s enemy-add half (`drvlib.c:2470-2476,
    /// 2496,2507,2519-2527`), shared by every branch above - same shape as
    /// `world::npc::area25::warpfighter`'s own `warpfighter_add_standard_
    /// enemy`, reimplemented directly here since `CDR_GLADIATOR` is not a
    /// `CharacterDriverState::SimpleBaddy`.
    fn gladiator_add_standard_enemy(
        &mut self,
        gladiator_id: CharacterId,
        target_id: CharacterId,
        priority: i32,
        require_visible: bool,
        hurtme: bool,
    ) {
        if !self.simple_baddy_can_add_standard_enemy(
            gladiator_id,
            target_id,
            require_visible,
            hurtme,
        ) {
            return;
        }
        let tick = self.tick.0 as i32;
        let tracking = self.simple_baddy_enemy_tracking(gladiator_id, target_id);
        if let Some(character) = self.characters.get_mut(&gladiator_id) {
            let _ = add_simple_baddy_enemy_unchecked(character, target_id, priority, tick);
            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
        }
        self.sort_simple_baddy_enemies_like_c(gladiator_id);
    }

    /// C `gladiator_dead(cn, co)` (`arkhata.c:1176-1178`): reports the
    /// killer back to any nearby Fiona (`world::npc::area37::fiona`'s
    /// `NT_NPC`/`NTID_GLADIATOR` handler) via a plain `notify_area`
    /// broadcast, not a `LegacyHurtEvent`-keyed death hook - see module doc
    /// comment.
    pub fn apply_gladiator_death(&mut self, gladiator_id: CharacterId, killer_id: CharacterId) {
        if killer_id.0 == 0
            || !self
                .characters
                .get(&killer_id)
                .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER))
        {
            return;
        }
        self.notify_area(
            GLADIATOR_DEAD_NOTIFY_X,
            GLADIATOR_DEAD_NOTIFY_Y,
            NT_NPC,
            NTID_GLADIATOR,
            gladiator_id.0 as i32,
            killer_id.0 as i32,
        );
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::{CDR_GLADIATOR, NTID_GLADIATOR};

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`): only `last_talk`
/// is read/written by `gladiator_driver` itself - no field for `misc`/
/// `current_victim` here, same "only port fields the driver actually
/// uses" precedent as `world::npc::area37::rammy`'s `RammyDriverData` doc
/// comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GladiatorDriverData {
    #[serde(default)]
    pub last_talk: u64,
}
