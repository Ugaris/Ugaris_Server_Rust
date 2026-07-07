//! `tutorial::apply_tutorial_outcomes`/`tutorial_player_facts` (the
//! `PlayerRuntime`-side wiring for `ugaris_core::world::tutorial`).

use super::*;
use crate::tutorial::{apply_tutorial_outcomes, tutorial_player_facts};

fn connected_player(session_id: u64, character_id: CharacterId) -> PlayerRuntime {
    let mut player = PlayerRuntime::connected(session_id, 0);
    player.character_id = Some(character_id);
    player.character_number = character_id.0;
    player
}

#[test]
fn tutorial_player_facts_snapshots_hints_and_area1_lydia_state() {
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(1);
    let mut player = connected_player(1, character_id);
    player.hints_disabled = true;
    player.set_area1_lydia_state(4);
    runtime.players.insert(1, player);

    let facts = tutorial_player_facts(&runtime, 1000);
    let player_facts = facts.get(&character_id).expect("facts present");
    assert!(player_facts.hints_disabled);
    assert_eq!(player_facts.area1_lydia_state, 4);
}

#[test]
fn apply_tutorial_outcomes_bumps_the_matching_counter_and_resets_the_timer() {
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(1);
    runtime.players.insert(1, connected_player(1, character_id));

    let applied = apply_tutorial_outcomes(
        &mut runtime,
        vec![TutorialOutcome {
            character_id,
            fired: Some(TutorialHintKind::Welcome),
            citem_start: None,
        }],
        4000,
    );
    assert_eq!(applied, 1);
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.tutorial.welcome_cnt, 1);
    assert_eq!(player.tutorial.welcome_last_realtime_seconds, 4000);
    assert_eq!(player.tutorial.timer_realtime_seconds, 4000);
}

#[test]
fn apply_tutorial_outcomes_only_bumps_torch_cnt_when_the_gate_had_elapsed() {
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(1);
    let mut player = connected_player(1, character_id);
    player.tutorial.torch_last_realtime_seconds = 3990;
    runtime.players.insert(1, player);

    // C `player_driver.c:536-561`'s "create a torch" sub-branches always
    // reset `torch_last`/`timer`, but only bump `torch_cnt` if the usual
    // one-hour gate had separately elapsed since the last stamp - here it
    // hasn't (10 seconds), so `torch_cnt` must stay put.
    let applied = apply_tutorial_outcomes(
        &mut runtime,
        vec![TutorialOutcome {
            character_id,
            fired: Some(TutorialHintKind::Torch),
            citem_start: None,
        }],
        4000,
    );
    assert_eq!(applied, 1);
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.tutorial.torch_cnt, 0);
    assert_eq!(player.tutorial.torch_last_realtime_seconds, 4000);
    assert_eq!(player.tutorial.timer_realtime_seconds, 4000);

    // A later call, an hour-plus after the last stamp, does bump it.
    let applied = apply_tutorial_outcomes(
        &mut runtime,
        vec![TutorialOutcome {
            character_id,
            fired: Some(TutorialHintKind::Torch),
            citem_start: None,
        }],
        4000 + 3601,
    );
    assert_eq!(applied, 1);
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.tutorial.torch_cnt, 1);
}

#[test]
fn apply_tutorial_outcomes_applies_citem_start_even_without_a_fired_hint() {
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(1);
    runtime.players.insert(1, connected_player(1, character_id));

    let applied = apply_tutorial_outcomes(
        &mut runtime,
        vec![TutorialOutcome {
            character_id,
            fired: None,
            citem_start: Some(1234),
        }],
        4000,
    );
    assert_eq!(applied, 0);
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.tutorial.citem_start_realtime_seconds, 1234);
    // No hint fired, so the outer `ppd->timer` throttle is untouched.
    assert_eq!(player.tutorial.timer_realtime_seconds, 0);
}
