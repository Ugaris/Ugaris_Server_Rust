use super::*;
use crate::character_driver::{CDR_BRIDGEGUARD, NT_CHAR, NT_NPC};
use crate::world::npc::area37::bridgeguard::BridgeGuardDriverData;

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn guard_npc(id: u32, group: u16, bless: i16) -> Character {
    let mut guard = character(id);
    guard.name = "Bridge Guard".into();
    guard.driver = CDR_BRIDGEGUARD;
    guard.group = group;
    guard.rest_x = 10;
    guard.rest_y = 10;
    guard.values[0][CharacterValue::Bless as usize] = bless;
    guard.driver_state = Some(CharacterDriverState::BridgeGuard(
        BridgeGuardDriverData::default(),
    ));
    guard
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

#[test]
fn talker_guard_greets_low_level_player_with_the_warning() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 10), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_bridgeguard_actions(1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("no place for such inexperienced")));
}

#[test]
fn talker_guard_greets_high_level_player_with_permission_to_pass() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_bridgeguard_actions(1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("mayest pass the bridge")));
}

#[test]
fn silent_guard_with_bless_flag_never_talks() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, 2, 100), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_bridgeguard_actions(1);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn repeated_greeting_within_memory_is_a_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_bridgeguard_actions(1);
    assert!(!world.drain_pending_area_texts().is_empty());

    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_bridgeguard_actions(1);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn npc_char_message_out_of_sight_range_is_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 40, 40));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_bridgeguard_actions(1);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn non_player_of_a_different_group_is_added_as_an_enemy() {
    let mut world = World::default();
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    let mut monster = character(2);
    monster.group = 5;
    assert!(world.spawn_character(monster, 11, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_bridgeguard_actions(1);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    let enemies: Vec<_> = guard
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.iter().map(|e| e.target_id).collect())
        .unwrap_or_default();
    assert!(enemies.contains(&CharacterId(2)));
}

#[test]
fn npc_message_from_a_same_group_ally_adds_the_reported_target_as_an_enemy() {
    // The reported target must actually be visible (or its last-known
    // position must not already coincide with the guard's own tile),
    // otherwise `fight_driver_follow_invisible`'s own "already arrived,
    // nothing here" cleanup would immediately drop it again this same
    // tick - same real C behavior every other "add enemy, then attack it"
    // test in this codebase relies on.
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, 2, 0), 10, 10));
    let mut ally = character(3);
    ally.group = 2;
    world.add_character(ally);
    let mut foe = character(4);
    foe.group = 5;
    assert!(world.spawn_character(foe, 11, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_NPC, 0, 3, 4);
    }
    world.process_bridgeguard_actions(1);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    let enemies: Vec<_> = guard
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.iter().map(|e| e.target_id).collect())
        .unwrap_or_default();
    assert!(enemies.contains(&CharacterId(4)));
}
