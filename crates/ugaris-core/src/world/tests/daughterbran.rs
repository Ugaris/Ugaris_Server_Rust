use std::collections::HashMap;

use super::*;
use crate::character_driver::{DaughterBranDriverData, CDR_DAUGHTERBRAN, NT_CHAR, NT_GIVE};
use crate::world::npc::area29::daughterbran::{DaughterBranOutcomeEvent, DaughterBranPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn daughterbran_npc(id: u32) -> Character {
    let mut daughterbran = character(id);
    daughterbran.name = "Daughter Brannington".into();
    daughterbran.driver = CDR_DAUGHTERBRAN;
    daughterbran.driver_state = Some(CharacterDriverState::DaughterBran(
        DaughterBranDriverData::default(),
    ));
    daughterbran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    daughterbran_state: i32,
    countbran_bits: i32,
) -> HashMap<CharacterId, DaughterBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        DaughterBranPlayerFacts {
            daughterbran_state,
            countbran_bits,
        },
    );
    map
}

#[test]
fn state0_without_jewel_bit_greets_and_advances_to_1_with_no_reward() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(daughterbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_daughterbran_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(
        events.contains(&DaughterBranOutcomeEvent::UpdateDaughterBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, DaughterBranOutcomeEvent::GiveLollipop { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("bring me back my grandmother's jewel")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 0);
}

#[test]
fn state0_with_jewel_bit_already_set_cascades_straight_to_the_reward() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(daughterbran_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Bit 4 (daughter jewel returned) set, bit 16 (rewarded) unset: the C
    // switch cascades from case 0 through case 1 straight into case 2's
    // reward in the same driver call.
    let events = world.process_daughterbran_actions(&facts(CharacterId(2), 0, 4), 1);
    assert!(
        events.contains(&DaughterBranOutcomeEvent::SetDaughterBranRewardedBit {
            player_id: CharacterId(2),
        })
    );
    assert!(events.contains(&DaughterBranOutcomeEvent::GiveLollipop {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&DaughterBranOutcomeEvent::UpdateDaughterBranState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("you are my hero")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 30000);
}

#[test]
fn state0_with_both_bits_set_cascades_all_the_way_to_state3_silently() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(daughterbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_daughterbran_actions(&facts(CharacterId(2), 0, 4 | 16), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        DaughterBranOutcomeEvent::SetDaughterBranRewardedBit { .. }
    )));
    assert!(
        events.contains(&DaughterBranOutcomeEvent::UpdateDaughterBranState {
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
    assert!(world.spawn_character(daughterbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_daughterbran_actions(&facts(CharacterId(2), 3, 4 | 16), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_unconditionally() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(daughterbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_daughterbran_actions(&facts(CharacterId(2), 3, 4 | 16), 1);
    assert!(
        events.contains(&DaughterBranOutcomeEvent::UpdateDaughterBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn give_any_item_is_always_handed_back() {
    let mut world = World::default();
    let mut daughterbran = daughterbran_npc(1);
    daughterbran.cursor_item = Some(ItemId(50));
    world.add_character(daughterbran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(daughterbran) = world.characters.get_mut(&CharacterId(1)) {
        daughterbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_daughterbran_actions(&HashMap::new(), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
