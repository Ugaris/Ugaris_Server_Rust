use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_KELLY, NT_CHAR, NT_GIVE};
use crate::item_driver::{
    IID_AREA15_HEAD, IID_AREA2_CREEPERHEAD, IID_CALIGARLETTER, IID_CALIGARPLAQUE,
};
use crate::world::kelly::{KellyDriverData, KellyOutcomeEvent, KellyPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn kelly_npc(id: u32) -> Character {
    let mut kelly = character(id);
    kelly.name = "Kelly".into();
    kelly.driver = CDR_KELLY;
    kelly.driver_state = Some(CharacterDriverState::Kelly(KellyDriverData::default()));
    kelly
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[allow(clippy::too_many_arguments)]
fn facts(
    player_id: CharacterId,
    kelly_state: i32,
    seymour_state: i32,
    quest14_done: bool,
    quest15_done: bool,
    clara_state: i32,
    found1: bool,
    found2: bool,
    found3: bool,
    found_cnt: i32,
    quest54_count: u8,
    quest60_count: u8,
) -> HashMap<CharacterId, KellyPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        KellyPlayerFacts {
            kelly_state,
            seymour_state,
            quest14_done,
            quest15_done,
            clara_state,
            found1,
            found2,
            found3,
            found_cnt,
            quest54_count,
            quest60_count,
        },
    );
    map
}

/// Convenience wrapper for facts that only ever vary `kelly_state` (most
/// tests below).
fn simple_facts(
    player_id: CharacterId,
    kelly_state: i32,
) -> HashMap<CharacterId, KellyPlayerFacts> {
    facts(
        player_id,
        kelly_state,
        0,
        false,
        false,
        0,
        false,
        false,
        false,
        0,
        0,
        0,
    )
}

fn kelly_state(world: &World, kelly_id: CharacterId) -> KellyDriverData {
    match world
        .characters
        .get(&kelly_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Kelly(data)) => data,
        _ => panic!("expected kelly driver state"),
    }
}

fn spawn_pair(world: &mut World) {
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    world.tick = Tick(BASELINE_TICK);
    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.push_driver_message(NT_CHAR, 2, 0, 0);
    }
}

#[test]
fn kelly_greets_new_player_and_advances_to_state1() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 0), 1);
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Greetings, Godmode")));
}

#[test]
fn kelly_state1_blocked_when_seymour_not_far_enough() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            1,
            10,
            false,
            false,
            0,
            false,
            false,
            false,
            0,
            0,
            0,
        ),
        1,
    );
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn kelly_state1_opens_quest13_and_jumps_to_3_when_seymour_ready() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            1,
            16,
            false,
            false,
            0,
            false,
            false,
            false,
            0,
            0,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 13,
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Stone Creepers")));
}

#[test]
fn kelly_state9_awards_points_per_newly_found_shrine_without_completing() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            9,
            0,
            false,
            false,
            0,
            true,
            false,
            false,
            0,
            0,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::UpdateFoundCnt {
        player_id: CharacterId(2),
        new_found_cnt: 1,
    }));
    // Not all three found yet - state stays at 9, no quest completion.
    assert!(!events
        .iter()
        .any(|e| matches!(e, KellyOutcomeEvent::UpdateKellyState { .. })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, KellyOutcomeEvent::ParkShrinesQuestDone { .. })));
    // C `give_military_pts(cn, co, (1-0)*2, (1-0)*EXP_AREA3_SHRINE)`.
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        2
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("discovered 1 shrine, Godmode")));
}

#[test]
fn kelly_state9_completes_quest14_when_all_three_shrines_found() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            9,
            0,
            false,
            false,
            0,
            true,
            true,
            true,
            2,
            0,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::UpdateFoundCnt {
        player_id: CharacterId(2),
        new_found_cnt: 3,
    }));
    assert!(events.contains(&KellyOutcomeEvent::ParkShrinesQuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("just three of them")));
}

#[test]
fn kelly_state9_already_at_three_only_completes_once() {
    let mut world = World::default();
    spawn_pair(&mut world);

    // found_cnt already 3, no new shrine progress this visit.
    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            9,
            0,
            false,
            false,
            0,
            true,
            true,
            true,
            3,
            0,
            0,
        ),
        1,
    );
    assert!(!events
        .iter()
        .any(|e| matches!(e, KellyOutcomeEvent::UpdateFoundCnt { .. })));
    assert!(events.contains(&KellyOutcomeEvent::ParkShrinesQuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
}

#[test]
fn kelly_state13_blocked_below_level22() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 21;
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 13), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn kelly_state13_falls_through_to_state14_body_at_level22() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 22;
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 13), 1);
    assert!(events.contains(&KellyOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 15,
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("lost contact with our outpost")));
}

#[test]
fn kelly_state14_reachable_directly_regardless_of_level() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 1;
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 14), 1);
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
}

#[test]
fn kelly_state14_skips_ahead_to_19_when_quest15_already_done() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            14,
            0,
            false,
            true,
            0,
            false,
            false,
            false,
            0,
            0,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 19,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn kelly_state15_falls_through_to_clara_report_when_clara_ready() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            15,
            0,
            false,
            false,
            5,
            false,
            false,
            false,
            0,
            0,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::ClaraReportDone {
        player_id: CharacterId(2),
        kelly_id: CharacterId(1),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 17,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("So Clara is well")));
}

#[test]
fn kelly_state19_sells_carried_heads_and_stays_below_level56() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 10;
        player.inventory[30] = Some(ItemId(50));
    }
    let mut head = item(50, ItemFlags::empty());
    head.template_id = IID_AREA15_HEAD;
    head.carried_by = Some(CharacterId(2));
    head.driver_data = vec![1, 0, 0, 0]; // drdata[0] == 1
    world.add_item(head);

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 19), 1);
    // 125 + 1*75 = 200 silver.
    assert!(events.contains(&KellyOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 200,
    }));
    assert!(!events
        .iter()
        .any(|e| matches!(e, KellyOutcomeEvent::UpdateKellyState { .. })));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 200);
    assert!(!world.items.contains_key(&ItemId(50)));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("1 heads")));
}

#[test]
fn kelly_state19_opens_quests54_and_60_at_level56_first_visit() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 56;
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 19), 1);
    assert!(events.contains(&KellyOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 54,
    }));
    assert!(events.contains(&KellyOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 60,
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 21,
    }));
}

#[test]
fn kelly_state19_skips_to_26_when_quest60_already_done() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 56;
    }

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            19,
            0,
            false,
            false,
            0,
            false,
            false,
            false,
            0,
            0,
            1,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 26,
    }));
}

#[test]
fn kelly_state19_skips_to_21_when_quest54_already_done() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.level = 56;
    }

    let events = world.process_kelly_actions(
        &facts(
            CharacterId(2),
            19,
            0,
            false,
            false,
            0,
            false,
            false,
            false,
            0,
            1,
            0,
        ),
        1,
    );
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 21,
    }));
}

#[test]
fn kelly_state24_grants_caligar_letter_when_not_carried() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 24), 1);
    assert!(events.contains(&KellyOutcomeEvent::GrantCaligarLetter {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 25,
    }));
}

#[test]
fn kelly_state24_skips_grant_when_letter_already_carried() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.inventory[30] = Some(ItemId(50));
    }
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_CALIGARLETTER;
    letter.carried_by = Some(CharacterId(2));
    world.add_item(letter);

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 24), 1);
    assert!(!events
        .iter()
        .any(|e| matches!(e, KellyOutcomeEvent::GrantCaligarLetter { .. })));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 25,
    }));
}

#[test]
fn kelly_text_repeat_resets_state_to_bucket_start_and_skips_transient_states() {
    let cases: [(i32, i32); 6] = [(5, 0), (9, 6), (13, 10), (15, 14), (19, 17), (25, 21)];
    for (start_state, expected_reset) in cases {
        let mut world = World::default();
        spawn_pair(&mut world);
        if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
            kelly.driver_state = Some(CharacterDriverState::Kelly(KellyDriverData {
                last_talk: 12345,
                current_victim: None,
            }));
            kelly.driver_messages.clear();
            kelly.push_driver_text_message(CharacterId(2), "repeat");
        }

        let events = world.process_kelly_actions(&simple_facts(CharacterId(2), start_state), 1);
        assert!(
            events.contains(&KellyOutcomeEvent::UpdateKellyState {
                player_id: CharacterId(2),
                new_state: expected_reset,
            }),
            "start_state {start_state} should reset to {expected_reset}"
        );
        assert_eq!(kelly_state(&world, CharacterId(1)).last_talk, 0);
    }
}

#[test]
fn kelly_text_repeat_leaves_transient_states_untouched() {
    for start_state in [16, 20, 26] {
        let mut world = World::default();
        spawn_pair(&mut world);
        if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
            kelly.driver_state = Some(CharacterDriverState::Kelly(KellyDriverData::default()));
            kelly.driver_messages.clear();
            kelly.push_driver_text_message(CharacterId(2), "repeat");
        }

        let events = world.process_kelly_actions(&simple_facts(CharacterId(2), start_state), 1);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, KellyOutcomeEvent::UpdateKellyState { .. })),
            "start_state {start_state} should not reset"
        );
    }
}

#[test]
fn kelly_text_shortcut_to_caligar_jumps_god_player_to_state19() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.flags.insert(CharacterFlags::GOD);
    }
    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.driver_messages.clear();
        kelly.push_driver_text_message(CharacterId(2), "shortcut to caligar");
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 5), 1);
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 19,
    }));
}

#[test]
fn kelly_text_shortcut_to_caligar_ignored_for_non_god_player() {
    let mut world = World::default();
    spawn_pair(&mut world);
    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.driver_messages.clear();
        kelly.push_driver_text_message(CharacterId(2), "shortcut to caligar");
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
}

#[test]
fn kelly_receiving_creeperhead_completes_quest13_and_advances_state() {
    let mut world = World::default();
    let mut kelly = kelly_npc(1);
    kelly.cursor_item = Some(ItemId(50));
    world.add_character(kelly);
    let mut head = item(50, ItemFlags::empty());
    head.name = "Creeper Head".into();
    head.template_id = IID_AREA2_CREEPERHEAD;
    head.carried_by = Some(CharacterId(1));
    world.add_item(head);
    world.add_character(player(2, "Godmode"));

    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 5), 1);
    assert!(events.contains(&KellyOutcomeEvent::CreeperHeadQuestDone {
        player_id: CharacterId(2),
        kelly_id: CharacterId(1),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn kelly_receiving_plaque_completes_quest60_and_awards_gold() {
    let mut world = World::default();
    let mut kelly = kelly_npc(1);
    kelly.cursor_item = Some(ItemId(50));
    world.add_character(kelly);
    let mut plaque = item(50, ItemFlags::empty());
    plaque.name = "Emperor's Plaque".into();
    plaque.template_id = IID_CALIGARPLAQUE;
    plaque.carried_by = Some(CharacterId(1));
    world.add_item(plaque);
    world.add_character(player(2, "Godmode"));

    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 25), 1);
    assert!(events.contains(&KellyOutcomeEvent::PlaqueQuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&KellyOutcomeEvent::UpdateKellyState {
        player_id: CharacterId(2),
        new_state: 26,
    }));
    assert!(events.contains(&KellyOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 5000 * 100,
    }));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        5000 * 100
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("thank you so much")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn kelly_give_other_item_hands_it_back_to_giver() {
    let mut world = World::default();
    let mut kelly = kelly_npc(1);
    kelly.cursor_item = Some(ItemId(50));
    world.add_character(kelly);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(kelly) = world.characters.get_mut(&CharacterId(1)) {
        kelly.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kelly_actions(&simple_facts(CharacterId(2), 9), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
