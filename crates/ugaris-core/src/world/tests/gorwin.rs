use std::collections::HashMap;

use super::*;
use crate::character_driver::{GorwinDriverData, CDR_TUNNELER_GORWIN, NT_CHAR};
use crate::player::MAX_TUNNEL_LEVEL;
use crate::world::npc::area33::gorwin::{GorwinOutcomeEvent, GorwinPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;
const AREA_ID: u16 = 33;

fn gorwin_npc(id: u32) -> Character {
    let mut gorwin = character(id);
    gorwin.name = "Gorwin".into();
    gorwin.driver = CDR_TUNNELER_GORWIN;
    gorwin.driver_state = Some(CharacterDriverState::Gorwin(GorwinDriverData::default()));
    gorwin
}

fn gorwin_state_with(state: i32, tick: u64) -> CharacterDriverState {
    CharacterDriverState::Gorwin(GorwinDriverData {
        state,
        last_talk: tick,
        ..Default::default()
    })
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

fn facts(
    player_id: CharacterId,
    gorwin_tunnel_level: i32,
    tunnel_clevel: i32,
    used_overrides: &[(i32, u8)],
) -> HashMap<CharacterId, GorwinPlayerFacts> {
    let mut tunnel_used = vec![0u8; (MAX_TUNNEL_LEVEL + 1) as usize];
    for &(level, value) in used_overrides {
        tunnel_used[level as usize] = value;
    }
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GorwinPlayerFacts {
            gorwin_tunnel_level,
            tunnel_clevel,
            tunnel_used,
        },
    );
    map
}

fn gorwin_state(world: &World, gorwin_id: CharacterId) -> GorwinDriverData {
    match world
        .characters
        .get(&gorwin_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Gorwin(data)) => data,
        other => panic!("expected gorwin driver state, got {other:?}"),
    }
}

#[test]
fn gorwin_greets_new_player_with_first_intro_line() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 30), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 0, 0, &[]), AREA_ID);
    // `gorwin_tunnel_level` is 0 in the facts, so `initialize_gorwin_ppd`
    // fires too (C `tunnel.c:1042-1045`).
    assert!(events
        .iter()
        .any(|event| matches!(event, GorwinOutcomeEvent::SetGorwinTunnelLevel { player_id, .. } if *player_id == CharacterId(2))));

    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message)
            .contains("I'm Gorwin, and I'm here to help you navigate the challenges ahead")));

    let data = gorwin_state(&world, CharacterId(1));
    assert_eq!(data.state, 1);
    assert_eq!(data.current_victim, Some(CharacterId(2)));
}

#[test]
fn gorwin_tunnel_qa_answers_with_handle_tunnel_info() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 30), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "tunnel");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 20, 20, &[]), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("magical labyrinth that adapts to your skills")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("using the central door")));
}

#[test]
fn gorwin_repeat_resets_intro_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut gorwin = gorwin_npc(1);
    gorwin.driver_state = Some(gorwin_state_with(5, 0));
    assert!(world.spawn_character(gorwin, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 30), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 20, 20, &[]), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Let me explain everything about the tunnels again")));

    let data = gorwin_state(&world, CharacterId(1));
    assert_eq!(data.state, 0);
    assert_eq!(data.last_talk, 0);
}

#[test]
fn gorwin_level_command_out_of_range_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    assert!(world.spawn_character(player(2, "Godmode", 30), 250, 248));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 5");
    }

    // MIN_TUNNEL_LEVEL is 10, so level 5 is below the allowed range.
    let events = world.process_gorwin_actions(&facts(CharacterId(2), 20, 20, &[]), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I can only set your tunnel level between 10 and 30")));
}

#[test]
fn gorwin_level_command_free_default_reset() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    // char level 30 -> default_level = 30 - 10 = 20.
    let mut speaker = player(2, "Godmode", 30);
    speaker.gold = 0; // below 10000, so the "forgot it was free" easter egg cannot fire.
    assert!(world.spawn_character(speaker, 250, 248));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 20");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 15, 15, &[]), AREA_ID);
    assert!(events.contains(&GorwinOutcomeEvent::SetTunnelLevelBoth {
        player_id: CharacterId(2),
        level: 20,
    }));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I've reset your tunnel level to 20 (your level - 10) for free")));
}

#[test]
fn gorwin_level_command_same_level_charges_no_fee_below_gold_floor() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    let mut speaker = player(2, "Godmode", 50);
    speaker.gold = 100; // below 500, so the petty fee cannot fire regardless of the roll.
    assert!(world.spawn_character(speaker, 250, 248));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 15");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 15, 15, &[]), AREA_ID);
    assert!(events.is_empty());
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
}

#[test]
fn gorwin_level_command_maxed_level_auto_promotes_without_charging() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    let mut speaker = player(2, "Godmode", 50);
    speaker.gold = 100_000;
    assert!(world.spawn_character(speaker, 250, 248));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 15");
    }

    // level 15 is fully completed (used == MAX_TUNNEL_USES); level 16 is
    // the next available one.
    let events = world.process_gorwin_actions(&facts(CharacterId(2), 10, 10, &[(15, 10)]), AREA_ID);
    assert!(events.contains(&GorwinOutcomeEvent::SetTunnelLevelBoth {
        player_id: CharacterId(2),
        level: 16,
    }));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100_000);
}

#[test]
fn gorwin_level_command_paid_change_charges_exact_fee_with_no_fudge() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    let mut speaker = player(2, "Godmode", 50);
    speaker.gold = 100_000;
    assert!(world.spawn_character(speaker, 250, 248));

    world.tick = Tick(BASELINE_TICK);
    // Seed chosen so the fudge roll lands >= 13 (no fudge) and the
    // counts-out-loud roll lands >= 3 (no extra line) - see this module's
    // Python precomputation note in the ledger entry.
    world.legacy_random_seed = 1;
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        // current gorwin level 10, default_level = 50-10 = 40, so
        // requesting 13 hits neither the same-level, maxed, tiny (abs==1),
        // nor default-level branches - only the generic paid-fee path.
        gorwin.push_driver_text_message(CharacterId(2), "level 13");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 10, 10, &[]), AREA_ID);
    assert!(events.contains(&GorwinOutcomeEvent::SetTunnelLevelBoth {
        player_id: CharacterId(2),
        level: 13,
    }));
    // fee = abs(10 - 13) * 10000 = 30000.
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        100_000 - 30_000
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("For the small fee of 300 gold your tunnel level has been set to 13")));
}

#[test]
fn gorwin_level_command_insufficient_gold_rejects_paid_change() {
    let mut world = World::default();
    world.map.tile_mut(250, 248).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 248, 248));
    let mut speaker = player(2, "Godmode", 50);
    speaker.gold = 100;
    assert!(world.spawn_character(speaker, 250, 248));

    world.tick = Tick(BASELINE_TICK);
    world.legacy_random_seed = 1;
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 13");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 10, 10, &[]), AREA_ID);
    assert!(events.is_empty());
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("You need 300 gold to change your tunnel level to 13")));
}

#[test]
fn gorwin_level_command_blocked_while_inside_tunnel() {
    let mut world = World::default();
    // Position outside the entry lobby bounds (244..=254), so
    // `is_player_in_tunnel` reports the speaker as inside a tunnel.
    world.map.tile_mut(20, 20).unwrap().light = 255;
    assert!(world.spawn_character(gorwin_npc(1), 18, 18));
    assert!(world.spawn_character(player(2, "Godmode", 30), 20, 20));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gorwin) = world.characters.get_mut(&CharacterId(1)) {
        gorwin.push_driver_text_message(CharacterId(2), "level 20");
    }

    let events = world.process_gorwin_actions(&facts(CharacterId(2), 15, 15, &[]), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("You can't change your tunnel level while inside a tunnel")));
}
