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

// `process_lostcon_messages` (C `lostcon_driver`'s per-message loop,
// `src/module/lostcon.c:117-141`).

#[test]
fn process_lostcon_messages_notes_hit_and_adds_the_attacker_as_an_enemy() {
    let mut world = World::default();
    world.tick = Tick(42);
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_GOTHIT, 2, 0, 0);
    assert!(world.spawn_character(lingering, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.driver_messages.is_empty());
    let data = character
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 42);
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(2));
    assert_eq!(data.enemies[0].priority, 1);
    assert!(data.enemies[0].visible);
    assert_eq!(data.enemies[0].last_x, 11);
    assert_eq!(data.enemies[0].last_y, 11);
}

#[test]
fn process_lostcon_messages_notes_hit_without_an_attacker_id() {
    // C: `fight_driver_note_hit(cn)` always runs on `NT_GOTHIT`; the
    // `fight_driver_add_enemy` call is skipped when `msg->dat1` (`co`) is
    // `0`.
    let mut world = World::default();
    world.tick = Tick(7);
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_GOTHIT, 0, 0, 0);
    assert!(world.spawn_character(lingering, 10, 10));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    let data = character
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 7);
    assert!(data.enemies.is_empty());
}

#[test]
fn process_lostcon_messages_ignores_sighting_and_text_messages() {
    // C's own message loop leaves `NT_CHAR`'s aggro-on-sight commented out
    // and `NT_TEXT` is a no-op comment - neither message type should touch
    // `fight_driver` at all.
    let mut world = World::default();
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_CHAR, 2, 0, 0);
    lingering.push_driver_message(NT_TEXT, 1, 0, 1);
    assert!(world.spawn_character(lingering, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.driver_messages.is_empty());
    assert!(character.fight_driver.is_none());
}

#[test]
fn process_lostcon_messages_is_a_no_op_for_a_normal_playing_character() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.push_driver_message(NT_GOTHIT, 2, 0, 0);
    assert!(world.spawn_character(player, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver_messages.len(), 1);
    assert!(character.fight_driver.is_none());
}
