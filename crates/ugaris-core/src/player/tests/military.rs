use super::*;

#[test]
fn military_ppd_mission_slot_and_progress_accessors_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.military_took_mission(), 0);
    assert!(!player.military_solved_mission());

    let mission = crate::world::SingleMission {
        mission_type: crate::world::MISSION_TYPE_DEMON,
        opt1: 5,
        opt2: 40,
        pts: 10,
        exp: 200,
    };
    player.set_military_mission(1, mission);
    assert_eq!(player.military_mission(1), mission);
    // Untouched slots stay zeroed.
    assert!(player.military_mission(0).is_empty());

    player.set_military_took_mission(2);
    assert_eq!(player.military_took_mission(), 2);
    player.set_military_solved_mission(true);
    assert!(player.military_solved_mission());
}

#[test]
fn military_ppd_mission_preference_and_yday_accessors_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.mission_type_preference(), 0);
    assert_eq!(player.mission_difficulty_preference(), 0);
    assert_eq!(player.mission_yday(), 0);

    player.set_mission_type_preference(2);
    player.set_mission_difficulty_preference(3);
    player.set_mission_yday(100);

    assert_eq!(player.mission_type_preference(), 2);
    assert_eq!(player.mission_difficulty_preference(), 3);
    assert_eq!(player.mission_yday(), 100);
    // These fields are distinct from the mis[]/took_mission/
    // solved_mission fields exercised above - writing them must not
    // disturb an already-set mission slot.
    let mission = crate::world::SingleMission {
        mission_type: crate::world::MISSION_TYPE_SILVER,
        opt1: 7,
        opt2: 0,
        pts: 1,
        exp: 10,
    };
    player.set_military_mission(4, mission);
    player.set_mission_yday(200);
    assert_eq!(player.military_mission(4), mission);
}

#[test]
fn military_ppd_advisor_last_and_reroll_yday_accessors_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    for idx in 0..MILITARY_PPD_MAXADVISOR {
        assert_eq!(player.military_advisor_last(idx), 0);
    }
    assert_eq!(player.military_reroll_yday(), 0);

    player.set_military_advisor_last(0, 10);
    player.set_military_advisor_last(19, 200);
    player.set_military_reroll_yday(55);

    assert_eq!(player.military_advisor_last(0), 10);
    assert_eq!(player.military_advisor_last(19), 200);
    // Untouched slots stay zeroed.
    assert_eq!(player.military_advisor_last(5), 0);
    assert_eq!(player.military_reroll_yday(), 55);
    // Out-of-range index clamps to the last valid slot, matching
    // every other slot accessor's guard in this file.
    assert_eq!(player.military_advisor_last(999), 200);

    // These fields must not disturb the mission-slot/preference
    // fields exercised by the neighboring tests.
    let mission = crate::world::SingleMission {
        mission_type: crate::world::MISSION_TYPE_RATLING,
        opt1: 3,
        opt2: 20,
        pts: 5,
        exp: 50,
    };
    player.set_military_mission(2, mission);
    player.set_mission_yday(99);
    assert_eq!(player.military_mission(2), mission);
    assert_eq!(player.mission_yday(), 99);
    assert_eq!(player.military_advisor_last(0), 10);
    assert_eq!(player.military_reroll_yday(), 55);
}

#[test]
fn military_ppd_remaining_opaque_fields_gained_accessors_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.master_state(), 0);
    assert_eq!(player.current_advisor(), 0);
    assert_eq!(player.advisor_state(), 0);
    assert_eq!(player.advisor_cost(), 0);
    assert_eq!(player.advisor_storage_nr(), 0);
    assert_eq!(player.military_pts(), 0);
    assert_eq!(player.military_normal_exp_ppd(), 0);
    assert_eq!(player.military_recommend(), 0);
    assert_eq!(player.temp_mission_type(), 0);
    assert_eq!(player.temp_mission_difficulty(), 0);

    player.set_master_state(2);
    player.set_current_advisor(31);
    player.set_advisor_state(2);
    player.set_advisor_cost(1200);
    player.set_advisor_storage_nr(4);
    player.set_military_pts(1000);
    player.set_military_normal_exp_ppd(42);
    player.set_military_recommend(101);
    player.set_temp_mission_type(2);
    player.set_temp_mission_difficulty(3);

    assert_eq!(player.master_state(), 2);
    assert_eq!(player.current_advisor(), 31);
    assert_eq!(player.advisor_state(), 2);
    assert_eq!(player.advisor_cost(), 1200);
    assert_eq!(player.advisor_storage_nr(), 4);
    assert_eq!(player.military_pts(), 1000);
    assert_eq!(player.military_normal_exp_ppd(), 42);
    assert_eq!(player.military_recommend(), 101);
    assert_eq!(player.temp_mission_type(), 2);
    assert_eq!(player.temp_mission_difficulty(), 3);

    // These fields must not disturb the existing header/tail fields
    // (`current_pts`, `advisor_last`, `reroll_yday`) already
    // exercised by neighboring tests.
    player.set_military_current_pts(77);
    player.set_military_advisor_last(0, 10);
    player.set_military_reroll_yday(55);
    assert_eq!(player.military_current_pts(), 77);
    assert_eq!(player.military_advisor_last(0), 10);
    assert_eq!(player.military_reroll_yday(), 55);
    assert_eq!(player.master_state(), 2);
    assert_eq!(player.current_advisor(), 31);
    assert_eq!(player.military_pts(), 1000);
    assert_eq!(player.temp_mission_difficulty(), 3);
}

// C `generate_mission_with_preference(cn, ppd, preferred_type)`
// (`military.c:1036-1131`)'s ppd-mutating half, exercised via
// `PlayerRuntime::apply_mission_offer`.
#[test]
fn apply_mission_offer_writes_missions_preference_and_yday() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut seed = 17u32;

    // Level 5 (floored to 7 internally) is below both the ratling
    // (9..=39) and silver (>=12) level gates, so every slot stays a
    // demon mission regardless of the "small chance of other mission
    // types" branch (`preferred_type == 1` matches C's `case 1:
    // default:` no-extra-preference path).
    player.apply_mission_offer(5, 0, 1, 50, &mut seed);

    for idx in 0..5 {
        assert_eq!(
            player.military_mission(idx).mission_type,
            crate::world::MISSION_TYPE_DEMON
        );
    }
    assert_eq!(player.mission_type_preference(), 1);
    assert_eq!(player.mission_yday(), 51);
}

#[test]
fn apply_mission_offer_uses_stored_difficulty_preference() {
    let mut player = PlayerRuntime::connected(1, 0);
    // Silver preference (type 3) at difficulty 4 (insane): the base
    // demon-mission fill would leave slot 4 as a demon mission unless
    // overwritten, but the stored difficulty preference forces slot 4
    // specifically to a silver mission (C's own final `ppd->mis[diff]
    // = mission` override, applied after the main preferred-type
    // switch already ran).
    player.set_mission_difficulty_preference(4);
    let mut seed = 3u32;

    player.apply_mission_offer(20, 0, 3, 0, &mut seed);

    assert_eq!(
        player.military_mission(4).mission_type,
        crate::world::MISSION_TYPE_SILVER
    );
    assert_eq!(player.military_mission(4).pts, 25);
}

#[test]
fn military_ppd_blob_round_trips_through_encode_decode() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mission = crate::world::SingleMission {
        mission_type: crate::world::MISSION_TYPE_RATLING,
        opt1: 3,
        opt2: 15,
        pts: 2,
        exp: 40,
    };
    player.set_military_mission(0, mission);
    player.set_military_took_mission(1);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut round_tripped = PlayerRuntime::connected(1, 0);
    assert!(round_tripped.decode_legacy_ppd_blob(&encoded));
    assert_eq!(round_tripped.military_ppd, player.military_ppd);
    assert_eq!(round_tripped.military_mission(0), mission);
    assert_eq!(round_tripped.military_took_mission(), 1);
}

#[test]
fn clear_turn_seyan_ppd_clears_military_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    assert!(!player.military_ppd.is_empty());

    player.clear_turn_seyan_ppd();
    assert!(player.military_ppd.is_empty());
    assert_eq!(player.military_took_mission(), 0);
}

// C `check_military_solve` (`src/system/death.c:290-383`).
#[test]
fn check_military_solve_no_active_mission_is_no_match() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(
        player.check_military_solve(700, 40),
        crate::world::MilitaryMissionProgress::NoMatch
    );
}

#[test]
fn check_military_solve_already_solved_is_no_match() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 1,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    player.set_military_solved_mission(true);
    assert_eq!(
        player.check_military_solve(700, 40),
        crate::world::MilitaryMissionProgress::NoMatch
    );
}

#[test]
fn check_military_solve_demon_mission_wrong_class_is_no_match() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 5,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    // Class 200 is neither a pent demon nor a ratling.
    assert_eq!(
        player.check_military_solve(200, 40),
        crate::world::MilitaryMissionProgress::NoMatch
    );
    // opt1 must stay untouched on a non-match.
    assert_eq!(player.military_mission(0).opt1, 5);
}

#[test]
fn check_military_solve_demon_mission_wrong_level_is_no_match() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 5,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    // Level 38 is more than 1 away from target level 40.
    assert_eq!(
        player.check_military_solve(52, 38),
        crate::world::MilitaryMissionProgress::NoMatch
    );
}

#[test]
fn check_military_solve_demon_mission_accepts_adjacent_levels() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 5,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    // Level 39 (target - 1) matches.
    assert_eq!(
        player.check_military_solve(52, 39),
        crate::world::MilitaryMissionProgress::Progress {
            remaining: 4,
            elite_count: 1
        }
    );
}

#[test]
fn check_military_solve_elite_demon_counts_as_ten() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 15,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    let elite_class = crate::world::ELITE_DEMON_CLASS_BASE;
    assert_eq!(
        player.check_military_solve(elite_class, 40),
        crate::world::MilitaryMissionProgress::Progress {
            remaining: 5,
            elite_count: 10
        }
    );
    assert_eq!(player.military_mission(0).opt1, 5);
}

#[test]
fn check_military_solve_ratling_mission_progress_and_solve() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        2,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_RATLING,
            opt1: 2,
            opt2: 20,
            pts: 4,
            exp: 80,
        },
    );
    player.set_military_took_mission(3); // slot index 2 (took_mission - 1)

    assert_eq!(
        player.check_military_solve(90, 20),
        crate::world::MilitaryMissionProgress::Progress {
            remaining: 1,
            elite_count: 1
        }
    );
    assert!(!player.military_solved_mission());

    assert_eq!(
        player.check_military_solve(90, 21),
        crate::world::MilitaryMissionProgress::Solved
    );
    assert!(player.military_solved_mission());
    assert_eq!(player.military_mission(2).opt1, 0);

    // Once solved, further kills are a no-op even if they'd otherwise
    // match.
    assert_eq!(
        player.check_military_solve(90, 20),
        crate::world::MilitaryMissionProgress::NoMatch
    );
}

#[test]
fn check_military_solve_never_underflows_opt1_below_zero() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_mission(
        0,
        crate::world::SingleMission {
            mission_type: crate::world::MISSION_TYPE_DEMON,
            opt1: 3,
            opt2: 40,
            pts: 1,
            exp: 1,
        },
    );
    player.set_military_took_mission(1);
    // An elite kill (worth 10) against a remaining count of 3 must
    // clamp at 0, not go negative, and still solve the mission.
    assert_eq!(
        player.check_military_solve(crate::world::ELITE_DEMON_CLASS_BASE, 40),
        crate::world::MilitaryMissionProgress::Solved
    );
    assert_eq!(player.military_mission(0).opt1, 0);
}
