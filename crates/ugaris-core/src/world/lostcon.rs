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
//! lingering (heal/potion/magicshield/fight-back-visible-enemy/idle) is not
//! yet ported; a lingering character is attackable and takes/deals damage
//! like any other character already in the world, but will not proactively
//! heal, drink potions, or fight back on its own yet.

use super::*;

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
}
