use std::collections::HashMap;

use super::*;
use crate::character_driver::{SupermaxDriverData, CDR_SUPERMAX, NT_CHAR};
use crate::entity::CharacterValue;
use crate::world::supermax::{SupermaxOutcomeEvent, SupermaxPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;
const SUPERMAX_RAISE_FEE: u32 = 2000 * 100;

fn supermax_npc(id: u32) -> Character {
    let mut supermax = character(id);
    supermax.name = "Supermax".into();
    supermax.driver = CDR_SUPERMAX;
    supermax.driver_state = Some(CharacterDriverState::Supermax(SupermaxDriverData::default()));
    supermax
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    supermax_state: i32,
    supermax_gold: u32,
) -> HashMap<CharacterId, SupermaxPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        SupermaxPlayerFacts {
            supermax_state,
            supermax_gold,
        },
    );
    map
}

fn supermax_state(world: &World, supermax_id: CharacterId) -> SupermaxDriverData {
    match world
        .characters
        .get(&supermax_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Supermax(data)) => data,
        _ => panic!("expected supermax driver state"),
    }
}

#[test]
fn greeting_sequence_advances_state_0_through_4_and_then_plateaus() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    // state 0 -> 1: "Hello, ... I can turn your life upside down."
    world.tick = Tick(BASELINE_TICK);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&SupermaxOutcomeEvent::UpdateSupermaxState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("turn your life upside down")));

    // state 3 -> 4 (the last defined case).
    world.tick = Tick(BASELINE_TICK * 2);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    let events = world.process_supermax_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(events.contains(&SupermaxOutcomeEvent::UpdateSupermaxState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("raise SKILLNAME")));

    // state 4 has no matching C `case`, so nothing further is said and no
    // event is queued.
    world.tick = Tick(BASELINE_TICK * 3);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    let events = world.process_supermax_actions(&facts(CharacterId(2), 4, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn greeting_suppressed_within_min_talk_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(supermax) = world.characters.get_mut(&CharacterId(1)) {
        supermax.driver_state = Some(CharacterDriverState::Supermax(SupermaxDriverData {
            last_talk: BASELINE_TICK,
            current_victim: None,
        }));
        supermax.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_greeting_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "repeat");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 2, 0), 1);
    assert!(events.contains(&SupermaxOutcomeEvent::UpdateSupermaxState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(supermax_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn text_list_reports_maxed_raisable_skills_privately() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 10_000_000;
    godmode.exp_used = 0;
    godmode.values[1][CharacterValue::Dagger as usize] = 50;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "list");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.character_id == CharacterId(2)
        && text.message.contains("You can raise the following skills")));
    assert!(
        texts
            .iter()
            .any(|text| text.character_id == CharacterId(2)
                && text.message.starts_with("Dagger\u{8}"))
    );
}

#[test]
fn text_list_with_no_maxed_skills_reports_oops() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 10_000_000;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "list");

    world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Oops. You cannot raise any skill")));
}

#[test]
fn text_money_reports_gold_spent_only_when_positive() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "money");
    world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(world.drain_pending_area_texts().is_empty());

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "money");
    world.process_supermax_actions(&facts(CharacterId(2), 0, SUPERMAX_RAISE_FEE), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("You spent 2000 gold already")));
}

#[test]
fn raise_maxed_skill_charges_gold_and_exp_and_bumps_value() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 5_000_000;
    godmode.exp_used = 0;
    godmode.gold = 300_000;
    godmode.values[1][CharacterValue::Dagger as usize] = 50;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.iter().any(
        |event| matches!(event, SupermaxOutcomeEvent::AddSupermaxGold { player_id, amount }
            if *player_id == CharacterId(2) && *amount == SUPERMAX_RAISE_FEE)
    ));

    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 51);
    assert_eq!(godmode.gold, 300_000 - SUPERMAX_RAISE_FEE);
    assert!(godmode.exp_used > 0);
    assert!(godmode.flags.contains(CharacterFlags::ITEMS));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Your Dagger has been raised")));
}

#[test]
fn raise_below_skillmax_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 5_000_000;
    godmode.gold = 300_000;
    godmode.values[1][CharacterValue::Dagger as usize] = 10; // below skillmax(50)
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 10);
    assert_eq!(godmode.gold, 300_000);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("only raise skills you have already maxed")));
}

#[test]
fn raise_without_enough_gold_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 5_000_000;
    godmode.gold = 100; // below the 2000g fee
    godmode.values[1][CharacterValue::Dagger as usize] = 50;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 50);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("cannot pay the fee of 2000 gold")));
}

#[test]
fn raise_without_enough_exp_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 1; // nowhere near the multi-million cost
    godmode.exp_used = 0;
    godmode.gold = 300_000;
    godmode.values[1][CharacterValue::Dagger as usize] = 50;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 50);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("do not have enough experience to raise Dagger")));
}

#[test]
fn raise_beyond_250_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.exp = 999_000_000;
    godmode.gold = 300_000;
    godmode.values[1][CharacterValue::Dagger as usize] = 250;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("cannot raise any skill beyond 250 yet")));
}

#[test]
fn lower_above_skillmax_refunds_exp_without_gold_or_update_char_flag() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.values[1][CharacterValue::Dagger as usize] = 55; // past skillmax(50)
    godmode.exp_used = 5_000_000;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "lower dagger");

    let events = world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    // Lowering never touches PlayerRuntime (no gold field to update).
    assert!(events.is_empty());

    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 54);
    assert!(godmode.exp_used < 5_000_000);
    // C's `supermax_lower` does not set `CF_ITEMS`.
    assert!(!godmode.flags.contains(CharacterFlags::ITEMS));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Your Dagger has been lowered")));
}

#[test]
fn lower_at_or_below_skillmax_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.values[1][CharacterValue::Dagger as usize] = 50; // == skillmax, not past it
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "lower dagger");

    world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 50);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("only lower skills you have already raised past the max")));
}

#[test]
fn noexp_flag_blocks_both_raise_and_lower_with_private_messages() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(supermax_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::NOEXP;
    godmode.values[1][CharacterValue::Dagger as usize] = 55;
    godmode.exp = 5_000_000;
    godmode.gold = 300_000;
    assert!(world.spawn_character(godmode, 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "raise dagger");
    world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("cannot raise your skills when /noexp is set")));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "lower dagger");
    world.process_supermax_actions(&facts(CharacterId(2), 0, 0), 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("cannot lower your skills when /lockexp is set")));

    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.values[1][CharacterValue::Dagger as usize], 55);
}
