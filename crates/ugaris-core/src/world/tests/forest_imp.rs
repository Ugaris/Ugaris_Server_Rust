// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use std::collections::HashMap;

use super::*;
use crate::character_driver::{ForestImpDriverData, CDR_FORESTIMP, NT_CHAR, NT_GIVE, NT_SEEHIT};
use crate::world::npc::area16::imp::{ForestImpOutcomeEvent, ForestImpPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn imp_npc(id: u32) -> Character {
    let mut imp = character(id);
    imp.name = "Imp".into();
    imp.driver = CDR_FORESTIMP;
    imp.driver_state = Some(CharacterDriverState::ForestImp(
        ForestImpDriverData::default(),
    ));
    imp
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    imp_state: i32,
    hermit_state: i32,
    quest23_done: bool,
    has_hardkill_item: bool,
    hardkill_ritual_progress: u8,
) -> HashMap<CharacterId, ForestImpPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ForestImpPlayerFacts {
            imp_state,
            hermit_state,
            quest23_done,
            has_hardkill_item,
            hardkill_ritual_progress,
        },
    );
    map
}

fn imp_state(world: &World, imp_id: CharacterId) -> ForestImpDriverData {
    match world
        .characters
        .get(&imp_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ForestImp(data)) => data,
        _ => panic!("expected forest imp driver state"),
    }
}

#[test]
fn imp_greets_new_player_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 0, 0, false, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("A human. Now this is interesting")));
    let data = imp_state(&world, CharacterId(1));
    assert_eq!(data.current_victim, Some(CharacterId(2)));
    // C `dat->mode = 1; dat->backtime = ticker + TICKS*8;`
    // (`forest.c:351-352`) - the imp reveals itself while talking.
    assert_eq!(data.mode, 1);
}

#[test]
fn imp_state3_completes_bear_hunt_and_resets_kills() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 3, 0, false, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert!(events.contains(&ForestImpOutcomeEvent::ResetImpKills {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ForestImpOutcomeEvent::QuestDoneBearHunt {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Nicely done")));
}

#[test]
fn imp_state4_sends_player_back_to_william() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 4, 0, false, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateWilliamState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
}

#[test]
fn imp_state5_with_quest23_done_jumps_silently_to_state11() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 5, 0, true, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn imp_state5_with_quest23_pending_is_a_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 5, 0, false, false, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn imp_state7_falls_through_to_state8_hint_when_hermit_state_gt3_no_weapon() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // `hermit_state == 4`, no hardkill item at all - `case 8`'s "no item"
    // hint branch.
    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 7, 4, false, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou needst a holy weapon")));
}

#[test]
fn imp_state7_hint_mentions_another_stone_circle_when_weapon_underpowered() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Has the hardkill weapon but ritual progress is below the 38
    // threshold - `case 8`'s "has item" hint branch.
    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 7, 4, false, true, 10), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("another stone circle")));
}

#[test]
fn imp_state7_stays_put_when_hermit_state_not_past_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 7, 3, false, false, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn imp_state9_falls_through_to_state10_treasure_hint_when_hermit_state_gt4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 9, 5, false, false, 0), 1);
    assert!(events.contains(&ForestImpOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("truly worthy")));
}

#[test]
fn imp_state9_stays_put_when_hermit_state_not_past_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(imp_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 9, 4, false, false, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn imp_give_message_silently_destroys_any_item() {
    let mut world = World::default();
    let mut imp = imp_npc(1);
    imp.cursor_item = Some(ItemId(50));
    world.add_character(imp);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forest_imp_actions(&facts(CharacterId(2), 4, 0, false, false, 0), 1);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn imp_seehit_heals_low_hp_player_and_reveals_itself_when_random_hits() {
    let mut world = World::default();
    // Seed 3 -> `legacy_random_below_from_seed(_, 4) == 0`, the 1-in-4
    // chance the C `!RANDOM(4)` guard needs to fire.
    world.legacy_random_seed = 3;

    let mut imp = imp_npc(1);
    imp.flags.insert(CharacterFlags::INVISIBLE);
    imp.mana = 10 * POWERSCALE;
    imp.values[0][CharacterValue::Heal as usize] = 40;
    world.add_character(imp);

    let mut victim = player(2, "Godmode");
    victim.values[1][CharacterValue::Hp as usize] = 30;
    victim.values[0][CharacterValue::Hp as usize] = 30;
    victim.hp = 5 * POWERSCALE;
    world.add_character(victim);

    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_SEEHIT, 0, 2, 0);
    }

    world.process_forest_imp_actions(&facts(CharacterId(2), 11, 0, false, false, 0), 1);

    let imp_char = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!imp_char.flags.contains(CharacterFlags::INVISIBLE));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Don't die, dear human")));
}

#[test]
fn imp_seehit_does_nothing_when_random_misses() {
    let mut world = World::default();
    // Seed 0 -> `legacy_random_below_from_seed(_, 4) == 1`, missing the
    // 1-in-4 chance.
    world.legacy_random_seed = 0;

    let mut imp = imp_npc(1);
    imp.flags.insert(CharacterFlags::INVISIBLE);
    imp.mana = 10 * POWERSCALE;
    imp.values[0][CharacterValue::Heal as usize] = 40;
    world.add_character(imp);

    let mut victim = player(2, "Godmode");
    victim.values[1][CharacterValue::Hp as usize] = 30;
    victim.values[0][CharacterValue::Hp as usize] = 30;
    victim.hp = 5 * POWERSCALE;
    world.add_character(victim);

    if let Some(imp) = world.characters.get_mut(&CharacterId(1)) {
        imp.push_driver_message(NT_SEEHIT, 0, 2, 0);
    }

    world.process_forest_imp_actions(&facts(CharacterId(2), 11, 0, false, false, 0), 1);

    let imp_char = world.characters.get(&CharacterId(1)).unwrap();
    assert!(imp_char.flags.contains(CharacterFlags::INVISIBLE));
    assert!(world.drain_pending_area_texts().is_empty());
}
