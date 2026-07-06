//! `CDR_MACRO` "Macro Daemon" anti-bot NPC: the `World`-side (no
//! `PlayerRuntime` needed) slice of `src/module/base.c`'s `macro_driver`
//! (`base.c:802-1235`) - appearance reskin, the non-PPD half of the
//! `MACRO_STATE_IDLE` victim-candidate filter, the `NT_GIVE` message
//! branch, and idle mutterings.
//!
//! Everything else `macro_driver` does touches a player's persistent
//! `DRD_MACRO_PPD` (`crate::player::MacroPpd`), which lives on
//! `PlayerRuntime`, outside `World`'s visibility (same constraint
//! `world/military.rs`/`world/bank.rs` document for their own PPD-backed
//! drivers) - the full state machine (victim search's PPD filter,
//! `MACRO_STATE_FOUND`/`TELEPORTED`/`CHALLENGING`/`TIMEOUT`, the `NT_TEXT`
//! answer-checking branch, and reward granting) is therefore wired
//! directly in `ugaris-server/src/macro_daemon.rs`'s `apply_macro_events`
//! (mirroring `apply_military_master_events`'s `World`+`ServerRuntime`
//! split), reusing [`crate::macro_daemon`]'s already-ported pure "brain"
//! plus [`World::macro_search_candidates`]/[`World::macro_update_appearance`]/
//! [`World::macro_handle_give_message`]/[`World::macro_idle_mutter`] below
//! for the parts that don't need it.
//!
//! Known, disclosed simplifications carried over from `macro_daemon.rs`'s
//! own module doc comment (not resolved by this slice either): the
//! `force_summon`/cross-server "challenge room" pickup branches of
//! `MACRO_STATE_IDLE`, `MACRO_STATE_FOUND`'s suspicion/failure-triggered
//! challenge-room banishment, the reward item grants' `ZoneLoader`
//! dependency, and the `isxmas`/pentagram-restore fields. Two further
//! simplifications specific to this slice: (1) C's `realtime - ch[co].
//! login_time < 60*5` recent-login grace period is not applied - this
//! codebase does not yet track a player's login timestamp anywhere on
//! `Character`/`World`/`PlayerRuntime`, so a just-logged-in player is not
//! given the 5-minute grace period C grants before being eligible for a
//! challenge; (2) `macro_teleport_char_driver`'s `MF_SOUNDBLOCK`/
//! `MF_SHOUTBLOCK` destination-tile pre-check (`base.c:417-421`) is
//! skipped, reusing the plain [`World::teleport_char_driver`] instead -
//! both are narrow, rarely-hit edge cases, not central to anti-macro
//! gameplay.

use super::*;
use crate::character_driver::CDR_MACRO;

pub use crate::character_driver::{MacroDriverData, MacroDriverState};
pub use crate::macro_daemon::{
    macro_apply_correct_answer, macro_apply_failure, macro_ask_challenge_lines, macro_check_answer,
    macro_generate_challenge, macro_is_area_excluded, macro_is_player_active,
    macro_next_check_delay, macro_record_history, macro_reward_fallback,
    macro_reward_item_template, macro_reward_success_message, macro_roll_reward,
    macro_xmas_reward_message, MacroChallenge, MacroFailureUpdate, MacroReward,
    MACRO_ACTIVITY_TIMEOUT, MACRO_CHALLENGE_TIME, MACRO_REPEAT_INTERVAL,
};

/// C `static const char *macro_mutterings[]` (`base.c:1217-1230`).
const MACRO_MUTTERINGS: [&str; 12] = [
    "Another day of keeping things fair...",
    "I wonder if anyone actually likes seeing me...",
    "To teleport or not to teleport... that is the question.",
    "I can see everything. EVERYTHING. Well, mostly.",
    "Math problems for everyone! Education is a gift!",
    "Some say I'm annoying. I prefer 'diligently persistent'.",
    "The shadows whisper of bots. I listen.",
    "Twenty-four plus seventeen? I'll never tell.",
    "Lurking, watching, waiting... it's a living.",
    "They run when they see me. I try not to take it personally.",
    "Is it paranoia if they really ARE scripting?",
    "I should get a cape. Every daemon deserves a cape.",
];

impl World {
    /// C `macro_driver`'s appearance-update block (`base.c:912-921`).
    /// `is_xmas` mirrors the C global `isxmas`, which `World` has no
    /// awareness of on its own (see `ugaris-server/src/xmas.rs`).
    pub fn macro_update_appearance(&mut self, macro_id: CharacterId, is_xmas: bool) {
        let Some(character) = self.characters.get_mut(&macro_id) else {
            return;
        };
        if is_xmas {
            character.name = "Saint Nick".to_string();
            character.description =
                "A jolly fellow in red, here to spread cheer and check on adventurers.".to_string();
            character.sprite = 13;
        } else {
            character.name = "Macro Daemon".to_string();
            character.description =
                "A friendly guardian who ensures adventurers are playing fairly.".to_string();
            character.sprite = 161;
        }
    }

    /// Every character with a live `CDR_MACRO` driver (used, not dead).
    pub fn macro_daemon_ids(&self) -> Vec<CharacterId> {
        self.characters
            .values()
            .filter(|character| {
                character.driver == CDR_MACRO
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `macro_driver`'s `MACRO_STATE_IDLE` victim-candidate filter
    /// (`base.c:963-993`), minus the PPD-dependent `immune_until`/
    /// `nextcheck`/`macro_is_player_active` checks (`ugaris-server`'s job,
    /// see this module's doc comment) and minus C's recent-login grace
    /// period (also documented there). Returns every eligible candidate
    /// id `>= from`, ascending, matching C's `for (co = dat->victim; co <
    /// MAXCHARS; co++)` continuation; the caller applies the PPD filter
    /// in the same order and stops at the first fully-eligible one.
    pub fn macro_search_candidates(&self, area_id: u16, from: u32) -> Vec<CharacterId> {
        let mut ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| character.id.0 >= from)
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .filter(|character| character.level >= 9)
            .filter(|character| {
                !character.flags.intersects(
                    CharacterFlags::INVISIBLE | CharacterFlags::STAFF | CharacterFlags::GOD,
                )
            })
            .filter(|character| {
                !macro_is_area_excluded(area_id, usize::from(character.x), usize::from(character.y))
            })
            .map(|character| character.id)
            .collect();
        ids.sort_by_key(|id| id.0);
        ids
    }

    /// C `macro_driver`'s `NT_GIVE` branch (`base.c:902-907`): any gift is
    /// unconditionally destroyed.
    pub fn macro_handle_give_message(&mut self, macro_id: CharacterId) {
        if let Some(item_id) = self
            .characters
            .get_mut(&macro_id)
            .and_then(|character| character.cursor_item.take())
        {
            self.destroy_item(item_id);
        }
    }

    /// C `macro_driver`'s idle mutterings (`base.c:1216-1231`): `1/200`
    /// odds per tick while idle. Threads `self.legacy_random_seed`
    /// internally so `ugaris-server` callers don't need to manage the
    /// round-trip themselves.
    pub fn macro_idle_mutter(&mut self, macro_id: CharacterId) {
        let mut seed = self.legacy_random_seed;
        let fires = legacy_random_below_from_seed(&mut seed, 200) == 0;
        let idx = legacy_random_below_from_seed(&mut seed, MACRO_MUTTERINGS.len() as u32) as usize;
        self.legacy_random_seed = seed;
        if fires {
            self.npc_murmur(macro_id, MACRO_MUTTERINGS[idx]);
        }
    }

    /// Threads `self.legacy_random_seed` through [`macro_generate_challenge`].
    pub fn macro_roll_challenge(
        &mut self,
        suspicion: i32,
        challenge_failures: i32,
    ) -> MacroChallenge {
        let mut seed = self.legacy_random_seed;
        let challenge = macro_generate_challenge(&mut seed, suspicion, challenge_failures);
        self.legacy_random_seed = seed;
        challenge
    }

    /// Threads `self.legacy_random_seed` through
    /// [`macro_apply_correct_answer`] - see that function's doc comment.
    /// `ppd` lives on `PlayerRuntime`, entirely independent of `World`, so
    /// this takes it by separate `&mut` reference (same shape as every
    /// other `World`-method-plus-borrowed-`PlayerRuntime`-field call this
    /// codebase already uses, e.g. `ugaris-server/src/military.rs`).
    pub fn macro_apply_correct_answer_seeded(
        &mut self,
        ppd: &mut crate::player::MacroPpd,
        now: i64,
        response_time: i32,
        challenge_type: i32,
    ) {
        let mut seed = self.legacy_random_seed;
        macro_apply_correct_answer(ppd, now, response_time, challenge_type, &mut seed);
        self.legacy_random_seed = seed;
    }

    /// Threads `self.legacy_random_seed` through [`macro_apply_failure`].
    pub fn macro_apply_failure_seeded(
        &mut self,
        ppd: &mut crate::player::MacroPpd,
        victim_name: &str,
        now: i64,
        challenge_type: i32,
    ) -> MacroFailureUpdate {
        let mut seed = self.legacy_random_seed;
        let update = macro_apply_failure(ppd, victim_name, now, challenge_type, &mut seed);
        self.legacy_random_seed = seed;
        update
    }

    /// C `macro_give_reward`'s `RANDOM(100)` reward-type roll
    /// (`base.c:664`).
    pub fn macro_roll_reward_type(&mut self) -> i32 {
        let mut seed = self.legacy_random_seed;
        let roll = legacy_random_below_from_seed(&mut seed, 100) as i32;
        self.legacy_random_seed = seed;
        roll
    }

    /// C `macro_give_reward`'s gold roll (`base.c:689`): `base +
    /// RANDOM(random_span)`.
    pub fn macro_roll_gold_reward(&mut self, base: u32, random_span: u32) -> u32 {
        let mut seed = self.legacy_random_seed;
        let roll = legacy_random_below_from_seed(&mut seed, random_span.max(1));
        self.legacy_random_seed = seed;
        base + roll
    }
}

#[cfg(test)]
pub(crate) const MACRO_MUTTERINGS_FOR_TESTS: [&str; 12] = MACRO_MUTTERINGS;
