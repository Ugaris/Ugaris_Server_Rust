use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARSMITH, NT_CHAR};
use crate::item_driver::{IID_CALIGARKEYP1, IID_CALIGARKEYP2, IID_CALIGARKEYP3};
use crate::world::npc::area36::smith::{CaligarSmithOutcomeEvent, CaligarSmithPlayerFacts};

fn smith_npc(id: u32) -> Character {
    let mut smith = character(id);
    smith.name = "Smith".into();
    smith.driver = CDR_CALIGARSMITH;
    // Matches the spawn position below so `secure_move_driver`'s "return
    // to post" fallback (run every tick when `clan_serial == 0`) is a
    // no-op instead of teleporting Smith away mid-test.
    smith.rest_x = 10;
    smith.rest_y = 10;
    smith
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    smith_state: i32,
    glori_state: i32,
    arkhata_monk_state: i32,
) -> HashMap<CharacterId, CaligarSmithPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CaligarSmithPlayerFacts {
            smith_state,
            smith_last_talk: 0,
            glori_state,
            arkhata_monk_state,
        },
    );
    map
}

fn give_three_key_parts(world: &mut World, player_id: CharacterId) {
    if let Some(character) = world.characters.get_mut(&player_id) {
        character.inventory[30] = Some(ItemId(50));
        character.inventory[31] = Some(ItemId(51));
        character.inventory[32] = Some(ItemId(52));
    }
    for (id, template) in [
        (50, IID_CALIGARKEYP1),
        (51, IID_CALIGARKEYP2),
        (52, IID_CALIGARKEYP3),
    ] {
        let mut part = item(id, ItemFlags::empty());
        part.template_id = template;
        part.carried_by = Some(player_id);
        world.add_item(part);
    }
}

#[test]
fn state0_requires_glori_state16_and_all_key_parts_then_falls_through_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smith_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    give_three_key_parts(&mut world, CharacterId(2));

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // glori_state is 15, not 16 - no reaction.
    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 0, 15, 0), 100, world.area_id);
    assert!(events.is_empty());

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 0, 16, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarSmithOutcomeEvent::AdvanceSmithTalk {
            player_id: CharacterId(2),
            new_state: 2,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("for a small fee of 5000 gold")));
}

#[test]
fn text_yes_okay_with_parts_and_gold_forges_the_key() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smith_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 5000 * 100;
    world.spawn_character(godmode, 12, 10);
    give_three_key_parts(&mut world, CharacterId(2));

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_text_message(CharacterId(2), "yes okay");
    }

    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 8, 16, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarSmithOutcomeEvent::ForgeUndergroundKey {
            smith_id: CharacterId(1),
            player_id: CharacterId(2),
        })
    );
}

#[test]
fn text_yes_okay_without_all_parts_reports_missing_parts() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smith_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 5000 * 100;
    world.spawn_character(godmode, 12, 10);

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_text_message(CharacterId(2), "yes okay");
    }

    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 8, 16, 0), 100, world.area_id);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CaligarSmithOutcomeEvent::ForgeUndergroundKey { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("do not appear to have all the neccessary parts")));
}

#[test]
fn text_yes_okay_without_enough_gold_reports_cannot_pay() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smith_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    give_three_key_parts(&mut world, CharacterId(2));

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_text_message(CharacterId(2), "yes okay");
    }

    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 8, 16, 0), 100, world.area_id);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CaligarSmithOutcomeEvent::ForgeUndergroundKey { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("cannot pay me")));
}

#[test]
fn text_pay_10000g_requires_monk_state_gate() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(smith_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 10_000 * 100;
    world.spawn_character(godmode, 12, 10);

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_text_message(CharacterId(2), "pay 10000g");
    }

    // monk_state below the gate - no purchase.
    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 8, 16, 19), 100, world.area_id);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CaligarSmithOutcomeEvent::PurchaseDictionary { .. })));

    if let Some(smith) = world.characters.get_mut(&CharacterId(1)) {
        smith.push_driver_text_message(CharacterId(2), "pay 10000g");
    }
    let events =
        world.process_caligar_smith_actions(&facts(CharacterId(2), 8, 16, 21), 100, world.area_id);
    assert!(
        events.contains(&CaligarSmithOutcomeEvent::PurchaseDictionary {
            smith_id: CharacterId(1),
            player_id: CharacterId(2),
        })
    );
}
