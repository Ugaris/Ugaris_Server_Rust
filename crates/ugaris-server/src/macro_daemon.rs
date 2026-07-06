//! Server-side wiring for the anti-macro/anti-bot "Macro Daemon" activity
//! tracker (`ugaris-core`'s `macro_daemon` module, `src/module/base.c`'s
//! `macro_track_exp_gain`/`macro_track_combat`/`macro_track_gold_change`,
//! `src/system/tool.c:385-426` + `death.c:1112-1117`).
//!
//! [`apply_macro_activity_events`] drains the three `World` queues
//! `pending_exp_gain_events`/`pending_combat_events`/
//! `pending_gold_change_events` (see their doc comments on `ugaris-core`'s
//! `world/mod.rs`) and stamps the matching `MacroPpd::last_exp_gain`/
//! `last_combat`/`last_gold_change` field on each character's
//! `PlayerRuntime`, mirroring `apply_bank_events`'s `World`/
//! `PlayerRuntime` split - `World` cannot reach `PlayerRuntime` directly,
//! only `ugaris-server` (which owns `ServerRuntime`) can.
//!
//! This is only the activity-tracking slice of the Macro Daemon system;
//! see `ugaris-core/src/macro_daemon.rs`'s module doc comment for the full
//! list of remaining gaps (the NPC-side state machine, the challenge
//! room cross-server hand-off, the reward item grants, and the `isxmas`
//! reskin are not part of this slice). Deliberately named
//! `apply_macro_activity_events`, not `apply_macro_events`: a future
//! iteration porting the live `CDR_MACRO` NPC driver itself will need its
//! own, larger `World`+`ServerRuntime` bridge (challenge asking/
//! checking, reward grants, the challenge-room hand-off) that the
//! `PORTING_TODO.md` task's REMAINING note calls `apply_macro_events` -
//! keep that name free for that bridge instead of colliding with this
//! narrower activity-tracking one.

use super::*;

/// Drains `World`'s `pending_exp_gain_events`/`pending_combat_events`/
/// `pending_gold_change_events` queues and stamps the matching
/// `MacroPpd::last_exp_gain`/`last_combat`/`last_gold_change` field (each
/// to `now`) on every character with a live `PlayerRuntime`. A no-op for
/// any `CharacterId` with no online `PlayerRuntime` (matching C's own
/// `ppd = set_data(...)` "no PPD, do nothing" guard - a character with no
/// session simply has no `MacroPpd` to update). Returns how many of the
/// three counters were actually applied (for the same "log iff nonzero"
/// convention `apply_bank_events`/`apply_military_master_events` use).
pub(crate) fn apply_macro_activity_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    now: i64,
) -> usize {
    let mut applied = 0;

    for character_id in world.drain_exp_gain_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_exp_gain = now;
            applied += 1;
        }
    }

    for character_id in world.drain_combat_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_combat = now;
            applied += 1;
        }
    }

    for character_id in world.drain_gold_change_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_gold_change = now;
            applied += 1;
        }
    }

    applied
}
