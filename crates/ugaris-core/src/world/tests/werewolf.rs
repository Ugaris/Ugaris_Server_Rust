use super::*;
use crate::character_driver::{CDR_SHR_WEREWOLF, CDR_SIMPLEBADDY};

fn werewolf_npc(id: u32) -> Character {
    let mut npc = character(id);
    npc.driver = CDR_SHR_WEREWOLF;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc
}

#[test]
fn full_night_clears_invisibility_and_attacks_a_visible_player_enemy() {
    // C `shr_werewolf_driver`'s full-night branch (`shrike.c:380-384`):
    // `ch[cn].flags &= ~CF_INVISIBLE; char_driver(CDR_SIMPLEBADDY, ...)`.
    let mut world = World::default();
    world.date.moonlight = 1;
    world.date.sunlight = 0;
    world.tick = Tick(TICKS_PER_SECOND * 11);

    let mut npc = werewolf_npc(1);
    npc.flags |= CharacterFlags::INVISIBLE;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        lastfight: 0,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    let mut target = character(2);
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(target, 11, 10);

    let acted = world.process_shr_werewolf_actions_with_random(1, |_| 0);
    assert_eq!(acted, 1);

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!npc.flags.contains(CharacterFlags::INVISIBLE));
    assert_ne!(npc.action, 0, "an attack task should have been queued");
}

#[test]
fn day_sets_invisibility_and_walks_toward_home() {
    // C `shr_werewolf_driver`'s day branch (`shrike.c:386-390`):
    // `ch[cn].flags |= CF_INVISIBLE; secure_move_driver(cn, ch[cn].tmpx,
    // ch[cn].tmpy, DX_DOWN, ret, lastact);`.
    let mut world = World::default();
    world.date.moonlight = 0;
    world.date.sunlight = 255;

    let mut npc = werewolf_npc(1);
    npc.rest_x = 11;
    npc.rest_y = 10;
    world.spawn_character(npc, 10, 10);

    let acted = world.process_shr_werewolf_actions_with_random(1, |_| 0);
    assert_eq!(acted, 1);

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.flags.contains(CharacterFlags::INVISIBLE));
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
}

#[test]
fn day_stays_invisible_without_a_queued_move_once_already_home() {
    let mut world = World::default();
    world.date.moonlight = 0;
    world.date.sunlight = 255;

    let mut npc = werewolf_npc(1);
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.dir = Direction::Down as u8;
    world.spawn_character(npc, 10, 10);

    let acted = world.process_shr_werewolf_actions_with_random(1, |_| 0);
    assert_eq!(acted, 0);

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.flags.contains(CharacterFlags::INVISIBLE));
    assert_eq!(npc.action, 0);
}

#[test]
fn simple_baddy_gate_functions_accept_shr_werewolf_but_batch_sweeps_ignore_it() {
    // `CDR_SHR_WEREWOLF`'s day/night gate lives in `world::npc::
    // area38::werewolf`, not in the shared batch sweeps - those must keep
    // ignoring it (or it would fight/wander during the day too).
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc = werewolf_npc(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    let mut target = character(2);
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(target, 11, 10);

    // The single-character function accepts the driver directly...
    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    // ...but the werewolf must not appear in the shared driver-agnostic
    // batch sweep used by every unconditional `CDR_SIMPLEBADDY` tail-call
    // NPC (`ugaris-server`'s `tick_npc::area22::pass_0`).
    let mut world2 = World::default();
    world2.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc2 = werewolf_npc(1);
    npc2.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    world2.spawn_character(npc2, 10, 10);
    let mut target2 = character(2);
    target2.flags.insert(CharacterFlags::PLAYER);
    world2.spawn_character(target2, 11, 10);
    assert_eq!(
        world2.process_simple_baddy_attack_actions_with_random(1, |_| 0),
        0
    );

    // Sanity: a plain `CDR_SIMPLEBADDY` sibling with the same state *is*
    // picked up by the batch sweep.
    let mut world3 = World::default();
    world3.tick = Tick(TICKS_PER_SECOND * 11);
    let mut plain = werewolf_npc(1);
    plain.driver = CDR_SIMPLEBADDY;
    plain.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    world3.spawn_character(plain, 10, 10);
    let mut target3 = character(2);
    target3.flags.insert(CharacterFlags::PLAYER);
    world3.spawn_character(target3, 11, 10);
    assert_eq!(
        world3.process_simple_baddy_attack_actions_with_random(1, |_| 0),
        1
    );
}
