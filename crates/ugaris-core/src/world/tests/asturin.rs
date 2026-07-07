use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    AsturinDriverData, CDR_ASTURIN, NTID_ASTURIN, NT_CHAR, NT_GOTHIT, NT_NPC,
};
use crate::world::asturin::{AsturinOutcomeEvent, AsturinPlayerFacts};

fn asturin_npc(id: u32) -> Character {
    let mut asturin = character(id);
    asturin.name = "Asturin".into();
    asturin.driver = CDR_ASTURIN;
    asturin.driver_state = Some(CharacterDriverState::Asturin(AsturinDriverData::default()));
    asturin
}

fn player_char(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    seen_timer: i32,
) -> HashMap<CharacterId, AsturinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, AsturinPlayerFacts { state, seen_timer });
    map
}

fn asturin_state(world: &World, id: CharacterId) -> AsturinDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Asturin(data)) => data,
        _ => panic!("expected asturin driver state"),
    }
}

/// Spawns Asturin and a player 2 tiles apart at `x` (the player's x
/// coordinate is what every boundary check in `asturin_driver` reads),
/// lights the player's tile for `char_see_char`, and sets Asturin's own
/// `rest_x`/`rest_y` to its spawn tile so the unconditional every-tick
/// return-to-post `secure_move_driver` call is a no-op (same precedent as
/// `world::reskin`'s own test helper).
fn spawn_asturin_and_player(world: &mut World, x: usize) {
    let mut asturin = asturin_npc(1);
    asturin.rest_x = x as u16;
    asturin.rest_y = 10;
    assert!(world.spawn_character(asturin, x, 10));
    world.map.tile_mut(x, 12).unwrap().light = 255;
    assert!(world.spawn_character(player_char(2, "Godmode"), x, 12));
}

#[test]
fn asturin_greets_at_state_0_in_private_room() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 200);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_asturin_actions(&facts(CharacterId(2), 0, 0), 1, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&AsturinOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Hello Godmode. These rooms are private.")));
}

#[test]
fn asturin_warns_player_in_warning_zone() {
    let mut world = World::default();
    // C `ch[co].x < 118` gate: 116 is inside [115, 118).
    spawn_asturin_and_player(&mut world, 116);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_asturin_actions(&facts(CharacterId(2), 0, 0), 1, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Go back Godmode, you have no business here!")));
}

#[test]
fn asturin_forgives_at_states_4_and_5_in_warning_zone() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 116);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_asturin_actions(&facts(CharacterId(2), 4, 0), 1, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Alright, alright, Godmode, go ahead, just don't hit me again!")));
}

#[test]
fn asturin_shouts_guards_and_notifies_area_past_the_far_boundary() {
    let mut world = World::default();
    // C `ch[co].x < 115` gate.
    spawn_asturin_and_player(&mut world, 100);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_asturin_actions(&facts(CharacterId(2), 0, 0), 1, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Asturin shouts: \"GUARDS!\"")));
    // C `notify_area(ch[cn].x, ch[cn].y, NT_NPC, NTID_ASTURIN, cn, co);`:
    // every nearby character (including the player) receives the message.
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert!(player.driver_messages.iter().any(|message| {
        message.message_type == NT_NPC
            && message.dat1 == NTID_ASTURIN
            && message.dat2 == 1
            && message.dat3 == 2
    }));
}

#[test]
fn asturin_resets_state_1_to_3_after_ten_second_reminder_window() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 200);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 0, now = 11 (> 10 seconds ago), state = 2 (in 1..=3):
    // resets to 0 before the x >= 118 greeting switch runs, so state 0's
    // greeting line fires and advances to 1.
    let events = world.process_asturin_actions(&facts(CharacterId(2), 2, 0), 11, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn asturin_resets_state_7_to_8_after_thirty_second_window() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 200);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 0, now = 31 (> 30 seconds ago), state = 7: resets to 8
    // before the greeting switch runs, so case 7's line does NOT fire
    // (state is already 8 by the time the switch executes).
    let events = world.process_asturin_actions(&facts(CharacterId(2), 7, 0), 31, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(|text| text.message.contains("Be greeted")));
}

#[test]
fn asturin_welcomes_back_at_state_7() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 200);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // now - seen_timer <= 30, so the reset guard doesn't fire this time.
    let events = world.process_asturin_actions(&facts(CharacterId(2), 7, 20), 21, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Be greeted, Godmode. Welcome.")));
}

#[test]
fn asturin_text_repeat_resets_state_to_zero() {
    let mut world = World::default();
    spawn_asturin_and_player(&mut world, 200);

    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_asturin_actions(&facts(CharacterId(2), 3, 0), 1, 1);

    assert!(events.contains(&AsturinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn asturin_tracks_victim_from_gothit_message_and_attacks_when_adjacent() {
    let mut world = World::default();
    let mut asturin = asturin_npc(1);
    asturin.group = 0;
    asturin.rest_x = 10;
    asturin.rest_y = 10;
    assert!(world.spawn_character(asturin, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let events = world.process_asturin_actions(&HashMap::new(), 1, 1);

    assert!(events.is_empty());
    assert_eq!(
        asturin_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let asturin = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(asturin.action, action::ATTACK1);
}

#[test]
fn asturin_does_not_track_victim_from_same_group_gothit() {
    let mut world = World::default();
    let mut asturin = asturin_npc(1);
    asturin.rest_x = 10;
    asturin.rest_y = 10;
    assert!(world.spawn_character(asturin, 10, 10));
    let attacker = character(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(asturin) = world.characters.get_mut(&CharacterId(1)) {
        asturin.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    world.process_asturin_actions(&HashMap::new(), 1, 1);

    assert_eq!(asturin_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn asturin_returns_to_post_when_away_from_it() {
    let mut world = World::default();
    let mut asturin = asturin_npc(1);
    asturin.rest_x = 20;
    asturin.rest_y = 20;
    assert!(world.spawn_character(asturin, 10, 10));

    world.process_asturin_actions(&HashMap::new(), 1, 1);

    let asturin = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(asturin.action, action::WALK);
}
