use super::*;
use crate::character_driver::{
    CharacterDriverMessage, SuperiorDriverData, CDR_SUPERIOR, NT_CHAR, NT_TEXT,
};
use crate::world::npc::area2::superior::{SUPERIOR_MODE_FIGHT, SUPERIOR_MODE_RUN};

fn superior_npc(id: u32, nr: i32) -> Character {
    let mut superior = character(id);
    superior.name = "Nazimah".into();
    superior.driver = CDR_SUPERIOR;
    superior.driver_state = Some(CharacterDriverState::Superior(SuperiorDriverData {
        nr,
        mode: SUPERIOR_MODE_FIGHT,
        ..Default::default()
    }));
    superior
}

fn superior_state(world: &World, id: CharacterId) -> SuperiorDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Superior(data)) => data,
        _ => panic!("expected superior driver state"),
    }
}

#[test]
fn superior_is_stunned_when_true_name_is_said_nearby() {
    // C `area2.c:106-107`: `strcasestr(msg->dat2, "Nazimah") && dat->nr == 1`.
    let mut world = World::default();
    let superior = superior_npc(1, 1);
    assert!(world.spawn_character(superior, 10, 10));
    let mut speaker = character(2);
    speaker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(speaker, 11, 10));
    world.tick.0 = 1000;
    if let Some(superior) = world.characters.get_mut(&CharacterId(1)) {
        superior.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 1,
            dat2: 0,
            dat3: 2,
            text: Some("Oh great Nazimah, hear me!".to_string()),
        });
    }

    world.process_superior_actions(1);

    let state = superior_state(&world, CharacterId(1));
    assert_eq!(state.stun, 1000 + 60 * TICKS_PER_SECOND);
    let superior = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(superior.speed_mode, SpeedMode::Stealth);
    assert_eq!(superior.action, action::IDLE);
}

#[test]
fn superior_true_name_match_is_case_insensitive_and_gated_by_nr() {
    let mut world = World::default();
    // nr == 2 wants "Argatoth", not "Nazimah" - saying the wrong guardian's
    // name must not stun this one.
    let superior = superior_npc(1, 2);
    assert!(world.spawn_character(superior, 10, 10));
    if let Some(superior) = world.characters.get_mut(&CharacterId(1)) {
        superior.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 1,
            dat2: 0,
            dat3: 99,
            text: Some("NAZIMAH!".to_string()),
        });
    }

    world.process_superior_actions(1);

    assert_eq!(superior_state(&world, CharacterId(1)).stun, 0);
}

#[test]
fn superior_tracks_victim_from_char_sighting_and_attacks_when_adjacent() {
    // C `standard_message_driver(cn, msg, 1, 0)` with `aggressive=1`: any
    // valid enemy seen via `NT_CHAR` becomes the tracked victim.
    let mut world = World::default();
    let mut superior = superior_npc(1, 1);
    superior.group = 0;
    superior.values[0][CharacterValue::Hp as usize] = 100;
    superior.hp = 100 * POWERSCALE;
    superior.lifeshield = 20 * POWERSCALE;
    assert!(world.spawn_character(superior, 10, 10));
    let mut enemy = character(2);
    enemy.group = 1;
    enemy.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(enemy, 11, 10));
    if let Some(superior) = world.characters.get_mut(&CharacterId(1)) {
        superior.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }

    world.process_superior_actions(1);

    assert_eq!(
        superior_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let superior = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(superior.action, action::ATTACK1);
}

#[test]
fn superior_switches_to_run_mode_below_ten_hp() {
    // C `area2.c:135-138`: `hp < 10*POWERSCALE || lifeshield < POWERSCALE*5`.
    let mut world = World::default();
    let mut superior = superior_npc(1, 1);
    superior.hp = 5 * POWERSCALE;
    superior.values[0][CharacterValue::Hp as usize] = 100;
    superior.values[0][CharacterValue::Mana as usize] = 50;
    superior.values[0][CharacterValue::MagicShield as usize] = 20;
    assert!(world.spawn_character(superior, 10, 10));

    world.process_superior_actions(1);

    assert_eq!(
        superior_state(&world, CharacterId(1)).mode,
        SUPERIOR_MODE_RUN
    );
}

#[test]
fn superior_switches_back_to_fight_mode_once_fully_recovered() {
    // C `area2.c:139-142`.
    let mut world = World::default();
    let mut superior = superior_npc(1, 1);
    superior.driver_state = Some(CharacterDriverState::Superior(SuperiorDriverData {
        nr: 1,
        mode: SUPERIOR_MODE_RUN,
        ..Default::default()
    }));
    superior.values[0][CharacterValue::Hp as usize] = 100;
    superior.values[0][CharacterValue::Mana as usize] = 50;
    superior.values[0][CharacterValue::MagicShield as usize] = 20;
    superior.hp = 100 * POWERSCALE;
    superior.mana = 50 * POWERSCALE;
    superior.lifeshield = 20 * POWERSCALE;
    assert!(world.spawn_character(superior, 10, 10));

    world.process_superior_actions(1);

    assert_eq!(
        superior_state(&world, CharacterId(1)).mode,
        SUPERIOR_MODE_FIGHT
    );
}
