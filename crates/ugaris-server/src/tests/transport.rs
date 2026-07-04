use super::*;

#[test]
fn transport_travel_moves_to_seen_same_area_destination() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(300, 300);
    let login = login_block("Ralph");
    assert!(world.spawn_character(login_character(CharacterId(1), &login, 1, 10, 10), 10, 10));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.touch_transport(0);

    let result = apply_transport_travel(&mut world, &player, CharacterId(1), 1, 1 + 2 * 256);

    assert_eq!(
        result,
        TransportTravelResult::SameArea {
            x: 139,
            y: 75,
            mirror: 2
        }
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (139, 75));
}

#[test]
fn transport_travel_rejects_unseen_destination_with_legacy_text() {
    let world = World::default();
    let player = PlayerRuntime::connected(1, 0);

    let result = resolve_transport_travel(&world, &player, CharacterId(1), 1, 1);

    assert_eq!(
        result,
        TransportTravelResult::Blocked(
            "You've never been to Cameron before. You cannot go there.".to_string()
        )
    );
}

#[test]
fn transport_travel_keeps_cross_area_as_handoff_boundary() {
    let world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.touch_transport(2);

    let result = resolve_transport_travel(&world, &player, CharacterId(1), 1, 3 + 4 * 256);

    assert_eq!(
        result,
        TransportTravelResult::CrossArea {
            area: 3,
            x: 129,
            y: 201,
            mirror: 4,
        }
    );
}

#[test]
fn transport_travel_randomizes_invalid_mirror_like_c() {
    let world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.touch_transport(2);

    let low = resolve_transport_travel_with_random(&world, &player, CharacterId(1), 1, 3, |_| 7);
    let high = resolve_transport_travel_with_random(
        &world,
        &player,
        CharacterId(1),
        1,
        3 + 27 * 256,
        |_| 25,
    );

    assert_eq!(
        low,
        TransportTravelResult::CrossArea {
            area: 3,
            x: 129,
            y: 201,
            mirror: 8,
        }
    );
    assert_eq!(
        high,
        TransportTravelResult::CrossArea {
            area: 3,
            x: 129,
            y: 201,
            mirror: 26,
        }
    );
}

#[test]
fn transport_travel_clamps_injected_random_mirror_roll() {
    let world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.touch_transport(2);

    let result =
        resolve_transport_travel_with_random(&world, &player, CharacterId(1), 1, 3, |_| 99);

    assert_eq!(
        result,
        TransportTravelResult::CrossArea {
            area: 3,
            x: 129,
            y: 201,
            mirror: 26,
        }
    );
}

#[test]
fn transport_clan_access_marks_direct_member_byte() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
    character.clan = 17;
    world.add_character(character);

    assert_eq!(transport_clan_access(&world, CharacterId(1)), [0, 0, 1, 0]);
}

#[test]
fn transport_clan_travel_uses_legacy_hall_coordinates() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(300, 300);
    let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
    character.clan = 17;
    assert!(world.spawn_character(character, 10, 10));
    let player = PlayerRuntime::connected(1, 0);

    let result = apply_transport_travel(&mut world, &player, CharacterId(1), 3, 81 + 2 * 256);

    assert_eq!(
        result,
        TransportTravelResult::SameArea {
            x: 28,
            y: 58,
            mirror: 2,
        }
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (28, 58));
}

/// `may_enter_clan` (`transport.rs`) is wired to `ClanRelations::may_enter`
/// (`clan.c:881-905`, called from `transport.c:185-223`): an allied clan's
/// hall must be reachable even though the traveler is not a direct member,
/// while a merely-neutral clan's hall must stay blocked.
#[test]
fn transport_clan_travel_allows_allied_clan_hall_not_just_direct_member() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(300, 300);
    let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
    character.clan = 1;
    assert!(world.spawn_character(character, 10, 10));

    let relations = world.clan_registry.relations_mut();
    relations.found_clan(1, 0);
    relations.found_clan(17, 0);
    relations
        .set_relation(17, 1, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(1, 17, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);

    let player = PlayerRuntime::connected(1, 0);

    // clan hall 17 (`nr = 81`, `81 - 63 = 18` -> hall index 17, 1-based clan 17)
    let result = apply_transport_travel(&mut world, &player, CharacterId(1), 3, 81 + 2 * 256);

    assert_eq!(
        result,
        TransportTravelResult::SameArea {
            x: 28,
            y: 58,
            mirror: 2,
        }
    );
}

#[test]
fn transport_clan_travel_blocks_merely_neutral_clan_hall() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
    character.clan = 1;
    world.add_character(character);

    let relations = world.clan_registry.relations_mut();
    relations.found_clan(1, 0);
    relations.found_clan(17, 0);
    // left at the default Neutral relation - no Alliance set.

    let player = PlayerRuntime::connected(1, 0);

    let result = resolve_transport_travel(&world, &player, CharacterId(1), 3, 81);

    assert_eq!(
        result,
        TransportTravelResult::Blocked("You may not enter (17).".to_string())
    );
}

#[test]
fn transport_clan_travel_rejects_non_member_with_legacy_text() {
    let world = World::default();
    let player = PlayerRuntime::connected(1, 0);

    let result = resolve_transport_travel(&world, &player, CharacterId(1), 3, 65);

    assert_eq!(
        result,
        TransportTravelResult::Blocked("You may not enter (1).".to_string())
    );
}

#[test]
fn character_save_request_persists_runtime_transport_mirror() {
    let login = login_block("Tester");
    let character = login_character(CharacterId(7), &login, 1, 10, 10);
    let mut world = World::default();
    world.add_character(character.clone());
    let mut player = PlayerRuntime::connected(5, 0);
    player.set_current_mirror(9);

    let request = character_save_request(&world, &player, &character, None, 1, 2);

    assert!(matches!(
        request.mode,
        ugaris_db::character::CharacterSaveMode::Logout { mirror: 9, .. }
    ));
}
