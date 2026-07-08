use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_SWAMPCLARA, NT_CHAR, NT_GIVE};
use crate::world::npc::area3::clara::{ClaraDriverData, ClaraOutcomeEvent, ClaraPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn clara_npc(id: u32) -> Character {
    let mut clara = character(id);
    clara.name = "Clara".into();
    clara.driver = CDR_SWAMPCLARA;
    clara.driver_state = Some(CharacterDriverState::Clara(ClaraDriverData::default()));
    clara
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    clara_state: i32,
    kelly_state: i32,
    has_hardkill_item: bool,
    hardkill_ritual_progress: u8,
    questlog_21_count: i32,
) -> HashMap<CharacterId, ClaraPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ClaraPlayerFacts {
            clara_state,
            kelly_state,
            has_hardkill_item,
            hardkill_ritual_progress,
            questlog_21_count,
        },
    );
    map
}

fn clara_state(world: &World, clara_id: CharacterId) -> ClaraDriverData {
    match world
        .characters
        .get(&clara_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Clara(data)) => data,
        _ => panic!("expected clara driver state"),
    }
}

#[test]
fn clara_greets_new_player_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 0, 0, false, 0, 0), 15);
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I am Clara, First Sergeant of the Seyan'Du")));
    assert_eq!(
        clara_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn clara_state1_is_silent_no_op_when_kelly_state_below_15() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 1, 10, false, 0, 0), 15);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn clara_state1_falls_through_to_status_report_when_kelly_state_ge_15() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 1, 15, false, 0, 0), 15);
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("no longer secure")));
}

#[test]
fn clara_state5_opens_quest21_when_kelly_state_ge_18() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 5, 18, false, 0, 0), 15);
    assert!(events.contains(&ClaraOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("huge swamp beast")));
}

#[test]
fn clara_state9_with_hardkill_item_awards_military_points_and_exp() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.military_points = 0;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // questlog_21_count == 0 -> reward granted.
    let events = world.process_clara_actions(&facts(CharacterId(2), 9, 18, true, 5, 0), 15);
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points
            > 0
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("find all three stone circles")));
}

#[test]
fn clara_state9_hardkill_reward_skipped_when_quest_already_counted() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.military_points = 0;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // questlog_21_count != 0 -> no reward, but the state still advances.
    let events = world.process_clara_actions(&facts(CharacterId(2), 9, 18, true, 40, 1), 15);
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        0
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("So that is how one can kill them.")));
}

#[test]
fn clara_state14_completes_quest21_and_awards_bonus_on_first_completion() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.military_points = 0;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 14, 18, false, 0, 1), 15);
    assert!(events.contains(&ClaraOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ClaraOutcomeEvent::UpdateClaraState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
    assert!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points
            > 0
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Well done indeed, Godmode!")));
}

#[test]
fn clara_text_repeat_resets_state_to_bucket_start() {
    let cases: [(i32, i32); 5] = [(3, 0), (7, 6), (11, 10), (13, 12), (16, 15)];
    for (start_state, expected_reset) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(clara_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
            clara.driver_state = Some(CharacterDriverState::Clara(ClaraDriverData {
                last_talk: 12345,
                current_victim: None,
            }));
            clara.push_driver_text_message(CharacterId(2), "repeat");
        }

        let events =
            world.process_clara_actions(&facts(CharacterId(2), start_state, 0, false, 0, 0), 15);
        assert!(
            events.contains(&ClaraOutcomeEvent::UpdateClaraState {
                player_id: CharacterId(2),
                new_state: expected_reset,
            }),
            "start_state {start_state} should reset to {expected_reset}"
        );
        // C `dat->last_talk = 0;` inside the matching bucket.
        assert_eq!(clara_state(&world, CharacterId(1)).last_talk, 0);
    }
}

#[test]
fn clara_text_hello_replies_with_canned_greeting() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clara_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_text_message(CharacterId(2), "hello");
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 0, 0, false, 0, 0), 15);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
    assert_eq!(
        clara_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn clara_give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut clara = clara_npc(1);
    clara.cursor_item = Some(ItemId(50));
    world.add_character(clara);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(clara) = world.characters.get_mut(&CharacterId(1)) {
        clara.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_clara_actions(&facts(CharacterId(2), 0, 0, false, 0, 0), 15);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
