use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_JUDGE, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_ARKHATA_LETTER1, IID_ARKHATA_LETTER2};
use crate::world::npc::area37::judge::{JudgeDriverData, JudgeOutcomeEvent, JudgePlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn judge_npc(id: u32) -> Character {
    let mut judge = character(id);
    judge.name = "Judge".into();
    judge.driver = CDR_JUDGE;
    judge.driver_state = Some(CharacterDriverState::Judge(JudgeDriverData::default()));
    judge
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    judge_state: i32,
    captain_state: i32,
    letter_bits: i32,
) -> HashMap<CharacterId, JudgePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        JudgePlayerFacts {
            judge_state,
            captain_state,
            letter_bits,
        },
    );
    map
}

#[test]
fn state0_without_captain_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_captain_progress_greets_by_rank_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 0, 1, 0), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("authorization letters")));
}

#[test]
fn state3_queues_all_three_letters_when_none_are_held() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 3, 1, 0), 1);
    assert!(events.contains(&JudgeOutcomeEvent::GiveEntranceLetters {
        player_id: CharacterId(2),
        give_letter2: true,
        give_letter3: true,
        give_letter4: true,
    }));
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
}

#[test]
fn state3_skips_letters_whose_bit_is_already_set() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // bit 2 (letter2) already set.
    let events = world.process_judge_actions(&facts(CharacterId(2), 3, 1, 2), 1);
    assert!(events.contains(&JudgeOutcomeEvent::GiveEntranceLetters {
        player_id: CharacterId(2),
        give_letter2: false,
        give_letter3: true,
        give_letter4: true,
    }));
}

#[test]
fn state3_skips_letter_already_carried_even_with_bit_unset() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(70));
    assert!(world.spawn_character(godmode, 12, 10));
    let mut letter2 = item(70, ItemFlags::empty());
    letter2.template_id = IID_ARKHATA_LETTER2;
    letter2.carried_by = Some(CharacterId(2));
    world.add_item(letter2);

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // letter_bits bit for letter2 (2) is unset, but the player already
    // carries one - C's `!has_item(co, IID_ARKHATA_LETTER2)` half of the
    // gate must still block it.
    let events = world.process_judge_actions(&facts(CharacterId(2), 3, 1, 0), 1);
    assert!(events.contains(&JudgeOutcomeEvent::GiveEntranceLetters {
        player_id: CharacterId(2),
        give_letter2: false,
        give_letter3: true,
        give_letter4: true,
    }));
}

#[test]
fn state4_queues_entrance_pass_when_not_already_held() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 4, 1, 2 | 4 | 8), 1);
    assert!(events.contains(&JudgeOutcomeEvent::GiveEntrancePass {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
}

#[test]
fn state5_with_all_letters_delivered_advances_silently() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 5, 1, 2 | 4 | 8), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_with_letters_still_pending_speaks_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 5, 1, 2), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("deliver those agreements")));
}

#[test]
fn state6_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 6, 1, 2 | 4 | 8), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_letter1_at_state0_is_a_silent_turn_in() {
    let mut world = World::default();
    let mut judge = judge_npc(1);
    judge.cursor_item = Some(ItemId(50));
    world.add_character(judge);
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_ARKHATA_LETTER1;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    world.add_character(player(2, "Godmode"));

    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_judge_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut judge = judge_npc(1);
    judge.cursor_item = Some(ItemId(50));
    world.add_character(judge);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_judge_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_1_when_letters_not_all_delivered() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_judge_actions(&facts(CharacterId(2), 5, 1, 2), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_resets_to_4_when_all_letters_delivered() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(judge_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(judge) = world.characters.get_mut(&CharacterId(1)) {
        judge.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_judge_actions(&facts(CharacterId(2), 5, 1, 2 | 4 | 8), 1);
    assert!(events.contains(&JudgeOutcomeEvent::UpdateJudgeState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
}
