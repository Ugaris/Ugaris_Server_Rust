use std::collections::HashMap;

use super::*;
use crate::character_driver::{LydiaDriverData, CDR_LYDIA, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA1_WOODKEY2, IID_AREA1_WOODPOTION};
use crate::world::lydia::{LydiaOutcomeEvent, LydiaPlayerFacts};

/// Same rationale as `world::nook`'s own `BASELINE_TICK` (its module's C
/// source, `gwendylon.c`, shares the same `dat->current_victim != co`
/// boot-time-only quirk).
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn lydia_npc(id: u32) -> Character {
    let mut lydia = character(id);
    lydia.name = "Lydia".into();
    lydia.driver = CDR_LYDIA;
    lydia.driver_state = Some(CharacterDriverState::Lydia(LydiaDriverData::default()));
    lydia
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
    seen_timer: i32,
) -> HashMap<CharacterId, LydiaPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, LydiaPlayerFacts { state, seen_timer });
    map
}

fn lydia_state(world: &World, lydia_id: CharacterId) -> LydiaDriverData {
    match world
        .characters
        .get(&lydia_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lydia(data)) => data,
        _ => panic!("expected lydia driver state"),
    }
}

#[test]
fn lydia_entry_greets_opens_quest_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LydiaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&LydiaOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Oohh, my head")));
}

#[test]
fn lydia_state4_stays_silent_until_reminder_gate() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer very recent (now - seen_timer <= 60): no reminder.
    let events = world.process_lydia_actions(&facts(CharacterId(2), 4, 990), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LydiaOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
    // Seen timer is still refreshed unconditionally.
    assert!(events.contains(&LydiaOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
}

#[test]
fn lydia_state4_reminds_after_gate_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer old (now - seen_timer > 60): reminder fires, no state change.
    let events = world.process_lydia_actions(&facts(CharacterId(2), 4, 900), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LydiaOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Didst thou find the potion")));
}

#[test]
fn lydia_state_reset_after_120_seconds_for_states_1_to_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // state 2, seen_timer 121s in the past: resets to 0 then greets again,
    // advancing to 1.
    let events = world.process_lydia_actions(&facts(CharacterId(2), 2, 800), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Oohh, my head")));
}

#[test]
fn lydia_state7_is_a_silent_done_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 7, 1_000), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LydiaOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn lydia_give_woodpotion_finishes_quest_in_state_range() {
    let mut world = World::default();
    let mut lydia = lydia_npc(1);
    lydia.cursor_item = Some(ItemId(50));
    world.add_character(lydia);
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_AREA1_WOODPOTION;
    potion.carried_by = Some(CharacterId(1));
    world.add_item(potion);
    world.add_character(player(2, "Godmode"));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 3, 0), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&LydiaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&LydiaOutcomeEvent::GrantPotion {
        player_id: CharacterId(2),
        template: "healing_potion1",
    }));
    assert!(world.items.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("That feels so much better")));
}

#[test]
fn lydia_give_woodpotion_grants_mana_potion_to_pure_mage() {
    let mut world = World::default();
    let mut lydia = lydia_npc(1);
    lydia.cursor_item = Some(ItemId(50));
    world.add_character(lydia);
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_AREA1_WOODPOTION;
    potion.carried_by = Some(CharacterId(1));
    world.add_item(potion);
    let mut mage = player(2, "Godmode");
    mage.flags |= CharacterFlags::MAGE;
    world.add_character(mage);

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 0, 0), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::GrantPotion {
        player_id: CharacterId(2),
        template: "mana_potion1",
    }));
}

#[test]
fn lydia_give_woodpotion_outside_state_range_is_a_normal_give_back() {
    let mut world = World::default();
    let mut lydia = lydia_npc(1);
    lydia.cursor_item = Some(ItemId(50));
    world.add_character(lydia);
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_AREA1_WOODPOTION;
    potion.carried_by = Some(CharacterId(1));
    world.add_item(potion);
    world.add_character(player(2, "Godmode"));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 5: outside the `<= 4` range the C code checks.
    let events = world.process_lydia_actions(&facts(CharacterId(2), 5, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LydiaOutcomeEvent::QuestDone { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn lydia_give_robberkey2_alone_is_a_distinct_item() {
    // Sanity: IID_AREA1_WOODKEY2 is distinct from IID_AREA1_WOODPOTION.
    assert_ne!(IID_AREA1_WOODKEY2, IID_AREA1_WOODPOTION);
}

#[test]
fn lydia_text_repeat_resets_early_states_to_one() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.driver_state = Some(CharacterDriverState::Lydia(LydiaDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        lydia.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 3, 0), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert_eq!(lydia_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        lydia_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn lydia_text_repeat_resets_late_states_to_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.driver_state = Some(CharacterDriverState::Lydia(LydiaDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        lydia.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_lydia_actions(&facts(CharacterId(2), 7, 0), 1_000, 1);
    assert!(events.contains(&LydiaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert_eq!(lydia_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn lydia_text_repeat_state5_does_not_reset_either_range() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.driver_state = Some(CharacterDriverState::Lydia(LydiaDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        lydia.push_driver_text_message(CharacterId(2), "repeat");
    }

    // state 5 is outside both [0,4] and [6,7], so C's two disjoint `if`s
    // (`gwendylon.c:3592-3603`) never fire.
    let events = world.process_lydia_actions(&facts(CharacterId(2), 5, 0), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, LydiaOutcomeEvent::UpdateState { .. })));
}

#[test]
fn lydia_text_ignores_non_current_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lydia_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Other"), 12, 10));

    if let Some(lydia) = world.characters.get_mut(&CharacterId(1)) {
        lydia.driver_state = Some(CharacterDriverState::Lydia(LydiaDriverData {
            last_talk: BASELINE_TICK,
            current_victim: Some(CharacterId(2)),
        }));
        lydia.push_driver_text_message(CharacterId(3), "hello");
    }
    world.tick = Tick(BASELINE_TICK);

    let mut player_facts = facts(CharacterId(2), 4, 0);
    player_facts.insert(
        CharacterId(3),
        LydiaPlayerFacts {
            state: 4,
            seen_timer: 0,
        },
    );

    world.process_lydia_actions(&player_facts, 1_000, 1);
    assert!(world.drain_pending_area_texts().is_empty());
}
