use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_TRAINER, NT_CHAR, NT_GIVE};
use crate::world::npc::area37::trainer::{
    TrainerDriverData, TrainerOutcomeEvent, TrainerPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn trainer_npc(id: u32) -> Character {
    let mut trainer = character(id);
    trainer.name = "Trainer".into();
    trainer.driver = CDR_TRAINER;
    trainer.driver_state = Some(CharacterDriverState::Trainer(TrainerDriverData::default()));
    trainer
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    trainer_state: i32,
    fiona_state: i32,
    kid_state: i32,
) -> HashMap<CharacterId, TrainerPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TrainerPlayerFacts {
            trainer_state,
            fiona_state,
            kid_state,
        },
    );
    map
}

#[test]
fn state0_without_fiona_progress_or_level_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_trainer_actions(&facts(CharacterId(2), 0, 3, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_level_and_fiona_progress_greets_opens_quest75_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 53;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_trainer_actions(&facts(CharacterId(2), 0, 4, 0), 1);
    assert!(events.contains(&TrainerOutcomeEvent::QuestOpen75 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&TrainerOutcomeEvent::UpdateTrainerState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("your loyalty is expected")));
}

#[test]
fn state6_without_kid_rescued_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_trainer_actions(&facts(CharacterId(2), 6, 4, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state6_with_kid_rescued_completes_quest75_and_collapses_to_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_trainer_actions(&facts(CharacterId(2), 6, 4, 5), 1);
    assert!(events.contains(&TrainerOutcomeEvent::QuestDone75 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&TrainerOutcomeEvent::UpdateTrainerState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("proven your skills once again")));
}

#[test]
fn state8_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_trainer_actions(&facts(CharacterId(2), 8, 4, 5), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_to_1_when_at_or_below_state6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_trainer_actions(&facts(CharacterId(2), 4, 4, 0), 1);
    assert!(events.contains(&TrainerOutcomeEvent::UpdateTrainerState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_resets_to_7_when_between_states_7_and_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trainer_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_trainer_actions(&facts(CharacterId(2), 8, 4, 5), 1);
    assert!(events.contains(&TrainerOutcomeEvent::UpdateTrainerState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut trainer = trainer_npc(1);
    trainer.cursor_item = Some(ItemId(50));
    world.add_character(trainer);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(trainer) = world.characters.get_mut(&CharacterId(1)) {
        trainer.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_trainer_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
