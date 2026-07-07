//! Session-side wiring for the newbie in-window hint system
//! (`ugaris_core::world::tutorial`): builds the per-player facts snapshot
//! `World::process_tutorial_hints` needs from `PlayerRuntime` and applies
//! the returned outcomes back. See that module's doc comment for the
//! full behavior and documented C deviations.

use super::*;

/// C `#define TF_TIMEOUT (60 * 60)` (`player_driver.c:373`), duplicated
/// here (rather than exported from `ugaris_core::world::tutorial`) only
/// for the one re-derivation `apply_tutorial_outcomes` needs - see its
/// `TutorialHintKind::Torch` arm.
const TUTORIAL_TF_TIMEOUT: u64 = 60 * 60;

pub(crate) fn tutorial_player_facts(
    runtime: &ServerRuntime,
    now: u64,
) -> HashMap<CharacterId, TutorialPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TutorialPlayerFacts {
                    hints_disabled: player.hints_disabled,
                    login_realtime_seconds: (player.login_tick / TICKS_PER_SECOND).min(now),
                    ppd: player.tutorial,
                    area1_lydia_state: player.area1_lydia_state(),
                    area1_lydia_seen_timer_realtime_seconds: player.area1_lydia_seen_timer().max(0)
                        as u64,
                },
            ))
        })
        .collect()
}

pub(crate) fn apply_tutorial_outcomes(
    runtime: &mut ServerRuntime,
    outcomes: Vec<TutorialOutcome>,
    now: u64,
) -> usize {
    let mut applied = 0;
    for outcome in outcomes {
        let Some(player) = runtime.player_for_character_mut(outcome.character_id) else {
            continue;
        };
        if let Some(citem_start) = outcome.citem_start {
            player.tutorial.citem_start_realtime_seconds = citem_start;
        }
        let Some(hint) = outcome.fired else {
            continue;
        };
        match hint {
            TutorialHintKind::Welcome => {
                player.tutorial.welcome_cnt += 1;
                player.tutorial.welcome_last_realtime_seconds = now;
            }
            TutorialHintKind::Lydia => {
                player.tutorial.lydia_cnt += 1;
                player.tutorial.lydia_last_realtime_seconds = now;
            }
            TutorialHintKind::Thief => {
                player.tutorial.thief_cnt += 1;
                player.tutorial.thief_last_realtime_seconds = now;
            }
            TutorialHintKind::Torch => {
                // C `player_driver.c:536-561`: the "create a torch"
                // sub-branches reset `torch_last`/`timer` unconditionally
                // but only bump `torch_cnt` if the usual gate had
                // separately elapsed - re-derive it from the pre-update
                // stamp still in `player.tutorial` (see
                // `world::tutorial`'s module doc comment).
                if now.saturating_sub(player.tutorial.torch_last_realtime_seconds)
                    > TUTORIAL_TF_TIMEOUT
                {
                    player.tutorial.torch_cnt += 1;
                }
                player.tutorial.torch_last_realtime_seconds = now;
            }
            TutorialHintKind::Battle => {
                player.tutorial.battle_cnt += 1;
                player.tutorial.battle_last_realtime_seconds = now;
            }
            TutorialHintKind::Battle2 => {
                player.tutorial.battle2_cnt += 1;
                player.tutorial.battle2_last_realtime_seconds = now;
            }
            TutorialHintKind::Shop => {
                player.tutorial.shop_cnt += 1;
                player.tutorial.shop_last_realtime_seconds = now;
            }
            TutorialHintKind::Chest => {
                player.tutorial.chest_cnt += 1;
                player.tutorial.chest_last_realtime_seconds = now;
            }
            TutorialHintKind::Citem => {
                player.tutorial.citem_cnt += 1;
                player.tutorial.citem_last_realtime_seconds = now;
            }
            TutorialHintKind::Raise => {
                player.tutorial.raise_cnt += 1;
                player.tutorial.raise_last_realtime_seconds = now;
            }
            TutorialHintKind::Potion => {
                player.tutorial.potion_cnt += 1;
                player.tutorial.potion_last_realtime_seconds = now;
            }
            TutorialHintKind::Shift => {
                player.tutorial.shift_cnt += 1;
                player.tutorial.shift_last_realtime_seconds = now;
            }
            TutorialHintKind::Ctrl => {
                player.tutorial.ctrl_cnt += 1;
                player.tutorial.ctrl_last_realtime_seconds = now;
            }
            TutorialHintKind::Left => {
                player.tutorial.left_cnt += 1;
                player.tutorial.left_last_realtime_seconds = now;
            }
            TutorialHintKind::Chat => {
                player.tutorial.chat_cnt += 1;
                player.tutorial.chat_last_realtime_seconds = now;
            }
            TutorialHintKind::Chat2 => {
                player.tutorial.chat2_cnt += 1;
                player.tutorial.chat2_last_realtime_seconds = now;
            }
            TutorialHintKind::Raise2 => {
                player.tutorial.raise2_cnt += 1;
                player.tutorial.raise2_last_realtime_seconds = now;
            }
        }
        player.tutorial.timer_realtime_seconds = now;
        applied += 1;
    }
    applied
}
