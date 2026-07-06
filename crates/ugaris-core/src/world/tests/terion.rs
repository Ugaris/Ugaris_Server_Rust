use std::collections::HashMap;

use super::*;
use crate::character_driver::{TerionDriverData, CDR_TERION, NTID_DIDSAY, NT_CHAR, NT_NPC};
use crate::world::terion::{TerionOutcomeEvent, TerionPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn terion_npc(id: u32) -> Character {
    let mut terion = character(id);
    terion.name = "Terion".into();
    terion.driver = CDR_TERION;
    terion.driver_state = Some(CharacterDriverState::Terion(TerionDriverData::default()));
    // Match the spawn tile used by every test in this module so the
    // "return to post" idle branch (`secure_move_driver` toward
    // `rest_x`/`rest_y`) is a no-op instead of relocating the NPC away
    // from the position the rest of the test asserts against - `World::
    // spawn_character` (unlike zone-file loading via `zone.rs`) never
    // seeds `rest_x`/`rest_y` from the spawn position on its own.
    terion.rest_x = 10;
    terion.rest_y = 10;
    terion
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    gwendy_state: i32,
    reskin_state: i32,
) -> HashMap<CharacterId, TerionPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TerionPlayerFacts {
            state,
            gwendy_state,
            reskin_state,
        },
    );
    map
}

fn terion_state(world: &World, terion_id: CharacterId) -> TerionDriverData {
    match world
        .characters
        .get(&terion_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Terion(data)) => data,
        _ => panic!("expected terion driver state"),
    }
}

#[test]
fn terion_entry_silent_before_second_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state = 5 < GWENDYLON_STATE_FIRST_SKULL_DONE (6): stay silent.
    let events = world.process_terion_actions(&facts(CharacterId(2), 0, 5, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn terion_entry_greets_and_advances_when_gwendy_state_in_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state = 6 (GWENDYLON_STATE_FIRST_SKULL_DONE): in window
    // [6, 9], so terion greets and advances to state 1.
    let events = world.process_terion_actions(&facts(CharacterId(2), 0, 6, 0), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Be greeted, Godmode! My name is Terion.")));
    assert_eq!(
        terion_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn terion_entry_jumps_silently_to_state4_when_skull2_already_solved() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state = 10 > GWENDYLON_STATE_SECOND_SKULL_WAIT (9): silent
    // jump straight to state 4, no dialogue, no facing update.
    let events = world.process_terion_actions(&facts(CharacterId(2), 0, 10, 0), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(terion_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn terion_state2_notifies_area_with_ntid_terion() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    // Bystander to observe the `notify_area` broadcast.
    assert!(world.spawn_character(player(3, "Bystander"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 2, 6, 0), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));

    let bystander = world.characters.get(&CharacterId(3)).unwrap();
    assert!(bystander
        .driver_messages
        .iter()
        .any(|m| m.message_type == NT_NPC && m.dat1 == 2 /* NTID_TERION */ && m.dat3 == 1));
    assert!(bystander
        .driver_messages
        .iter()
        .any(|m| m.message_type == NT_NPC && m.dat1 == NTID_DIDSAY));
}

#[test]
fn terion_state6_gated_on_gwendy_state_third_skull_wait() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state = 12 < 13: stays at state 6, no dialogue.
    let events = world.process_terion_actions(&facts(CharacterId(2), 6, 12, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, TerionOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn terion_state6_advances_once_gwendy_state_reaches_13() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MALE;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 6, 13, 0), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("brave men about")));
}

#[test]
fn terion_state9_gated_on_reskin_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 9, 20, 8), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, TerionOutcomeEvent::UpdateState { .. })));

    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_terion_actions(&facts(CharacterId(2), 9, 20, 9), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
}

#[test]
fn terion_state13_notifies_and_advances_beyond_final_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 13, 20, 20), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hey! Reskin!")));

    // State 14 (beyond the last case) is a permanent no-op - advance past
    // the talk-throttle window so this isn't just re-observing the
    // throttle from the previous call.
    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND * 6);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_terion_actions(&facts(CharacterId(2), 14, 20, 20), 1);
    assert!(events.is_empty());
}

#[test]
fn terion_didsay_broadcast_throttles_next_terion_tick() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    // Someone else's `NT_NPC`/`NTID_DIDSAY` broadcast bumps our own
    // `last_talk` throttle, silently, without removing the message from
    // the queue processing (it isn't a `NT_CHAR`/`NT_TEXT`/`NT_GIVE`
    // message so the second loop ignores it, but the pre-pass still
    // consumes its effect).
    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.driver_state = Some(CharacterDriverState::Terion(TerionDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        terion.push_driver_message(NT_NPC, NTID_DIDSAY, 99, 0);
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_terion_actions(&facts(CharacterId(2), 0, 6, 0), 1);
    // `last_talk` throttle reset to the current tick means the `NT_CHAR`
    // greeting (processed in the same call, after the pre-pass) is
    // blocked by the `TICKS * 5` minimum-gap guard.
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(
        terion_state(&world, CharacterId(1)).last_talk,
        BASELINE_TICK
    );
}

#[test]
fn terion_didsay_broadcast_from_self_is_ignored() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.driver_state = Some(CharacterDriverState::Terion(TerionDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        // `dat2 == cn` (our own id): the pre-pass condition
        // (`msg->dat2 != cn`) must not match this.
        terion.push_driver_message(NT_NPC, NTID_DIDSAY, 1, 0);
        terion.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_terion_actions(&facts(CharacterId(2), 0, 6, 0), 1);
    // last_talk was NOT bumped by the pre-pass, so the greeting still
    // fires normally.
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Be greeted, Godmode!")));
}

#[test]
fn terion_text_repeat_resets_state_bucket_5_to_6_back_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.driver_state = Some(CharacterDriverState::Terion(TerionDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        terion.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 5, 10, 0), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert_eq!(terion_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        terion_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn terion_text_repeat_resets_state_bucket_10_to_14_back_to_9() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(terion_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.driver_state = Some(CharacterDriverState::Terion(TerionDriverData::default()));
        terion.push_driver_text_message(CharacterId(2), "restart");
    }

    let events = world.process_terion_actions(&facts(CharacterId(2), 12, 20, 20), 1);
    assert!(events.contains(&TerionOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
}

#[test]
fn terion_give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut terion = terion_npc(1);
    terion.cursor_item = Some(ItemId(50));
    world.add_character(terion);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(terion) = world.characters.get_mut(&CharacterId(1)) {
        terion.push_driver_message(crate::character_driver::NT_GIVE, 2, 50, 0);
    }

    world.process_terion_actions(&HashMap::new(), 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
