use std::collections::HashMap;

use super::*;
use crate::character_driver::{SmuggleComDriverData, CDR_SMUGGLECOM, NT_CHAR, NT_GIVE};
use crate::item_driver::{
    IID_STAFF_SMUGGLEBOOK, IID_STAFF_SMUGGLECAPE, IID_STAFF_SMUGGLENECKLACE,
    IID_STAFF_SMUGGLEPEARLS, IID_STAFF_SMUGGLERING,
};
use crate::world::npc::area26::smugglecom::{SmuggleComOutcomeEvent, SmuggleComPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn smugglecom_npc(id: u32) -> Character {
    let mut smugglecom = character(id);
    smugglecom.name = "Imp. Commander".into();
    smugglecom.driver = CDR_SMUGGLECOM;
    smugglecom.driver_state = Some(CharacterDriverState::SmuggleCom(
        SmuggleComDriverData::default(),
    ));
    smugglecom
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    smugglecom_state: i32,
    smugglecom_bits: i32,
    quest36_count: u8,
    quest36_done: bool,
    quest37_done: bool,
) -> HashMap<CharacterId, SmuggleComPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        SmuggleComPlayerFacts {
            smugglecom_state,
            smugglecom_bits,
            quest36_count,
            quest36_done,
            quest37_done,
        },
    );
    map
}

fn smugglecom_state(world: &World, smugglecom_id: CharacterId) -> SmuggleComDriverData {
    match world
        .characters
        .get(&smugglecom_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::SmuggleCom(data)) => data,
        _ => panic!("expected smugglecom driver state"),
    }
}

#[test]
fn state0_greets_opens_quest35_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 0, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 35,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Greetings")));
    assert_eq!(
        smugglecom_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn state2_and_3_both_fall_through_to_state3_body() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 2, 0, 0, false, false), 1);
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 4,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Go now, and may Ishtar")));
}

#[test]
fn state4_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 4, 0, 0, false, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_jumps_to_7_when_quest36_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 5, 0, 0, true, false), 1);
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
    // No dialogue is spoken and `didsay` stays false for this sub-branch.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_opens_quest36_and_lists_items_when_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 5, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 36,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Rainbow Pearls")));
}

#[test]
fn state6_completes_quest36_only_when_all_bits_set() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Not all bits set yet: no-op.
    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 6, 7, 0, false, false), 1);
    assert!(events.is_empty());
}

#[test]
fn state6_completes_quest36_and_advances_to_7_when_bits_equal_15() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_smugglecom_actions(&facts(CharacterId(2), 6, 15, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 36,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
    // No dialogue is spoken for this transition (C never sets `didsay`).
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(
        smugglecom_state(&world, CharacterId(1)).current_victim,
        None
    );
}

#[test]
fn state7_speaks_thank_you_then_jumps_silently_to_10_when_quest37_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.driver_state = Some(CharacterDriverState::SmuggleCom(SmuggleComDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 7, 0, 0, false, true), 1);
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 10,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("great help in hurting")));
    // The "kill the leader" line is never spoken for this sub-branch, and
    // `didsay` stays false: `last_talk`/`current_victim` are untouched.
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("kill the smuggler's leader")));
    assert_eq!(smugglecom_state(&world, CharacterId(1)).last_talk, 500);
    assert_eq!(
        smugglecom_state(&world, CharacterId(1)).current_victim,
        None
    );
}

#[test]
fn state7_opens_quest37_and_advances_to_8_when_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 7, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 37,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 8,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("kill the smuggler's leader")));
    assert_eq!(
        smugglecom_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn state9_thanks_player_completes_quest37_and_advances_to_10() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 9, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 37,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 10,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for helping us")));
}

#[test]
fn text_repeat_resets_state_within_disjoint_ranges() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 6, 0, 0, false, false), 1);
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 8, 0, 0, false, false), 1);
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
}

#[test]
fn text_reset_me_is_ignored_without_god_flag() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events =
        world.process_smugglecom_actions(&facts(CharacterId(2), 9, 15, 0, false, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SmuggleComOutcomeEvent::ResetSmugglecom { .. })));
}

#[test]
fn text_reset_me_wipes_state_and_bits_for_god() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smugglecom_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events =
        world.process_smugglecom_actions(&facts(CharacterId(2), 9, 15, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::ResetSmugglecom {
        player_id: CharacterId(2),
    }));
}

#[test]
fn give_book_completes_quest35_destroys_book_and_sets_state5() {
    let mut world = World::default();
    let mut smugglecom = smugglecom_npc(1);
    smugglecom.cursor_item = Some(ItemId(50));
    world.add_character(smugglecom);
    let mut book = item(50, ItemFlags::empty());
    book.name = "the contraband book".into();
    book.template_id = IID_STAFF_SMUGGLEBOOK;
    book.carried_by = Some(CharacterId(1));
    world.add_item(book);
    world.add_character(player(2, "Godmode"));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 4, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 35,
    }));
    assert!(
        events.contains(&SmuggleComOutcomeEvent::UpdateSmugglecomState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for the book")));
    // The book is destroyed, not handed back.
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_book_is_ignored_once_state_is_past_4() {
    let mut world = World::default();
    let mut smugglecom = smugglecom_npc(1);
    smugglecom.cursor_item = Some(ItemId(50));
    world.add_character(smugglecom);
    let mut book = item(50, ItemFlags::empty());
    book.template_id = IID_STAFF_SMUGGLEBOOK;
    book.carried_by = Some(CharacterId(1));
    world.add_item(book);
    world.add_character(player(2, "Godmode"));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 5, 0, 0, false, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SmuggleComOutcomeEvent::QuestDone { .. })));
    // Falls into the generic "hand it back" branch instead.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_contraband_pearls_grants_scaled_exp_and_sets_bit() {
    let mut world = World::default();
    let mut smugglecom = smugglecom_npc(1);
    smugglecom.cursor_item = Some(ItemId(50));
    world.add_character(smugglecom);
    let mut pearls = item(50, ItemFlags::empty());
    pearls.name = "the Rainbow Pearls".into();
    pearls.template_id = IID_STAFF_SMUGGLEPEARLS;
    pearls.carried_by = Some(CharacterId(1));
    world.add_item(pearls);
    let mut godmode = player(2, "Godmode");
    godmode.level = 10;
    world.add_character(godmode);

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 6, 0, 0, false, false), 1);
    assert!(events.contains(&SmuggleComOutcomeEvent::SetSmugglecomBit {
        player_id: CharacterId(2),
        bit: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for bringing back")));
    // level 10 -> level_value(10)/4 = 4641/4 = 1160 >= 1000, so the full
    // scale_exp(0, 1000) = 1000 reward is granted uncapped.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 1000);
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_contraband_piece_already_collected_falls_back_to_hand_back() {
    let mut world = World::default();
    let mut smugglecom = smugglecom_npc(1);
    smugglecom.cursor_item = Some(ItemId(50));
    world.add_character(smugglecom);
    let mut ring = item(50, ItemFlags::empty());
    ring.template_id = IID_STAFF_SMUGGLERING;
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);
    world.add_character(player(2, "Godmode"));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // SMUGGLEBIT_RING (2) already set.
    let events = world.process_smugglecom_actions(&facts(CharacterId(2), 6, 2, 0, false, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SmuggleComOutcomeEvent::SetSmugglecomBit { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert_eq!(godmode.exp, 0);
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut smugglecom = smugglecom_npc(1);
    smugglecom.cursor_item = Some(ItemId(50));
    world.add_character(smugglecom);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
        smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_smugglecom_actions(&HashMap::new(), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn cape_and_necklace_template_ids_map_to_their_own_bits() {
    for (template_id, bit) in [(IID_STAFF_SMUGGLECAPE, 4), (IID_STAFF_SMUGGLENECKLACE, 8)] {
        let mut world = World::default();
        let mut smugglecom = smugglecom_npc(1);
        smugglecom.cursor_item = Some(ItemId(50));
        world.add_character(smugglecom);
        let mut piece = item(50, ItemFlags::empty());
        piece.template_id = template_id;
        piece.carried_by = Some(CharacterId(1));
        world.add_item(piece);
        let mut godmode = player(2, "Godmode");
        godmode.level = 10;
        world.add_character(godmode);

        if let Some(smugglecom) = world.characters.get_mut(&CharacterId(1)) {
            smugglecom.push_driver_message(NT_GIVE, 2, 50, 0);
        }

        let events =
            world.process_smugglecom_actions(&facts(CharacterId(2), 6, 0, 0, false, false), 1);
        assert!(events.contains(&SmuggleComOutcomeEvent::SetSmugglecomBit {
            player_id: CharacterId(2),
            bit,
        }));
    }
}
