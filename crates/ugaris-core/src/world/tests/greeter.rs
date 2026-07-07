use std::collections::HashMap;

use super::*;
use crate::character_driver::{GreeterDriverData, CDR_GREETER, NT_CHAR};
use crate::world::greeter::{GreeterOutcomeEvent, GreeterPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn greeter_npc(id: u32) -> Character {
    let mut greeter = character(id);
    greeter.name = "Cameron".into();
    greeter.driver = CDR_GREETER;
    greeter.driver_state = Some(CharacterDriverState::Greeter(GreeterDriverData::default()));
    // Same rest-tile convention as `world::terion`'s test module: keep
    // `secure_move_driver`'s idle "return to post" branch a no-op.
    greeter.rest_x = 10;
    greeter.rest_y = 10;
    greeter
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
    james_state: i32,
    lydia_quest_done: bool,
) -> HashMap<CharacterId, GreeterPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GreeterPlayerFacts {
            state,
            seen_timer,
            james_state,
            lydia_quest_done,
        },
    );
    map
}

fn greeter_state(world: &World, greeter_id: CharacterId) -> GreeterDriverData {
    match world
        .characters
        .get(&greeter_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Greeter(data)) => data,
        _ => panic!("expected greeter driver state"),
    }
}

#[test]
fn greeter_entry_greets_warrior_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 0, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&GreeterOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 100,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I see thou art a mighty Warrior.")));
    assert_eq!(
        greeter_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn greeter_entry_greets_mage_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MAGE;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 0, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I see thou art a wise Mage.")));
}

#[test]
fn greeter_entry_seyan_du_jumps_straight_to_final_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR | CharacterFlags::MAGE;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 0, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Seyan'Du in quite some time")));
}

#[test]
fn greeter_entry_silent_when_no_class_chosen_yet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    // Neither WARRIOR nor MAGE set.
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 0, 0, 0, false), 100, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn greeter_state1_skips_weapon_tutorial_above_level7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR;
    godmode.level = 8;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // silent jump straight to state 12, no dialogue.
    let events = world.process_greeter_actions(&facts(CharacterId(2), 1, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(greeter_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn greeter_state1_advances_at_or_below_level7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR;
    godmode.level = 7;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 1, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("choosing which weapon")));
}

#[test]
fn greeter_state6_includes_james_reminder_only_when_james_state_zero() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 6, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    // C `case 6:` wraps "learn" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`gwendylon.c:1633-1634`); goes out via `npc_quiet_say_bytes`. The
    // James-whimpering follow-up line has no color markers and still goes
    // out via the plain `npc_quiet_say`/`pending_area_texts` queue.
    let byte_texts = world.drain_pending_area_text_bytes();
    assert!(byte_texts
        .iter()
        .any(|text| text.message.windows(8).any(|w| w == b"\xb0c4learn")));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("poor James whimpering")));
}

#[test]
fn greeter_state6_omits_james_reminder_when_james_state_nonzero() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 6, 0, 1, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("poor James whimpering")));
}

#[test]
fn greeter_state12_final_line_when_lydia_quest_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 12, 0, 0, true), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
    // C `case 12:` wraps "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`gwendylon.c:1686-1687`); goes out via `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("repeat") && !text.contains("James is audibly in requirement of aid!")
    }));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(9).any(|w| w == b"\xb0c4repeat")));
}

#[test]
fn greeter_state12_reminder_line_when_lydia_quest_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 12, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message)
            .contains("James is audibly in requirement of aid!")));
}

#[test]
fn greeter_state13_silently_advances_when_lydia_quest_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 13, 0, 0, true), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn greeter_state13_reminds_after_60_seconds_when_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // now=161, seen_timer=100 -> 61 > 60: reminder fires.
    let events = world.process_greeter_actions(&facts(CharacterId(2), 13, 100, 0, false), 161, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
    // C `case 13:` wraps "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`gwendylon.c:1706-1707`); goes out via `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message).contains("Hail, Godmode!")));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(9).any(|w| w == b"\xb0c4repeat")));
}

#[test]
fn greeter_state13_stays_silent_within_60_second_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // now=130, seen_timer=100 -> 30 <= 60: stay silent.
    let events = world.process_greeter_actions(&facts(CharacterId(2), 13, 100, 0, false), 130, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn greeter_state14_is_terminal_and_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 14, 0, 0, false), 100, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
    // Seen timer is still updated unconditionally.
    assert!(events.contains(&GreeterOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 100,
    }));
}

#[test]
fn greeter_ignores_npcs_and_lostcon_players() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    // Not a player: CF_PLAYER unset.
    assert!(world.spawn_character(character(2), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 0, 0, 0, false), 100, 1);
    assert!(events.is_empty());
}

#[test]
fn greeter_repeat_resets_to_entry_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 9, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(greeter_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn greeter_learn_rewinds_to_rest_area_state_from_final_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_text_message(CharacterId(2), "learn");
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 14, 0, 0, false), 100, 1);
    assert!(events.contains(&GreeterOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
}

#[test]
fn greeter_learn_does_nothing_mid_dialogue() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_text_message(CharacterId(2), "learn");
    }

    // state 3 is neither the "empty" checkpoint (7) nor >= 13: no rewind.
    let events = world.process_greeter_actions(&facts(CharacterId(2), 3, 0, 0, false), 100, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
}

#[test]
fn greeter_hello_says_canned_answer() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(greeter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_text_message(CharacterId(2), "hello");
    }

    let events = world.process_greeter_actions(&facts(CharacterId(2), 3, 0, 0, false), 100, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GreeterOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn greeter_give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut greeter = greeter_npc(1);
    greeter.cursor_item = Some(ItemId(50));
    world.add_character(greeter);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(greeter) = world.characters.get_mut(&CharacterId(1)) {
        greeter.push_driver_message(crate::character_driver::NT_GIVE, 2, 50, 0);
    }

    world.process_greeter_actions(&HashMap::new(), 100, 1);

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
