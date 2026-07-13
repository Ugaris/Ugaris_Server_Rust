use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_ARKHATACLERK, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_ARKHATA_NOTE1, IID_ARKHATA_NOTE3};
use crate::world::npc::area37::clerk::{ClerkDriverData, ClerkOutcomeEvent, ClerkPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;
const CLERKTIME: i32 = 60 * 15;

fn clerk_npc(id: u32) -> Character {
    let mut clerk = character(id);
    clerk.name = "Clerk".into();
    clerk.driver = CDR_ARKHATACLERK;
    clerk.driver_state = Some(CharacterDriverState::Clerk(ClerkDriverData::default()));
    clerk
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[allow(clippy::too_many_arguments)]
fn facts(
    player_id: CharacterId,
    clerk_state: i32,
    clerk_time: i32,
    clerk_bits: i32,
    captain_state: i32,
    is_god: bool,
) -> HashMap<CharacterId, ClerkPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ClerkPlayerFacts {
            clerk_state,
            clerk_time,
            clerk_bits,
            captain_state,
            is_god,
        },
    );
    map
}

#[test]
fn state0_without_captain_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clerk_actions(&facts(CharacterId(2), 0, 0, 0, 0, false), 0, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_captain_progress_opens_quest76_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_clerk_actions(&facts(CharacterId(2), 0, 0, 0, 5, false), 0, 1);
    assert!(events.contains(&ClerkOutcomeEvent::QuestOpen76 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ClerkOutcomeEvent::UpdateClerkState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
}

#[test]
fn state5_expired_timer_fails_the_quest_and_advances_to_6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_clerk_actions(
        &facts(CharacterId(2), 5, now - CLERKTIME - 1, 0, 5, false),
        now,
        1,
    );
    assert!(events.contains(&ClerkOutcomeEvent::UpdateClerkState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("too late")));
}

#[test]
fn state5_within_time_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_clerk_actions(&facts(CharacterId(2), 5, now, 0, 5, false), now, 1);
    assert!(events.is_empty());
}

#[test]
fn text_aye_at_state4_starts_the_timer_and_gives_a_stopwatch() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_text_message(CharacterId(2), "aye");
    }

    let now = 100_000;
    let events = world.process_clerk_actions(&facts(CharacterId(2), 4, 0, 0, 5, false), now, 1);
    assert!(events.contains(&ClerkOutcomeEvent::StartClerkTimer {
        player_id: CharacterId(2),
        realtime_seconds: now,
    }));
    assert!(events.contains(&ClerkOutcomeEvent::GiveStopwatch {
        player_id: CharacterId(2),
    }));
}

#[test]
fn text_aye_before_state4_without_god_is_ignored() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(clerk_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_text_message(CharacterId(2), "aye");
    }

    let now = 100_000;
    let events = world.process_clerk_actions(&facts(CharacterId(2), 2, 0, 0, 5, false), now, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ClerkOutcomeEvent::StartClerkTimer { .. })));
}

#[test]
fn give_note1_and_note2_hope_message_note3_completes_and_sets_state6() {
    let mut world = World::default();
    let mut clerk = clerk_npc(1);
    clerk.cursor_item = Some(ItemId(50));
    world.add_character(clerk);
    let mut note3 = item(50, ItemFlags::empty());
    note3.template_id = IID_ARKHATA_NOTE3;
    note3.carried_by = Some(CharacterId(1));
    world.add_item(note3);
    world.add_character(player(2, "Godmode"));

    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let now = 100_000;
    // bits 1|2 already set (notes 1 and 2 turned in earlier).
    let events =
        world.process_clerk_actions(&facts(CharacterId(2), 5, now - 10, 1 | 2, 5, false), now, 1);
    assert!(events.contains(&ClerkOutcomeEvent::QuestDone76 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ClerkOutcomeEvent::UpdateClerkState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(events.contains(&ClerkOutcomeEvent::UpdateClerkBits {
        player_id: CharacterId(2),
        bits: 1 | 2 | 4,
    }));
}

#[test]
fn give_note1_completing_the_set_does_not_advance_state_c_quirk() {
    let mut world = World::default();
    let mut clerk = clerk_npc(1);
    clerk.cursor_item = Some(ItemId(50));
    world.add_character(clerk);
    let mut note1 = item(50, ItemFlags::empty());
    note1.template_id = IID_ARKHATA_NOTE1;
    note1.carried_by = Some(CharacterId(1));
    world.add_item(note1);
    world.add_character(player(2, "Godmode"));

    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let now = 100_000;
    // bits 2|4 already set (notes 2 and 3 turned in earlier).
    let events =
        world.process_clerk_actions(&facts(CharacterId(2), 5, now - 10, 2 | 4, 5, false), now, 1);
    assert!(events.contains(&ClerkOutcomeEvent::QuestDone76 {
        player_id: CharacterId(2),
    }));
    // C quirk: only the NOTE3 branch advances `clerk_state` to 6.
    assert!(!events
        .iter()
        .any(|event| matches!(event, ClerkOutcomeEvent::UpdateClerkState { .. })));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut clerk = clerk_npc(1);
    clerk.cursor_item = Some(ItemId(50));
    world.add_character(clerk);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(clerk) = world.characters.get_mut(&CharacterId(1)) {
        clerk.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_clerk_actions(&HashMap::new(), 0, 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
