// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_LAB3PASSGUARD, NT_CREATE};
use crate::world::npc::area22::lab3_passguard::{
    Lab3PassguardDriverData, Lab3PassguardOutcomeEvent, Lab3PassguardPlayerFacts,
};

fn guard_npc(id: u32) -> Character {
    let mut guard = character(id);
    guard.name = "Guard".into();
    guard.driver = CDR_LAB3PASSGUARD;
    guard.x = 10;
    guard.y = 10;
    guard.rest_x = 10;
    guard.rest_y = 10;
    guard.driver_state = Some(CharacterDriverState::Lab3Passguard(
        Lab3PassguardDriverData {
            talk: true,
            ..Default::default()
        },
    ));
    guard
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.x = 10;
    player.y = 11;
    player
}

fn facts(
    player_id: CharacterId,
    guard_talkstep: u8,
    password1: &[u8],
    password2: &[u8],
) -> HashMap<CharacterId, Lab3PassguardPlayerFacts> {
    let mut p1 = [0u8; 8];
    p1[..password1.len()].copy_from_slice(password1);
    let mut p2 = [0u8; 8];
    p2[..password2.len()].copy_from_slice(password2);
    let mut map = HashMap::new();
    map.insert(
        player_id,
        Lab3PassguardPlayerFacts {
            guard_talkstep,
            password1: p1,
            password2: p2,
        },
    );
    map
}

fn guard_state(world: &World, guard_id: CharacterId) -> Lab3PassguardDriverData {
    match world
        .characters
        .get(&guard_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab3Passguard(data)) => data,
        _ => panic!("expected lab3 passguard driver state"),
    }
}

#[test]
fn nt_create_latches_talk_once_process_wide() {
    let mut world = World::default();
    let mut guard = guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::Lab3Passguard(
        Lab3PassguardDriverData::default(),
    ));
    guard.push_driver_message(NT_CREATE, 0, 0, 0);
    world.add_character(guard);

    assert!(!world.lab3_passguard_talk_latched);
    world.process_lab3_passguard_actions(&HashMap::new(), 22);
    assert!(world.lab3_passguard_talk_latched);
    assert!(guard_state(&world, CharacterId(1)).talk);
}

#[test]
fn nt_create_second_guard_stays_mute_once_latch_already_set() {
    // C's `static int talk` is process-wide, not per-character: once *any*
    // guard has ever latched it, a later respawn's fresh `dat->talk`
    // never gets set - see the module doc comment.
    let mut world = World::default();
    world.lab3_passguard_talk_latched = true;
    let mut guard = guard_npc(1);
    guard.driver_state = Some(CharacterDriverState::Lab3Passguard(
        Lab3PassguardDriverData::default(),
    ));
    guard.push_driver_message(NT_CREATE, 0, 0, 0);
    world.add_character(guard);

    world.process_lab3_passguard_actions(&HashMap::new(), 22);

    assert!(!guard_state(&world, CharacterId(1)).talk);
}

#[test]
fn greeting_ladder_step0_says_halt_and_advances() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 10);
    world.add_character(guard_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(crate::character_driver::NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_passguard_actions(&facts(CharacterId(2), 0, b"", b""), 22);

    assert!(
        events.contains(&Lab3PassguardOutcomeEvent::SetGuardTalkstep {
            player_id: CharacterId(2),
            value: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Halt, Traveler, say the password")));
}

#[test]
fn greeting_ladder_step6_triggers_fight() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 10);
    world.add_character(guard_npc(1));
    world.add_character(player(2, "Intruder"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(crate::character_driver::NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_passguard_actions(&facts(CharacterId(2), 6, b"", b""), 22);

    assert!(
        events.contains(&Lab3PassguardOutcomeEvent::SetGuardTalkstep {
            player_id: CharacterId(2),
            value: 0,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Three! Intruder, I'm coming!")));
    let state = guard_state(&world, CharacterId(1));
    assert!(state.attacking);
    assert!(state.pursuing);
    assert_eq!(state.co, Some(CharacterId(2)));
}

#[test]
fn far_away_resets_talkstep_and_warns_password_holder() {
    let mut world = World::default();
    world.date.daylight = 255;
    world.map.tile_mut(30, 30).unwrap().daylight = 255;
    world.map.tile_mut(30, 30).unwrap().light = 255;
    world.add_character(guard_npc(1));
    let mut traveler = player(2, "Traveler");
    traveler.x = 30;
    traveler.y = 30;
    world.add_character(traveler);
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(crate::character_driver::NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_passguard_actions(&facts(CharacterId(2), 20, b"", b""), 22);

    assert!(
        events.contains(&Lab3PassguardOutcomeEvent::SetGuardTalkstep {
            player_id: CharacterId(2),
            value: 0,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Do not forget thine password")));
}

#[test]
fn not_at_home_post_ignores_nt_char() {
    let mut world = World::default();
    let mut guard = guard_npc(1);
    guard.x = 11; // not at rest_x/rest_y anymore
    world.add_character(guard);
    world.add_character(player(2, "Traveler"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(crate::character_driver::NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_passguard_actions(&facts(CharacterId(2), 0, b"", b""), 22);

    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn correct_password_opens_door() {
    let mut world = World::default();
    world.add_character(guard_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_text_message(CharacterId(2), "The password is Geronimo!");
    }

    let events =
        world.process_lab3_passguard_actions(&facts(CharacterId(2), 6, b"Gero", b"nimo"), 22);

    assert!(
        events.contains(&Lab3PassguardOutcomeEvent::SetGuardTalkstep {
            player_id: CharacterId(2),
            value: 20,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("I will open the door for thee")));
}

#[test]
fn blub_confuses_guard_and_delays_next_talk() {
    let mut world = World::default();
    world.tick = Tick(1000);
    world.add_character(guard_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_text_message(CharacterId(2), "blub blub blub");
    }

    let events =
        world.process_lab3_passguard_actions(&facts(CharacterId(2), 0, b"Gero", b"nimo"), 22);

    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("What?")));
    assert_eq!(
        guard_state(&world, CharacterId(1)).last_talk_tick,
        1000 + TICKS_PER_SECOND * 10
    );
}

#[test]
fn repeat_resets_talkstep_to_zero() {
    let mut world = World::default();
    world.add_character(guard_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_text_message(CharacterId(2), "please repeat");
    }

    let events =
        world.process_lab3_passguard_actions(&facts(CharacterId(2), 3, b"Gero", b"nimo"), 22);

    assert!(
        events.contains(&Lab3PassguardOutcomeEvent::SetGuardTalkstep {
            player_id: CharacterId(2),
            value: 0,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("I'll repeat.")));
}
