use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_KRENACH, NT_CHAR, NT_GIVE};
use crate::world::npc::area37::krenach::{
    KrenachDriverData, KrenachOutcomeEvent, KrenachPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn krenach_npc(id: u32) -> Character {
    let mut krenach = character(id);
    krenach.name = "Krenach".into();
    krenach.driver = CDR_KRENACH;
    krenach.driver_state = Some(CharacterDriverState::Krenach(KrenachDriverData::default()));
    krenach
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    krenach_state: i32,
    krenach_time: i32,
    monk_state: i32,
) -> HashMap<CharacterId, KrenachPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        KrenachPlayerFacts {
            krenach_state,
            krenach_time,
            monk_state,
        },
    );
    map
}

#[test]
fn state0_without_monk_progress_grumbles_once_the_cooldown_expires() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(krenach_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&facts(CharacterId(2), 0, 0, 10), now, 1);
    assert!(events.contains(&KrenachOutcomeEvent::UpdateKrenachTime {
        player_id: CharacterId(2),
        realtime_seconds: now,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Mrec amil groowah")));
}

#[test]
fn state0_without_monk_progress_stays_silent_within_cooldown() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(krenach_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&facts(CharacterId(2), 0, now, 10), now, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_monk_progress_completes_quest78_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(krenach_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&facts(CharacterId(2), 0, 0, 29), now, 1);
    assert!(events.contains(&KrenachOutcomeEvent::QuestDone78 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&KrenachOutcomeEvent::UpdateKrenachState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("blessed human")));
}

#[test]
fn state3_gives_the_refund_gold_and_advances_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(krenach_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&facts(CharacterId(2), 3, 0, 29), now, 1);
    assert!(events.contains(&KrenachOutcomeEvent::UpdateKrenachState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 5000 * 100);
}

#[test]
fn state5_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(krenach_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&facts(CharacterId(2), 5, 0, 29), now, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut krenach = krenach_npc(1);
    krenach.cursor_item = Some(ItemId(50));
    world.add_character(krenach);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(krenach) = world.characters.get_mut(&CharacterId(1)) {
        krenach.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let now = 100_000;
    let events = world.process_krenach_actions(&HashMap::new(), now, 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
