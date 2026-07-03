use super::*;

fn player_character(id: u32) -> Character {
    let mut character = character(id);
    character.flags |= CharacterFlags::PLAYER;
    character
}

#[test]
fn enter_lostcon_sets_driver_and_arms_deadline() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world.enter_lostcon(CharacterId(1), 7_200));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver, CDR_LOSTCON);
    assert_eq!(
        character.driver_state,
        Some(CharacterDriverState::Lostcon(LostconDriverData {
            deadline: 7_200
        }))
    );
    assert!(world.is_lostcon(CharacterId(1)));
}

#[test]
fn enter_lostcon_returns_false_for_missing_character() {
    let mut world = World::default();
    assert!(!world.enter_lostcon(CharacterId(99), 100));
}

#[test]
fn lingering_character_stays_on_the_map_and_is_attackable() {
    // C `kick_player` does not call `remove_char`/`exit_char` on
    // disconnect; the character stays fully live until the lagout timer
    // expires or it is reclaimed.
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 7_200);

    assert!(world.characters.contains_key(&CharacterId(1)));
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.character, 1);
}

#[test]
fn reclaim_lostcon_clears_driver_and_state() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 7_200);

    assert!(world.reclaim_lostcon(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver, 0);
    assert_eq!(character.driver_state, None);
    assert!(!world.is_lostcon(CharacterId(1)));
}

#[test]
fn reclaim_lostcon_is_a_no_op_when_not_lingering() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(!world.reclaim_lostcon(CharacterId(1)));
    assert!(!world.reclaim_lostcon(CharacterId(404)));
}

#[test]
fn expired_lostcon_characters_matches_deadline_and_driver() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    assert!(world.spawn_character(player_character(2), 11, 11));
    assert!(world.spawn_character(player_character(3), 12, 12));
    world.enter_lostcon(CharacterId(1), 100);
    world.enter_lostcon(CharacterId(2), 200);
    // Character 3 never disconnected: still player-controlled.

    let expired = world.expired_lostcon_characters(150);
    assert_eq!(expired, vec![CharacterId(1)]);

    let mut expired = world.expired_lostcon_characters(200);
    expired.sort_by_key(|id| id.0);
    assert_eq!(expired, vec![CharacterId(1), CharacterId(2)]);

    let expired = world.expired_lostcon_characters(50);
    assert!(expired.is_empty());
}

#[test]
fn expired_lostcon_characters_ignores_reclaimed_characters() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 100);
    world.reclaim_lostcon(CharacterId(1));

    assert!(world.expired_lostcon_characters(200).is_empty());
}
