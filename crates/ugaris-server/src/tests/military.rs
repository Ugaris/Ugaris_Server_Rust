use super::*;
use ugaris_core::character_driver::{MilitaryMasterDriverData, CDR_MILITARY_MASTER};
use ugaris_core::world::{SingleMission, MISSION_TYPE_DEMON};

// C `complete_mission`'s mercenary bonus gold goes through `give_money`
// (`military.c:1391`), which also tracks the `achievement_add_gold_earned`
// wealth ladder (`tool.c:1475-1477`). `World::complete_mission` itself only
// does the inlined gold-add/message (matching `give_money`'s non-achievement
// half), so `apply_military_master_nearby_player` (`crate::military`) wires
// the achievement half itself via `award_swap_money_converted_achievement` -
// this exercises that wiring end-to-end through `apply_military_master_events`
// (queued by a real `process_military_master_actions` nearby-player scan,
// not a hand-built event).
#[tokio::test]
async fn apply_military_master_events_tracks_wealth_achievement_on_mercenary_gold_bonus() {
    let master_id = CharacterId(1);
    let player_id = CharacterId(2);
    let area_id: u16 = 1;

    let mut world = World::default();
    // Visibility for the master's nearby-player scan (`char_see_char`),
    // matching `ugaris-core`'s own
    // `military_master_greet_scan_queues_nearby_visible_player` test setup.
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let mut master = login_character(master_id, &login_block("Seymour"), area_id, 10, 10);
    master.flags = CharacterFlags::USED | CharacterFlags::ALIVE;
    master.driver = CDR_MILITARY_MASTER;
    master.driver_state = Some(CharacterDriverState::MilitaryMaster(
        MilitaryMasterDriverData {
            storage_id: 1,
            ..Default::default()
        },
    ));
    assert!(world.spawn_character(master, 10, 10));

    let mut merc = login_character(player_id, &login_block("Merc"), area_id, 12, 10);
    merc.professions[profession::MERCENARY] = 10;
    // Already ranked, so `greet_player`'s own text branches (irrelevant to
    // this test) don't matter either way.
    merc.military_points = 1000;
    assert!(world.spawn_character(merc, 12, 10));

    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(player_id);
    }
    {
        let player = runtime.player_for_character_mut(player_id).unwrap();
        player.set_military_took_mission(1); // difficulty 0
        player.set_military_solved_mission(true);
        // exp = 500 -> mercenary gold bonus = 500 / 5 = 100 silver = 1
        // whole gold unit (`amount / 100`, matching `give_money`'s cast).
        player.set_military_mission(
            0,
            SingleMission {
                mission_type: MISSION_TYPE_DEMON,
                opt1: 0,
                opt2: 0,
                pts: 10,
                exp: 500,
            },
        );
    }

    world.process_military_master_actions(area_id, 0);
    let applied = apply_military_master_events(&mut world, &mut runtime, &None, area_id).await;
    assert!(applied > 0);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.achievement_stats.gold_earned, 1);

    let character = &world.characters[&player_id];
    assert_eq!(character.gold, 100);
}

// Non-mercenary mission completions award no gold, so the wealth-ladder
// wiring must be a no-op (mirrors `complete_mission_awards_exp_and_points_
// for_non_mercenary`'s zero-`gold_awarded` case in `ugaris-core`).
#[tokio::test]
async fn apply_military_master_events_is_a_no_op_for_wealth_achievement_without_gold_bonus() {
    let master_id = CharacterId(1);
    let player_id = CharacterId(2);
    let area_id: u16 = 1;

    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let mut master = login_character(master_id, &login_block("Seymour"), area_id, 10, 10);
    master.flags = CharacterFlags::USED | CharacterFlags::ALIVE;
    master.driver = CDR_MILITARY_MASTER;
    master.driver_state = Some(CharacterDriverState::MilitaryMaster(
        MilitaryMasterDriverData {
            storage_id: 1,
            ..Default::default()
        },
    ));
    assert!(world.spawn_character(master, 10, 10));

    let mut recruit = login_character(player_id, &login_block("Recruit"), area_id, 12, 10);
    recruit.military_points = 1000;
    assert!(world.spawn_character(recruit, 12, 10));

    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(player_id);
    }
    {
        let player = runtime.player_for_character_mut(player_id).unwrap();
        player.set_military_took_mission(1);
        player.set_military_solved_mission(true);
        player.set_military_mission(
            0,
            SingleMission {
                mission_type: MISSION_TYPE_DEMON,
                opt1: 0,
                opt2: 0,
                pts: 10,
                exp: 500,
            },
        );
    }

    world.process_military_master_actions(area_id, 0);
    let applied = apply_military_master_events(&mut world, &mut runtime, &None, area_id).await;
    assert!(applied > 0);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.achievement_stats.gold_earned, 0);

    let character = &world.characters[&player_id];
    assert_eq!(character.gold, 0);
}
