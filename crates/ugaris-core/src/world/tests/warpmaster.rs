use super::*;
use crate::character_driver::{CDR_WARPMASTER, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ALCHEMY_INGREDIENT;
use crate::world::npc::area25::WarpmasterOutcomeEvent;

fn warpmaster_npc(id: u32) -> Character {
    let mut warpmaster = character(id);
    warpmaster.name = "Rodney".into();
    warpmaster.driver = CDR_WARPMASTER;
    warpmaster
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

#[test]
fn nt_char_greets_low_level_visitors_with_a_leave_warning() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Newbie", 10), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    let events = world.process_warpmaster_actions(1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("too dangerous for you")));
}

#[test]
fn nt_char_greets_high_level_visitors_with_the_keys_pitch() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    world.process_warpmaster_actions(1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Warped World") && text.message.contains("keys")));
}

#[test]
fn nt_char_does_not_greet_the_same_character_twice() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    world.process_warpmaster_actions(1);
    assert!(!world.drain_pending_area_texts().is_empty());

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 2, 0, 0);
    world.process_warpmaster_actions(1);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nt_text_hello_replies_with_the_canned_greeting() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "hello");
    let events = world.process_warpmaster_actions(1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn nt_text_reset_says_done_and_queues_the_ppd_reset_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "reset");
    let events = world.process_warpmaster_actions(1);
    assert_eq!(
        events,
        vec![WarpmasterOutcomeEvent::ResetWarpPpd {
            player_id: CharacterId(2)
        }]
    );

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Done.")));
}

#[test]
fn nt_text_ignores_non_player_speakers() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    // A non-player character (no `CF_PLAYER` flag).
    assert!(world.spawn_character(character(2), 12, 10));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "hello");
    let events = world.process_warpmaster_actions(1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nt_give_alchemy_stone_type_23_trades_for_one_key() {
    let mut world = World::default();
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    let mut stone = item(50, ItemFlags::empty());
    stone.template_id = IID_ALCHEMY_INGREDIENT;
    stone.driver_data = vec![23];
    world.add_item(stone);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .cursor_item = Some(ItemId(50));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_GIVE, 2, 0, 0);
    let events = world.process_warpmaster_actions(1);
    assert_eq!(
        events,
        vec![WarpmasterOutcomeEvent::GiveKeys {
            warpmaster_id: CharacterId(1),
            player_id: CharacterId(2),
            ingredient_item_id: ItemId(50),
            count: 1,
        }]
    );
    assert!(world
        .drain_pending_area_texts()
        .iter()
        .any(|text| text.message.contains("Here you go, one key.")));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn nt_give_alchemy_stone_type_21_trades_for_two_keys() {
    let mut world = World::default();
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    let mut stone = item(50, ItemFlags::empty());
    stone.template_id = IID_ALCHEMY_INGREDIENT;
    stone.driver_data = vec![21];
    world.add_item(stone);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .cursor_item = Some(ItemId(50));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_GIVE, 2, 0, 0);
    let events = world.process_warpmaster_actions(1);
    assert_eq!(
        events,
        vec![WarpmasterOutcomeEvent::GiveKeys {
            warpmaster_id: CharacterId(1),
            player_id: CharacterId(2),
            ingredient_item_id: ItemId(50),
            count: 2,
        }]
    );
}

#[test]
fn nt_give_unrelated_item_is_handed_back_to_the_giver() {
    let mut world = World::default();
    assert!(world.spawn_character(warpmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    let junk = item(50, ItemFlags::empty());
    world.add_item(junk);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .cursor_item = Some(ItemId(50));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_GIVE, 2, 0, 0);
    let events = world.process_warpmaster_actions(1);
    assert!(events.is_empty());
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
}
