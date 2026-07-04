//! Server-side wiring for `ugaris-core`'s military mission-progress model
//! (`crate::world::MilitaryMissionKillCheck` / `PlayerRuntime::
//! check_military_solve`): drains the queue `World::kill_character_
//! followup` fills on every player kill and applies the resulting
//! `check_military_solve` (`src/system/death.c:290-383`) text feedback.

use super::*;
use ugaris_core::world::{
    military_mission_progress_message_should_display, MilitaryMissionKillCheck,
    MilitaryMissionProgress,
};

/// C `check_military_solve(co, cn)`'s killer-side (`co`, `check.killer_id`
/// here) mission-progress update, queued as a [`MilitaryMissionKillCheck`]
/// by `World::kill_character_followup` for every kill by a player
/// character. A no-op if the killer has no live `PlayerRuntime`, or if
/// [`ugaris_core::PlayerRuntime::check_military_solve`] reports
/// [`MilitaryMissionProgress::NoMatch`] (no active unsolved mission, or
/// the victim didn't match its type/class/level target).
pub(crate) fn apply_military_mission_kill_check(
    world: &mut World,
    runtime: &mut ServerRuntime,
    check: MilitaryMissionKillCheck,
) {
    let Some(player) = runtime.player_for_character_mut(check.killer_id) else {
        return;
    };
    let outcome = player.check_military_solve(check.victim_class, check.victim_level as i32);

    let message: Option<Vec<u8>> = match outcome {
        MilitaryMissionProgress::NoMatch => None,
        MilitaryMissionProgress::Progress {
            remaining,
            elite_count,
        } => {
            if !military_mission_progress_message_should_display(remaining) {
                None
            } else {
                let mut line = COL_DARK_GRAY.to_vec();
                if elite_count > 1 {
                    // C: `log_char(cn, LOG_SYSTEM, 0, COL_DARK_GRAY "Elite
                    // demon slain! Counts as %d. %d to go.", count_value,
                    // ppd->mis[nr].opt1)` (`death.c:343-344`).
                    line.extend_from_slice(
                        format!("Elite demon slain! Counts as {elite_count}. {remaining} to go.")
                            .as_bytes(),
                    );
                } else {
                    // C: `log_char(cn, LOG_SYSTEM, 0, COL_DARK_GRAY
                    // "Mission kill, %d to go.", ppd->mis[nr].opt1)`
                    // (`death.c:346` / `:371`).
                    line.extend_from_slice(format!("Mission kill, {remaining} to go.").as_bytes());
                }
                Some(line)
            }
        }
        MilitaryMissionProgress::Solved => {
            // C: `log_char(cn, LOG_SYSTEM, 0, "You solved your mission.
            // Talk to the governor to claim your reward.")` (`death.c:
            // 350-351` / `:374-375`) - no color prefix on this one.
            Some(b"You solved your mission. Talk to the governor to claim your reward.".to_vec())
        }
    };

    if let Some(message) = message {
        world.queue_system_text_bytes(check.killer_id, message);
    }
}
