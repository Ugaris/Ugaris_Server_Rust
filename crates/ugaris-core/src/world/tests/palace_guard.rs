use super::*;
use crate::character_driver::{
    parse_palace_guard_driver_args, PalaceGuardDriverData, CDR_PALACEGUARD, NTID_PALACE_ALERT,
    NT_CHAR, NT_GOTHIT, NT_NPC,
};
use crate::item_driver::IDR_PALACECAP;

/// C `WN_HEAD` worn-slot index (0-based) - see `world::npc::area11::
/// palace_guard`'s own local copy of this constant.
const WN_HEAD: usize = 1;

fn palace_guard_npc(id: u32) -> Character {
    let mut guard = character(id);
    guard.name = "Ice Demon".into();
    guard.driver = CDR_PALACEGUARD;
    guard.group = 3;
    guard
}

fn palace_guard_state(world: &World, id: CharacterId) -> PalaceGuardDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::PalaceGuard(data)) => data,
        _ => panic!("expected palace guard driver state"),
    }
}

#[test]
fn parse_palace_guard_driver_args_parses_patrol_pairs_and_flags() {
    // C `palace_guard_parse` (`palace.c:103-136`), same shape as
    // `zones/11/palace.chr`'s `palace_guard1` template.
    let data = parse_palace_guard_driver_args(
        "patrol=1;patrolx=155;patroly=111;patrolx=140;patroly=100;scream=1;",
    );
    assert_eq!(data.patrol, 1);
    assert_eq!(data.scream, 1);
    assert_eq!(data.patrolx[0], 155);
    assert_eq!(data.patroly[0], 111);
    assert_eq!(data.patrolx[1], 140);
    assert_eq!(data.patroly[1], 100);
    assert_eq!(data.patrolx[2], 0);
    assert_eq!(data.patroly[2], 0);
}

#[test]
fn parse_palace_guard_driver_args_parses_reserve() {
    let data = parse_palace_guard_driver_args("reserve=1;patrolx=37;patroly=52;");
    assert_eq!(data.reserve, 1);
    assert_eq!(data.patrolx[0], 37);
    assert_eq!(data.patroly[0], 52);
}

#[test]
fn aggressive_guard_adds_visible_attackable_target_as_victim_from_sighting() {
    // C `standard_message_driver(cn, msg, 1, 1)`, the branch taken when
    // `dat->scream` is unset (`palace.c:217-218`) - most reserve/patrol
    // demon templates in `zones/11/palace.chr` run without `scream=1`, so
    // any valid enemy sighted via `NT_CHAR` is added unconditionally.
    let mut world = World::default();
    let guard = palace_guard_npc(1);
    assert!(world.spawn_character(guard, 10, 10));
    let mut player = character(2);
    player.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(player, 11, 10));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_palace_guard_actions(1);

    assert_eq!(
        palace_guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn screaming_guard_does_not_auto_aggro_from_sighting_but_still_screams() {
    // C `standard_message_driver(cn, msg, 0, 0)`, the branch taken when
    // `dat->scream` is set (`palace.c:215-216`): `NT_CHAR` sighting alone
    // never adds an enemy this way; only the explicit `dat->scream`
    // distance/cooldown block (`palace.c:195-200`) reacts, by shouting an
    // area alert instead of attacking directly.
    let mut world = World::default();
    world.tick.0 = TICKS_PER_SECOND * 1000; // past a fresh `lastfight = 0`'s 20s cooldown.
    let mut guard = palace_guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(PalaceGuardDriverData {
        scream: 1,
        ..Default::default()
    }));
    assert!(world.spawn_character(guard, 10, 10));
    let mut player = character(2);
    player.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(player, 11, 10));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_palace_guard_actions(1);

    let data = palace_guard_state(&world, CharacterId(1));
    assert_eq!(data.victim, None);
    assert_eq!(data.lastfight, world.tick.0);
}

#[test]
fn gothit_self_defense_sets_victim_even_when_screaming() {
    // C `standard_message_driver`'s own `NT_GOTHIT` case is unconditional
    // regardless of the `agressive`/`helper` params (`drvlib.c:2512-
    // 2538`).
    let mut world = World::default();
    let mut guard = palace_guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(PalaceGuardDriverData {
        scream: 1,
        ..Default::default()
    }));
    assert!(world.spawn_character(guard, 10, 10));
    let mut attacker = character(2);
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let acted = world.process_palace_guard_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        palace_guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn palace_cap_wearer_is_immune_and_clears_existing_victim_tracking() {
    // C `palace_guard`'s `NT_CHAR` cap-immunity short-circuit
    // (`palace.c:169-174`).
    let mut world = World::default();
    let mut guard = palace_guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(PalaceGuardDriverData {
        victim: Some(CharacterId(2)),
        ..Default::default()
    }));
    assert!(world.spawn_character(guard, 10, 10));
    let mut player = character(2);
    player.flags |= CharacterFlags::PLAYER;
    player.inventory[WN_HEAD] = Some(ItemId(50));
    assert!(world.spawn_character(player, 11, 10));
    let mut cap = item(50, ItemFlags::empty());
    cap.driver = IDR_PALACECAP;
    cap.driver_data = vec![1];
    cap.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(50), cap);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_palace_guard_actions(1);

    assert_eq!(palace_guard_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn inactive_palace_cap_does_not_grant_immunity() {
    // C's cap check also requires `it[in].drdata[0]` truthy
    // (`palace.c:170`) - a carried-but-inactive cap does not protect.
    let mut world = World::default();
    let guard = palace_guard_npc(1);
    assert!(world.spawn_character(guard, 10, 10));
    let mut player = character(2);
    player.flags |= CharacterFlags::PLAYER;
    player.inventory[WN_HEAD] = Some(ItemId(50));
    assert!(world.spawn_character(player, 11, 10));
    let mut cap = item(50, ItemFlags::empty());
    cap.driver = IDR_PALACECAP;
    cap.driver_data = vec![0];
    cap.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(50), cap);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_palace_guard_actions(1);

    assert_eq!(
        palace_guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn reserve_guard_walks_toward_scout_alert() {
    // C's reserve-scout `NT_NPC`/`NTID_PALACE_ALERT` handler
    // (`palace.c:205-212`) plus the `doscout`-walk block (`palace.c:254-
    // 284`).
    let mut world = World::default();
    world.tick.0 = TICKS_PER_SECOND * 1000; // nonzero, so `doscout = tick.0` is a real sentinel.
    let mut guard = palace_guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(PalaceGuardDriverData {
        reserve: 1,
        ..Default::default()
    }));
    assert!(world.spawn_character(guard, 10, 10));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_NPC, NTID_PALACE_ALERT, 20, 20);
    }

    let acted = world.process_palace_guard_actions(1);

    assert_eq!(acted, 1);
    let data = palace_guard_state(&world, CharacterId(1));
    assert_eq!(data.dox, 20);
    assert_eq!(data.doy, 20);
    assert_ne!(data.doscout, 0);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(guard.action, action::WALK);
}

#[test]
fn non_reserve_guard_ignores_scout_alert() {
    // C `if (dat->reserve && msg->dat1 == NTID_PALACE_ALERT)`
    // (`palace.c:205`): the two active patrol guards themselves
    // (`reserve` unset) never react to the broadcast they emit.
    let mut world = World::default();
    let guard = palace_guard_npc(1);
    assert!(world.spawn_character(guard, 10, 10));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_NPC, NTID_PALACE_ALERT, 20, 20);
    }

    world.process_palace_guard_actions(1);

    let data = palace_guard_state(&world, CharacterId(1));
    assert_eq!(data.doscout, 0);
}

#[test]
fn patrol_guard_walks_toward_first_waypoint() {
    // C `if (dat->patrol || dat->docheck) { ... move_driver(...
    // patrolx[pat] ...) }` (`palace.c:324-339`).
    let mut world = World::default();
    let mut guard = palace_guard_npc(1);
    let mut data = PalaceGuardDriverData {
        patrol: 1,
        ..Default::default()
    };
    data.patrolx[0] = 50;
    data.patroly[0] = 50;
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(data));
    assert!(world.spawn_character(guard, 10, 10));

    let acted = world.process_palace_guard_actions(1);

    assert_eq!(acted, 1);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(guard.action, action::WALK);
}

#[test]
fn patrol_guard_advances_waypoint_index_on_arrival() {
    let mut world = World::default();
    let mut guard = palace_guard_npc(1);
    let mut data = PalaceGuardDriverData {
        patrol: 1,
        ..Default::default()
    };
    data.patrolx[0] = 10;
    data.patroly[0] = 11;
    data.patrolx[1] = 50;
    data.patroly[1] = 50;
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(data));
    // already standing within 2 tiles of waypoint 0.
    assert!(world.spawn_character(guard, 10, 10));

    world.process_palace_guard_actions(1);

    assert_eq!(palace_guard_state(&world, CharacterId(1)).pat, 1);
}

#[test]
fn stationary_reserve_guard_without_route_walks_home_when_displaced() {
    // C's `else` branch (`palace.c:340-348`): no `patrol`/`docheck`, so
    // the guard just walks back to its own `tmpx`/`tmpy` rest position.
    let mut world = World::default();
    let mut guard = palace_guard_npc(1);
    guard.rest_x = 10;
    guard.rest_y = 10;
    guard.driver_state = Some(CharacterDriverState::PalaceGuard(
        PalaceGuardDriverData::default(),
    ));
    assert!(world.spawn_character(guard, 15, 15));

    let acted = world.process_palace_guard_actions(1);

    assert_eq!(acted, 1);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(guard.action, action::WALK);
}
