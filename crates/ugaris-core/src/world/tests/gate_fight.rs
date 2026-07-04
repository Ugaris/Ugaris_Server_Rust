use super::*;
use crate::character_driver::{
    GateFightDriverData, CDR_GATE_FIGHT, NTID_GATEKEEPER, NT_CREATE, NT_NPC,
};

const SELF_DESTRUCT: u64 = TICKS_PER_SECOND * 60 * 10;

fn fight_npc(id: u32) -> Character {
    let mut fighter = character(id);
    fighter.name = "Gatekeeper".into();
    fighter.driver = CDR_GATE_FIGHT;
    fighter.driver_state = Some(CharacterDriverState::GateFight(
        GateFightDriverData::default(),
    ));
    fighter
}

fn fight_state(world: &World, id: CharacterId) -> GateFightDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::GateFight(data)) => data,
        _ => panic!("expected gate-fight driver state"),
    }
}

#[test]
fn gate_fight_sets_creation_time_from_nt_create() {
    let mut world = World::default();
    assert!(world.spawn_character(fight_npc(1), 10, 10));
    world.tick = Tick(42);
    if let Some(fighter) = world.characters.get_mut(&CharacterId(1)) {
        fighter.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    world.process_gate_fight_actions(1);

    assert_eq!(fight_state(&world, CharacterId(1)).creation_time, 42);
}

#[test]
fn gate_fight_tracks_victim_from_gatekeeper_npc_message() {
    let mut world = World::default();
    assert!(world.spawn_character(fight_npc(1), 10, 10));
    assert!(world.spawn_character(character(7), 30, 30));
    if let Some(fighter) = world.characters.get_mut(&CharacterId(1)) {
        fighter.push_driver_message(NT_NPC, NTID_GATEKEEPER, 7, 0);
    }

    world.process_gate_fight_actions(1);

    assert_eq!(
        fight_state(&world, CharacterId(1)).victim,
        Some(CharacterId(7))
    );
}

#[test]
fn gate_fight_self_destructs_after_ten_minutes() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.driver_state = Some(CharacterDriverState::GateFight(GateFightDriverData {
        creation_time: 0,
        ..GateFightDriverData::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    world.tick = Tick(SELF_DESTRUCT + 1);

    let acted = world.process_gate_fight_actions(1);

    assert_eq!(acted, 1);
    assert!(world.characters.get(&CharacterId(1)).is_none());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thats all folks!")));
}

#[test]
fn gate_fight_does_not_self_destruct_before_ten_minutes() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.driver_state = Some(CharacterDriverState::GateFight(GateFightDriverData {
        creation_time: 0,
        ..GateFightDriverData::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    world.tick = Tick(SELF_DESTRUCT - 1);

    world.process_gate_fight_actions(1);

    assert!(world.characters.get(&CharacterId(1)).is_some());
}

#[test]
fn gate_fight_attacks_adjacent_visible_victim() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.driver_state = Some(CharacterDriverState::GateFight(GateFightDriverData {
        victim: Some(CharacterId(2)),
        ..GateFightDriverData::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    let acted = world.process_gate_fight_actions(1);

    assert_eq!(acted, 1);
    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::ATTACK1);
    assert_eq!(fight_state(&world, CharacterId(1)).victim_visible, true);
}

#[test]
fn gate_fight_walks_toward_visible_but_distant_victim() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.driver_state = Some(CharacterDriverState::GateFight(GateFightDriverData {
        victim: Some(CharacterId(2)),
        ..GateFightDriverData::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    assert!(world.spawn_character(character(2), 13, 10));
    world.map.tile_mut(13, 10).unwrap().light = 255;

    let acted = world.process_gate_fight_actions(1);

    assert_eq!(acted, 1);
    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::WALK);
    assert_eq!((fighter.tox, fighter.toy), (11, 10));
}

#[test]
fn gate_fight_returns_to_post_when_no_victim() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.rest_x = 15;
    fighter.rest_y = 10;
    world.map.tile_mut(15, 10).unwrap().light = 255;
    assert!(world.spawn_character(fighter, 10, 10));

    let acted = world.process_gate_fight_actions(1);

    assert_eq!(acted, 1);
    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::WALK);
    assert_eq!((fighter.tox, fighter.toy), (11, 10));
}

#[test]
fn gate_fight_gives_up_chasing_invisible_victim_once_arrived() {
    let mut world = World::default();
    let mut fighter = fight_npc(1);
    fighter.driver_state = Some(CharacterDriverState::GateFight(GateFightDriverData {
        victim: Some(CharacterId(2)),
        victim_last_x: 10,
        victim_last_y: 10,
        victim_visible: false,
        ..GateFightDriverData::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    // victim character no longer exists (dead/removed) - `process_gate_fight_tick`
    // treats this the same as C's stale/deleted enemy-slot trash.

    world.process_gate_fight_actions(1);

    assert_eq!(fight_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn apply_gate_fight_reward_arch_warrior_success() {
    let mut world = World::default();
    let mut killer = character(2);
    killer.name = "Godmode".into();
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 10, 10));

    let applied = world.apply_gate_fight_reward(CharacterId(2), 5);

    assert!(applied);
    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!(killer.values[1][CharacterValue::Rage as usize], 1);
    assert_eq!((killer.x, killer.y), (181, 198));

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|t| t.message == "Well done."));
    assert!(texts
        .iter()
        .any(|t| t.message == "You are an Arch-Warrior now."));
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 1);
    assert_eq!(broadcasts[0].channel, 6);
    assert!(String::from_utf8_lossy(&broadcasts[0].message_bytes)
        .contains("Grats: Godmode is an Arch-Warrior now!"));
}

#[test]
fn apply_gate_fight_reward_arch_warrior_rejected_when_already_mage() {
    let mut world = World::default();
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER | CharacterFlags::MAGE;
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 5);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!killer.flags.contains(CharacterFlags::ARCH));
    // C's early `return` in the failing case guard skips the teleport too.
    assert_eq!((killer.x, killer.y), (10, 10));
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert!(broadcasts.is_empty());
}

#[test]
fn apply_gate_fight_reward_arch_mage_success() {
    let mut world = World::default();
    let mut killer = character(2);
    killer.name = "Godmode".into();
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 6);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!(killer.values[1][CharacterValue::Duration as usize], 1);
    assert_eq!((killer.x, killer.y), (181, 198));
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert!(String::from_utf8_lossy(&broadcasts[0].message_bytes)
        .contains("Grats: Godmode is an Arch-Mage now!"));
}

#[test]
fn apply_gate_fight_reward_arch_seyandu_requires_both_classes() {
    let mut world = World::default();
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER | CharacterFlags::WARRIOR;
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 7);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!((killer.x, killer.y), (10, 10));
}

#[test]
fn apply_gate_fight_reward_arch_seyandu_success() {
    let mut world = World::default();
    let mut killer = character(2);
    killer.name = "Godmode".into();
    killer.flags |= CharacterFlags::PLAYER | CharacterFlags::WARRIOR | CharacterFlags::MAGE;
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 7);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!((killer.x, killer.y), (181, 198));
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert!(String::from_utf8_lossy(&broadcasts[0].message_bytes)
        .contains("Grats: Godmode is an Arch-Seyan'Du now!"));
}

#[test]
fn apply_gate_fight_reward_seyandu_class_still_teleports_without_reroll() {
    // C `case 8` calls the still-unported `turn_seyan` and always falls
    // through to the teleport (no early `return`, unlike cases 5-7's
    // guards) - see `world::gate_fight`'s module doc comment for the
    // documented gap.
    let mut world = World::default();
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 8);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!((killer.x, killer.y), (181, 198));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("not supported on this server build yet")));
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert!(broadcasts.is_empty());
}

#[test]
fn apply_gate_fight_reward_unmatched_class_still_teleports() {
    let mut world = World::default();
    let killer = character(2);
    assert!(world.spawn_character(killer, 10, 10));

    world.apply_gate_fight_reward(CharacterId(2), 0);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((killer.x, killer.y), (181, 198));
}
