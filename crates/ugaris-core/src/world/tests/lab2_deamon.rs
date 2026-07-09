use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_LAB2DEAMON, NTID_LAB2_DEAMONCHECK, NT_CREATE, NT_NPC};
use crate::item_driver::{
    IID_LAB2_ELIASBELT, IID_LAB2_ELIASBOOTS, IID_LAB2_ELIASCAPE, IID_LAB2_ELIASHAT,
};
use crate::world::npc::area22::lab2_deamon::{
    Lab2DeamonDriverData, Lab2DeamonOutcomeEvent, Lab2DeamonPlayerFacts,
};

const WN_HEAD: usize = 1;
const WN_CLOAK: usize = 2;
const WN_BELT: usize = 5;
const WN_FEET: usize = 10;

fn deamon_npc(id: u32, co: CharacterId, serial: u32) -> Character {
    let mut deamon = character(id);
    deamon.name = "Deamon".into();
    deamon.driver = CDR_LAB2DEAMON;
    deamon.x = 10;
    deamon.y = 10;
    deamon.rest_x = 10;
    deamon.rest_y = 10;
    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(Lab2DeamonDriverData {
        co: Some(co),
        serial,
        ..Default::default()
    }));
    deamon
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.x = 10;
    player.y = 10;
    player
}

fn wear_elias_part(
    world: &mut World,
    co: &mut Character,
    item_id: u32,
    slot: usize,
    template_id: u32,
) {
    let mut part = item(item_id, ItemFlags::empty());
    part.template_id = template_id;
    part.carried_by = Some(co.id);
    world.add_item(part);
    co.inventory[slot] = Some(ItemId(item_id));
}

fn facts(
    player_id: CharacterId,
    deamon_checked: bool,
) -> HashMap<CharacterId, Lab2DeamonPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, Lab2DeamonPlayerFacts { deamon_checked });
    map
}

fn deamon_state(world: &World, deamon_id: CharacterId) -> Lab2DeamonDriverData {
    match world
        .characters
        .get(&deamon_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab2Deamon(data)) => data,
        _ => panic!("expected lab2 deamon driver state"),
    }
}

#[test]
fn lab2_deamon_already_tracking_detects_live_duplicate() {
    let mut world = World::default();
    world.add_character(deamon_npc(1, CharacterId(2), 42));

    assert!(world.lab2_deamon_already_tracking(CharacterId(2), 42));
    assert!(!world.lab2_deamon_already_tracking(CharacterId(2), 99));
    assert!(!world.lab2_deamon_already_tracking(CharacterId(3), 42));
}

#[test]
fn init_lab2_deamon_sets_co_serial_dir_and_pushes_create() {
    let mut world = World::default();
    let mut deamon = character(1);
    deamon.driver = CDR_LAB2DEAMON;
    deamon.x = 20;
    deamon.y = 30;
    world.add_character(deamon);

    world.init_lab2_deamon(CharacterId(1), CharacterId(2), 42);

    let deamon = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(deamon.dir, Direction::Down as u8);
    assert_eq!((deamon.rest_x, deamon.rest_y), (20, 30));
    assert_eq!(deamon.driver_messages.len(), 1);
    assert_eq!(deamon.driver_messages[0].message_type, NT_CREATE);
    match deamon.driver_state.as_ref().unwrap() {
        CharacterDriverState::Lab2Deamon(data) => {
            assert_eq!(data.co, Some(CharacterId(2)));
            assert_eq!(data.serial, 42);
        }
        _ => panic!("expected lab2 deamon driver state"),
    }
}

#[test]
fn lab2_deamon_create_no_elias_stays_talkstep_0_then_advances_to_1() {
    let mut world = World::default();
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    world.add_character(player(2, "Intruder"));

    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    world.tick = Tick(100);
    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);
    assert!(events.is_empty());

    let state = deamon_state(&world, CharacterId(1));
    // Case 0 has no ticker gate: it fires unconditionally the first tick.
    assert_eq!(state.talkstep, 1);
    assert_eq!(state.talkticker, 100 + TICKS_PER_SECOND / 8);
}

#[test]
fn lab2_deamon_create_full_elias_first_time_sets_observing_and_marks_checked() {
    let mut world = World::default();
    let mut co = player(2, "Elias");
    wear_elias_part(&mut world, &mut co, 50, WN_HEAD, IID_LAB2_ELIASHAT);
    wear_elias_part(&mut world, &mut co, 51, WN_CLOAK, IID_LAB2_ELIASCAPE);
    wear_elias_part(&mut world, &mut co, 52, WN_BELT, IID_LAB2_ELIASBELT);
    wear_elias_part(&mut world, &mut co, 53, WN_FEET, IID_LAB2_ELIASBOOTS);
    world.add_character(co);
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);
    assert!(events.contains(&Lab2DeamonOutcomeEvent::MarkDeamonChecked {
        player_id: CharacterId(2),
    }));

    let state = deamon_state(&world, CharacterId(1));
    assert!(state.observing);
    // Case 10 has no ticker gate either.
    assert_eq!(state.talkstep, 11);
}

#[test]
fn lab2_deamon_create_full_elias_already_checked_uses_quick_greeting() {
    let mut world = World::default();
    let mut co = player(2, "Elias");
    wear_elias_part(&mut world, &mut co, 50, WN_HEAD, IID_LAB2_ELIASHAT);
    wear_elias_part(&mut world, &mut co, 51, WN_CLOAK, IID_LAB2_ELIASCAPE);
    wear_elias_part(&mut world, &mut co, 52, WN_BELT, IID_LAB2_ELIASBELT);
    wear_elias_part(&mut world, &mut co, 53, WN_FEET, IID_LAB2_ELIASBOOTS);
    world.add_character(co);
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), true), 22);
    assert!(events.is_empty());

    let state = deamon_state(&world, CharacterId(1));
    // Case 20 (quick elias) has no ticker gate.
    assert_eq!(state.talkstep, 21);
}

#[test]
fn lab2_deamon_create_partial_elias_sets_masquerade_talkstep() {
    let mut world = World::default();
    let mut co = player(2, "Impostor");
    wear_elias_part(&mut world, &mut co, 50, WN_HEAD, IID_LAB2_ELIASHAT);
    world.add_character(co);
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    let state = deamon_state(&world, CharacterId(1));
    // Case 50 (masquerade) has no ticker gate.
    assert_eq!(state.talkstep, 51);
}

#[test]
fn lab2_deamon_warn_ladder_progresses_and_halts_player_on_stop() {
    let mut world = World::default();
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    world.add_character(player(2, "Intruder"));

    world.tick = Tick(0);
    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);
    assert!(events.is_empty());
    assert_eq!(deamon_state(&world, CharacterId(1)).talkstep, 1);

    // Not enough time passed yet: stays at 1, no text.
    world.tick = Tick(1);
    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);
    assert!(events.is_empty());
    assert_eq!(deamon_state(&world, CharacterId(1)).talkstep, 1);
    assert!(world.drain_pending_area_texts().is_empty());

    // Ticker elapsed: "STOP!" fires, halts the player.
    world.tick = Tick(TICKS_PER_SECOND / 8 + 1);
    let events = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);
    assert!(events.contains(&Lab2DeamonOutcomeEvent::HaltPlayer {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("STOP!")));
    assert_eq!(deamon_state(&world, CharacterId(1)).talkstep, 2);
}

#[test]
fn lab2_deamon_elias_ladder_ends_by_clearing_co_and_self_destructing() {
    // Case 17's `dat->co = 0; dat->talkstep = 255;` falls straight into
    // the same tick's "remove deamon" check (`!dat->co` -> C's
    // `remove_destroy_char(cn)`, `lab2.c:650-653,752-758`): the farewell
    // to Elias immediately dismisses the guardian for good.
    let mut world = World::default();
    let mut deamon = deamon_npc(1, CharacterId(2), 2);
    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(Lab2DeamonDriverData {
        co: Some(CharacterId(2)),
        serial: 2,
        talkstep: 17,
        ..Default::default()
    }));
    world.add_character(deamon);
    world.add_character(player(2, "Elias"));

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn lab2_deamon_check_engages_attack_when_not_full_elias() {
    let mut world = World::default();
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    let mut co = player(2, "Intruder");
    // C's "stop attacking when player tries to run away" check
    // (`lab2.c:709-716`) only *keeps* the attack engaged while the victim's
    // tile has `MF_NOMAGIC` set (the vault-approach trigger tile it was
    // standing on to trip this check in the first place) - see the module
    // doc comment.
    co.flags.insert(CharacterFlags::NOMAGIC);
    world.add_character(co);
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_NPC, NTID_LAB2_DEAMONCHECK, 2, 0);
    }

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    let state = deamon_state(&world, CharacterId(1));
    assert!(state.attacking);
    assert!(state.pursuing);
    assert_eq!(state.talkstep, 255);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I warned thee. Now thou shalt die, Intruder!")));
}

#[test]
fn lab2_deamon_check_uses_observing_shout_variant() {
    let mut world = World::default();
    let mut deamon = deamon_npc(1, CharacterId(2), 2);
    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(Lab2DeamonDriverData {
        co: Some(CharacterId(2)),
        serial: 2,
        observing: true,
        ..Default::default()
    }));
    world.add_character(deamon);
    let mut co = player(2, "Intruder");
    co.flags.insert(CharacterFlags::NOMAGIC);
    world.add_character(co);
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_NPC, NTID_LAB2_DEAMONCHECK, 2, 0);
    }

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Hey! Thou are not Elias. Now thou shalt die, Intruder!")));
}

#[test]
fn lab2_deamon_check_ignores_full_elias_wearer() {
    let mut world = World::default();
    let mut co = player(2, "Elias");
    co.flags.insert(CharacterFlags::NOMAGIC);
    wear_elias_part(&mut world, &mut co, 50, WN_HEAD, IID_LAB2_ELIASHAT);
    wear_elias_part(&mut world, &mut co, 51, WN_CLOAK, IID_LAB2_ELIASCAPE);
    wear_elias_part(&mut world, &mut co, 52, WN_BELT, IID_LAB2_ELIASBELT);
    wear_elias_part(&mut world, &mut co, 53, WN_FEET, IID_LAB2_ELIASBOOTS);
    world.add_character(co);
    world.add_character(deamon_npc(1, CharacterId(2), 2));
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_NPC, NTID_LAB2_DEAMONCHECK, 2, 0);
    }

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    let state = deamon_state(&world, CharacterId(1));
    assert!(!state.attacking);
}

#[test]
fn lab2_deamon_check_ignores_serial_mismatch_and_self_destructs() {
    // Deamon was created for serial 7, but the character now in that slot
    // has a different serial (a departed-and-replaced character) - C's
    // `ch[co].serial == dat->serial` guard (`lab2.c:517`) keeps the
    // deamoncheck message from engaging an attack, *and* separately the
    // same tick's "remove deamon" check's own `ch[dat->co].serial !=
    // dat->serial` branch (`lab2.c:753-754`) self-destructs the guardian,
    // since it no longer recognizes who it was created for.
    let mut world = World::default();
    world.add_character(deamon_npc(1, CharacterId(2), 7));
    world.add_character(player(2, "Someone Else"));
    if let Some(co) = world.characters.get_mut(&CharacterId(2)) {
        co.serial = 999;
    }
    if let Some(deamon) = world.characters.get_mut(&CharacterId(1)) {
        deamon.push_driver_message(NT_NPC, NTID_LAB2_DEAMONCHECK, 2, 0);
    }

    let _ = world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn lab2_deamon_self_destructs_when_target_gone() {
    let mut world = World::default();
    // `co` was never actually added to `world.characters` - matches C's
    // `!ch[dat->co].flags` (a fully empty slot).
    world.add_character(deamon_npc(1, CharacterId(99), 5));

    world.process_lab2_deamon_actions(&facts(CharacterId(99), false), 22);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn lab2_deamon_running_to_nomagic_zone_stops_attack() {
    let mut world = World::default();
    let mut deamon = deamon_npc(1, CharacterId(2), 2);
    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(Lab2DeamonDriverData {
        co: Some(CharacterId(2)),
        serial: 2,
        attacking: true,
        pursuing: true,
        talkstep: 255,
        ..Default::default()
    }));
    world.add_character(deamon);
    let mut co = player(2, "Runner");
    // C's condition releases the enemy when the victim's tile does *not*
    // have `MF_NOMAGIC` set - see the module doc comment. `CharacterFlags
    // ::NOMAGIC` defaults to unset, matching that.
    co.x = 10;
    co.y = 10;
    world.add_character(co);

    world.process_lab2_deamon_actions(&facts(CharacterId(2), false), 22);

    let state = deamon_state(&world, CharacterId(1));
    assert!(!state.attacking);
    assert!(!state.pursuing);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Master Elias told me to let them run away")));
}
