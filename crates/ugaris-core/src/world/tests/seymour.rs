use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_SEYMOUR, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA2_LOISANNOTE, IID_AREA2_ZOMBIESKULL1, IID_AREA2_ZOMBIESKULL2};
use crate::world::seymour::{SeymourDriverData, SeymourOutcomeEvent, SeymourPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn seymour_npc(id: u32) -> Character {
    let mut seymour = character(id);
    seymour.name = "Seymour".into();
    seymour.driver = CDR_SEYMOUR;
    seymour.driver_state = Some(CharacterDriverState::Seymour(SeymourDriverData::default()));
    seymour
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    seymour_state: i32,
    quest11_done: bool,
    quest12_done: bool,
) -> HashMap<CharacterId, SeymourPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        SeymourPlayerFacts {
            seymour_state,
            quest11_done,
            quest12_done,
        },
    );
    map
}

fn seymour_state(world: &World, seymour_id: CharacterId) -> SeymourDriverData {
    match world
        .characters
        .get(&seymour_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Seymour(data)) => data,
        _ => panic!("expected seymour driver state"),
    }
}

#[test]
fn seymour_greets_new_player_opens_quest10_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(seymour_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 0, false, false), 1);
    assert!(events.contains(&SeymourOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 10,
    }));
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome to Aston, Godmode")));
    assert_eq!(
        seymour_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn seymour_state9_is_a_silent_no_op_waiting_for_zombieskull1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(seymour_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 9, false, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn seymour_state10_opens_quest11_when_not_yet_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(seymour_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 10, false, false), 1);
    assert!(events.contains(&SeymourOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 11,
    }));
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("search Loisan's house here in Aston")));
}

#[test]
fn seymour_state10_skips_ahead_to_12_when_quest11_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(seymour_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 10, true, false), 1);
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    // C's `case 10` `break;` after the isdone shortcut never sets
    // `didsay`, so no text is spoken here.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn seymour_state12_and_13_fall_through_to_state14_logic() {
    for start_state in [12, 13] {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(seymour_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
            seymour.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events =
            world.process_seymour_actions(&facts(CharacterId(2), start_state, true, false), 1);
        assert!(
            events.contains(&SeymourOutcomeEvent::QuestOpen {
                player_id: CharacterId(2),
                quest: 12,
            }),
            "start_state {start_state} should open quest 12"
        );
        assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
            player_id: CharacterId(2),
            new_state: 15,
        }));
    }
}

#[test]
fn seymour_text_repeat_resets_state_to_bucket_start() {
    let cases: [(i32, i32); 5] = [(5, 0), (10, 10), (12, 12), (14, 14), (17, 16)];
    for (start_state, expected_reset) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(seymour_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
            seymour.driver_state = Some(CharacterDriverState::Seymour(SeymourDriverData {
                last_talk: 12345,
                current_victim: None,
            }));
            seymour.push_driver_text_message(CharacterId(2), "repeat");
        }

        let events =
            world.process_seymour_actions(&facts(CharacterId(2), start_state, false, false), 1);
        assert!(
            events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
                player_id: CharacterId(2),
                new_state: expected_reset,
            }),
            "start_state {start_state} should reset to {expected_reset}"
        );
        // C `dat->last_talk = 0;` inside the matching bucket.
        assert_eq!(seymour_state(&world, CharacterId(1)).last_talk, 0);
    }
}

#[test]
fn seymour_receiving_loisan_note_completes_quest12_and_awards_military_pts() {
    let mut world = World::default();
    let mut seymour = seymour_npc(1);
    seymour.cursor_item = Some(ItemId(50));
    world.add_character(seymour);
    let mut note = item(50, ItemFlags::empty());
    note.name = "Proof of Loisan's Death".into();
    note.template_id = IID_AREA2_LOISANNOTE;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 15, false, false), 1);
    assert!(events.contains(&SeymourOutcomeEvent::LoisanNoteQuestDone {
        player_id: CharacterId(2),
        seymour_id: CharacterId(1),
    }));
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 16,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("So he is dead")));
    // The NPC's cursor item (the note) is destroyed, not handed back.
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn seymour_receiving_zombieskull1_completes_quest10_and_enrolls_unranked_player() {
    let mut world = World::default();
    let mut seymour = seymour_npc(1);
    seymour.cursor_item = Some(ItemId(50));
    world.add_character(seymour);
    let mut skull = item(50, ItemFlags::empty());
    skull.name = "Strange Skull".into();
    skull.template_id = IID_AREA2_ZOMBIESKULL1;
    skull.carried_by = Some(CharacterId(1));
    world.add_item(skull);
    let mut godmode = player(2, "Godmode");
    godmode.military_points = 0;
    world.add_character(godmode);

    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 9, false, false), 1);
    assert!(
        events.contains(&SeymourOutcomeEvent::ZombieSkull1QuestDone {
            player_id: CharacterId(2),
        })
    );
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 10,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("A strange skull indeed")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome to the Imperial Army")));
    // Direct rank-1 enrollment (C's `set_army_rank(co, 1)`).
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        1
    );
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn seymour_receiving_zombieskull1_skips_enrollment_for_already_ranked_player() {
    let mut world = World::default();
    let mut seymour = seymour_npc(1);
    seymour.cursor_item = Some(ItemId(50));
    world.add_character(seymour);
    let mut skull = item(50, ItemFlags::empty());
    skull.template_id = IID_AREA2_ZOMBIESKULL1;
    skull.carried_by = Some(CharacterId(1));
    world.add_item(skull);
    let mut godmode = player(2, "Godmode");
    // Already ranked (any value that derives to rank > 0).
    godmode.military_points = 100;
    world.add_character(godmode);

    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 9, false, false), 1);
    assert!(
        events.contains(&SeymourOutcomeEvent::ZombieSkull1QuestDone {
            player_id: CharacterId(2),
        })
    );

    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("Welcome to the Imperial Army")));
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        100
    );
}

#[test]
fn seymour_receiving_zombieskull2_completes_quest11_and_awards_military_pts() {
    let mut world = World::default();
    let mut seymour = seymour_npc(1);
    seymour.cursor_item = Some(ItemId(50));
    world.add_character(seymour);
    let mut skull = item(50, ItemFlags::empty());
    skull.name = "Silver Skull".into();
    skull.template_id = IID_AREA2_ZOMBIESKULL2;
    skull.carried_by = Some(CharacterId(1));
    world.add_item(skull);
    world.add_character(player(2, "Godmode"));

    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 11, false, false), 1);
    assert!(
        events.contains(&SeymourOutcomeEvent::ZombieSkull2QuestDone {
            player_id: CharacterId(2),
            seymour_id: CharacterId(1),
        })
    );
    assert!(events.contains(&SeymourOutcomeEvent::UpdateSeymourState {
        player_id: CharacterId(2),
        new_state: 12,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn seymour_give_other_item_hands_it_back_to_giver() {
    let mut world = World::default();
    let mut seymour = seymour_npc(1);
    seymour.cursor_item = Some(ItemId(50));
    world.add_character(seymour);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(seymour) = world.characters.get_mut(&CharacterId(1)) {
        seymour.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_seymour_actions(&facts(CharacterId(2), 9, false, false), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    // C's `seymour_driver` calls plain `give_char_item`, not `give_char_
    // item_smart` - the item lands on the (empty) cursor, not inventory.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
