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
//! [`World::macro_banish_to_challenge_room`]/[`World::
//! macro_restore_original_respawn`] plus [`MacroCrossAreaTransfer`]/
//! [`World::queue_macro_cross_area_transfer`]/[`World::
//! drain_pending_macro_cross_area_transfers`] below are the `World`-side
//! half of the cross-server "challenge room" teleport-and-restore flow
//! (`base.c:1054-1123`'s banishment, `840-891`'s return trip): the
//! `Character`-side mutations (respawn fields, `CF_RESPAWN`, position)
//! and the cross-area hand-off queue, mirroring `world/jail.rs`'s
//! `JailCrossAreaTransfer` shape exactly (`World` has no DB handle or
//! `ServerRuntime` of its own, so the actual `attempt_cross_area_transfer`
//! call happens in `ugaris-server`'s `world_events.rs::
//! apply_macro_cross_area_transfers`). The `MacroPpd`-side halves of both
//! the banishment and the return trip (`macro_begin_challenge_room_
//! banishment`/`macro_save_pentagram_progress` from [`crate::
//! macro_daemon`], plus the `in_challenge_room`/`needs_challenge` reads
//! and the `original_*` restore) are applied directly in
//! `ugaris-server/src/macro_daemon.rs`, which is the only place with
//! both a `World` and a `PlayerRuntime` in hand.
//!
//! Known, disclosed simplifications carried over from `macro_daemon.rs`'s
//! own module doc comment (not resolved by this slice either): the
//! reward item grants' `ZoneLoader` dependency and the `isxmas` reskin
//! field. Two further simplifications specific to this slice: (1) C's
//! `realtime - ch[co].login_time < 60*5` recent-login grace period is not
//! applied - this codebase does not yet track a player's login timestamp
//! anywhere on `Character`/`World`/`PlayerRuntime`, so a just-logged-in
//! player is not given the 5-minute grace period C grants before being
//! eligible for a challenge; (2) `macro_teleport_char_driver`'s
//! `MF_SOUNDBLOCK`/`MF_SHOUTBLOCK` destination-tile pre-check (`base.c:
//! 417-421`) is skipped, reusing the plain [`World::teleport_char_driver`]
//! instead - both are narrow, rarely-hit edge cases, not central to
//! anti-macro gameplay.

use crate::character_driver::CDR_MACRO;
use crate::world::*;

pub use crate::macro_daemon::{
    macro_apply_correct_answer, macro_apply_failure, macro_ask_challenge_lines,
    macro_begin_challenge_room_banishment, macro_check_answer, macro_generate_challenge,
    macro_is_area_excluded, macro_is_pents_area, macro_is_player_active, macro_next_check_delay,
    macro_record_history, macro_reward_fallback, macro_reward_item_template,
    macro_reward_success_message, macro_roll_reward, macro_save_pentagram_progress,
    macro_should_banish_to_challenge_room, macro_xmas_reward_message, MacroChallenge,
    MacroFailureUpdate, MacroReward, CHALLENGE_ROOM_AREA, CHALLENGE_ROOM_X, CHALLENGE_ROOM_Y,
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

/// A macro-daemon-triggered "challenge room" hand-off whose destination
/// area differs from this area server's own `area_id` - queued for
/// `ugaris-server`'s `world_events.rs::apply_macro_cross_area_transfers`
/// since `World` has no DB handle or `ServerRuntime` to perform the
/// `change_area` hand-off itself, same shape as `world/jail.rs`'s
/// `JailCrossAreaTransfer`. Used for both directions of the trip: the
/// initial banishment (`target_area/x/y` = the challenge room) and the
/// correct-answer return (`target_area/x/y` = `MacroPpd::original_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroCrossAreaTransfer {
    pub character_id: CharacterId,
    pub target_area: u16,
    pub target_x: u16,
    pub target_y: u16,
}

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

    /// C `macro_driver`'s `MACRO_STATE_FOUND` banishment's `Character`
    /// half (`base.c:1060-1069`): stashes the victim's current position
    /// and respawn point, then points their respawn at the challenge room
    /// and flags `CF_RESPAWN` - unconditionally, matching C's own
    /// unconditional field writes that happen before the local-vs-
    /// cross-server branch. Returns `(original_x, original_y,
    /// original_restx, original_resty, original_resta)` for the caller to
    /// mirror into `MacroPpd` (`macro_begin_challenge_room_banishment`,
    /// `ugaris-server`'s job - `World` cannot reach `PlayerRuntime`); this
    /// area server's own `area_id` is `original_area`, not returned here
    /// since the caller already has it. `None` if `victim_id` no longer
    /// resolves.
    pub fn macro_banish_to_challenge_room(
        &mut self,
        victim_id: CharacterId,
    ) -> Option<(i32, i32, i32, i32, i32)> {
        let character = self.characters.get_mut(&victim_id)?;
        let original = (
            i32::from(character.x),
            i32::from(character.y),
            i32::from(character.rest_x),
            i32::from(character.rest_y),
            i32::from(character.rest_area),
        );
        character.rest_x = CHALLENGE_ROOM_X;
        character.rest_y = CHALLENGE_ROOM_Y;
        character.rest_area = CHALLENGE_ROOM_AREA;
        character.flags.insert(CharacterFlags::RESPAWN);
        Some(original)
    }

    /// C `macro_driver`'s correct-answer return trip's respawn-point
    /// restore (`base.c:872-874`): `ch[co].restx/resty/resta =
    /// ppd->original_restx/resty/resta`. `CF_RESPAWN` is deliberately left
    /// set - C never clears it here either. A no-op if `victim_id` no
    /// longer resolves.
    pub fn macro_restore_original_respawn(
        &mut self,
        victim_id: CharacterId,
        original_restx: i32,
        original_resty: i32,
        original_resta: i32,
    ) {
        if let Some(character) = self.characters.get_mut(&victim_id) {
            character.rest_x = original_restx as u16;
            character.rest_y = original_resty as u16;
            character.rest_area = original_resta as u16;
        }
    }

    /// Queues a cross-server "challenge room" hand-off - see
    /// [`MacroCrossAreaTransfer`].
    pub fn queue_macro_cross_area_transfer(
        &mut self,
        character_id: CharacterId,
        target_area: u16,
        target_x: u16,
        target_y: u16,
    ) {
        self.pending_macro_cross_area_transfers
            .push(MacroCrossAreaTransfer {
                character_id,
                target_area,
                target_x,
                target_y,
            });
    }

    /// Drains every cross-server "challenge room" hand-off queued this
    /// tick - see [`MacroCrossAreaTransfer`].
    pub fn drain_pending_macro_cross_area_transfers(&mut self) -> Vec<MacroCrossAreaTransfer> {
        self.pending_macro_cross_area_transfers.drain(..).collect()
    }
}

#[cfg(test)]
pub(crate) const MACRO_MUTTERINGS_FOR_TESTS: [&str; 12] = MACRO_MUTTERINGS;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `MACRO_STATE_*` (`base.c:263-268`): the `CDR_MACRO` "Macro Daemon"
/// anti-bot NPC's own state machine, driving [`MacroDriverData`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MacroDriverState {
    /// `MACRO_STATE_IDLE` (`0`): looking for a victim.
    #[default]
    Idle,
    /// `MACRO_STATE_FOUND` (`1`): found a victim, preparing.
    Found,
    /// `MACRO_STATE_TELEPORTED` (`2`): teleported to the victim.
    Teleported,
    /// `MACRO_STATE_CHALLENGING` (`3`): asking the challenge.
    Challenging,
    /// `MACRO_STATE_TIMEOUT` (`4`): time ran out.
    Timeout,
}

/// C `struct macro_data` (`base.c:242-254`): the `CDR_MACRO` NPC's own
/// per-victim state. C's `victim`/`v_ID` pair (a `cn` array index plus its
/// `ch[].ID` generation check, guarding against the slot being recycled by
/// a different character between ticks) collapses to a single
/// [`CharacterId`] here, since this codebase's `CharacterId` is already
/// the stable, non-recycled identity every other ported NPC driver
/// compares directly (see e.g. `World::dungeonmaster_handle_char_message`'s
/// `speaker_id == dungeonmaster_id` check) - a stale `victim` simply stops
/// resolving via `World::characters.get`, which every consumer already
/// treats as "victim is gone, advance". C's six loose challenge fields
/// (`challenge_type`/`val1`/`val2`/`challenge_word`/`expected_answer`/
/// `choice_answer`) fold into a single [`crate::macro_daemon::
/// MacroChallenge`] (already ported whole, see that module).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MacroDriverData {
    pub state: MacroDriverState,
    pub victim: Option<CharacterId>,
    /// C `dat->victim`'s *other* role: while `state ==
    /// MacroDriverState::Idle`, the next victim search resumes from this
    /// `CharacterId.0` value (C's `for (co = dat->victim; ...)`
    /// continuation) - split into its own field since Rust's `victim`
    /// above is `None` exactly when there is no *current* target, whereas
    /// C's single `int victim` always holds a meaningful value in both
    /// roles at once.
    pub search_cursor: u32,
    /// C `start` (`ticker` when the current challenge began).
    pub start: u64,
    /// C `last` (`ticker` of the last time the challenge was (re-)asked).
    pub last: u64,
    pub challenge: Option<crate::macro_daemon::MacroChallenge>,
    pub teleported_to_jail: bool,
}
