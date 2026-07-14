use super::military::*;
use super::*;
use crate::character_driver::parse_military_master_driver_args;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};

//-----------------------
// CDR_MILITARY_MASTER driver (military.c:2108-2206).

// C `military_master_parse` (`military.c:1634-1644`): the only zone-file
// arg this driver reads is `storage=N;`.
#[test]
fn military_master_driver_args_parse_storage_field() {
    let data = parse_military_master_driver_args("storage=42;");
    assert_eq!(data.storage_id, 42);
}

#[test]
fn military_master_driver_args_default_when_absent() {
    let data = parse_military_master_driver_args("");
    assert_eq!(data.storage_id, 0);
}

// C `diff_name[difficulty]`/`get_colored_difficulty_name`'s clamp
// (`military.c:339,1350-1361`).
#[test]
fn mission_difficulty_name_matches_legacy_table_and_clamps_out_of_range() {
    assert_eq!(mission_difficulty_name(0), "easy");
    assert_eq!(mission_difficulty_name(1), "normal");
    assert_eq!(mission_difficulty_name(2), "hard");
    assert_eq!(mission_difficulty_name(3), "impossible");
    assert_eq!(mission_difficulty_name(4), "insane");
    assert_eq!(mission_difficulty_name(99), "easy");
}

// C `describe_mission` (`military.c:1194-1220`).
#[test]
fn describe_mission_text_renders_each_mission_type() {
    let demon = SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1: 3,
        opt2: 10,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&demon, 0, "Godmode").unwrap(),
        format!(
            "I have an {COL_STR_LIGHT_BLUE}easy{COL_STR_RESET} mission for you, Godmode. It is \
             to slay 3 level 10 demons in the Pentagram Quest."
        )
    );

    let ratling = SingleMission {
        mission_type: MISSION_TYPE_RATLING,
        opt1: 4,
        opt2: 12,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&ratling, 2, "Godmode").unwrap(),
        format!(
            "I have an {COL_STR_LIGHT_BLUE}hard{COL_STR_RESET} mission for you, Godmode. It is \
             to slay 4 level 12 ratlings in the Sewers."
        )
    );

    let silver = SingleMission {
        mission_type: MISSION_TYPE_SILVER,
        opt1: 50,
        opt2: 0,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&silver, 4, "Godmode").unwrap(),
        format!(
            "I have an {COL_STR_LIGHT_BLUE}insane{COL_STR_RESET} mission for you, Godmode. It \
             is to find 50 units of silver in the Mine."
        )
    );

    assert!(describe_mission_text(&SingleMission::default(), 0, "Godmode").is_none());
}

// C `display_mission` (`military.c:1261-1288`).
#[test]
fn display_mission_text_renders_each_mission_type() {
    let demon = SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1: 3,
        opt2: 10,
        ..Default::default()
    };
    assert_eq!(
        display_mission_text(&demon).unwrap(),
        "Your mission is to slay 3 level 10 demons in the Pentagram Quest."
    );

    let silver = SingleMission {
        mission_type: MISSION_TYPE_SILVER,
        opt1: 50,
        ..Default::default()
    };
    assert_eq!(
        display_mission_text(&silver).unwrap(),
        "Your mission is to find 50 units of silver in the Mine."
    );

    assert!(display_mission_text(&SingleMission::default()).is_none());
}

// C `offer_missions` (`military.c:1231-1246`): skips missions the player
// can't afford (`pts > 1 && pts > current_pts`), falling back to the "no
// suitable missions" line if none qualified.
#[test]
fn offer_missions_text_skips_unaffordable_missions() {
    let missions = [
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 1,
            opt2: 5,
            pts: 1,
            exp: 10,
        },
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 2,
            opt2: 6,
            pts: 500,
            exp: 20,
        },
        SingleMission::default(),
        SingleMission::default(),
        SingleMission::default(),
    ];

    let lines = offer_missions_text(&missions, 10, "Godmode");
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("slay 1 level 5 demons"));
}

#[test]
fn offer_missions_text_falls_back_when_nothing_affordable() {
    let missions: [SingleMission; 5] = std::array::from_fn(|_| SingleMission::default());
    let lines = offer_missions_text(&missions, 0, "Godmode");
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("I don't have any suitable missions"));
}

// C `handle_mission_request` (`military.c:1842-1896`): already has a
// mission.
#[test]
fn handle_mission_request_blocked_by_active_mission() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::AlreadyHasMission);
}

#[test]
fn handle_mission_request_blocked_by_completed_today() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::AlreadyCompletedToday);
}

// C: `!get_army_rank_int(co)` -> not enrolled in the army yet.
#[test]
fn handle_mission_request_rejects_unenrolled_player() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::NotEnrolled);
}

#[test]
fn handle_mission_request_generates_and_offers_missions_for_enrolled_player() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    character_data.military_points = 1000; // rank 10, enrolled.
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::Offered(lines) = outcome else {
        panic!("expected Offered, got {outcome:?}");
    };
    // Reroll-footer line always appended last. C `military.c:1893` wraps
    // "reroll" in `COL_LIGHT_BLUE`/`COL_RESET`.
    assert!(lines.last().unwrap().contains(&format!(
        "saying {COL_STR_LIGHT_BLUE}reroll{COL_STR_RESET} for 200 gold"
    )));
    assert_eq!(player.mission_yday(), 101);
    // A fresh 5-slot offer table was generated (matches `mission_reroll`'s
    // own equivalent assertion).
    for idx in 0..5 {
        assert_ne!(player.military_mission(idx).mission_type, 0);
    }
}

// C: re-requesting the same day's already-generated offer table doesn't
// regenerate it (`ppd->mission_yday == yday + 1` guard) - still renders
// the listing from the existing table.
#[test]
fn handle_mission_request_reuses_todays_offer_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_mission(
        0,
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 7,
            opt2: 9,
            pts: 1,
            exp: 10,
        },
    );
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::Offered(lines) = outcome else {
        panic!("expected Offered, got {outcome:?}");
    };
    assert!(lines
        .iter()
        .any(|line| line.contains("slay 7 level 9 demons")));
}

// C: a fresh advisor-recommended mission short-circuits the general
// offer listing.
#[test]
fn handle_mission_request_advisor_recommendation_short_circuits() {
    let mut world = World::default();
    let mut character_data = character(1);
    // Odd level so `generate_single_ratling_mission`'s `adjusted_level &
    // 1 == 0` rejection doesn't kick in for difficulty 1 (`adjusted_level
    // == level` below difficulty 3).
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::AdvisorRecommendation {
        description,
        prompt,
    } = outcome
    else {
        panic!("expected AdvisorRecommendation, got {outcome:?}");
    };
    assert!(description.contains("ratlings in the Sewers"));
    // C `military.c:1876` wraps the difficulty word in
    // `COL_LIGHT_BLUE`/`COL_RESET`.
    assert!(prompt.contains(&format!("saying {COL_STR_LIGHT_BLUE}normal{COL_STR_RESET}")));
}

// C `process_advisor_recommendation` (`military.c:1685-1755`): already
// processed today (`ppd->recommend == yday + 1`) is a total no-op.
#[test]
fn process_advisor_recommendation_skips_when_already_processed_today() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_recommend(101);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, AdvisorRecommendationOutcome::AlreadyProcessed);
    // Untouched - C's own guard returns before the trailing `ppd->recommend
    // = yday + 1` stamp too.
    assert_eq!(player.military_recommend(), 101);
}

// C: no specific-mission preference and no matching `advisor_last[n]` ->
// an empty `StandardRecommendations` list, but `recommend` is still
// stamped (C's own unconditional trailing assignment).
#[test]
fn process_advisor_recommendation_standard_branch_empty_when_nothing_matched() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(
        outcome,
        AdvisorRecommendationOutcome::StandardRecommendations(Vec::new())
    );
    assert_eq!(player.military_recommend(), 101);
}

// C: the standard branch reports every `advisor_last[n]` entry stamped
// today, by index.
#[test]
fn process_advisor_recommendation_standard_branch_reports_every_matching_advisor() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(0, 101);
    player.set_military_advisor_last(3, 101);
    player.set_military_advisor_last(5, 50); // Not today - excluded.
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::StandardRecommendations(lines) = outcome else {
        panic!("expected StandardRecommendations, got {outcome:?}");
    };
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("advisor 0"));
    assert!(lines[1].contains("advisor 3"));
}

// C: a specific-mission preference short-circuits into the paid-favor
// greeting, regenerating a fresh offer table for today
// (`mission_yday != yday + 1`), describing the preferred slot, and
// prompting "say <difficulty>" since nothing blocks acceptance.
#[test]
fn process_advisor_recommendation_specific_mission_regenerates_and_prompts_accept() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission {
        greeting,
        description,
        followup,
    } = outcome
    else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(greeting.contains("oddly specific request for normal ratling-hunting"));
    assert!(description.unwrap().contains("ratlings in the Sewers"));
    // C `military.c:1742` wraps the difficulty word in `COL_LIGHT_BLUE`/`COL_RESET`.
    assert!(followup.contains(&format!(
        "Say {COL_STR_LIGHT_BLUE}normal{COL_STR_RESET} to accept this mission"
    )));
    assert_eq!(player.mission_yday(), 101);
    assert_eq!(player.military_recommend(), 101);
}

// C: reuses today's already-generated offer table instead of
// regenerating (`mission_yday == yday + 1` guard) - still describes
// whatever is already sitting in the preferred slot.
#[test]
fn process_advisor_recommendation_specific_mission_reuses_todays_offer_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_mission_yday(101);
    player.set_military_mission(
        1,
        SingleMission {
            mission_type: MISSION_TYPE_RATLING,
            opt1: 7,
            opt2: 9,
            pts: 1,
            exp: 10,
        },
    );
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { description, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(description.unwrap().contains("slay 7 level 9 ratlings"));
}

// C: the already-completed-today follow-up line wins over the accept
// prompt.
#[test]
fn process_advisor_recommendation_specific_mission_already_completed_today_followup() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_military_solved_yday(101);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { followup, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(followup.contains("you've already completed a mission today"));
}

// C: the active-mission-conflict follow-up line wins over the accept
// prompt when the player already took a (different) mission.
#[test]
fn process_advisor_recommendation_specific_mission_active_mission_conflict_followup() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_military_took_mission(3);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { followup, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(followup.contains("you already have an active mission"));
}

// C: the difficulty-name ternary used in this function's own text falls
// through to "insane" for any preference other than 0-3 (unlike
// `mission_difficulty_name`'s out-of-range clamp to "easy") - exercised
// here via preference `4` ("insane" itself, the highest real difficulty)
// to also confirm the description embeds the demon-mission text (no
// type preference set, so C's `describe_mission` falls back on whatever
// was last generated - here nothing, so `None`).
#[test]
fn process_advisor_recommendation_difficulty_text_falls_through_to_insane() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(4);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission {
        greeting, followup, ..
    } = outcome
    else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(greeting.contains("oddly specific request for insane ratling-hunting"));
    assert!(followup.contains(&format!(
        "Say {COL_STR_LIGHT_BLUE}insane{COL_STR_RESET} to accept this mission"
    )));
}

// C `military_master_driver`'s `NT_CHAR` branch (`military.c:2153-2177`),
// ported as a periodic nearby-player scan.
#[test]
fn military_master_greet_scan_queues_nearby_visible_player() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let mut visitor = recruit(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    world.process_military_master_actions(0, 0);

    let events = world.drain_pending_military_master_events();
    assert!(events.contains(&MilitaryMasterEvent::NearbyPlayer {
        master_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_master_greet_scan_skips_out_of_range_player() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    world.process_military_master_actions(0, 0);

    assert!(world.drain_pending_military_master_events().is_empty());
}

#[test]
fn military_master_replies_to_small_talk_keyword_directly() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let mut visitor = recruit(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_military_master_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
    // The visitor is also in range of the periodic `NT_CHAR` greet scan
    // (same tile as the master) - only that event, no message-driven one,
    // should have been queued for a plain "hello".
    let events = world.drain_pending_military_master_events();
    assert!(events
        .iter()
        .all(|event| matches!(event, MilitaryMasterEvent::NearbyPlayer { .. })));
}

#[test]
fn military_master_whats_your_name_replies_with_own_name() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "what's your name");
    }
    world.process_military_master_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'm Seymour.")));
}

#[test]
fn military_master_mission_keyword_queues_mission_request_event() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "mission");
    }
    world.process_military_master_actions(0, 0);

    let events = world.drain_pending_military_master_events();
    assert!(events.contains(&MilitaryMasterEvent::MissionRequest {
        master_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_master_difficulty_keywords_queue_accept_mission_events_with_correct_difficulty() {
    let cases = [
        ("easy", 0usize),
        ("normal", 1),
        ("hard", 2),
        ("impossible", 3),
        ("insane", 4),
    ];
    for (keyword, expected_difficulty) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&MilitaryMasterEvent::AcceptMission {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
                difficulty: expected_difficulty,
            }),
            "keyword {keyword:?} expected difficulty {expected_difficulty}, got {events:?}"
        );
    }
}

#[test]
fn military_master_repeat_failed_hear_and_reroll_keywords_queue_matching_events() {
    let cases = [
        (
            "repeat",
            MilitaryMasterEvent::Repeat {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "failed",
            MilitaryMasterEvent::Failed {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "hear",
            MilitaryMasterEvent::Hear {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "reroll",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "decline",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "new missions",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ];
    for (keyword, expected_event) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&expected_event),
            "keyword {keyword:?} expected {expected_event:?}, got {events:?}"
        );
    }
}

// Advisor-only codes (favor/small/medium/big/huge/vast/pay) and
// advisor-recommendation combo codes (e.g. "easy demon") are matched by
// the shared qa table but not handled by the Master driver - matches C's
// own `default: return 0`. The admin-only codes (info/reset/raise/
// promote) are also matched here but require `CF_GOD` on the speaker
// (`military.c:2037-2089`'s shared guard) - a non-admin speaker gets the
// same silent no-op, exercised below with `recruit` (no `GOD` flag).
#[test]
fn military_master_ignores_advisor_and_non_admin_codes() {
    for keyword in [
        "favor",
        "small",
        "pay",
        "info",
        "reset",
        "raise",
        "promote",
        "easy demon",
    ] {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        // The visitor is also in range of the periodic `NT_CHAR` greet
        // scan (same tile as the master) - only that event, never a
        // message-driven one, should have been queued for these
        // Master-ignored/non-admin codes.
        let events = world.drain_pending_military_master_events();
        assert!(
            events
                .iter()
                .all(|event| matches!(event, MilitaryMasterEvent::NearbyPlayer { .. })),
            "keyword {keyword:?} should not queue a message-driven event, got {events:?}"
        );
    }
}

// C `military.c:2037-2089`'s shared `if (!(ch[co].flags & CF_GOD)) break;`
// guard: a `CF_GOD`-flagged speaker's "info"/"reset"/"raise"/"promote"
// keywords each queue their matching admin-only event.
#[test]
fn military_master_admin_codes_queue_matching_events_for_god_speaker() {
    let cases = [
        (
            "info",
            MilitaryMasterEvent::Info {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "reset",
            MilitaryMasterEvent::Reset {
                player_id: CharacterId(2),
            },
        ),
        (
            "raise",
            MilitaryMasterEvent::Raise {
                player_id: CharacterId(2),
            },
        ),
        (
            "promote",
            MilitaryMasterEvent::Promote {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ];
    for (keyword, expected_event) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let mut admin = recruit(2);
        admin.flags |= CharacterFlags::GOD;
        assert!(world.spawn_character(admin, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&expected_event),
            "keyword {keyword:?} expected {expected_event:?}, got {events:?}"
        );
    }
}

#[test]
fn military_master_ignores_text_from_speaker_out_of_range() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "mission");
    }
    world.process_military_master_actions(0, 0);

    assert!(world.drain_pending_military_master_events().is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn military_master_given_item_is_destroyed_and_replies_junk() {
    let mut world = World::default();
    let mut master = master_npc(1);
    master.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(master, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_military_master_actions(0, 0);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("That's junk.")));
}
