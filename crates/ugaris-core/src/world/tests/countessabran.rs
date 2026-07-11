use std::collections::HashMap;

use super::*;
use crate::character_driver::{CountessaBranDriverData, CDR_COUNTESSABRAN, NT_CHAR, NT_GIVE};
use crate::world::npc::area29::countessabran::{
    CountessaBranOutcomeEvent, CountessaBranPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn countessabran_npc(id: u32) -> Character {
    let mut countessabran = character(id);
    countessabran.name = "Countessa Brannington".into();
    countessabran.driver = CDR_COUNTESSABRAN;
    countessabran.driver_state = Some(CharacterDriverState::CountessaBran(
        CountessaBranDriverData::default(),
    ));
    countessabran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    countessabran_state: i32,
    countbran_bits: i32,
) -> HashMap<CharacterId, CountessaBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CountessaBranPlayerFacts {
            countessabran_state,
            countbran_bits,
        },
    );
    map
}

#[test]
fn state0_without_jewel_bit_greets_and_advances_to_1_with_no_reward() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_countessabran_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(
        events.contains(&CountessaBranOutcomeEvent::UpdateCountessaBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        CountessaBranOutcomeEvent::SetCountessaBranRewardedBit { .. }
    )));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("return to us the jewelry")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 0);
    assert_eq!(godmode.gold, 0);
}

#[test]
fn state0_with_jewel_bit_already_set_cascades_straight_to_the_reward() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Bit 2 (countessa jewel returned) set, bit 8 (rewarded) unset: the C
    // switch cascades from case 0 through case 1 straight into case 2's
    // reward in the same driver call.
    let events = world.process_countessabran_actions(&facts(CharacterId(2), 0, 2), 1);
    assert!(
        events.contains(&CountessaBranOutcomeEvent::SetCountessaBranRewardedBit {
            player_id: CharacterId(2),
        })
    );
    assert!(
        events.contains(&CountessaBranOutcomeEvent::UpdateCountessaBranState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for returning my jewelry")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 30000);
    assert_eq!(godmode.gold, 500 * 100);
}

#[test]
fn state0_with_both_bits_set_cascades_all_the_way_to_state3_silently() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Bits 2 and 8 both set: already rewarded, cascades to state 3 with
    // no dialogue and no reward.
    let events = world.process_countessabran_actions(&facts(CharacterId(2), 0, 2 | 8), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        CountessaBranOutcomeEvent::SetCountessaBranRewardedBit { .. }
    )));
    assert!(
        events.contains(&CountessaBranOutcomeEvent::UpdateCountessaBranState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state3_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_countessabran_actions(&facts(CharacterId(2), 3, 2 | 8), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_unconditionally() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_text_message(CharacterId(2), "repeat");
    }
    // Even at the terminal state 3, `case 2` has no `<=` guard.
    let events = world.process_countessabran_actions(&facts(CharacterId(2), 3, 2 | 8), 1);
    assert!(
        events.contains(&CountessaBranOutcomeEvent::UpdateCountessaBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn text_reset_me_has_no_special_handling_but_still_turns_to_speaker() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(countessabran_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_text_message(CharacterId(2), "reset me");
    }

    // Unlike `world::npc::area29::countbran`, this driver's `switch` has no
    // `case 3` at all: no state-reset event is pushed.
    let events = world.process_countessabran_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn give_any_item_is_always_handed_back() {
    let mut world = World::default();
    let mut countessabran = countessabran_npc(1);
    countessabran.cursor_item = Some(ItemId(50));
    world.add_character(countessabran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(countessabran) = world.characters.get_mut(&CharacterId(1)) {
        countessabran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_countessabran_actions(&HashMap::new(), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
