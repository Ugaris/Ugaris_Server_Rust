use std::collections::HashMap;

use super::*;
use crate::character_driver::{BrithildieDriverData, CDR_BRITHILDIE, NT_CHAR, NT_GIVE};
use crate::world::brithildie::{BrithildieOutcomeEvent, BrithildiePlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn brithildie_npc(id: u32) -> Character {
    let mut brithildie = character(id);
    brithildie.name = "Brithildie".into();
    brithildie.driver = CDR_BRITHILDIE;
    brithildie.driver_state = Some(CharacterDriverState::Brithildie(
        BrithildieDriverData::default(),
    ));
    // Match the spawn tile used by every test in this module so the
    // "return to post" idle branch (`secure_move_driver` toward
    // `rest_x`/`rest_y`) is a no-op instead of relocating the NPC away
    // from the position the rest of the test asserts against - `World::
    // spawn_character` (unlike zone-file loading via `zone.rs`) never
    // seeds `rest_x`/`rest_y` from the spawn position on its own.
    brithildie.rest_x = 10;
    brithildie.rest_y = 10;
    brithildie
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    seen_timer: i32,
) -> HashMap<CharacterId, BrithildiePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, BrithildiePlayerFacts { state, seen_timer });
    map
}

fn brithildie_state(world: &World, brithildie_id: CharacterId) -> BrithildieDriverData {
    match world
        .characters
        .get(&brithildie_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Brithildie(data)) => data,
        _ => panic!("expected brithildie driver state"),
    }
}

#[test]
fn entry_greets_and_branches_on_player_level() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 5), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("mine eldest son Walter")));
}

#[test]
fn entry_branches_to_story_1_1_for_mid_level_player() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 15), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
}

#[test]
fn entry_branches_to_wait_story_2_for_higher_level_player() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 25), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn entry_branches_to_story_2_1_for_high_level_player() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 40), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
}

#[test]
fn wait_story_1_stays_silent_within_extend_wait_time_for_low_level() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 5), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 990, now = 1_000: 10 <= 60, stays silent.
    let events = world.process_brithildie_actions(&facts(CharacterId(2), 1, 990), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrithildieOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn wait_story_1_reminds_after_extend_wait_time_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 5), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 1, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrithildieOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Come back another day")));
}

#[test]
fn wait_story_1_silently_advances_when_player_levels_up() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 15), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 1, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn story_chain_progresses_one_state_per_visit() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 15), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 2, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("hunter. Just as proud as Yoakin")));
}

#[test]
fn wait_story_3_skips_story_3_states_and_reaches_story_4_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // BRITHILDIE_STATE_WAIT_STORY_3 = 12, level >= 43 jumps directly to
    // BRITHILDIE_STATE_STORY_4_1 = 17, skipping the dead 13/14/15 states -
    // see the module doc comment on `world::brithildie` for why.
    let events = world.process_brithildie_actions(&facts(CharacterId(2), 12, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 17,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn story_4_3_opens_the_questlog() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 19, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 20,
    }));
}

#[test]
fn nomoretales_reminds_after_extend_wait_time_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 20, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrithildieOutcomeEvent::UpdateState { .. })));
    // The reminder line wraps "Repeat all" in `COL_LIGHT_BLUE`/
    // `COL_RESET` markers (`gwendylon.c:2716-2717`); goes out via
    // `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(
        |text| String::from_utf8_lossy(&text.message).contains("I have no more tales to tell")
    ));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(13).any(|w| w == b"\xb0c4Repeat all")));
}

#[test]
fn text_repeat_resets_wait_story_2_to_story_1_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.driver_state = Some(CharacterDriverState::Brithildie(BrithildieDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        brithildie.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 7, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    assert_eq!(brithildie_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        brithildie_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn text_repeat_all_resets_nomoretales_qopen_to_wait_story_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.driver_state = Some(CharacterDriverState::Brithildie(
            BrithildieDriverData::default(),
        ));
        brithildie.push_driver_text_message(CharacterId(2), "repeat all");
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 20, 0), 1_000, 1);
    assert!(events.contains(&BrithildieOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn text_repeat_all_does_not_reset_non_qopen_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.driver_state = Some(CharacterDriverState::Brithildie(
            BrithildieDriverData::default(),
        ));
        brithildie.push_driver_text_message(CharacterId(2), "repeat all");
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 7, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrithildieOutcomeEvent::UpdateState { .. })));
}

#[test]
fn give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut brithildie = brithildie_npc(1);
    brithildie.cursor_item = Some(ItemId(50));
    world.add_character(brithildie);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode", 10));

    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_brithildie_actions(&HashMap::new(), 1, 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    // C's plain `give_char_item` (unlike `give_char_item_smart`) hands the
    // item back onto the giver's cursor when it's empty, not into a fixed
    // inventory slot - see `World::give_char_item`'s doc comment.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn beyond_greet_distance_is_ignored() {
    let mut world = World::default();
    // `dx = 8` (`x = 18`) yields `char_dist == 16 > 15`.
    world.map.tile_mut(18, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 5), 18, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brithildie_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.is_empty());
}

#[test]
fn talk_throttle_blocks_second_greet_within_ten_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brithildie_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 15), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_brithildie_actions(&facts(CharacterId(2), 2, 0), 1_000, 1);
    world.drain_pending_area_texts();

    // Only 5 ticks later (< TICKS * 10): still throttled, no dialogue.
    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND * 5);
    if let Some(brithildie) = world.characters.get_mut(&CharacterId(1)) {
        brithildie.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_brithildie_actions(&facts(CharacterId(2), 3, 0), 1_005, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrithildieOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}
