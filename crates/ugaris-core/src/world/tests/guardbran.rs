use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_GUARDBRAN, NT_CHAR, NT_GIVE};
use crate::world::npc::area29::guardbran::{
    GuardBranDriverData, GuardBranOutcomeEvent, GuardBranPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn guardbran_npc(id: u32) -> Character {
    let mut guardbran = character(id);
    guardbran.name = "Guard Brannington".into();
    guardbran.driver = CDR_GUARDBRAN;
    guardbran.driver_state = Some(CharacterDriverState::GuardBran(
        GuardBranDriverData::default(),
    ));
    guardbran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    guardbran_state: i32,
    countbran_state: i32,
    countbran_bits: i32,
    rammy_state: i32,
) -> HashMap<CharacterId, GuardBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GuardBranPlayerFacts {
            guardbran_state,
            countbran_state,
            countbran_bits,
            rammy_state,
        },
    );
    map
}

fn spawn_pair(world: &mut World) {
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guardbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
}

fn push_char_message(world: &mut World) {
    world.tick = Tick(BASELINE_TICK);
    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }
}

#[test]
fn state0_greets_when_count_not_started_and_advances_to_1() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 0, 0, 0, 0), 1);
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("welcome to the town of Brannington")));
}

#[test]
fn state0_advances_silently_when_count_already_started() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    // countbran_state != 0: no dialogue, but the state still advances (C
    // unconditionally increments and sets `didsay = 1`).
    let events = world.process_guardbran_actions(&facts(CharacterId(2), 0, 1, 0, 0), 1);
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state1_is_a_silent_no_op_below_level_45() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.level = 44;
    }

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 1, 0, 1 | 2 | 4, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state1_is_a_silent_no_op_without_all_three_jewel_bits() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.level = 45;
    }

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 1, 0, 1 | 2, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state1_falls_through_to_state2_dialogue_in_one_tick_when_satisfied() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.level = 45;
    }

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 1, 0, 1 | 2 | 4, 0), 1);
    // Real C fallthrough: state jumps straight from 1 to 3 (case 1's
    // silent `++` plus case 2's own `++`), speaking case 2's line.
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("helped him retrieve his family heirlooms")));
}

#[test]
fn states2_through_4_advance_one_state_each_with_dialogue() {
    let cases = [
        (2, 3, "helped him retrieve his family heirlooms"),
        (3, 4, "met someone up in the mountains"),
        (4, 5, "not an easily fooled man"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        spawn_pair(&mut world);
        push_char_message(&mut world);

        let events = world.process_guardbran_actions(&facts(CharacterId(2), state, 0, 0, 0), 1);
        assert!(
            events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
                player_id: CharacterId(2),
                new_state: next_state,
            }),
            "state {state} should advance to {next_state}"
        );
        let texts = world.drain_pending_area_texts();
        assert!(
            texts.iter().any(|text| text.message.contains(snippet)),
            "state {state} should speak {snippet:?}"
        );
    }
}

#[test]
fn state5_speaks_rank_string_opens_quest64_and_advances_to_6() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 5, 0, 0, 0), 1);
    assert!(events.contains(&GuardBranOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
    let texts = world.drain_pending_area_texts();
    // military_points == 0 -> army_rank_for_points == 0 -> "nobody".
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Your mission, nobody is to find out")));
}

#[test]
fn state6_is_a_silent_no_op_when_rammy_state_is_zero() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 6, 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state6_falls_through_to_state7_completes_quest_in_one_tick_when_rammy_started() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 6, 0, 0, 1), 1);
    assert!(events.contains(&GuardBranOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    // Real C fallthrough: state jumps straight from 6 to 8.
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 8,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Excellent! The Count will be most pleased")));
}

#[test]
fn state8_is_a_silent_no_op() {
    let mut world = World::default();
    spawn_pair(&mut world);
    push_char_message(&mut world);

    let events = world.process_guardbran_actions(&facts(CharacterId(2), 8, 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_to_0_below_state2() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_guardbran_actions(&facts(CharacterId(2), 1, 0, 0, 0), 1);
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn text_repeat_resets_to_2_in_middle_range() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_guardbran_actions(&facts(CharacterId(2), 4, 0, 0, 0), 1);
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
}

#[test]
fn text_repeat_resets_to_7_in_final_range() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_guardbran_actions(&facts(CharacterId(2), 8, 0, 0, 0), 1);
    assert!(
        events.contains(&GuardBranOutcomeEvent::UpdateGuardBranState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
}

#[test]
fn text_reset_me_has_no_case_and_produces_no_event() {
    let mut world = World::default();
    spawn_pair(&mut world);
    let mut god = world.characters.get_mut(&CharacterId(2)).unwrap().clone();
    god.flags |= CharacterFlags::GOD;
    world.characters.insert(CharacterId(2), god);
    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_text_message(CharacterId(2), "reset me");
    }
    // Unlike every other Brannington-family sibling, this driver has no
    // `case 3` at all - even a god's "reset me" produces no event.
    let events = world.process_guardbran_actions(&facts(CharacterId(2), 3, 0, 0, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn give_any_item_is_always_handed_back() {
    let mut world = World::default();
    let mut guardbran = guardbran_npc(1);
    guardbran.cursor_item = Some(ItemId(50));
    world.add_character(guardbran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(guardbran) = world.characters.get_mut(&CharacterId(1)) {
        guardbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_guardbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
