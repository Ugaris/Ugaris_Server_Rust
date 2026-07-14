use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoSanwynDriverData, CDR_TWOSANWYN, NT_CHAR, NT_GIVE, NT_TEXT,
};
use crate::item_driver::{IID_AREA17_PALACENOTE1, IID_AREA17_PALACENOTE2, IID_AREA17_PALACENOTE3};
use crate::world::npc::area17::sanwyn::{TwoSanwynOutcomeEvent, TwoSanwynPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn sanwyn_npc(id: u32) -> Character {
    let mut sanwyn = character(id);
    sanwyn.name = "Sanwyn".into();
    sanwyn.driver = CDR_TWOSANWYN;
    sanwyn.driver_state = Some(CharacterDriverState::TwoSanwyn(
        TwoSanwynDriverData::default(),
    ));
    sanwyn
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    sanwyn_state: i32,
    sanwyn_bits: i32,
) -> HashMap<CharacterId, TwoSanwynPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TwoSanwynPlayerFacts {
            sanwyn_state,
            sanwyn_bits,
        },
    );
    map
}

fn sanwyn_state(world: &World, sanwyn_id: CharacterId) -> TwoSanwynDriverData {
    match world
        .characters
        .get(&sanwyn_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoSanwyn(data)) => data,
        _ => panic!("expected two sanwyn driver state"),
    }
}

#[test]
fn sanwyn_greets_new_player_opens_quest_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 0, 0), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&TwoSanwynOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("noble and honorable city of Exkordon")));
    assert_eq!(
        sanwyn_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn sanwyn_state1_uses_army_rank_string_and_advances_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 1, 0), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    // military_points == 0 -> army_rank_for_points == 0 -> "nobody".
    assert!(texts
        .iter()
        .any(|text| text.message.contains("lawless town") && text.message.contains("nobody")));
}

#[test]
fn sanwyn_state6_waits_silently_for_documents() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 6, 0), 17);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn sanwyn_state7_completes_quest_and_advances_to_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 7, 7), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    assert!(events.contains(&TwoSanwynOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Dirty hands")));
}

#[test]
fn sanwyn_state8_stays_silent_once_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 8, 7), 17);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn sanwyn_repeat_command_resets_state_to_zero_when_at_or_below_six() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 3, 0), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn sanwyn_repeat_command_resets_state_8_back_to_7_not_zero() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 8, 7), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    assert!(!events.iter().any(|event| matches!(
        event,
        TwoSanwynOutcomeEvent::UpdateSanwynState { new_state: 0, .. }
    )));
}

#[test]
fn sanwyn_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(sanwyn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_sanwyn_actions(&facts(CharacterId(2), 0, 0), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn sanwyn_receiving_first_palace_note_sets_its_bit_and_awards_military_points() {
    let mut world = World::default();
    let mut sanwyn = sanwyn_npc(1);
    sanwyn.cursor_item = Some(ItemId(50));
    world.add_character(sanwyn);

    let mut note = item(50, ItemFlags::empty());
    note.name = "Note".into();
    note.template_id = IID_AREA17_PALACENOTE1;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);

    let mut godmode = player(2, "Godmode");
    godmode.level = 30;
    world.add_character(godmode);

    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 0, 0), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynBits {
        player_id: CharacterId(2),
        new_bits: 1,
    }));
    // Only one note turned in - state is not yet promoted to 7.
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoSanwynOutcomeEvent::UpdateSanwynState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
    // Cursor note item destroyed.
    assert!(!world.items.contains_key(&ItemId(50)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    // C `give_military_pts(cn, co, 15, min(level_value(30)/5, 15000))`
    // actually awards nonzero military points.
    assert!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points
            > 0
    );
}

#[test]
fn sanwyn_receiving_third_note_completing_the_set_promotes_state_to_seven() {
    let mut world = World::default();
    let mut sanwyn = sanwyn_npc(1);
    sanwyn.cursor_item = Some(ItemId(50));
    world.add_character(sanwyn);

    let mut note = item(50, ItemFlags::empty());
    note.name = "Note".into();
    note.template_id = IID_AREA17_PALACENOTE3;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);

    world.add_character(player(2, "Godmode"));

    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // Notes 1 and 2 already turned in (bits 1|2 == 3); this is the last.
    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 6, 3), 17);
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynBits {
        player_id: CharacterId(2),
        new_bits: 7,
    }));
    assert!(events.contains(&TwoSanwynOutcomeEvent::UpdateSanwynState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn sanwyn_receiving_already_turned_in_note_hands_it_back() {
    let mut world = World::default();
    let mut sanwyn = sanwyn_npc(1);
    sanwyn.cursor_item = Some(ItemId(50));
    world.add_character(sanwyn);

    let mut note = item(50, ItemFlags::empty());
    note.name = "Note".into();
    note.template_id = IID_AREA17_PALACENOTE1;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // Note 1's bit (1) is already set.
    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 3, 1), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn sanwyn_receiving_note_after_quest_completed_hands_it_back() {
    let mut world = World::default();
    let mut sanwyn = sanwyn_npc(1);
    sanwyn.cursor_item = Some(ItemId(50));
    world.add_character(sanwyn);

    let mut note = item(50, ItemFlags::empty());
    note.name = "Note".into();
    note.template_id = IID_AREA17_PALACENOTE2;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // sanwyn_state > 6 - already progressed past the collection phase.
    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 7, 7), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
}

#[test]
fn sanwyn_receiving_unrelated_item_hands_it_back() {
    let mut world = World::default();
    let mut sanwyn = sanwyn_npc(1);
    sanwyn.cursor_item = Some(ItemId(50));
    world.add_character(sanwyn);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(sanwyn) = world.characters.get_mut(&CharacterId(1)) {
        sanwyn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_sanwyn_actions(&facts(CharacterId(2), 0, 0), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
