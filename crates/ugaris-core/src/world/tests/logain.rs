use std::collections::HashMap;

use super::*;
use crate::character_driver::{LogainDriverData, CDR_LOGAIN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA1_MADKEY6, IID_AREA1_MADKEY9, IID_AREA1_MADNOTE2};
use crate::world::logain::{LogainOutcomeEvent, LogainPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn logain_npc(id: u32) -> Character {
    let mut logain = character(id);
    logain.name = "Logain".into();
    logain.driver = CDR_LOGAIN;
    logain.driver_state = Some(CharacterDriverState::Logain(LogainDriverData::default()));
    logain
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
    guiwynn_state: i32,
) -> HashMap<CharacterId, LogainPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        LogainPlayerFacts {
            state,
            seen_timer,
            guiwynn_state,
        },
    );
    map
}

fn logain_state(world: &World, logain_id: CharacterId) -> LogainDriverData {
    match world
        .characters
        .get(&logain_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Logain(data)) => data,
        _ => panic!("expected logain driver state"),
    }
}

#[test]
fn logain_entry_stays_silent_before_mad_mages_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // guiwynn_state 10 < 11: no quest offer yet.
    let events = world.process_logain_actions(&facts(CharacterId(2), 0, 0, 10), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn logain_entry_greets_opens_quest9_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 0, 0, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Canst thou spare a moment")));
}

#[test]
fn logain_state4_grants_madkey6_when_missing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 4, 950, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::GrantMadKey6 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("need this key to gain entry")));
}

#[test]
fn logain_state4_skips_key_grant_when_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(player_char) = world.characters.get_mut(&CharacterId(2)) {
        player_char.cursor_item = Some(ItemId(99));
    }
    let mut key = item(99, ItemFlags::empty());
    key.template_id = IID_AREA1_MADKEY6;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 4, 950, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::GrantMadKey6 { .. })));
}

#[test]
fn logain_state5_reminds_after_gate_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer old (now - seen_timer > 60): reminder fires, no state change.
    let events = world.process_logain_actions(&facts(CharacterId(2), 5, 900, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::UpdateState { .. })));
    // C `case 5:` wraps "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`gwendylon.c:5020-5021`); goes out via `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message)
            .contains("Couldst thou find out who is responsible")));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(9).any(|w| w == b"\xb0c4repeat")));
}

#[test]
fn logain_state5_stays_silent_within_reminder_gate() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 5, 990, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn logain_state6_advances_to_seven() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 6, 0, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Loisan? This is strange indeed")));
}

#[test]
fn logain_state7_grants_madkey9_when_missing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 7, 1_000, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::GrantMadKey9 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Here. I won't use it")));
}

#[test]
fn logain_state7_skips_key_grant_when_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(player_char) = world.characters.get_mut(&CharacterId(2)) {
        player_char.cursor_item = Some(ItemId(99));
    }
    let mut key = item(99, ItemFlags::empty());
    key.template_id = IID_AREA1_MADKEY9;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 7, 1_000, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::GrantMadKey9 { .. })));
}

#[test]
fn logain_state8_grants_madkey6_and_advances_to_nine() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 8, 1_000, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::GrantMadKey6 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("pay this Loisan a visit")));
}

#[test]
fn logain_state9_is_a_reminder_only_done_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 9, 900, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I am pleased to see thee")));
}

#[test]
fn logain_give_madnote2_finishes_quest9_in_state_range() {
    let mut world = World::default();
    let mut logain = logain_npc(1);
    logain.cursor_item = Some(ItemId(50));
    world.add_character(logain);
    let mut note = item(50, ItemFlags::empty());
    note.template_id = IID_AREA1_MADNOTE2;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 3, 0, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(world.items.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Now let's see. Ah. I thank thee")));
}

#[test]
fn logain_give_madnote2_outside_state_range_is_a_normal_give_back() {
    let mut world = World::default();
    let mut logain = logain_npc(1);
    logain.cursor_item = Some(ItemId(50));
    world.add_character(logain);
    let mut note = item(50, ItemFlags::empty());
    note.template_id = IID_AREA1_MADNOTE2;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 6: outside the `<= 5` range the C code checks for the note.
    let events = world.process_logain_actions(&facts(CharacterId(2), 6, 0, 11), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LogainOutcomeEvent::QuestDone { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn logain_text_repeat_resets_early_states_to_zero() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.driver_state = Some(CharacterDriverState::Logain(LogainDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        logain.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 3, 0, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(logain_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        logain_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn logain_text_repeat_resets_mid_states_to_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.driver_state = Some(CharacterDriverState::Logain(LogainDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        logain.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_logain_actions(&facts(CharacterId(2), 8, 0, 11), 1_000, 1);
    assert!(events.contains(&LogainOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert_eq!(logain_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn logain_idle_moves_toward_post_after_talk_gate_elapses() {
    let mut world = World::default();
    assert!(world.spawn_character(logain_npc(1), 10, 10));
    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.rest_x = 10;
        logain.rest_y = 10;
        logain.x = 15;
        logain.y = 15;
        logain.driver_state = Some(CharacterDriverState::Logain(LogainDriverData {
            last_talk: 0,
            current_victim: None,
        }));
    }

    world.tick = Tick(TICKS_PER_SECOND * 100);
    world.process_logain_actions(&HashMap::new(), 1_000, 1);
    // No panics, no crash: the idle move attempt runs to completion.
}

#[test]
fn logain_terion_dat3_2_replies_without_broadcast() {
    let mut world = World::default();
    let mut logain = logain_npc(1);
    logain.x = 10;
    logain.y = 10;
    world.add_character(logain);
    world.add_character(player(2, "Terion"));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_NPC, crate::character_driver::NTID_TERION, 2, 2);
    }

    world.process_logain_actions(&HashMap::new(), 1_000, 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Fools, yes fools they are")));
}

#[test]
fn logain_terion_dat3_3_replies_and_rebroadcasts_dat3_4() {
    let mut world = World::default();
    let mut logain = logain_npc(1);
    logain.x = 10;
    logain.y = 10;
    world.add_character(logain);
    world.add_character(player(2, "Terion"));

    if let Some(logain) = world.characters.get_mut(&CharacterId(1)) {
        logain.push_driver_message(NT_NPC, crate::character_driver::NTID_TERION, 2, 3);
    }

    world.process_logain_actions(&HashMap::new(), 1_000, 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("skeletons hunting him in a dark, moist place")));
}
