use crate::character_driver::{
    apply_lab4_gnalb_create_message, Lab4GnalbDriverData, CDR_LAB4GNALB, NT_CREATE, NT_GIVE,
};

use super::*;

fn gnalb_npc(id: u32) -> Character {
    let mut gnalb = character(id);
    gnalb.name = "Patrol Guard".into();
    gnalb.driver = CDR_LAB4GNALB;
    gnalb
}

fn gnalb_state(world: &World, id: CharacterId) -> Lab4GnalbDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab4Gnalb(data)) => data,
        _ => panic!("expected lab4 gnalb driver state"),
    }
}

#[test]
fn apply_lab4_gnalb_create_message_parses_type_and_pushes_nt_create() {
    let mut character = character(1);
    apply_lab4_gnalb_create_message(&mut character, Some("type=1;"));

    let Some(CharacterDriverState::Lab4Gnalb(data)) = character.driver_state else {
        panic!("expected lab4 gnalb driver state");
    };
    assert_eq!(data.gnalb_type, 1);
    assert!(!data.aggressive);
    assert!(!data.helper);
    assert_eq!(
        character
            .driver_messages
            .iter()
            .filter(|message| message.message_type == NT_CREATE)
            .count(),
        1
    );
}

#[test]
fn apply_lab4_gnalb_create_message_no_args_defaults_to_type_0() {
    let mut character = character(1);
    apply_lab4_gnalb_create_message(&mut character, None);
    let Some(CharacterDriverState::Lab4Gnalb(data)) = character.driver_state else {
        panic!("expected lab4 gnalb driver state");
    };
    assert_eq!(data.gnalb_type, 0);
}

#[test]
fn nt_create_type_1_finds_nearest_path_node() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    apply_lab4_gnalb_create_message(&mut gnalb, Some("type=1;"));
    // Node 1 is at (52, 228); spawn far enough away (`map_dist >= 4`) that
    // the patrol logic doesn't immediately trigger an arrival branch-pick
    // in the same tick, so `path` still reflects the raw nearest-node
    // lookup afterward.
    assert!(world.spawn_character(gnalb, 55, 228));

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert_eq!(state.path, 1);
}

#[test]
fn type_1_init_sets_aggressive_and_helper() {
    // C `lab4_gnalb_driver_init`'s `type==1` branch sets both
    // `aggressive`/`helper` unconditionally, independent of the nearest-
    // path-node lookup (`lab4.c:454-458`).
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    apply_lab4_gnalb_create_message(&mut gnalb, Some("type=1;"));
    assert!(world.spawn_character(gnalb, 55, 228));

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert!(state.aggressive);
    assert!(state.helper);
}

#[test]
fn type_3_and_others_leave_aggressive_and_helper_false() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    apply_lab4_gnalb_create_message(&mut gnalb, Some("type=3;"));
    assert!(world.spawn_character(gnalb, 20, 20));

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert_eq!(state.path, 0);
    assert!(!state.aggressive);
    assert!(!state.helper);
}

#[test]
fn nt_give_destroys_cursor_item_unconditionally() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 3,
        ..Default::default()
    }));
    gnalb.cursor_item = Some(ItemId(50));
    world.add_character(gnalb);
    let mut junk = item(50, ItemFlags::empty());
    junk.carried_by = Some(CharacterId(1));
    world.add_item(junk);

    if let Some(gnalb) = world.characters.get_mut(&CharacterId(1)) {
        gnalb.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_lab4_gnalb_actions(1);

    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn type_1_gothit_tracks_attacker_and_attacks() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 1,
        aggressive: true,
        helper: true,
        ..Default::default()
    }));
    gnalb.x = 20;
    gnalb.y = 20;
    gnalb.group = 1;
    world.add_character(gnalb);

    let mut attacker = character(2);
    attacker.x = 20;
    attacker.y = 21;
    attacker.group = 2;
    attacker.flags |= CharacterFlags::PLAYER;
    world.add_character(attacker);

    world.map.tile_mut(20, 20).unwrap().light = 255;
    world.map.tile_mut(20, 21).unwrap().light = 255;

    if let Some(gnalb) = world.characters.get_mut(&CharacterId(1)) {
        gnalb.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert_eq!(state.victim, Some(CharacterId(2)));
}

#[test]
fn patrol_arrival_picks_a_valid_neighbor_and_updates_lastpath() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    // Node 1 = (52, 228), next = [45, 2, 0, 0]: place the gnalb 1 tile
    // away (matching `swap_move_driver`'s own `min_dist=1`, so the walk
    // attempt reports "already close enough" instead of queuing a move)
    // with `map_dist(...) == 2 < 4`, so the arrival branch-pick fires
    // immediately.
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 1,
        aggressive: true,
        helper: true,
        path: 1,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnalb, 53, 228));

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert_eq!(state.lastpath, 1);
    assert!(state.path == 45 || state.path == 2);
}

#[test]
fn patrol_never_reverses_to_lastpath_at_a_branch_point() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    // Node 1 = (52, 228), next = [45, 2, 0, 0]: arrive with `lastpath`
    // already set to `2`, so the retry loop must never re-pick `2`
    // (leaving `45` as the only valid choice, deterministically).
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 1,
        aggressive: true,
        helper: true,
        path: 1,
        lastpath: 2,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnalb, 53, 228));

    world.process_lab4_gnalb_actions(1);

    let state = gnalb_state(&world, CharacterId(1));
    assert_eq!(state.lastpath, 1);
    assert_eq!(state.path, 45);
}

#[test]
fn type_3_crazy_gnalb_always_acts_and_never_panics() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 3,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnalb, 20, 20));

    let acted = world.process_lab4_gnalb_actions(1);
    assert_eq!(acted, 1);
}

#[test]
fn default_type_falls_back_to_idle_wander() {
    let mut world = World::default();
    let mut gnalb = gnalb_npc(1);
    gnalb.driver_state = Some(CharacterDriverState::Lab4Gnalb(Lab4GnalbDriverData {
        gnalb_type: 5,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnalb, 20, 20));

    // Should not panic; either walks toward its own rest position or idles.
    world.process_lab4_gnalb_actions(1);
}
