//! `CL_RAISE` skill raising (C `cl_raise` in `src/system/player.c`, which
//! calls `raise_value` in `src/system/skill.c`).

use super::*;

/// Outcome of a `CL_RAISE` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaiseSkillOutcome {
    /// The skill's bare value was raised by 1; `exp_used` was spent from
    /// already-earned, unspent exp (`character.exp` itself is unchanged).
    Raised {
        value: usize,
        bare: i16,
        effective: i16,
        exp: u32,
        exp_used: u32,
    },
    /// C `raise_value` returned 0: out-of-range/unraisable value, skill not
    /// present, already at `skillmax`, insufficient unspent exp, or
    /// `CF_NOEXP`. `cl_raise` sends no client feedback on this path, so
    /// callers should stay silent too.
    Blocked,
}

impl World {
    /// Applies a `CL_RAISE` packet: raise `value` by 1 for `character_id`.
    ///
    /// C: `cl_raise(nr, buf)` reads a little-endian `unsigned short` value
    /// index directly out of the packet and calls `raise_value(cn, n)`,
    /// discarding the return value (no feedback packet on failure).
    pub fn raise_skill(&mut self, character_id: CharacterId, value: u16) -> RaiseSkillOutcome {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return RaiseSkillOutcome::Blocked;
        };
        let value = value as usize;
        let raised = crate::item_driver::raise_value(character, value).is_some();
        if !raised {
            return RaiseSkillOutcome::Blocked;
        }
        // C `raise_value` (`src/system/skill.c:256`): `update_char(cn)`
        // right after bumping `value[1]`, so derived bonuses (e.g. Body
        // Control's armor/weapon boost) apply immediately.
        self.update_character(character_id);
        let character = self.characters.get(&character_id).expect("checked above");
        RaiseSkillOutcome::Raised {
            value,
            bare: character.values[1][value],
            effective: character.values[0][value],
            exp: character.exp,
            exp_used: character.exp_used,
        }
    }
}
