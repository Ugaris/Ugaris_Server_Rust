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
}
