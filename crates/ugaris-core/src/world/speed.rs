//! `CL_SPEED` (`src/system/player.c::cl_speed`).
//!
//! `cl_fightmode` (`CL_FIGHTMODE`) is a genuine no-op in this C build - its
//! body is just `return;` and `ch[cn].fight_mode` is otherwise unused
//! anywhere in the C tree - so there is no behavior to port for it; the
//! server only needs to consume the packet without acting on it (see the
//! `ClientAction::FightMode` match arm in `ugaris-server`).

use super::*;

impl World {
    /// Applies a `CL_SPEED` packet.
    ///
    /// C `cl_speed`:
    /// ```c
    /// mode = *(unsigned char *)(buf + 0);
    /// if (mode != SM_NORMAL && mode != SM_FAST && mode != SM_STEALTH) return;
    /// if (mode == SM_FAST && ch[cn].endurance < POWERSCALE) return;
    /// ch[cn].speed_mode = mode;
    /// ```
    /// Returns `true` if the mode was applied (matches C's silent-ignore
    /// semantics on invalid mode byte or insufficient endurance for fast
    /// mode; there is no client feedback in either case).
    pub fn set_speed_mode(&mut self, character_id: CharacterId, mode: u8) -> bool {
        let Some(mode) = SpeedMode::from_client_mode(mode) else {
            return false;
        };
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if mode == SpeedMode::Fast && character.endurance < POWERSCALE {
            return false;
        }
        character.speed_mode = mode;
        true
    }
}
