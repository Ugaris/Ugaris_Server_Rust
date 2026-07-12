use std::collections::HashMap;

use super::*;
use crate::character_driver::{CountBranDriverData, CDR_COUNTBRAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{
    IID_ARKHATA_LETTER3, IID_STAFF_COUNTESSAJEWEL, IID_STAFF_COUNTJEWEL, IID_STAFF_DAUGHTERJEWEL,
    IID_STAFF_MAUSOLEUMKEY1,
};
use crate::world::npc::area29::countbran::{CountBranOutcomeEvent, CountBranPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn countbran_npc(id: u32) -> Character {
    let mut countbran = character(id);
    countbran.name = "Count Brannington".into();
    countbran.driver = CDR_COUNTBRAN;
    countbran.driver_state = Some(CharacterDriverState::CountBran(
        CountBranDriverData::default(),
    ));
    countbran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    countbran_state: i32,
    countbran_bits: i32,
    quest40_count: u8,
    quest40_is_done: bool,
) -> HashMap<CharacterId, CountBranPlayerFacts> {
    facts_with_letter_bits(
        player_id,
        countbran_state,
        countbran_bits,
        quest40_count,
        quest40_is_done,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn facts_with_letter_bits(
    player_id: CharacterId,
    countbran_state: i32,
    countbran_bits: i32,
    quest40_count: u8,
    quest40_is_done: bool,
    arkhata_letter_bits: i32,
) -> HashMap<CharacterId, CountBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CountBranPlayerFacts {
            countbran_state,
            countbran_bits,
            quest40_count,
            quest40_is_done,
            arkhata_letter_bits,
        },
    );
    map
}

#[test]
fn state0_greets_opens_quest40_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 0, 0, 0, false), 1);
    assert!(events.contains(&CountBranOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&CountBranOutcomeEvent::UpdateCountBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("welcome to Brannington")));
}

#[test]
fn state0_does_not_reopen_an_already_done_quest40() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 0, 7, 1, true), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CountBranOutcomeEvent::QuestOpen { .. })));
}

#[test]
fn states1_through_3_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "guards told me"),
        (2, 3, "robbed by three thief mages"),
        (3, 4, "very thankful indeed"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(countbran_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
            countbran.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_countbran_actions(&facts(CharacterId(2), state, 0, 0, false), 1);
        assert!(
            events.contains(&CountBranOutcomeEvent::UpdateCountBranState {
                player_id: CharacterId(2),
                new_state: next_state,
            }),
            "state {state} should advance to {next_state}"
        );
        let texts = world.drain_pending_area_texts();
        assert!(
            texts.iter().any(|text| text.message.contains(snippet)),
            "state {state} should speak {snippet:?}"
        );
    }
}

#[test]
fn state4_is_a_silent_no_op_waiting_for_jewelry() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_count_jewel_rewards_exp_gold_bit_and_mausoleum_key() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut jewel = item(50, ItemFlags::empty());
    jewel.template_id = IID_STAFF_COUNTJEWEL;
    jewel.carried_by = Some(CharacterId(1));
    world.add_item(jewel);
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    world.add_character(godmode);

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.contains(&CountBranOutcomeEvent::SetCountBranBit {
        player_id: CharacterId(2),
        bit: 1,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CountBranOutcomeEvent::MarkQuestDone { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        CountBranOutcomeEvent::GiveMausoleumKeys { player_id, keys }
            if *player_id == CharacterId(2) && keys == &vec![1u8]
    )));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 60000);
    assert_eq!(godmode.gold, 1000 * 100);
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_all_three_jewels_marks_quest40_done() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut jewel = item(50, ItemFlags::empty());
    jewel.template_id = IID_STAFF_DAUGHTERJEWEL;
    jewel.carried_by = Some(CharacterId(1));
    world.add_item(jewel);
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    world.add_character(godmode);

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // Count's own (1) and Countessa's (2) jewels are already returned;
    // this hand-in sets bit 4, completing `1 | 2 | 4`.
    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 1 | 2, 0, false), 1);
    assert!(events.contains(&CountBranOutcomeEvent::MarkQuestDone {
        player_id: CharacterId(2),
    }));
}

#[test]
fn give_arkhata_letter3_sets_letter_bit_and_destroys_letter() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_ARKHATA_LETTER3;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_countbran_actions(
        &facts_with_letter_bits(CharacterId(2), 4, 1 | 2 | 4, 0, false, 0),
        1,
    );
    assert!(
        events.contains(&CountBranOutcomeEvent::SetArkhataLetterBit {
            player_id: CharacterId(2),
            bit: 4,
        })
    );
    assert!(world.items.get(&ItemId(50)).is_none());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("most clever solution")));
}

#[test]
fn give_arkhata_letter3_when_bit_already_set_falls_back_to_no_use_for_it() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_ARKHATA_LETTER3;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // bit 4 already set: not accepted again, handed back instead.
    let events = world.process_countbran_actions(
        &facts_with_letter_bits(CharacterId(2), 4, 1 | 2, 0, false, 4),
        1,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, CountBranOutcomeEvent::SetArkhataLetterBit { .. })));
    // Handed back, not destroyed.
    assert!(world.items.get(&ItemId(50)).is_some());
}

#[test]
fn give_countessa_jewel_when_already_holding_bit_grants_no_reward() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut jewel = item(50, ItemFlags::empty());
    jewel.template_id = IID_STAFF_COUNTESSAJEWEL;
    jewel.carried_by = Some(CharacterId(1));
    world.add_item(jewel);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(51));
    world.add_character(godmode);
    let mut key = item(51, ItemFlags::empty());
    key.template_id = IID_STAFF_MAUSOLEUMKEY1;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // bit 2 already set: not accepted again.
    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 2, 0, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CountBranOutcomeEvent::SetCountBranBit { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 0);
    // Handed back, not destroyed.
    assert!(world.items.get(&ItemId(50)).is_some());
}

#[test]
fn text_repeat_resets_state_and_reissues_unclaimed_keys() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    // Bit 1 is unlocked but the player doesn't carry key 1 yet.
    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 1, 0, false), 1);
    assert!(
        events.contains(&CountBranOutcomeEvent::UpdateCountBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
    assert!(events.iter().any(|event| matches!(
        event,
        CountBranOutcomeEvent::GiveMausoleumKeys { player_id, keys }
            if *player_id == CharacterId(2) && keys == &vec![1u8]
    )));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.contains(&CountBranOutcomeEvent::ResetAllBranStates {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_countbran_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut countbran = countbran_npc(1);
    countbran.cursor_item = Some(ItemId(50));
    world.add_character(countbran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(countbran) = world.characters.get_mut(&CharacterId(1)) {
        countbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_countbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
