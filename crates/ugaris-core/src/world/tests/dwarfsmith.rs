use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_DWARFSMITH, NT_CHAR, NT_GIVE};
use crate::item_driver::{IDR_ENHANCE, IID_LIZARDMOLD};
use crate::world::npc::area31::dwarfsmith::{
    DwarfSmithDriverData, DwarfsmithEliteKey, DwarfsmithOutcomeEvent, DwarfsmithPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn dwarfsmith_npc(id: u32) -> Character {
    let mut dwarfsmith = character(id);
    dwarfsmith.name = "Dwarven Blacksmith".into();
    dwarfsmith.driver = CDR_DWARFSMITH;
    dwarfsmith.driver_state = Some(CharacterDriverState::DwarfSmith(
        DwarfSmithDriverData::default(),
    ));
    dwarfsmith
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    dwarfsmith_state: i32,
    dwarfsmith_type: i32,
) -> HashMap<CharacterId, DwarfsmithPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        DwarfsmithPlayerFacts {
            dwarfsmith_state,
            dwarfsmith_type,
        },
    );
    map
}

fn silver_stack(id: u32, amount: u32) -> Item {
    let mut stack = item(id, ItemFlags::empty());
    stack.driver = IDR_ENHANCE;
    stack.driver_data = vec![1, 0, 0, 0, 0];
    stack.driver_data[1..5].copy_from_slice(&amount.to_le_bytes());
    stack
}

#[test]
fn state0_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfsmith_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(
        events.contains(&DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome to my smithy")));
}

#[test]
fn giving_mold_remembers_type_and_advances_to_2() {
    let mut world = World::default();
    let mut dwarfsmith = dwarfsmith_npc(1);
    dwarfsmith.cursor_item = Some(ItemId(50));
    world.add_character(dwarfsmith);
    let mut mold = item(50, ItemFlags::empty());
    mold.template_id = IID_LIZARDMOLD;
    mold.driver_data = vec![2];
    mold.carried_by = Some(CharacterId(1));
    world.add_item(mold);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 1, 0), 1);
    assert!(
        events.contains(&DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    assert!(
        events.contains(&DwarfsmithOutcomeEvent::UpdateDwarfsmithType {
            player_id: CharacterId(2),
            new_type: 2,
        })
    );
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn giving_exactly_5000_silver_at_state2_forges_the_remembered_key() {
    let mut world = World::default();
    let mut dwarfsmith = dwarfsmith_npc(1);
    dwarfsmith.cursor_item = Some(ItemId(50));
    world.add_character(dwarfsmith);
    let mut silver = silver_stack(50, 5000);
    silver.carried_by = Some(CharacterId(1));
    world.add_item(silver);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 2, 2), 1);
    assert!(events.contains(&DwarfsmithOutcomeEvent::GrantEliteKey {
        player_id: CharacterId(2),
        key: DwarfsmithEliteKey::Key2,
    }));
    assert!(
        events.contains(&DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    assert!(
        events.contains(&DwarfsmithOutcomeEvent::UpdateDwarfsmithType {
            player_id: CharacterId(2),
            new_type: 0,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("one key for the adventurer")));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn giving_wrong_silver_amount_at_state2_is_returned_with_exact_amount_message() {
    let mut world = World::default();
    let mut dwarfsmith = dwarfsmith_npc(1);
    dwarfsmith.cursor_item = Some(ItemId(50));
    world.add_character(dwarfsmith);
    let mut silver = silver_stack(50, 4000);
    silver.carried_by = Some(CharacterId(1));
    world.add_item(silver);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 2, 2), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, DwarfsmithOutcomeEvent::GrantEliteKey { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("exactly 5000 units of silver")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn giving_exact_silver_before_a_mold_is_returned_needing_a_mold_first() {
    let mut world = World::default();
    let mut dwarfsmith = dwarfsmith_npc(1);
    dwarfsmith.cursor_item = Some(ItemId(50));
    world.add_character(dwarfsmith);
    let mut silver = silver_stack(50, 5000);
    silver.carried_by = Some(CharacterId(1));
    world.add_item(silver);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 1 (no mold given yet) is outside the ==2 acceptance window.
    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 1, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, DwarfsmithOutcomeEvent::GrantEliteKey { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("need a mold first")));
}

#[test]
fn text_reset_me_wipes_state_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfsmith_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 2, 1), 1);
    assert!(events.contains(&DwarfsmithOutcomeEvent::ResetDwarfsmith {
        player_id: CharacterId(2),
    }));
}

#[test]
fn text_repeat_is_a_documented_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfsmith_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(dwarfsmith) = world.characters.get_mut(&CharacterId(1)) {
        dwarfsmith.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_dwarfsmith_actions(&facts(CharacterId(2), 1, 0), 1);
    assert!(events.is_empty());
}
