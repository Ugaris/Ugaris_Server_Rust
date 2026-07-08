use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoGuardDriverData, CDR_TWOGUARD, NTID_TWOCITY, NTID_TWOCITY_PICK,
    NT_CHAR, NT_GOTHIT, NT_NPC, NT_SEEHIT, NT_TEXT,
};
use crate::world::npc::area17::{
    TwoGuardPlayerFacts, CS_CITIZEN, CS_ENEMY, CS_GUEST, LS_CLEAN, LS_DEAD, LS_FINE,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn guard_npc(id: u32) -> Character {
    let mut guard = character(id);
    guard.name = "Guard".into();
    guard.driver = CDR_TWOGUARD;
    guard.group = 1;
    guard.driver_state = Some(CharacterDriverState::TwoGuard(TwoGuardDriverData::default()));
    guard
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(legal_status: i32, legal_fine: i32, citizen_status: i32) -> TwoGuardPlayerFacts {
    TwoGuardPlayerFacts {
        legal_status,
        legal_fine,
        citizen_status,
        current_guard: 0,
        current_guard_time: 0,
        last_attack: 0,
        guard_intro: 0,
        bank_gold: 0,
    }
}

fn facts_map(
    player_id: CharacterId,
    facts: TwoGuardPlayerFacts,
) -> HashMap<CharacterId, TwoGuardPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, facts);
    map
}

fn guard_state(world: &World, guard_id: CharacterId) -> TwoGuardDriverData {
    match world
        .characters
        .get(&guard_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoGuard(data)) => data,
        _ => panic!("expected two guard driver state"),
    }
}

fn torch_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            torch:
              name="Torch"
              sprite=50023
              flag=IF_TAKE
              flag=IF_WNLHAND
              flag=IF_USE
              driver=12
              arg="00007878"
            ;
            "#,
        )
        .unwrap();
    loader
}

#[test]
fn torch_lights_up_in_darkness_and_stays_off_in_bright_light() {
    // Dark tile: `check_dlight`/`check_light` both signal "on".
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));

    let acted = world.process_two_guard_actions(&HashMap::new(), 0, &mut loader, 17);
    assert_eq!(acted.len(), 0);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        guard.sprite, 317,
        "guard should have lit the torch in the dark"
    );
}

#[test]
fn torch_stays_unlit_in_bright_daylight() {
    let mut world = World::default();
    let mut loader = torch_loader();
    world.date.daylight = 255;
    world.map.tile_mut(10, 10).unwrap().daylight = 255;
    world.map.tile_mut(10, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1), 10, 10));

    world.process_two_guard_actions(&HashMap::new(), 0, &mut loader, 17);
    let guard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        guard.sprite, 318,
        "guard should not light the torch in daylight"
    );
}

#[test]
fn illegal_territory_triggers_leave_warning_and_turns_to_face_intruder() {
    let mut world = World::default();
    let mut loader = torch_loader();
    // Palace box: (1,3)-(15,15), level 4. citizen_status defaults to
    // CS_ENEMY(0) < 4, so this is a territory violation.
    assert!(world.spawn_character(guard_npc(1), 5, 5));
    assert!(world.spawn_character(player(2, "Intruder"), 6, 5));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_ENEMY));
    let events = world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    assert!(events
        .iter()
        .any(|event| event.player_id == CharacterId(2) && event.current_guard == 1));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("You have no business in there")));
    assert_eq!(guard_state(&world, CharacterId(1)).leave_state, 1);
    assert_eq!(
        guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn legal_status_dead_attacks_immediately_without_warning() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 5, 5));
    assert!(world.spawn_character(player(2, "Killer"), 6, 5));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_DEAD, 0, CS_ENEMY));
    world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);

    assert_eq!(
        guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    // No leave-warning text for the LS_DEAD branch.
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("You have no business")));
}

#[test]
fn guest_pass_intro_speech_fires_once_within_range() {
    let mut world = World::default();
    let mut loader = torch_loader();
    // Whole-city box requires citizen_status >= 1 (CS_GUEST); a clean
    // player standing at (200,50) is inside only the "whole city"
    // catch-all box (level 1), so CS_ENEMY(0) < 1 makes this illegal
    // too - use a citizen_status of CS_GUEST so `place <= citizen_status`
    // and the LS_CLEAN intro branch is reached instead.
    assert!(world.spawn_character(guard_npc(1), 200, 50));
    assert!(world.spawn_character(player(2, "Godmode"), 201, 50));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_GUEST));
    let events = world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    assert!(events.iter().any(|event| event.guard_intro == 1));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou art here on a guest pass")));
}

#[test]
fn fine_ladder_state0_warns_and_turns_to_face() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 200, 50));
    assert!(world.spawn_character(player(2, "Debtor"), 201, 50));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_FINE, 5000, CS_GUEST));
    world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("you owe the city 50.00G")));
    assert_eq!(guard_state(&world, CharacterId(1)).fine_state, 1);
}

#[test]
fn pay_command_succeeds_with_enough_carried_gold() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    let mut debtor = player(2, "Debtor");
    debtor.gold = 10000;
    assert!(world.spawn_character(debtor, 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay".to_string()),
        });
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_FINE, 5000, CS_CITIZEN));
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected an outcome event");
    assert_eq!(update.legal_status, LS_CLEAN);
    assert_eq!(update.legal_fine, 0);
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 5000);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Wise choice")));
}

#[test]
fn pay_command_falls_back_to_bank_balance() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    let mut debtor = player(2, "Debtor");
    debtor.gold = 1000;
    assert!(world.spawn_character(debtor, 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay".to_string()),
        });
    }

    let mut player_facts = facts(LS_FINE, 5000, CS_CITIZEN);
    player_facts.bank_gold = 10000;
    let facts_map = facts_map(CharacterId(2), player_facts);
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected an outcome event");
    assert_eq!(update.legal_status, LS_CLEAN);
    // need = 5000 - 1000 = 4000
    assert_eq!(update.bank_gold_deduction, Some(4000));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 0);
}

#[test]
fn pay_command_fails_when_gold_and_bank_both_insufficient() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    let mut debtor = player(2, "Debtor");
    debtor.gold = 100;
    assert!(world.spawn_character(debtor, 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay".to_string()),
        });
    }

    let mut player_facts = facts(LS_FINE, 5000, CS_CITIZEN);
    player_facts.bank_gold = 100;
    let facts_map = facts_map(CharacterId(2), player_facts);
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected an outcome event");
    assert_eq!(update.legal_status, LS_FINE);
    assert_eq!(update.bank_gold_deduction, None);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("broke")));
}

#[test]
fn gothit_applies_fine_for_attacking_a_guard() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut guard = guard_npc(1);
    guard.values[0][CharacterValue::Hp as usize] = 100;
    guard.hp = 100 * POWERSCALE;
    assert!(world.spawn_character(guard, 10, 10));
    assert!(world.spawn_character(player(2, "Attacker"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_CITIZEN));
    let events = world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected a fine event");
    assert_eq!(update.legal_status, LS_FINE);
    assert_eq!(update.legal_fine, 2000);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Fine for attacking a city guard")));
    assert_eq!(
        guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn gothit_alerts_nearby_higher_level_guard_when_hp_low() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut guard = guard_npc(1);
    guard.values[0][CharacterValue::Hp as usize] = 100;
    guard.hp = 10 * POWERSCALE; // below 50% -> triggers alert
    guard.level = 5;
    assert!(world.spawn_character(guard, 10, 10));

    let mut senior = guard_npc(3);
    senior.name = "Senior Guard".into();
    senior.level = 20;
    assert!(world.spawn_character(senior, 12, 10));

    assert!(world.spawn_character(player(2, "Attacker"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_DEAD, 0, CS_CITIZEN));
    world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Help! Officer under attack!")));

    // The alert message may land on the senior guard either before or
    // after its own tick already ran this call (`HashMap` iteration
    // order is unspecified), so run one more tick to guarantee it has
    // been consumed, then assert on its observable effect (`data.tx`/
    // `data.ty` set from the packed alert).
    world.process_two_guard_actions(&HashMap::new(), 0, &mut loader, 17);
    let senior_data = guard_state(&world, CharacterId(3));
    assert_ne!((senior_data.tx, senior_data.ty), (0, 0));
}

#[test]
fn seehit_fines_the_attacker_of_an_ally() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    let mut ally = guard_npc(3);
    ally.name = "Fellow Guard".into();
    assert!(world.spawn_character(ally, 50, 50));
    assert!(world.spawn_character(player(2, "Attacker"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_SEEHIT, 2, 3, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_CITIZEN));
    let events = world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected a fine event");
    assert_eq!(update.legal_status, LS_FINE);
    assert_eq!(update.legal_fine, 7500);
    assert_eq!(
        guard_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn pick_lock_fines_the_lockpicker() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Thief"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_NPC, NTID_TWOCITY_PICK, 2, 0);
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_CITIZEN));
    let events = world.process_two_guard_actions(&facts_map, 100, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected a fine event");
    assert_eq!(update.legal_status, LS_FINE);
    assert_eq!(update.legal_fine, 3000);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Fine for breaking a lock: 30G")));
}

#[test]
fn god_command_enemy_sets_citizen_status() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("enemy".to_string()),
        });
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_HONOR));
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    let update = events
        .iter()
        .find(|event| event.player_id == CharacterId(2))
        .expect("expected an event");
    assert_eq!(update.citizen_status, CS_ENEMY);
}

#[test]
fn non_god_text_command_is_ignored() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Mortal"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("enemy".to_string()),
        });
    }

    let facts_map = facts_map(CharacterId(2), facts(LS_CLEAN, 0, CS_HONOR));
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    assert!(events.is_empty());
}

#[test]
fn repeat_text_resets_guard_intro() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(guard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));
    world.tick = Tick(BASELINE_TICK);

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let mut player_facts = facts(LS_CLEAN, 0, CS_ENEMY);
    player_facts.guard_intro = 1;
    let facts_map = facts_map(CharacterId(2), player_facts);
    let events = world.process_two_guard_actions(&facts_map, 0, &mut loader, 17);
    assert!(events
        .iter()
        .any(|event| event.player_id == CharacterId(2) && event.guard_intro == 0));
}

#[test]
fn parse_two_guard_driver_args_parses_patrol_waypoint_pairs() {
    let data = crate::world::npc::area17::guard::parse_two_guard_driver_args(
        "patx=70;paty=4;patx=70;paty=49;patx=3;paty=49;",
    );
    assert_eq!(data.patx[0], 70);
    assert_eq!(data.paty[0], 4);
    assert_eq!(data.patx[1], 70);
    assert_eq!(data.paty[1], 49);
    assert_eq!(data.patx[2], 3);
    assert_eq!(data.paty[2], 49);
    assert_eq!(data.patx[3], 0);
}

#[test]
fn call_guard_alert_packs_caller_x_and_target_y() {
    let mut world = World::default();
    assert!(world.spawn_character(guard_npc(1), 40, 10));
    let mut senior = guard_npc(3);
    senior.level = 50;
    assert!(world.spawn_character(senior, 41, 10));
    assert!(world.spawn_character(player(2, "Attacker"), 45, 77));
    world.tick = Tick(BASELINE_TICK);

    world.two_city_call_guard(CharacterId(1), CharacterId(2));
    let senior = world.characters.get(&CharacterId(3)).unwrap();
    let message = senior
        .driver_messages
        .iter()
        .find(|msg| msg.message_type == NT_NPC && msg.dat1 == NTID_TWOCITY)
        .expect("expected an alert message");
    // C `ch[cn].x + ch[co].y * MAXMAP` = 40 + 77 * 256.
    assert_eq!(message.dat3, 40 + 77 * 256);
}
