use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    apply_lab5_mage_create_message, Lab5MageDriverData, CDR_LAB5MAGE, NT_CHAR, NT_CREATE,
};
use crate::world::npc::area22::lab5_mage::{Lab5MageOutcomeEvent, Lab5MagePlayerFacts};

fn mage_npc(id: u32) -> Character {
    let mut mage = character(id);
    mage.name = "Mathor".into();
    mage.driver = CDR_LAB5MAGE;
    apply_lab5_mage_create_message(&mut mage);
    mage
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    magestate: u8,
    ritualdaemon: u8,
    ritualstate: u8,
) -> HashMap<CharacterId, Lab5MagePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        Lab5MagePlayerFacts {
            magestate,
            ritualdaemon,
            ritualstate,
        },
    );
    map
}

fn mage_state(world: &World, mage_id: CharacterId) -> Lab5MageDriverData {
    match world
        .characters
        .get(&mage_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab5Mage(data)) => data,
        _ => panic!("expected lab5 mage driver state"),
    }
}

#[test]
fn apply_lab5_mage_create_message_pushes_nt_create() {
    let mut character = character(1);
    apply_lab5_mage_create_message(&mut character);
    assert!(matches!(
        character.driver_state,
        Some(CharacterDriverState::Lab5Mage(_))
    ));
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
fn nt_create_sets_namecoord0_from_mage_position() {
    let mut world = World::default();
    let mut mage = mage_npc(1);
    mage.x = 42;
    mage.y = 17;
    world.add_character(mage);

    world.process_lab5_mage_actions(&HashMap::new(), 22);

    assert_eq!(world.lab5_namecoord(0), (42, 17));
}

#[test]
fn lab5_mage_entry_state_greets_and_advances_to_1() {
    let mut world = World::default();
    world.add_character(mage_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 0, 0), 22);
    assert!(events.contains(&Lab5MageOutcomeEvent::SetMageState {
        player_id: CharacterId(2),
        magestate: 1,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("friend Laros surely mentioned")));

    let state = mage_state(&world, CharacterId(1));
    assert_eq!(state.lasttalk, 200);
    assert_eq!(state.cv_co, Some(CharacterId(2)));
}

#[test]
fn lab5_mage_state_4_clears_victim_without_speaking() {
    let mut world = World::default();
    world.add_character(mage_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 4, 0, 0), 22);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
    let state = mage_state(&world, CharacterId(1));
    assert_eq!(state.cv_co, None);
}

#[test]
fn lab5_mage_text_repeat_resets_state_to_0() {
    let mut world = World::default();
    world.add_character(mage_npc(1));
    world.add_character(player(2, "Godmode"));

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "please repeat that");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 30, 0, 0), 22);
    assert!(events.contains(&Lab5MageOutcomeEvent::SetMageState {
        player_id: CharacterId(2),
        magestate: 0,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I will repeat, Godmode")));
}

#[test]
fn lab5_mage_text_demon_is_reached_before_dead_demons_branch() {
    // C's `else if (strcasestr(str, "DEMONS"))` branch is unreachable
    // (the preceding "DEMON" check already matches "DEMONS") - both set
    // `magestate = 20` anyway, so sending "DEMONS" observably behaves
    // identically to "DEMON".
    let mut world = World::default();
    world.add_character(mage_npc(1));
    world.add_character(player(2, "Godmode"));

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "tell me about the DEMONS");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 0, 0), 22);
    assert!(events.contains(&Lab5MageOutcomeEvent::SetMageState {
        player_id: CharacterId(2),
        magestate: 20,
    }));
}

#[test]
fn lab5_mage_god_set_command_sets_ritual_state() {
    let mut world = World::default();
    world.add_character(mage_npc(1));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    world.add_character(god);

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "SET 1");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 0, 0), 22);
    assert!(events.contains(&Lab5MageOutcomeEvent::SetRitual {
        player_id: CharacterId(2),
        ritualdaemon: 1,
        ritualstate: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("set 1 3")));
}

#[test]
fn lab5_mage_non_god_set_command_is_ignored() {
    let mut world = World::default();
    world.add_character(mage_npc(1));
    world.add_character(player(2, "Godmode"));

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "SET 1");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 0, 0), 22);
    assert!(events.is_empty());
}

/// Shared setup for the "inside the name square" tests: the mage's own
/// `NT_CREATE`-derived position becomes `namecoordy[0]` (the square's
/// north bound), so the mage must stand strictly north of the square's
/// default south bound (`namecoordy[1] = 28`) - `y = 40` mirrors a
/// realistic "mage watches over the ritual room from outside it"
/// placement. `namecoordx[2]/namecoordy[1] = (85, 28)` (defaults) is the
/// exact "call" position; light is forced at that tile so `char_see_char`
/// succeeds despite the mage/speaker distance (same precedent as
/// `lab5_seyan`'s own tests).
fn setup_inside_square_world() -> World {
    let mut world = World::default();
    let mut mage = mage_npc(1);
    mage.x = 85;
    mage.y = 40;
    world.add_character(mage);
    world.map.tile_mut(85, 28).unwrap().light = 255;
    world
}

#[test]
fn lab5_mage_wrong_call_inside_square_hurts_and_resets_ritual() {
    let mut world = setup_inside_square_world();

    let mut speaker = player(2, "Godmode");
    speaker.x = 85;
    speaker.y = 28;
    world.add_character(speaker);

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        // Wrong real name for daemon 1 ("Fao Thals" is correct).
        mage.push_driver_text_message(CharacterId(2), "Godmode shouts: Breth Ona");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 1, 3), 1);
    assert!(events.contains(&Lab5MageOutcomeEvent::SetRitual {
        player_id: CharacterId(2),
        ritualdaemon: 0,
        ritualstate: 0,
    }));

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message.contains("ended")));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_PULSEBACK));
}

#[test]
fn lab5_mage_correct_call_inside_square_starts_ritual() {
    let mut world = setup_inside_square_world();

    let mut speaker = player(2, "Godmode");
    speaker.x = 85;
    speaker.y = 28;
    world.add_character(speaker);

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "Godmode shouts: Fao Thals");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 1, 3), 1);
    let plan_event = events.iter().find_map(|event| match event {
        Lab5MageOutcomeEvent::AttemptRitualStart {
            player_id, plan, ..
        } if *player_id == CharacterId(2) => Some(plan.clone()),
        _ => None,
    });
    let plan = plan_event.expect("expected AttemptRitualStart event");
    assert_eq!(plan.daemon, 1);
    assert_eq!((plan.door_x, plan.door_y), (119, 108));
    assert_eq!(plan.spawns.len(), 3);
    assert_eq!(plan.spawns[0].template, "lab5_one_servant");
    assert_eq!(plan.spawns[2].template, "lab5_one_master");

    // The room's statue tiles are placed immediately (pure `World` work),
    // independent of the deferred `ZoneLoader` demon spawn.
    assert_eq!(world.map.tile(121, 106).unwrap().foreground_sprite, 11165);
}

#[test]
fn lab5_mage_ritual_skips_occupied_room_and_picks_the_next_free_one() {
    let mut world = setup_inside_square_world();

    let mut speaker = player(2, "Godmode");
    speaker.x = 85;
    speaker.y = 28;
    world.add_character(speaker);

    // A second player standing inside room 0's rectangle
    // (`(119..=133, 102..=114)`) blocks it. `add_character` (a plain
    // registry insert, unlike `spawn_character`) does not place the
    // character on the map tile grid itself, so the tile is set directly.
    let mut blocker = player(3, "Blocker");
    blocker.x = 125;
    blocker.y = 108;
    world.add_character(blocker);
    world.map.tile_mut(125, 108).unwrap().character = 3;

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "Godmode shouts: Fao Thals");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 1, 3), 1);
    let plan_event = events.iter().find_map(|event| match event {
        Lab5MageOutcomeEvent::AttemptRitualStart { plan, .. } => Some(plan.clone()),
        _ => None,
    });
    let plan = plan_event.expect("expected AttemptRitualStart event");
    // Room 0 is blocked by the player at (125, 108); room 1 is next.
    assert_eq!((plan.door_x, plan.door_y), (119, 95));
}

#[test]
fn finish_ritual_start_success_teleports_and_logs_fulfilled() {
    let mut world = World::default();
    let mut mage = mage_npc(1);
    mage.x = 85;
    mage.y = 28;
    world.add_character(mage);
    let mut player = player(2, "Godmode");
    player.x = 85;
    player.y = 28;
    world.add_character(player);

    let success = world.finish_ritual_start(CharacterId(2), CharacterId(1), 119, 108, 1);
    assert!(success);

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message.contains("fulfilled")));
    let moved = world.characters.get(&CharacterId(2)).unwrap();
    assert_ne!((moved.x, moved.y), (85, 28));
}

#[test]
fn finish_ritual_start_failure_resets_endurance_and_logs_call_again() {
    let mut world = World::default();
    let mut mage = mage_npc(1);
    mage.x = 85;
    mage.y = 28;
    world.add_character(mage);
    // Player already adjacent to the target: `teleport_char_driver` is a
    // deliberate no-op within Manhattan distance 1, giving a deterministic
    // "failed" outcome without needing to physically block every
    // candidate tile.
    let mut player = player(2, "Godmode");
    player.x = 120;
    player.y = 108;
    player.values[0][CharacterValue::Endurance as usize] = 50;
    player.endurance = 0;
    world.add_character(player);

    let success = world.finish_ritual_start(CharacterId(2), CharacterId(1), 119, 108, 1);
    assert!(!success);

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message.contains("call again")));
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.endurance, 50 * POWERSCALE);
}

#[test]
fn lab5_mage_ritual_no_free_room_calls_again_without_spawning() {
    let mut world = setup_inside_square_world();

    let mut speaker = player(2, "Godmode");
    speaker.x = 85;
    speaker.y = 28;
    speaker.values[0][CharacterValue::Endurance as usize] = 30;
    speaker.endurance = 0;
    world.add_character(speaker);

    // Occupy all four candidate rooms with player-flagged blockers so
    // `attempt_ritual_start` finds no free room.
    for (index, (door_x, door_y)) in [(119, 108), (119, 95), (119, 82), (119, 69)]
        .into_iter()
        .enumerate()
    {
        let id = 10 + index as u32;
        let mut blocker = player(id, "Blocker");
        blocker.x = (door_x + 5) as u16;
        blocker.y = door_y as u16;
        world.add_character(blocker);
        world
            .map
            .tile_mut((door_x + 5) as usize, door_y as usize)
            .unwrap()
            .character = id as u16;
    }

    if let Some(mage) = world.characters.get_mut(&CharacterId(1)) {
        mage.push_driver_text_message(CharacterId(2), "Godmode shouts: Fao Thals");
    }

    let events = world.process_lab5_mage_actions(&facts(CharacterId(2), 0, 1, 3), 1);
    assert!(events
        .iter()
        .all(|event| !matches!(event, Lab5MageOutcomeEvent::AttemptRitualStart { .. })));

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message.contains("call again")));
    let speaker = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(speaker.endurance, 30 * POWERSCALE);
}
