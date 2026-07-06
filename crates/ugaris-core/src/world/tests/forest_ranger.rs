use std::collections::HashMap;

use super::*;
use crate::character_driver::{ForestRangerDriverData, CDR_FOREST_RANGER, NT_CHAR, NT_GIVE};
use crate::world::forest_ranger::{ForestRangerOutcomeEvent, ForestRangerPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn forest_ranger_npc(id: u32, level: u32) -> Character {
    let mut ranger = character(id);
    ranger.name = "Vert".into();
    ranger.driver = CDR_FOREST_RANGER;
    ranger.driver_state = Some(CharacterDriverState::ForestRanger(
        ForestRangerDriverData::default(),
    ));
    ranger.level = level;
    // Match the spawn tile used by every test in this module so the
    // "return to post" idle branch (`secure_move_driver` toward
    // `rest_x`/`rest_y`) is a no-op instead of relocating the NPC away
    // from the position the rest of the test asserts against - `World::
    // spawn_character` (unlike zone-file loading via `zone.rs`) never
    // seeds `rest_x`/`rest_y` from the spawn position on its own.
    ranger.rest_x = 10;
    ranger.rest_y = 10;
    ranger
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    seen_timer: i32,
) -> HashMap<CharacterId, ForestRangerPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, ForestRangerPlayerFacts { state, seen_timer });
    map
}

fn ranger_state(world: &World, ranger_id: CharacterId) -> ForestRangerDriverData {
    match world
        .characters
        .get(&ranger_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ForestRanger(data)) => data,
        _ => panic!("expected forest ranger driver state"),
    }
}

#[test]
fn entry_silently_advances_to_warning1_when_ranger_level_is_low() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    // Entry is a silent transition - no dialogue, no facing update.
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(ranger_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn entry_silently_advances_to_hint1_when_ranger_level_above_30() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 31), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn warning1_says_and_advances_to_warning2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 1, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Take heed of my warning")));
    assert_eq!(
        ranger_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn warning2_says_and_advances_to_greet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 2, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("dead wood")));
}

#[test]
fn hint1_says_and_advances_to_greet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 40), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 3, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("strange magic at work")));
}

#[test]
fn greet_repeats_after_extend_wait_time_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 0, now = 1_000: 1_000 - 0 = 1000 > 60, so it greets.
    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 4, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestRangerOutcomeEvent::UpdateState { .. })));
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hail thee, adventurer.")));
}

#[test]
fn greet_stays_silent_within_extend_wait_time() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 990, now = 1_000: 1_000 - 990 = 10 <= 60, stays silent.
    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 4, 990), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn greet_distance_is_wider_than_other_area1_ambient_npcs() {
    let mut world = World::default();
    // `char_dist`/`map_dist` doubles a same-row `dx` (`gwendylon.c`'s own
    // `char_dist` formula, ported in `drvlib::map_dist`), so `dx = 7`
    // (`x = 17`) yields `char_dist == 14`: within forest ranger's `15` but
    // beyond terion/yoakin/camhermit's shared `10`, confirming
    // `FOREST_RANGER_GREET_DISTANCE` is genuinely wider.
    world.map.tile_mut(17, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 17, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn beyond_greet_distance_is_ignored() {
    let mut world = World::default();
    // `dx = 8` (`x = 18`) yields `char_dist == 16 > 15`.
    world.map.tile_mut(18, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 18, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.is_empty());
}

#[test]
fn text_repeat_resets_greet_state_to_entry() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.driver_state = Some(CharacterDriverState::ForestRanger(ForestRangerDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        ranger.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 4, 0), 1_000, 1);
    assert!(events.contains(&ForestRangerOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(ranger_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        ranger_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn text_repeat_does_not_reset_non_greet_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.driver_state = Some(CharacterDriverState::ForestRanger(
            ForestRangerDriverData::default(),
        ));
        ranger.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 1, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestRangerOutcomeEvent::UpdateState { .. })));
}

#[test]
fn give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut ranger = forest_ranger_npc(1, 10);
    ranger.cursor_item = Some(ItemId(50));
    world.add_character(ranger);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_forest_ranger_actions(&HashMap::new(), 1, 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn talk_throttle_blocks_second_greet_within_ten_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forest_ranger_npc(1, 10), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_forest_ranger_actions(&facts(CharacterId(2), 1, 0), 1_000, 1);
    world.drain_pending_area_texts();

    // Only 5 ticks later (< TICKS * 10): still throttled, no dialogue.
    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND * 5);
    if let Some(ranger) = world.characters.get_mut(&CharacterId(1)) {
        ranger.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_forest_ranger_actions(&facts(CharacterId(2), 2, 0), 1_005, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestRangerOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}
