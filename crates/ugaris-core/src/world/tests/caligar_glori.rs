use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARGLORI, NT_CHAR, NT_GIVE};
use crate::item_driver::{
    IID_CALIGARDUNGEONKEY, IID_CALIGAROBELISK1, IID_CALIGAROBELISK2, IID_CALIGAROBELISK3,
};
use crate::world::npc::area36::glori::{CaligarGloriOutcomeEvent, CaligarGloriPlayerFacts};

fn glori_npc(id: u32) -> Character {
    let mut glori = character(id);
    glori.name = "Glori".into();
    glori.driver = CDR_CALIGARGLORI;
    // Matches the spawn position below so `secure_move_driver`'s "return
    // to post" fallback (run every tick when `clan_serial == 0`) is a
    // no-op instead of teleporting Glori away mid-test.
    glori.rest_x = 10;
    glori.rest_y = 10;
    glori
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    glori_state: i32,
    watch_flag: i32,
) -> HashMap<CharacterId, CaligarGloriPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CaligarGloriPlayerFacts {
            glori_state,
            glori_last_talk: 0,
            watch_flag,
        },
    );
    map
}

#[test]
fn state0_greets_closes_quest54_opens_quest55_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 0, 0), 100, world.area_id);
    assert!(events.contains(&CaligarGloriOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 54,
    }));
    assert!(events.contains(&CaligarGloriOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 55,
    }));
    assert!(
        events.contains(&CaligarGloriOutcomeEvent::AdvanceGloriTalk {
            player_id: CharacterId(2),
            new_state: 1,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for coming Godmode!")));
}

#[test]
fn state5_blocked_until_all_three_training_facilities_watched() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Only two of the three watch bits set (`1 | 2`, missing `4`).
    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 5, 1 | 2), 100, world.area_id);
    assert!(events.is_empty());

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_caligar_glori_actions(
        &facts(CharacterId(2), 5, 1 | 2 | 4),
        100,
        world.area_id,
    );
    assert!(
        events.contains(&CaligarGloriOutcomeEvent::AdvanceGloriTalk {
            player_id: CharacterId(2),
            new_state: 6,
            realtime_seconds: 100,
        })
    );
}

#[test]
fn state10_with_all_obelisks_falls_through_to_state12_with_quest_transition() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    godmode.inventory[31] = Some(ItemId(51));
    godmode.inventory[32] = Some(ItemId(52));
    world.spawn_character(godmode, 12, 10);

    for (id, template) in [
        (50, IID_CALIGAROBELISK1),
        (51, IID_CALIGAROBELISK2),
        (52, IID_CALIGAROBELISK3),
    ] {
        let mut obelisk = item(id, ItemFlags::empty());
        obelisk.template_id = template;
        obelisk.carried_by = Some(CharacterId(2));
        world.add_item(obelisk);
    }

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 10, 0), 100, world.area_id);
    assert!(events.contains(&CaligarGloriOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 56,
    }));
    assert!(events.contains(&CaligarGloriOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 57,
    }));
    assert!(
        events.contains(&CaligarGloriOutcomeEvent::AdvanceGloriTalk {
            player_id: CharacterId(2),
            new_state: 12,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Wow, these are most interesting")));
}

#[test]
fn state10_without_all_obelisks_stays_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 10, 0), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn state16_with_dungeon_key_only_closes_quest58_no_open() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.spawn_character(godmode, 12, 10);
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_CALIGARDUNGEONKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 16, 0), 100, world.area_id);
    assert!(events.contains(&CaligarGloriOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 58,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CaligarGloriOutcomeEvent::QuestOpen { .. })));
    assert!(
        events.contains(&CaligarGloriOutcomeEvent::AdvanceGloriTalk {
            player_id: CharacterId(2),
            new_state: 18,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Well done, Godmode. Did you talk to Homden yet?")));
}

#[test]
fn give_missing_obelisk_hints_and_returns_item() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(glori_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let mut obelisk = item(50, ItemFlags::empty());
    obelisk.template_id = IID_CALIGAROBELISK1;
    world.add_item(obelisk);
    if let Some(glori) = world.characters.get_mut(&CharacterId(1)) {
        glori.cursor_item = Some(ItemId(50));
        glori.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events =
        world.process_caligar_glori_actions(&facts(CharacterId(2), 10, 0), 100, world.area_id);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("heavy drinker tell about another dungeon")));
    // The obelisk was given back, not kept.
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
}
