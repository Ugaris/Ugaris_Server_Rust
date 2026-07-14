use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARHOMDEN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_CALIGARDUNGEONKEY, IID_CALIGARHOMDENRING};
use crate::world::npc::area36::homden::{CaligarHomdenOutcomeEvent, CaligarHomdenPlayerFacts};

fn homden_npc(id: u32) -> Character {
    let mut homden = character(id);
    homden.name = "Homden".into();
    homden.driver = CDR_CALIGARHOMDEN;
    // Matches the spawn position below so `secure_move_driver`'s "return
    // to post" fallback (run every tick when `clan_serial == 0`) is a
    // no-op instead of teleporting Homden away mid-test.
    homden.rest_x = 10;
    homden.rest_y = 10;
    homden
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    homden_state: i32,
) -> HashMap<CharacterId, CaligarHomdenPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CaligarHomdenPlayerFacts {
            homden_state,
            homden_last_talk: 0,
        },
    );
    map
}

#[test]
fn state0_requires_dungeon_key_then_falls_through_opening_quest59() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.spawn_character(godmode, 12, 10);
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_CALIGARDUNGEONKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 0), 100, world.area_id);
    assert!(events.contains(&CaligarHomdenOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&CaligarHomdenOutcomeEvent::AdvanceHomdenTalk {
            player_id: CharacterId(2),
            new_state: 2,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("You come seeking my help?")));
}

#[test]
fn state0_without_dungeon_key_stays_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 0), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn state4_is_a_silent_no_op_char_message_waiting_for_the_ring() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 4), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn give_ring_while_waiting_completes_quest59() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let mut ring = item(50, ItemFlags::empty());
    ring.template_id = IID_CALIGARHOMDENRING;
    world.add_item(ring);
    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.cursor_item = Some(ItemId(50));
        homden.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 4), 100, world.area_id);
    assert!(
        events.contains(&CaligarHomdenOutcomeEvent::CompleteRingQuest {
            player_id: CharacterId(2),
        })
    );
    // The ring is destroyed, not given back.
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_ring_while_not_waiting_gives_it_back() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let mut ring = item(50, ItemFlags::empty());
    ring.template_id = IID_CALIGARHOMDENRING;
    world.add_item(ring);
    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.cursor_item = Some(ItemId(50));
        homden.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 2), 100, world.area_id);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CaligarHomdenOutcomeEvent::CompleteRingQuest { .. })));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
}

#[test]
fn text_repeat_queues_reset_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(homden_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(homden) = world.characters.get_mut(&CharacterId(1)) {
        homden.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events =
        world.process_caligar_homden_actions(&facts(CharacterId(2), 7), 100, world.area_id);
    assert!(
        events.contains(&CaligarHomdenOutcomeEvent::ResetHomdenMiniBlock {
            player_id: CharacterId(2),
        })
    );
}
