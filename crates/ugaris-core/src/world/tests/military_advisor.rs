use super::military::*;
use super::*;
use crate::character_driver::{
    parse_military_advisor_driver_args, MilitaryAdvisorDriverData, CDR_MILITARY_ADVISOR,
};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};

//-----------------------
// CDR_MILITARY_ADVISOR driver (military.c:2607-2699).

fn advisor_npc(id: u32, storage_id: i32) -> Character {
    let mut advisor = character(id);
    advisor.name = "Advisor".into();
    advisor.driver = CDR_MILITARY_ADVISOR;
    advisor.driver_state = Some(CharacterDriverState::MilitaryAdvisor(
        MilitaryAdvisorDriverData { storage_id },
    ));
    advisor
}

// C `military_advisor_parse` (`military.c:2221-2230`): the only
// zone-file arg this driver reads is `storage=N;`.
#[test]
fn military_advisor_driver_args_parse_storage_field() {
    let data = parse_military_advisor_driver_args("storage=42;");
    assert_eq!(data.storage_id, 42);
}

#[test]
fn military_advisor_driver_args_default_when_absent() {
    let data = parse_military_advisor_driver_args("");
    assert_eq!(data.storage_id, 0);
}

#[test]
fn advisor_storage_id_reads_driver_state() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 33), 10, 10));
    assert_eq!(world.advisor_storage_id(CharacterId(1)), 33);
}

#[test]
fn advisor_storage_id_defaults_to_zero_for_mismatched_driver_state() {
    let mut world = World::default();
    assert!(world.spawn_character(character(1), 10, 10));
    assert_eq!(world.advisor_storage_id(CharacterId(1)), 0);
}

// C `military.c:339`'s favor-size name table (`offer_favor`'s switch).
#[test]
fn favor_size_name_matches_c_table() {
    assert_eq!(favor_size_name(0), "small");
    assert_eq!(favor_size_name(1), "medium");
    assert_eq!(favor_size_name(2), "big");
    assert_eq!(favor_size_name(3), "huge");
    assert_eq!(favor_size_name(4), "vast");
    // Out-of-range falls back to "vast" (C's own trailing `: "vast"`
    // ternary chain default).
    assert_eq!(favor_size_name(999), "vast");
}

#[test]
fn mission_type_name_matches_c_table() {
    assert_eq!(mission_type_name(1), "demon-slaying");
    assert_eq!(mission_type_name(2), "ratling-hunting");
    assert_eq!(mission_type_name(3), "silver-mining");
    assert_eq!(mission_type_name(0), "unknown");
    assert_eq!(mission_type_name(999), "unknown");
}

// C `adv_introduction` (`military.c:2262-2281`): 4 rotating greetings
// keyed by `storage_ID % 4`.
#[test]
fn adv_introduction_text_rotates_by_storage_id_modulo_four() {
    // C wraps every "favor" in `COL_LIGHT_BLUE`/`COL_RESET`
    // (`military.c:2262-2281`).
    assert!(adv_introduction_text(0, "Bob").contains(&format!(
        "I could do you a {COL_STR_LIGHT_BLUE}favor{COL_STR_RESET}, Bob"
    )));
    assert!(adv_introduction_text(1, "Bob").contains("Say, Bob, would you like to speed up"));
    assert!(
        adv_introduction_text(2, "Bob").contains("Not getting promoted as fast as you want, Bob?")
    );
    assert!(adv_introduction_text(3, "Bob").contains(&format!(
        "Need a {COL_STR_LIGHT_BLUE}favor{COL_STR_RESET}, Bob?"
    )));
    // Wraps around: storage_ID 4 behaves like 0, 7 like 3.
    assert_eq!(
        adv_introduction_text(4, "Bob"),
        adv_introduction_text(0, "Bob")
    );
    assert_eq!(
        adv_introduction_text(7, "Bob"),
        adv_introduction_text(3, "Bob")
    );
}

#[test]
fn adv_favor_desc_lines_matches_c_text() {
    // C wraps every favor-size word and the two example phrases in
    // `COL_LIGHT_BLUE`/`COL_RESET` (`military.c:2296-2308`).
    let lines = adv_favor_desc_lines();
    assert_eq!(
        lines[0],
        format!(
            "My favors come in five sizes, {COL_STR_LIGHT_BLUE}small{COL_STR_RESET}, \
             {COL_STR_LIGHT_BLUE}medium{COL_STR_RESET}, {COL_STR_LIGHT_BLUE}big{COL_STR_RESET}, \
             {COL_STR_LIGHT_BLUE}huge{COL_STR_RESET} and {COL_STR_LIGHT_BLUE}vast{COL_STR_RESET}."
        )
    );
    assert!(lines[1].contains("easy demon"));
    assert!(lines[1].contains("insane mining"));
}

// C `offer_favor` (`military.c:2339-2382`).
#[test]
fn offer_favor_already_used_today_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(5, 101);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 0, 100);

    assert_eq!(outcome, OfferFavorOutcome::AlreadyUsedToday);
}

#[test]
fn offer_favor_invalid_favor_size_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 99, 100);

    assert_eq!(outcome, OfferFavorOutcome::InvalidFavorSize);
}

#[test]
fn offer_favor_stamps_cost_state_and_storage_nr_matching_price_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 30;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 2, 100);

    // level 30 -> advisor_price = 800; favor_size 2 ("big") -> x10.
    assert_eq!(
        outcome,
        OfferFavorOutcome::Offered {
            favor_size: 2,
            cost: 8000
        }
    );
    assert_eq!(player.advisor_cost(), 8000);
    assert_eq!(player.advisor_state(), 2);
    assert_eq!(player.advisor_storage_nr(), 2);
    // `offer_favor` itself never stamps `advisor_last` (only
    // `process_favor_payment` does, on actual payment).
    assert_eq!(player.military_advisor_last(5), 0);
}

// C `handle_specific_mission_request` (`military.c:481-566`).
#[test]
fn handle_specific_mission_request_already_used_today_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(2, 101);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 2, 0, 1, 100);

    assert_eq!(outcome, SpecificMissionRequestOutcome::AlreadyUsedToday);
}

#[test]
fn handle_specific_mission_request_rejects_invalid_mission_type_and_difficulty() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 0, 100),
        SpecificMissionRequestOutcome::InvalidMissionType
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 4, 100),
        SpecificMissionRequestOutcome::InvalidMissionType
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, -1, 1, 100),
        SpecificMissionRequestOutcome::InvalidDifficulty
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 5, 1, 100),
        SpecificMissionRequestOutcome::InvalidDifficulty
    );
}

#[test]
fn handle_specific_mission_request_ratling_needs_odd_level_between_nine_and_thirty_nine() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 10; // even -> rejected
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 2, 100),
        SpecificMissionRequestOutcome::RatlingLevelGate
    );
}

#[test]
fn handle_specific_mission_request_silver_needs_level_twelve_or_above() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 11;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 3, 100),
        SpecificMissionRequestOutcome::SilverLevelGate
    );
}

#[test]
fn handle_specific_mission_request_offers_and_stamps_temp_preferences() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 2, 1, 100);

    match outcome {
        SpecificMissionRequestOutcome::Offered {
            difficulty,
            mission_type,
            cost,
            already_completed_today,
            has_active_mission,
        } => {
            assert_eq!(difficulty, 2);
            assert_eq!(mission_type, 1);
            assert_eq!(cost, specific_mission_price(20, 2, 1));
            assert!(!already_completed_today);
            assert!(!has_active_mission);
        }
        other => panic!("expected Offered, got {other:?}"),
    }
    assert_eq!(player.advisor_state(), 2);
    assert_eq!(player.advisor_storage_nr(), 2);
    assert_eq!(player.temp_mission_type(), 1);
    assert_eq!(player.temp_mission_difficulty(), 2);
}

#[test]
fn handle_specific_mission_request_surfaces_already_completed_and_active_mission_warnings() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);
    player.set_military_took_mission(1);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 1, 100);

    match outcome {
        SpecificMissionRequestOutcome::Offered {
            already_completed_today,
            has_active_mission,
            ..
        } => {
            assert!(already_completed_today);
            assert!(has_active_mission);
        }
        other => panic!("expected Offered, got {other:?}"),
    }
}

// C `process_favor_payment` (`military.c:2402-2474`).
#[test]
fn process_favor_payment_nothing_agreed_when_state_or_advisor_mismatches() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    // advisor_state defaults to 0, not 2.
    player.set_current_advisor(5);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 0, 5, 100);

    assert_eq!(outcome, ProcessFavorPaymentOutcome::NothingAgreed);
    assert_eq!(player.advisor_state(), 1);
}

#[test]
fn process_favor_payment_insufficient_gold() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 50;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(100);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 0, 5, 100);

    assert_eq!(outcome, ProcessFavorPaymentOutcome::InsufficientGold);
    assert_eq!(world.characters[&CharacterId(1)].gold, 50);
}

#[test]
fn process_favor_payment_arranges_plain_favor_and_grants_points() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(1200);
    player.set_advisor_storage_nr(2); // "big" favor

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 7, 5, 100);

    assert_eq!(
        outcome,
        ProcessFavorPaymentOutcome::FavorArranged { favor_size: 2 }
    );
    assert_eq!(world.characters[&CharacterId(1)].gold, 8_800);
    assert_eq!(player.military_current_pts(), 2 + 2 * 2);
    assert_eq!(player.advisor_state(), 1);
    assert_eq!(player.military_advisor_last(7), 101);
    // C `add_cost(ppd->advisor_cost, dat->storage_data + ppd->
    // advisor_storage_nr)` (`military.c:2421`): storage_id 5, slot 2
    // ("big" favor) records the 1200 payment.
    assert_eq!(world.military_advisor_storage.earned(5, 2), 1200);
    assert_eq!(world.military_advisor_storage.sold(5, 2), 1);
    // Other slots/storage ids stay untouched.
    assert_eq!(world.military_advisor_storage.sold(5, 0), 0);
    assert_eq!(world.military_advisor_storage.sold(6, 2), 0);
}

#[test]
fn process_favor_payment_records_cost_across_multiple_sales() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(9);
    player.set_advisor_state(2);
    player.set_advisor_cost(300);
    player.set_advisor_storage_nr(0); // "small" favor

    let _ = world.process_favor_payment(CharacterId(1), &mut player, 0, 9, 100);

    // A second sale of a different favor size on the same NPC.
    player.set_advisor_state(2);
    player.set_advisor_cost(700);
    player.set_advisor_storage_nr(0);
    let _ = world.process_favor_payment(CharacterId(1), &mut player, 0, 9, 100);

    assert_eq!(world.military_advisor_storage.earned(9, 0), 1000);
    assert_eq!(world.military_advisor_storage.sold(9, 0), 2);
}

#[test]
fn process_favor_payment_arranges_specific_mission_and_stamps_preferences() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(500);
    player.set_temp_mission_type(2);
    player.set_temp_mission_difficulty(3);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 7, 5, 100);

    assert_eq!(
        outcome,
        ProcessFavorPaymentOutcome::SpecificMissionArranged {
            mission_type: 2,
            difficulty: 3
        }
    );
    assert_eq!(player.mission_type_preference(), 2);
    assert_eq!(player.mission_difficulty_preference(), 3);
    assert_eq!(player.temp_mission_type(), 0);
    assert_eq!(player.temp_mission_difficulty(), -1);
    assert_eq!(player.military_advisor_last(7), 101);
    // Not a plain favor, so no `current_pts` were granted.
    assert_eq!(player.military_current_pts(), 0);
}

// Driver-level event generation (`military_advisor_driver`,
// `military.c:2607-2699`).
#[test]
fn military_advisor_greet_scan_queues_nearby_visible_player() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 11, 10));

    world.process_military_advisor_actions(0);

    let events = world.drain_pending_military_advisor_events();
    assert!(events.contains(&MilitaryAdvisorEvent::NearbyPlayer {
        advisor_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_advisor_ignores_text_from_speaker_out_of_range() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_text_message(CharacterId(2), "favor");
    }
    world.process_military_advisor_actions(0);

    assert!(world.drain_pending_military_advisor_events().is_empty());
}

#[test]
fn military_advisor_repeat_and_favor_keywords_queue_matching_events() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for (keyword, expected) in [
        (
            "repeat",
            MilitaryAdvisorEvent::Repeat {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "favor",
            MilitaryAdvisorEvent::FavorDesc {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "small",
            MilitaryAdvisorEvent::Favor {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                favor_size: 0,
            },
        ),
        (
            "vast",
            MilitaryAdvisorEvent::Favor {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                favor_size: 4,
            },
        ),
        (
            "pay",
            MilitaryAdvisorEvent::Pay {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events.contains(&expected),
            "keyword {keyword:?} should queue {expected:?}, got {events:?}"
        );
    }
}

#[test]
fn military_advisor_specific_mission_keywords_queue_matching_events() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for (keyword, difficulty, mission_type) in [
        ("easy demon", 0, 1),
        ("insane demon", 4, 1),
        ("easy ratling", 0, 2),
        ("insane ratling", 4, 2),
        ("easy silver", 0, 3),
        ("insane silver", 4, 3),
    ] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events.contains(&MilitaryAdvisorEvent::SpecificMissionRequest {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                difficulty,
                mission_type,
            }),
            "keyword {keyword:?} should queue difficulty {difficulty}/type {mission_type}, got \
             {events:?}"
        );
    }
}

#[test]
fn military_advisor_master_only_and_admin_keywords_are_silently_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for keyword in ["mission", "easy", "reroll", "info", "reset"] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events
                .iter()
                .all(|event| matches!(event, MilitaryAdvisorEvent::NearbyPlayer { .. })),
            "keyword {keyword:?} should not queue a message-driven event, got {events:?}"
        );
    }
}

// C `military.c:2523-2525`'s `if (!(ch[co].flags & CF_GOD)) { break; }`
// guard on the admin-only "info" code: a `CF_GOD`-flagged speaker queues
// the matching event (unlike the non-admin `recruit` speaker exercised by
// `military_advisor_master_only_and_admin_keywords_are_silently_ignored`
// above).
#[test]
fn military_advisor_info_keyword_queues_event_for_god_speaker() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    let mut admin = recruit(2);
    admin.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(admin, 10, 10));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_text_message(CharacterId(2), "info");
    }
    world.process_military_advisor_actions(0);

    let events = world.drain_pending_military_advisor_events();
    assert!(events.contains(&MilitaryAdvisorEvent::Info {
        advisor_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_advisor_given_item_is_destroyed_and_replies_junk() {
    let mut world = World::default();
    let mut advisor = advisor_npc(1, 10);
    advisor.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(advisor, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_military_advisor_actions(0);

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

#[test]
fn military_advisor_movement_rests_facing_dx_right() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));

    world.process_military_advisor_actions(0);

    assert_eq!(world.characters[&CharacterId(1)].dir, 4);
}
