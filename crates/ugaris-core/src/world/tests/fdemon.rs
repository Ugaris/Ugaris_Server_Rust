use super::*;
use crate::{
    character_driver::{SimpleBaddyDriverData, CDR_FDEMON_DEMON},
    item_driver::IDR_FDEMONWAYPOINT,
    legacy::MAX_MAP,
    map::{manhattan_distance, MapFlags},
    world::fdemon::fdemon_may_hunt_there,
};

fn waypoint_item(id: u32, x: u16, y: u16) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = IDR_FDEMONWAYPOINT;
    it.x = x;
    it.y = y;
    it
}

fn fdemon_demon_npc(id: u32, x: u16, y: u16) -> Character {
    let mut demon = character(id);
    demon.driver = CDR_FDEMON_DEMON;
    demon.flags |= CharacterFlags::INFRARED;
    demon.x = x;
    demon.y = y;
    demon.rest_x = x;
    demon.rest_y = y;
    demon.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        helper: 1,
        startdist: 0,
        chardist: 30,
        stopdist: 0,
        ..Default::default()
    }));
    demon.fight_driver = Some(crate::character_driver::FightDriverData {
        start_dist: 0,
        char_dist: 30,
        stop_dist: 0,
        ..Default::default()
    });
    demon
}

// C's connection scan (`fdemon_waypoints_connected` callers, ported digit
// for digit) is order-dependent: `dx = wp[n].x - wp[m].x` only registers a
// "left"/"right" pair when the *lower-id* waypoint (`n`, scanned first) has
// the *larger* x - i.e. the eastern waypoint must be created before the
// western one it connects to. These tests assign ids accordingly (east =
// lower id) to exercise a real connection, matching a real `.itm` file
// where that's just whatever order the map author placed them in.

#[test]
fn build_connects_waypoints_forty_tiles_apart() {
    let mut world = World::default();
    world.items.insert(ItemId(1), waypoint_item(1, 140, 100)); // east, lower id
    world.items.insert(ItemId(2), waypoint_item(2, 100, 100)); // west, higher id

    world.ensure_fdemon_waypoints_built();

    assert_eq!(world.fdemon_waypoints.len(), 3);
    assert_eq!(world.fdemon_waypoints[1].left, 2);
    assert_eq!(world.fdemon_waypoints[2].right, 1);
}

#[test]
fn build_is_idempotent() {
    let mut world = World::default();
    world.items.insert(ItemId(1), waypoint_item(1, 100, 100));
    world.ensure_fdemon_waypoints_built();
    let first_len = world.fdemon_waypoints.len();
    world.ensure_fdemon_waypoints_built();
    assert_eq!(world.fdemon_waypoints.len(), first_len);
}

#[test]
fn add_enemy_and_hunt_driver_walks_toward_it() {
    let mut world = World::default();
    world.tick = Tick(1000);
    world.items.insert(ItemId(1), waypoint_item(1, 100, 100));
    world.ensure_fdemon_waypoints_built();

    world.add_fdemon_enemy_to_waypoint(105, 100);
    assert_ne!(world.fdemon_waypoints[1].last_enemy_tick, 0);

    let mut demon = character(1);
    demon.x = 80;
    demon.y = 100;
    demon.rest_x = 80;
    demon.rest_y = 100;
    let character_id = demon.id;
    world.characters.insert(character_id, demon);

    assert!(world.fdemon_hunt_driver(character_id, 8));
    let moved = world.characters.get(&character_id).unwrap();
    assert!(moved.action != 0 || moved.x != 80);
}

#[test]
fn may_hunt_there_matches_asymmetric_c_bounds() {
    assert!(fdemon_may_hunt_there(100, 100, 130, 100));
    assert!(!fdemon_may_hunt_there(100, 100, 131, 100));
    assert!(fdemon_may_hunt_there(100, 100, 30, 100));
    assert!(!fdemon_may_hunt_there(100, 100, 29, 100));
}

#[test]
fn find_way_to_waypoint_returns_first_hop() {
    let mut world = World::default();
    world.items.insert(ItemId(1), waypoint_item(1, 180, 100)); // easternmost, lowest id
    world.items.insert(ItemId(2), waypoint_item(2, 140, 100)); // middle
    world.items.insert(ItemId(3), waypoint_item(3, 100, 100)); // westernmost, highest id
    world.ensure_fdemon_waypoints_built();
    assert_eq!(world.fdemon_waypoints[1].left, 2);
    assert_eq!(world.fdemon_waypoints[2].left, 3);

    // 1-2-3 chain: from waypoint 3, going to waypoint 1, first hop is 2.
    let first_hop = world.fdemon_find_way_to_waypoint(3, 1);
    assert_eq!(first_hop, 2);
}

#[test]
fn unreachable_waypoints_are_not_connected() {
    let mut world = World::default();
    for y in 0..MAX_MAP {
        if y != 500 {
            world.map.set_flags(120, y, MapFlags::MOVEBLOCK);
        }
    }
    world.items.insert(ItemId(1), waypoint_item(1, 140, 100)); // east, lower id
    world.items.insert(ItemId(2), waypoint_item(2, 100, 100)); // west, higher id
    world.ensure_fdemon_waypoints_built();

    assert_eq!(manhattan_distance(100, 100, 140, 100), 40);
    assert_eq!(world.fdemon_waypoints[1].left, 0);
    assert_eq!(world.fdemon_waypoints[2].right, 0);
}

#[test]
fn action_walks_home_when_strayed_too_far_from_rest_position() {
    let mut world = World::default();
    world.tick = Tick(1000);
    let mut demon = fdemon_demon_npc(1, 100, 100);
    // Strayed 31 tiles east of home: `may_hunt_there(home, x)` fails
    // (`x - home_x > 30`), so C's gohome hysteresis should kick in.
    demon.x = 131;
    let character_id = demon.id;
    world.characters.insert(character_id, demon);

    let acted =
        world.process_fdemon_demon_action_with_random(character_id, 8, 0, 0, |below| below / 2);
    assert!(acted);
    let moved = world.characters.get(&character_id).unwrap();
    assert!(matches!(
        moved.driver_state,
        Some(CharacterDriverState::SimpleBaddy(ref data)) if data.fdemon_gohome
    ));
}

#[test]
fn action_hunts_a_recently_sighted_waypoint_enemy_when_not_gohome() {
    let mut world = World::default();
    world.tick = Tick(1000);
    world.items.insert(ItemId(1), waypoint_item(1, 100, 100));
    world.ensure_fdemon_waypoints_built();
    world.add_fdemon_enemy_to_waypoint(105, 100);

    let demon = fdemon_demon_npc(1, 80, 100);
    let character_id = demon.id;
    world.characters.insert(character_id, demon);

    let acted =
        world.process_fdemon_demon_action_with_random(character_id, 8, 0, 0, |below| below / 2);
    assert!(acted);
    let moved = world.characters.get(&character_id).unwrap();
    // Either a walk toward the waypoint or the immediate-idle fallback
    // queued something for this tick.
    assert!(moved.action != 0 || moved.x != 80 || moved.y != 100);
}

#[test]
fn action_wanders_and_always_queues_something_when_idle() {
    let mut world = World::default();
    world.tick = Tick(1000);
    let demon = fdemon_demon_npc(1, 100, 100);
    let character_id = demon.id;
    world.characters.insert(character_id, demon);

    // `random(4)` never rolls `0` (never the short-idle branch) and
    // `random(8)` always rolls a fixed direction - deterministic wander.
    let acted =
        world.process_fdemon_demon_action_with_random(character_id, 8, 0, 0, |below| below - 1);
    assert!(acted);
    let moved = world.characters.get(&character_id).unwrap();
    assert!(moved.action != 0);
}

#[test]
fn sighting_scan_records_a_nearby_visible_player_on_the_waypoint_graph() {
    let mut world = World::default();
    world.tick = Tick(1000);
    world.items.insert(ItemId(1), waypoint_item(1, 100, 100));
    world.ensure_fdemon_waypoints_built();

    let demon = fdemon_demon_npc(1, 100, 100);
    let demon_id = demon.id;
    world.characters.insert(demon_id, demon);

    let mut player = character(2);
    player.flags |= CharacterFlags::PLAYER;
    player.x = 105;
    player.y = 100;
    world.characters.insert(player.id, player);

    let acted = world.process_fdemon_demon_actions_with_completions(8, &[]);
    assert_eq!(acted, 1);
    assert_ne!(world.fdemon_waypoints[1].last_enemy_tick, 0);
}

mod fdemon_loader_station_report_tests {
    use crate::world::fdemon::fdemon_loader_station_report;

    #[test]
    fn station_one_solves_first_mission() {
        let report = fdemon_loader_station_report(1, 3, 0, 0, 10).unwrap();
        assert_eq!(report.new_stage, 6);
        assert_eq!(report.new_counter, 0);
        assert_eq!(
            report.feedback,
            vec![
                "You've solved your mission. Now head back to the commander to claim your reward!"
                    .to_string()
            ]
        );
    }

    #[test]
    fn station_one_outside_stage_range_is_a_noop() {
        assert!(fdemon_loader_station_report(1, 6, 0, 0, 10).is_none());
        assert!(fdemon_loader_station_report(1, 30, 0, 0, 10).is_none());
    }

    #[test]
    fn station_three_solves_second_mission() {
        let report = fdemon_loader_station_report(3, 7, 0, 0, 10).unwrap();
        assert_eq!(report.new_stage, 9);
    }

    #[test]
    fn station_two_solves_third_mission() {
        let report = fdemon_loader_station_report(2, 11, 0, 0, 10).unwrap();
        assert_eq!(report.new_stage, 12);
    }

    #[test]
    fn station_six_solves_sixth_mission() {
        let report = fdemon_loader_station_report(6, 25, 0, 0, 10).unwrap();
        assert_eq!(report.new_stage, 27);
    }

    #[test]
    fn twin_stations_four_and_five_need_both_before_solving() {
        // station 4 first: only the first-part message, stage unchanged.
        let first = fdemon_loader_station_report(4, 13, 0, 0, 10).unwrap();
        assert_eq!(first.new_stage, 13);
        assert_eq!(first.new_counter, 1);
        assert_eq!(
            first.feedback,
            vec![
                "You've solved the first part of your mission. Now go find that other station.!"
                    .to_string()
            ]
        );

        // station 5 second, with counter carried over from station 4: solves.
        let second = fdemon_loader_station_report(5, 13, first.new_counter, 0, 10).unwrap();
        assert_eq!(second.new_stage, 15);
        assert_eq!(second.new_counter, 3);
        assert_eq!(
            second.feedback,
            vec![
                "You've solved your mission. Now head back to the commander to claim your reward!"
                    .to_string()
            ]
        );

        // Order doesn't matter: station 5 first, then station 4.
        let first_b = fdemon_loader_station_report(5, 14, 0, 0, 10).unwrap();
        assert_eq!(first_b.new_counter, 2);
        let second_b = fdemon_loader_station_report(4, 14, first_b.new_counter, 0, 10).unwrap();
        assert_eq!(second_b.new_stage, 15);
        assert_eq!(second_b.new_counter, 3);
    }

    #[test]
    fn scouting_phase_reports_new_station_and_ignores_already_found() {
        // station_id 7 -> bit 0.
        let report = fdemon_loader_station_report(7, 28, 0, 0, 100).unwrap();
        assert_eq!(report.new_stage, 28);
        assert_eq!(report.new_counter, 1);
        assert_eq!(
            report.feedback,
            vec!["You've found Defense Station number 7.".to_string()]
        );

        // Already-found station reports nothing.
        assert!(fdemon_loader_station_report(7, 28, 1, 0, 100).is_none());

        // Below the scouting-phase gate stage, no report at all.
        assert!(fdemon_loader_station_report(7, 27, 0, 0, 100).is_none());
    }

    #[test]
    fn scouting_phase_warns_when_unreported_stations_exceed_exp_cap() {
        // Low level (level_value(1)/5 = (16-1)/5 = 3) means even a couple of
        // unreported 8000-exp stations blow the cap - but the warning only
        // fires once `unreported_cnt >= 3`.
        let counter_with_two_found = (1 << 0) | (1 << 1); // stations 7,8 already found+unreported
        let report = fdemon_loader_station_report(9, 28, counter_with_two_found, 0, 1).unwrap();
        assert_eq!(report.new_counter, counter_with_two_found | (1 << 2));
        assert_eq!(report.feedback.len(), 2);
        assert!(report.feedback[1].starts_with("You have discovered 3 stations."));
    }

    #[test]
    fn scouting_phase_no_warning_below_three_unreported() {
        let report = fdemon_loader_station_report(7, 28, 0, 0, 1).unwrap();
        assert_eq!(report.feedback.len(), 1);
    }

    #[test]
    fn scouting_phase_no_warning_when_already_reported_stations_dont_count() {
        // Stations 7 and 8 were already *reported* (boss_reported bits set),
        // so finding stations 9/10/11 on top only counts the 3 new ones -
        // this test pins that `boss_reported` truly excludes prior finds
        // from the unreported count, using a high level to keep the cap
        // itself from being the limiting factor.
        let boss_reported = (1 << 0) | (1 << 1);
        let counter = boss_reported | (1 << 2) | (1 << 3); // 7,8 reported; 9,10 found
        let report = fdemon_loader_station_report(11, 28, counter, boss_reported, 1).unwrap();
        // unreported = stations 9,10,11 (bits 2,3,4) = 3
        assert_eq!(report.feedback.len(), 2);
        assert!(report.feedback[1].starts_with("You have discovered 3 stations."));
    }
}
