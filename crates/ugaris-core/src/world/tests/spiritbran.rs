use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_SPIRITBRAN, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_STAFF_HOLYRELIC;
use crate::world::npc::area29::spiritbran::{
    SpiritBranDriverData, SpiritBranOutcomeEvent, SpiritBranPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn spiritbran_npc(id: u32) -> Character {
    let mut spiritbran = character(id);
    spiritbran.name = "Spirit of Brannington".into();
    spiritbran.driver = CDR_SPIRITBRAN;
    spiritbran.driver_state = Some(CharacterDriverState::SpiritBran(
        SpiritBranDriverData::default(),
    ));
    spiritbran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    spiritbran_state: i32,
) -> HashMap<CharacterId, SpiritBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, SpiritBranPlayerFacts { spiritbran_state });
    map
}

#[test]
fn state0_greets_opens_quest44_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&SpiritBranOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&SpiritBranOutcomeEvent::UpdateSpiritBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Greetings Godmode, I have watched thee")));
}

#[test]
fn states1_through_3_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "revived their ancestors"),
        (2, 3, "Brannington Holy Relic"),
        (3, 4, "ask Count Brannington about his jewelry"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
            spiritbran.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_spiritbran_actions(&facts(CharacterId(2), state), 1);
        assert!(
            events.contains(&SpiritBranOutcomeEvent::UpdateSpiritBranState {
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
fn state4_is_a_silent_no_op_waiting_for_the_relic() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_to_0_when_not_yet_past_state4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 3), 1);
    assert!(
        events.contains(&SpiritBranOutcomeEvent::UpdateSpiritBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn text_repeat_does_not_reset_once_past_state4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 5), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SpiritBranOutcomeEvent::UpdateSpiritBranState { .. })));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&SpiritBranOutcomeEvent::ResetSpiritBran {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(spiritbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_holy_relic_in_range_completes_quest44_and_jumps_to_5() {
    let mut world = World::default();
    let mut spiritbran = spiritbran_npc(1);
    spiritbran.cursor_item = Some(ItemId(50));
    world.add_character(spiritbran);
    let mut relic = item(50, ItemFlags::empty());
    relic.name = "The Brannington Holy Relic".into();
    relic.template_id = IID_STAFF_HOLYRELIC;
    relic.carried_by = Some(CharacterId(1));
    world.add_item(relic);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&SpiritBranOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&SpiritBranOutcomeEvent::UpdateSpiritBranState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Ishtar's blessings")));
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_holy_relic_after_state5_is_handed_back() {
    let mut world = World::default();
    let mut spiritbran = spiritbran_npc(1);
    spiritbran.cursor_item = Some(ItemId(50));
    world.add_character(spiritbran);
    let mut relic = item(50, ItemFlags::empty());
    relic.template_id = IID_STAFF_HOLYRELIC;
    relic.carried_by = Some(CharacterId(1));
    world.add_item(relic);
    world.add_character(player(2, "Godmode"));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // spiritbran_state == 5, outside the `< 5` acceptance window.
    let events = world.process_spiritbran_actions(&facts(CharacterId(2), 5), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SpiritBranOutcomeEvent::QuestDone { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut spiritbran = spiritbran_npc(1);
    spiritbran.cursor_item = Some(ItemId(50));
    world.add_character(spiritbran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(spiritbran) = world.characters.get_mut(&CharacterId(1)) {
        spiritbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_spiritbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
