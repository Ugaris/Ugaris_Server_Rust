use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_TEUFELQUEST, NT_CHAR};
use crate::world::npc::area34::teufelquest::{TeufelQuestOutcomeEvent, TeufelQuestPlayerFacts};

const AREA_ID: u16 = 34;

fn teufelquest_npc(id: u32) -> Character {
    let mut quest = character(id);
    quest.name = "Rat Hunter".into();
    quest.driver = CDR_TEUFELQUEST;
    quest.sprite = 157;
    quest.level = 38;
    quest.driver_state = Some(CharacterDriverState::TeufelQuest(Default::default()));
    quest
}

fn player(id: u32, name: &str, sprite: i32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.sprite = sprite;
    player
}

fn facts(
    player_id: CharacterId,
    kills: u32,
    score: u32,
) -> HashMap<CharacterId, TeufelQuestPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TeufelQuestPlayerFacts {
            teufel_rat_kills: kills,
            teufel_rat_score: score,
        },
    );
    map
}

fn lit_world() -> World {
    let mut world = World::default();
    for x in 0..20 {
        for y in 0..20 {
            world.map.tile_mut(x, y).unwrap().light = 255;
        }
    }
    world
}

#[test]
fn teufelquest_greets_demon_disguised_player_with_reward_pitch() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    // sprite 157 == fire-demon-suit, matches `is_demon`.
    assert!(world.spawn_character(player(2, "Godmode", 157), 12, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 0, 0), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message).contains("There's a nice")));
}

#[test]
fn teufelquest_greets_undisguised_human_with_alarm() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    // sprite 0 (plain human) fails `is_demon`.
    assert!(world.spawn_character(player(2, "Godmode", 0), 12, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 0, 0), AREA_ID);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("AAAAAHHHHHHHHHH")));
}

#[test]
fn teufelquest_greet_once_via_driver_memory() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 12, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 0, 0), AREA_ID);
    assert!(!world.drain_pending_area_text_bytes().is_empty());

    // Second sighting of the same player is suppressed by
    // `mem_check_driver` (`teufel.c:1508-1511`).
    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 0, 0), AREA_ID);
    assert!(world.drain_pending_area_text_bytes().is_empty());
}

#[test]
fn teufelquest_far_away_sighting_is_ignored() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    // 17 tiles away > the 16-tile `char_dist` gate (`teufel.c:1502`).
    assert!(world.spawn_character(player(2, "Godmode", 157), 27, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 0, 0), AREA_ID);
    assert!(world.drain_pending_area_text_bytes().is_empty());
}

#[test]
fn teufelquest_give_experience_resets_score_and_grants_exp() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give experience");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 40, 4000), AREA_ID);

    assert!(events.iter().any(|event| matches!(
        event,
        TeufelQuestOutcomeEvent::SetRatKillsScore { player_id, kills: 0, score: 0 }
            if *player_id == CharacterId(2)
    )));

    // C `tmp = ppd->score/20 * ch[cn].level` = (4000/20)*38 = 7600.
    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert!(player_after.exp > 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Experience it is")));

    // Score reset to 0 also queues the HUD-clear `#90`/`#80` lines
    // (`teufel.c:1582-1585`).
    let system_texts = world.drain_pending_system_texts();
    assert!(system_texts
        .iter()
        .any(|text| text.character_id == CharacterId(2) && text.message == "#90"));
    assert!(system_texts
        .iter()
        .any(|text| text.character_id == CharacterId(2) && text.message == "#80"));
}

#[test]
fn teufelquest_give_experience_below_reward_threshold_grants_no_item() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give experience");
    }
    // Score below 1000: `special_rat_reward` has no matching tier
    // (`teufel.c:1442-1466`), so no item should appear.
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 5, 500), AREA_ID);

    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert!(player_after.cursor_item.is_none());
}

#[test]
fn teufelquest_give_experience_above_threshold_grants_healing_potion() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            healing_potion1:
                name="Healing Potion"
                sprite=1
                value=0
                flag=IF_TAKE
            ;
            "#,
        )
        .unwrap();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give experience");
    }
    // Score 1000 hits the lowest `special_rat_reward` tier
    // (`healing_potion1`, `teufel.c:1463-1465`).
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 10, 1000), AREA_ID);

    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    let item_id = player_after.cursor_item.expect("expected a granted potion");
    assert_eq!(world.items.get(&item_id).unwrap().name, "Healing Potion");
}

#[test]
fn teufelquest_give_military_applies_points() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give military");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 25, 2500), AREA_ID);

    assert!(events.iter().any(|event| matches!(
        event,
        TeufelQuestOutcomeEvent::SetRatKillsScore { player_id, kills: 0, score: 0 }
            if *player_id == CharacterId(2)
    )));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Military knowledge it is")));
}

#[test]
fn teufelquest_give_money_adds_gold() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give money");
    }
    world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 10, 1000), AREA_ID);

    // C `tmp = ppd->score * 12` = 1000*12 = 12000.
    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player_after.gold, 12_000);
}

#[test]
fn teufelquest_give_godly_requires_god_flag() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give godly");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 1, 100), AREA_ID);
    // Non-god player: C's `if (ch[co].flags & CF_GOD)` guard blocks the
    // set entirely (`teufel.c:1576-1579`).
    assert!(events.is_empty());
}

#[test]
fn teufelquest_give_godly_sets_fixed_kills_and_score_for_gods() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    let mut god = player(2, "Godmode", 157);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give godly");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 1, 100), AREA_ID);

    assert!(events.iter().any(|event| matches!(
        event,
        TeufelQuestOutcomeEvent::SetRatKillsScore { player_id, kills: 500, score: 25_000 }
            if *player_id == CharacterId(2)
    )));
}

#[test]
fn teufelquest_asking_own_name_replies_and_does_not_reward() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "who are you");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 10, 1000), AREA_ID);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'm Rat Hunter")));
}

#[test]
fn teufelquest_text_from_far_away_speaker_is_filtered() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelquest_npc(1), 10, 10));
    // 13 tiles away > the 12-tile `char_dist` gate inside
    // `analyse_text_driver` itself (`teufel.c:267-269`).
    assert!(world.spawn_character(player(2, "Godmode", 157), 23, 10));

    if let Some(quest) = world.characters.get_mut(&CharacterId(1)) {
        quest.push_driver_text_message(CharacterId(2), "give experience");
    }
    let events =
        world.process_teufelquest_actions(&mut loader, &facts(CharacterId(2), 10, 1000), AREA_ID);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}
