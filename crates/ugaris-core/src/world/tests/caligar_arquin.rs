use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARARQUIN, NT_CHAR};
use crate::item_driver::IID_CALIGARDUNGEONKEY;
use crate::world::npc::area36::arquin::{CaligarArquinOutcomeEvent, CaligarArquinPlayerFacts};

fn arquin_npc(id: u32) -> Character {
    let mut arquin = character(id);
    arquin.name = "Arquin".into();
    arquin.driver = CDR_CALIGARARQUIN;
    // Matches the spawn position below so `secure_move_driver`'s "return
    // to post" fallback (run every tick when `clan_serial == 0`) is a
    // no-op instead of teleporting Arquin away mid-test.
    arquin.rest_x = 10;
    arquin.rest_y = 10;
    arquin
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    arquin_state: i32,
    glori_state: i32,
) -> HashMap<CharacterId, CaligarArquinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CaligarArquinPlayerFacts {
            arquin_state,
            arquin_last_talk: 0,
            glori_state,
        },
    );
    map
}

#[test]
fn state0_requires_exact_glori_state_12_then_falls_through_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(arquin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(arquin) = world.characters.get_mut(&CharacterId(1)) {
        arquin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // glori_state == 13, not exactly 12 - no reaction.
    let events =
        world.process_caligar_arquin_actions(&facts(CharacterId(2), 0, 13), 100, world.area_id);
    assert!(events.is_empty());

    if let Some(arquin) = world.characters.get_mut(&CharacterId(1)) {
        arquin.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events =
        world.process_caligar_arquin_actions(&facts(CharacterId(2), 0, 12), 100, world.area_id);
    assert!(
        events.contains(&CaligarArquinOutcomeEvent::AdvanceArquinTalk {
            player_id: CharacterId(2),
            new_state: 2,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Obelisks? Well, they may be used")));
}

#[test]
fn state3_requires_dungeon_key_then_falls_through_to_state5() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(arquin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.spawn_character(godmode, 12, 10);
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_CALIGARDUNGEONKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    if let Some(arquin) = world.characters.get_mut(&CharacterId(1)) {
        arquin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_arquin_actions(&facts(CharacterId(2), 3, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarArquinOutcomeEvent::AdvanceArquinTalk {
            player_id: CharacterId(2),
            new_state: 5,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Aha, I see you have gotten a hold of the key")));
}

#[test]
fn state3_without_dungeon_key_stays_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(arquin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(arquin) = world.characters.get_mut(&CharacterId(1)) {
        arquin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_arquin_actions(&facts(CharacterId(2), 3, 0), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn text_repeat_queues_reset_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(arquin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(arquin) = world.characters.get_mut(&CharacterId(1)) {
        arquin.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events =
        world.process_caligar_arquin_actions(&facts(CharacterId(2), 5, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarArquinOutcomeEvent::ResetArquinMiniBlock {
            player_id: CharacterId(2),
        })
    );
}
