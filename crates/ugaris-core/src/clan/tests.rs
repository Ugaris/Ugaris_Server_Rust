use super::*;

#[test]
fn found_clan_resets_relations_to_neutral_both_ways() {
    let mut relations = ClanRelations::new();
    assert!(relations.found_clan(1, 1_000));
    assert!(relations.exists(1));
    assert_eq!(relations.current_relation(1, 5), ClanRelation::Neutral);
    assert_eq!(relations.current_relation(5, 1), ClanRelation::Neutral);
}

#[test]
fn found_clan_rejects_out_of_range_numbers() {
    let mut relations = ClanRelations::new();
    assert!(!relations.found_clan(0, 0));
    assert!(!relations.found_clan(32, 0));
}

#[test]
fn set_relation_validates_inputs() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    assert_eq!(
        relations.set_relation(0, 2, ClanRelation::War, 0),
        Err(ClanRelationError::InvalidClan(0))
    );
    assert_eq!(
        relations.set_relation(1, 32, ClanRelation::War, 0),
        Err(ClanRelationError::InvalidClan(32))
    );
    assert_eq!(
        relations.set_relation(1, 2, ClanRelation::None, 0),
        Err(ClanRelationError::InvalidRelation)
    );
}

#[test]
fn score_to_level_matches_c_integer_division() {
    assert_eq!(score_to_level(0), 0);
    assert_eq!(score_to_level(99), 0);
    assert_eq!(score_to_level(100), 1);
    assert_eq!(score_to_level(999), 9);
    assert_eq!(score_to_level(1000), 10);
}

#[test]
fn want_relation_and_want_date_read_the_set_relation_side() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ClanRelation::War, 500)
        .unwrap();

    assert_eq!(relations.want_relation(1, 2), ClanRelation::War);
    assert_eq!(relations.want_date(1, 2), 500);
    // The reverse direction and the current relation are unaffected.
    assert_eq!(relations.want_relation(2, 1), ClanRelation::Neutral);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);
}

#[test]
fn want_relation_and_want_date_are_none_zero_for_invalid_clans() {
    let relations = ClanRelations::new();
    assert_eq!(relations.want_relation(0, 1), ClanRelation::None);
    assert_eq!(relations.want_relation(1, 32), ClanRelation::None);
    assert_eq!(relations.want_date(0, 1), 0);
}

#[test]
fn set_relation_only_bumps_want_date_on_change() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ClanRelation::War, 100)
        .unwrap();
    // Re-requesting the same relation later must not reset the timer,
    // matching C's `if (clan[cnr].status.want_relation[onr] != rel)`
    // guard (`clan.c:850-852`).
    relations
        .set_relation(1, 2, ClanRelation::War, 500)
        .unwrap();
    assert_eq!(relations.want_date[1][2], 100);
}

#[test]
fn both_want_same_relation_takes_effect_immediately() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
    relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
    let events = relations.update(0);
    assert_eq!(
        events,
        vec![ClanRelationEvent {
            clan_a: 1,
            clan_b: 2,
            change: ClanRelationChange::Agreed {
                relation: ClanRelation::War
            },
        }]
    );
    assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
    assert_eq!(relations.current_relation(2, 1), ClanRelation::War);
}

#[test]
fn one_sided_war_request_needs_one_hour() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();

    // Before the 1h delay: no change yet.
    let events = relations.update(60 * 60 - 1);
    assert!(events.is_empty());
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);

    // After the 1h delay: escalates to war.
    let events = relations.update(60 * 60 + 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].change, ClanRelationChange::WarStarted);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
}

#[test]
fn both_want_same_relation_jumps_directly_even_across_multiple_steps() {
    // C: the `want1 == want2` check (`clan.c:980-985`) happens before the
    // per-level switch, so when both clans agree on a new relation it
    // takes effect in one tick regardless of how many levels away it is
    // from the current one - it does not step through intermediate
    // levels one tick at a time.
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ClanRelation::Alliance, 0)
        .unwrap();
    let events = relations.update(0);
    assert_eq!(
        events[0].change,
        ClanRelationChange::Agreed {
            relation: ClanRelation::Alliance
        }
    );
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);
}

#[test]
fn both_want_better_but_different_relations_deescalates_one_step() {
    // Clan 1 wants Alliance, clan 2 wants Peace-Treaty: both want
    // something better than Neutral, but they disagree, so the switch
    // branch applies (one step toward Alliance) rather than the
    // immediate-agreement branch.
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ClanRelation::PeaceTreaty, 0)
        .unwrap();
    let events = relations.update(0);
    assert_eq!(events[0].change, ClanRelationChange::PeaceTreatyStarted);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::PeaceTreaty);
}

#[test]
fn alliance_ends_after_24h_one_sided_request() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);

    // One side wants out.
    relations
        .set_relation(1, 2, ClanRelation::PeaceTreaty, 1_000)
        .unwrap();

    let events = relations.update(1_000 + 60 * 60 * 24 - 1);
    assert!(events.is_empty());
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);

    let events = relations.update(1_000 + 60 * 60 * 24 + 1);
    assert_eq!(events[0].change, ClanRelationChange::AllianceEnded);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::PeaceTreaty);
}

#[test]
fn war_ends_only_when_both_want_better() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
    relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
    relations.update(0);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::War);

    // Only clan 1 wants peace: war does not end, even after a long time.
    relations
        .set_relation(1, 2, ClanRelation::Neutral, 0)
        .unwrap();
    let events = relations.update(60 * 60 * 24 * 30);
    assert!(events.is_empty());
    assert_eq!(relations.current_relation(1, 2), ClanRelation::War);

    // Both want a better relation now, but *different* ones (Neutral vs
    // Peace-Treaty): this exercises the one-step de-escalation switch
    // branch rather than the immediate-agreement branch, since the two
    // wants differ.
    relations
        .set_relation(2, 1, ClanRelation::PeaceTreaty, 60 * 60 * 24 * 30)
        .unwrap();
    let events = relations.update(60 * 60 * 24 * 30);
    assert_eq!(events[0].change, ClanRelationChange::WarEnded);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);
}

#[test]
fn feud_ends_after_24h_one_sided_request() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.set_relation(1, 2, ClanRelation::Feud, 0).unwrap();
    relations.set_relation(2, 1, ClanRelation::Feud, 0).unwrap();
    relations.update(0);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::Feud);

    relations
        .set_relation(1, 2, ClanRelation::War, 5_000)
        .unwrap();
    let events = relations.update(5_000 + 60 * 60 * 24 + 1);
    assert_eq!(events[0].change, ClanRelationChange::FeudEnded);
    assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
}

#[test]
fn relation_change_log_messages_match_c_add_clanlog_text_exactly() {
    // `clan.c:980-1083`'s seven distinct `add_clanlog` message shapes,
    // letter-for-letter (note the "Peace-Treaty" vs "Peace Treaty"
    // discrepancy between the `rel_name[]`-driven `Agreed` message
    // and the other hardcoded ones is intentional, matching C).
    assert_eq!(
        ClanRelationChange::Agreed {
            relation: ClanRelation::War
        }
        .log_message("Enemies", 3),
        "War with Enemies (3) started"
    );
    assert_eq!(
        ClanRelationChange::Agreed {
            relation: ClanRelation::PeaceTreaty
        }
        .log_message("Friends", 2),
        "Peace-Treaty with Friends (2) started"
    );
    assert_eq!(
        ClanRelationChange::AllianceEnded.log_message("Foo", 1),
        "Alliance with Foo (1) ended"
    );
    assert_eq!(
        ClanRelationChange::PeaceTreatyEnded.log_message("Foo", 1),
        "Peace Treaty with Foo (1) ended"
    );
    assert_eq!(
        ClanRelationChange::WarStarted.log_message("Foo", 1),
        "War with Foo (1) started"
    );
    assert_eq!(
        ClanRelationChange::PeaceTreatyStarted.log_message("Foo", 1),
        "Peace Treaty with Foo (1) started"
    );
    assert_eq!(
        ClanRelationChange::WarEnded.log_message("Foo", 1),
        "War with Foo (1) ended"
    );
    assert_eq!(
        ClanRelationChange::FeudEnded.log_message("Foo", 1),
        "Feud with Foo (1) ended"
    );
}

#[test]
fn may_enter_own_clan_always_allowed() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    assert!(relations.may_enter(1, 1));
}

#[test]
fn may_enter_denied_for_non_clan_members() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    assert!(!relations.may_enter(0, 1));
}

#[test]
fn may_enter_denied_for_deleted_clan() {
    let relations = ClanRelations::new();
    assert!(!relations.may_enter(1, 5));
}

#[test]
fn may_enter_allowed_only_with_alliance() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    assert!(!relations.may_enter(1, 2)); // still neutral

    relations
        .set_relation(2, 1, ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(1, 2, ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);
    assert!(relations.may_enter(1, 2));
}

#[test]
fn attack_outside_requires_feud() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    assert!(!relations.can_attack_outside(1, 2)); // neutral

    relations.set_relation(1, 2, ClanRelation::Feud, 0).unwrap();
    relations.set_relation(2, 1, ClanRelation::Feud, 0).unwrap();
    relations.update(0);
    assert!(relations.can_attack_outside(1, 2));
    assert!(relations.can_attack_inside(1, 2)); // war/feud also allow inside
}

#[test]
fn attack_inside_allows_war_or_feud_not_neutral() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    assert!(!relations.can_attack_inside(1, 2));

    relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
    relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
    relations.update(0);
    assert!(relations.can_attack_inside(1, 2));
    assert!(!relations.can_attack_outside(1, 2)); // war alone doesn't allow outside
}

#[test]
fn alliance_query_matches_current_relation() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    assert!(!relations.alliance(1, 2));

    relations
        .set_relation(1, 2, ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);
    assert!(relations.alliance(1, 2));
    assert!(relations.alliance(2, 1));
}

#[test]
fn delete_clan_clears_existence_but_keeps_relations() {
    let mut relations = ClanRelations::new();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.delete_clan(2);
    assert!(!relations.exists(2));
    assert!(!relations.may_enter(1, 2));
}

#[test]
fn out_of_range_queries_return_safe_defaults() {
    let relations = ClanRelations::new();
    assert!(!relations.exists(0));
    assert!(!relations.exists(32));
    assert_eq!(relations.current_relation(0, 1), ClanRelation::None);
    assert!(!relations.can_attack_inside(0, 1));
    assert!(!relations.can_attack_outside(1, 100));
    assert!(!relations.alliance(1, 100));
}

fn test_character() -> Character {
    Character {
        merchant: None,
        template_key: String::new(),
        respawn_ticks: 0,
        id: crate::ids::CharacterId(1),
        serial: 1,
        name: "tester".to_string(),
        description: String::new(),
        flags: crate::entity::CharacterFlags::USED,
        sprite: 0,
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
        staff_code: String::new(),
        speed_mode: crate::entity::SpeedMode::Normal,
        x: 0,
        y: 0,
        rest_area: 0,
        rest_x: 0,
        rest_y: 0,
        tox: 0,
        toy: 0,
        dir: 4,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 1000,
        mana: 1000,
        endurance: 1000,
        lifeshield: 0,
        level: 1,
        exp: 0,
        exp_used: 0,
        military_points: 0,
        military_normal_exp: 0,
        gold: 0,
        karma: 0,
        creation_time: 0,
        saves: 0,
        got_saved: 0,
        deaths: 0,
        regen_ticker: 0,
        last_regen: 0,
        cursor_item: None,
        current_container: None,
        values: Character::empty_values(),
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
        driver_memory: crate::character_driver::DriverMemory::default(),
        class: 0,
        dungeonfighter: None,
        fight_driver: None,
        lq_usurp: None,
    }
}

#[test]
fn registry_found_clan_allocates_first_free_slot_with_standard_ranks() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("The Founders", 1_000).unwrap();
    assert_eq!(nr, 1);
    assert!(registry.exists(1));
    assert_eq!(registry.name(1), Some("The Founders"));
    let identity = registry.identity(1).unwrap();
    assert_eq!(
        identity.rank_names,
        ["Member", "Member", "Recruiter", "Treasurer", "Leader"]
    );
    // Relations were reset to neutral by the wrapped ClanRelations.
    assert_eq!(
        registry.relations().current_relation(1, 5),
        ClanRelation::Neutral
    );
}

#[test]
fn registry_found_clan_rejects_names_over_78_chars() {
    let mut registry = ClanRegistry::new();
    let long_name = "x".repeat(79);
    assert_eq!(
        registry.found_clan(&long_name, 0),
        Err(ClanFoundError::NameTooLong)
    );
}

#[test]
fn registry_found_clan_reuses_slot_after_delete_and_bumps_serial() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("First", 0).unwrap();
    assert_eq!(registry.serial(nr), 0);
    registry.delete_clan(nr);
    assert!(!registry.exists(nr));

    let nr2 = registry.found_clan("Second", 0).unwrap();
    assert_eq!(nr2, nr); // same slot reused
    assert_eq!(registry.name(nr2), Some("Second"));
    assert_eq!(registry.serial(nr2), 1); // serial bumped by delete_clan
}

#[test]
fn registry_found_clan_returns_list_full_when_all_slots_used() {
    let mut registry = ClanRegistry::new();
    for n in 1..MAX_CLAN {
        registry.found_clan(&format!("Clan{n}"), 0).unwrap();
    }
    assert_eq!(
        registry.found_clan("Overflow", 0),
        Err(ClanFoundError::ClanListFull)
    );
}

#[test]
fn add_member_then_get_char_clan_round_trips() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Allies", 0).unwrap();
    let mut character = test_character();

    registry.add_member(&mut character, nr).unwrap();
    assert_eq!(character.clan, nr);
    assert_eq!(character.clan_serial, registry.serial(nr));
    assert_eq!(character.clan_rank, 0); // add_member never sets rank

    assert_eq!(registry.get_char_clan(&mut character), Some(nr));
    assert_eq!(character.clan, nr); // untouched on success
}

#[test]
fn add_member_rejects_unknown_clan() {
    let registry = ClanRegistry::new();
    let mut character = test_character();
    assert_eq!(
        registry.add_member(&mut character, 5),
        Err(ClanMembershipError::NotFound)
    );
}

#[test]
fn add_member_rejects_club_numbers() {
    let registry = ClanRegistry::new();
    let mut character = test_character();
    assert_eq!(
        registry.add_member(&mut character, CLUB_OFFSET),
        Err(ClanMembershipError::IsClub)
    );
}

#[test]
fn remove_member_clears_all_three_fields() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Allies", 0).unwrap();
    let mut character = test_character();
    registry.add_member(&mut character, nr).unwrap();
    character.clan_rank = 3;

    registry.remove_member(&mut character);
    assert_eq!(character.clan, 0);
    assert_eq!(character.clan_rank, 0);
    assert_eq!(character.clan_serial, 0);
}

#[test]
fn get_char_clan_clears_stale_reference_after_clan_deleted_and_refounded() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Original", 0).unwrap();
    let mut character = test_character();
    registry.add_member(&mut character, nr).unwrap();

    registry.delete_clan(nr);
    let nr2 = registry.found_clan("Replacement", 0).unwrap();
    assert_eq!(nr2, nr);

    // Character still has the old serial - must be treated as former
    // member of a now-different clan, exactly like C's
    // `ch[cn].clan_serial != clan[cnr].status.serial` check.
    assert_eq!(registry.get_char_clan(&mut character), None);
    assert_eq!(character.clan, 0);
    assert_eq!(character.clan_rank, 0);
    assert_eq!(character.clan_serial, 0);
}

#[test]
fn get_char_clan_ignores_club_numbers() {
    let registry = ClanRegistry::new();
    let mut character = test_character();
    character.clan = CLUB_OFFSET + 3;
    character.clan_rank = 2;
    character.clan_serial = 7;

    assert_eq!(registry.get_char_clan(&mut character), None);
    // Untouched: club membership is a different (unported) system.
    assert_eq!(character.clan, CLUB_OFFSET + 3);
    assert_eq!(character.clan_rank, 2);
    assert_eq!(character.clan_serial, 7);
}

#[test]
fn get_char_clan_zero_means_no_clan() {
    let registry = ClanRegistry::new();
    let mut character = test_character();
    assert_eq!(registry.get_char_clan(&mut character), None);
}

#[test]
fn char_clan_name_resolves_through_get_char_clan() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Named Clan", 0).unwrap();
    let mut character = test_character();
    registry.add_member(&mut character, nr).unwrap();
    assert_eq!(registry.char_clan_name(&mut character), Some("Named Clan"));
}

#[test]
fn set_rankname_validates_rank_and_length() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Ranked", 0).unwrap();

    registry.set_rankname(nr, 4, "Warlord").unwrap();
    assert_eq!(registry.identity(nr).unwrap().rank_names[4], "Warlord");

    assert_eq!(
        registry.set_rankname(nr, 5, "Invalid"),
        Err(ClanIdentityError::InvalidRank)
    );
    assert_eq!(
        registry.set_rankname(nr, 0, &"x".repeat(38)),
        Err(ClanIdentityError::NameTooLong)
    );
    assert_eq!(
        registry.set_rankname(99, 0, "Nobody"),
        Err(ClanIdentityError::NotFound)
    );
}

#[test]
fn set_website_and_message_update_identity() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Web", 0).unwrap();
    registry.set_website(nr, "https://example.com").unwrap();
    registry.set_message(nr, "Welcome!").unwrap();
    let identity = registry.identity(nr).unwrap();
    assert_eq!(identity.website, "https://example.com");
    assert_eq!(identity.message, "Welcome!");
}

#[test]
fn set_website_truncates_to_79_chars() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Web", 0).unwrap();
    let long = "y".repeat(200);
    registry.set_website(nr, &long).unwrap();
    assert_eq!(registry.identity(nr).unwrap().website.len(), 79);
}

#[test]
fn set_name_renames_an_existing_clan() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Old Name", 0).unwrap();
    registry.set_name(nr, "New Name").unwrap();
    assert_eq!(registry.name(nr), Some("New Name"));
}

#[test]
fn set_name_truncates_to_78_chars_without_error() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Old Name", 0).unwrap();
    let long = "z".repeat(200);
    registry.set_name(nr, &long).unwrap();
    assert_eq!(registry.identity(nr).unwrap().name.len(), 78);
}

#[test]
fn set_name_rejects_nonexistent_clan() {
    let mut registry = ClanRegistry::new();
    assert_eq!(
        registry.set_name(5, "Ghost"),
        Err(ClanIdentityError::NotFound)
    );
}

#[test]
fn fresh_registry_is_not_dirty() {
    let registry = ClanRegistry::new();
    assert!(!registry.dirty());
}

#[test]
fn found_clan_marks_registry_dirty() {
    let mut registry = ClanRegistry::new();
    assert!(!registry.dirty());
    registry.found_clan("Dirty", 0).unwrap();
    assert!(registry.dirty());
}

#[test]
fn found_clan_failure_does_not_mark_dirty() {
    let mut registry = ClanRegistry::new();
    assert_eq!(
        registry.found_clan(&"x".repeat(79), 0),
        Err(ClanFoundError::NameTooLong)
    );
    assert!(!registry.dirty());
}

#[test]
fn clear_dirty_resets_the_flag() {
    let mut registry = ClanRegistry::new();
    registry.found_clan("Dirty", 0).unwrap();
    assert!(registry.dirty());
    registry.clear_dirty();
    assert!(!registry.dirty());
}

#[test]
fn delete_clan_marks_registry_dirty_only_when_valid() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Doomed", 0).unwrap();
    registry.clear_dirty();
    registry.delete_clan(999);
    assert!(!registry.dirty(), "out-of-range delete must not mutate");
    registry.delete_clan(nr);
    assert!(registry.dirty());
}

#[test]
fn identity_mutators_mark_registry_dirty() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Mutable", 0).unwrap();

    registry.clear_dirty();
    registry.set_rankname(nr, 0, "Chief").unwrap();
    assert!(registry.dirty());

    registry.clear_dirty();
    registry.set_website(nr, "https://example.com").unwrap();
    assert!(registry.dirty());

    registry.clear_dirty();
    registry.set_message(nr, "hi").unwrap();
    assert!(registry.dirty());

    registry.clear_dirty();
    registry.set_name(nr, "Renamed").unwrap();
    assert!(registry.dirty());
}

#[test]
fn identity_mutator_failure_does_not_mark_dirty() {
    let mut registry = ClanRegistry::new();
    assert_eq!(
        registry.set_rankname(1, 0, "Nobody"),
        Err(ClanIdentityError::NotFound)
    );
    assert!(!registry.dirty());
}

#[test]
fn relations_mut_marks_registry_dirty() {
    let mut registry = ClanRegistry::new();
    assert!(!registry.dirty());
    registry.relations_mut().found_clan(1, 0);
    assert!(registry.dirty());
}

#[test]
fn get_clan_raid_defaults_false_and_nonexistent_reads_false() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Raiders", 0).unwrap();
    assert!(!registry.get_clan_raid(nr));
    assert!(!registry.get_clan_raid(999));
}

#[test]
fn set_clan_raid_on_then_off_toggles_pending_timer_not_raid_itself() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Raiders", 0).unwrap();

    assert_eq!(registry.set_clan_raid(nr, true, 1_000), Ok(()));
    // Only the pending timer moves; `get_clan_raid` (`doraid`) stays
    // false until a GM `set_clan_raid_god` override, matching C.
    assert!(!registry.get_clan_raid(nr));
    assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 1_000);

    // Asking for "on" again while already pending is C's `return 1`
    // no-op case.
    assert_eq!(
        registry.set_clan_raid(nr, true, 2_000),
        Err(ClanRaidError::NoOp)
    );

    assert_eq!(registry.set_clan_raid(nr, false, 3_000), Ok(()));
    assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 0);

    // Asking for "off" again with nothing pending is also a no-op.
    assert_eq!(
        registry.set_clan_raid(nr, false, 4_000),
        Err(ClanRaidError::NoOp)
    );
}

#[test]
fn set_clan_raid_god_flips_raid_directly_and_clears_pending_timer() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Raiders", 0).unwrap();
    registry.set_clan_raid(nr, true, 1_000).unwrap();

    assert_eq!(registry.set_clan_raid_god(nr, true), Ok(()));
    assert!(registry.get_clan_raid(nr));
    assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 0);

    // Already on: no-op.
    assert_eq!(
        registry.set_clan_raid_god(nr, true),
        Err(ClanRaidError::NoOp)
    );

    assert_eq!(registry.set_clan_raid_god(nr, false), Ok(()));
    assert!(!registry.get_clan_raid(nr));

    assert_eq!(
        registry.set_clan_raid_god(nr, false),
        Err(ClanRaidError::NoOp)
    );
}

#[test]
fn set_clan_raid_nonexistent_clan_is_not_found() {
    let mut registry = ClanRegistry::new();
    assert_eq!(
        registry.set_clan_raid(999, true, 0),
        Err(ClanRaidError::NotFound)
    );
    assert_eq!(
        registry.set_clan_raid_god(999, true),
        Err(ClanRaidError::NotFound)
    );
}

#[test]
fn get_clan_dungeon_cost_matches_c_multiplier_table() {
    // warrior/mage/seyan +0..+5 tiers all repeat 1/2/4/8/12/16.
    assert_eq!(get_clan_dungeon_cost(1, 3), 3);
    assert_eq!(get_clan_dungeon_cost(6, 3), 48);
    assert_eq!(get_clan_dungeon_cost(7, 3), 3);
    assert_eq!(get_clan_dungeon_cost(12, 3), 48);
    assert_eq!(get_clan_dungeon_cost(13, 3), 3);
    assert_eq!(get_clan_dungeon_cost(18, 3), 48);
    assert_eq!(get_clan_dungeon_cost(19, 2), 16); // teleport traps *8
    assert_eq!(get_clan_dungeon_cost(20, 1), 16); // fake wall *16
    assert_eq!(get_clan_dungeon_cost(21, 2), 24); // locked door key *12
    assert_eq!(get_clan_dungeon_cost(22, 5), 0); // unknown type -> 0
}

#[test]
fn set_clan_dungeon_use_rejects_invalid_type_or_out_of_range_number() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Dungeoneers", 0).unwrap();
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 0, 1),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 22, 1),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    // warrior/mage/seyan slots cap at 10.
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 1, 11),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    // teleport traps cap at 25.
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 19, 26),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    // fake walls cap at 1.
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 20, 2),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    // locked doors cap at 2.
    assert_eq!(
        registry.set_clan_dungeon_use(nr, 21, 3),
        Err(ClanDungeonUseError::InvalidRequest)
    );
    assert_eq!(
        registry.set_clan_dungeon_use(999, 1, 1),
        Err(ClanDungeonUseError::InvalidRequest)
    );
}

#[test]
fn set_clan_dungeon_use_applies_within_budget_and_reads_back() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Dungeoneers", 0).unwrap();
    assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
    assert_eq!(registry.set_clan_dungeon_use(nr, 1, 5), Ok(()));
    assert_eq!(registry.get_clan_dungeon(nr, 1), 5);
    // Lowering back to 0 is always allowed.
    assert_eq!(registry.set_clan_dungeon_use(nr, 1, 0), Ok(()));
    assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
}

#[test]
fn set_clan_dungeon_use_rejects_over_budget_configuration_without_mutating() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Dungeoneers", 0).unwrap();
    // Warrior slots 1-5 (multipliers 1/2/4/8/12) maxed at 10 each:
    // running cost 10, 30, 70, 150, 270 - all within budget.
    assert_eq!(registry.set_clan_dungeon_use(nr, 1, 10), Ok(()));
    assert_eq!(registry.set_clan_dungeon_use(nr, 2, 10), Ok(()));
    assert_eq!(registry.set_clan_dungeon_use(nr, 3, 10), Ok(()));
    assert_eq!(registry.set_clan_dungeon_use(nr, 4, 10), Ok(()));
    assert_eq!(registry.set_clan_dungeon_use(nr, 5, 10), Ok(()));
    // Slot 6 (multiplier 16) at 8: 270 + 128 = 398, still <= 400.
    assert_eq!(registry.set_clan_dungeon_use(nr, 6, 8), Ok(()));
    assert_eq!(registry.get_clan_dungeon(nr, 6), 8);
    // Raising slot 6 to 9 would cost 270 + 144 = 414 > 400 - rejected
    // without mutating the stored value.
    match registry.set_clan_dungeon_use(nr, 6, 9) {
        Err(ClanDungeonUseError::OverBudget(cost)) => assert_eq!(cost, 414),
        other => panic!("expected OverBudget(414), got {other:?}"),
    }
    assert_eq!(registry.get_clan_dungeon(nr, 6), 8);
    // Lowering a different slot is always allowed even while the
    // clan sits near budget.
    assert_eq!(registry.set_clan_dungeon_use(nr, 1, 0), Ok(()));
    assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
}

#[test]
fn get_clan_dungeon_reads_zero_for_invalid_type_or_clan() {
    let registry = ClanRegistry::new();
    assert_eq!(registry.get_clan_dungeon(999, 1), 0);
    assert_eq!(registry.get_clan_dungeon(1, 0), 0);
    assert_eq!(registry.get_clan_dungeon(1, 22), 0);
}

#[test]
fn add_alc_potion_matches_attack_recipe_and_computes_tier() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Alchemists", 0).unwrap();
    let modifier_index = {
        let mut idx = [-1i16; MAX_MODIFIERS];
        idx[0] = CharacterValue::Attack as i16;
        idx[1] = CharacterValue::Parry as i16;
        idx[2] = CharacterValue::Immunity as i16;
        idx
    };
    let modifier_value = {
        let mut val = [0i16; MAX_MODIFIERS];
        val[0] = 12; // tier (12/4)-1 = 2
        val
    };
    assert!(registry.add_alc_potion(nr, modifier_index, modifier_value));
    assert_eq!(registry.identity(nr).unwrap().economy.alc_pot[0][2], 1);
}

#[test]
fn add_alc_potion_matches_flash_recipe_and_clamps_tier_at_five() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Alchemists", 0).unwrap();
    let modifier_index = {
        let mut idx = [-1i16; MAX_MODIFIERS];
        idx[0] = CharacterValue::Flash as i16;
        idx[1] = CharacterValue::MagicShield as i16;
        idx[2] = CharacterValue::Immunity as i16;
        idx
    };
    let modifier_value = {
        let mut val = [0i16; MAX_MODIFIERS];
        val[0] = 40; // (40/4)-1 = 9, clamped to 5
        val
    };
    assert!(registry.add_alc_potion(nr, modifier_index, modifier_value));
    assert_eq!(registry.identity(nr).unwrap().economy.alc_pot[1][5], 1);
}

#[test]
fn add_alc_potion_rejects_unmatched_modifiers() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Alchemists", 0).unwrap();
    assert!(!registry.add_alc_potion(nr, [-1; MAX_MODIFIERS], [0; MAX_MODIFIERS]));
    assert_eq!(
        registry.identity(nr).unwrap().economy.alc_pot,
        [[0; 6], [0; 6]]
    );
}

#[test]
fn add_alc_potion_returns_false_for_nonexistent_clan() {
    let mut registry = ClanRegistry::new();
    let modifier_index = {
        let mut idx = [-1i16; MAX_MODIFIERS];
        idx[0] = CharacterValue::Attack as i16;
        idx[1] = CharacterValue::Parry as i16;
        idx[2] = CharacterValue::Immunity as i16;
        idx
    };
    assert!(!registry.add_alc_potion(999, modifier_index, [4; MAX_MODIFIERS]));
}

#[test]
fn bump_simple_pot_increments_the_given_slot() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Alchemists", 0).unwrap();
    assert!(registry.bump_simple_pot(nr, 0, 1));
    assert!(registry.bump_simple_pot(nr, 0, 1));
    assert_eq!(registry.identity(nr).unwrap().economy.simple_pot[0][1], 2);
    assert_eq!(registry.identity(nr).unwrap().economy.simple_pot[0][0], 0);
}

#[test]
fn bump_simple_pot_returns_false_for_nonexistent_clan() {
    let mut registry = ClanRegistry::new();
    assert!(!registry.bump_simple_pot(999, 0, 0));
}

#[test]
fn money_change_log_message_matches_c_format() {
    assert_eq!(
        ClanMoneyChange::Deposited(150).log_message("Godmode"),
        "Godmode deposited 150G"
    );
    assert_eq!(
        ClanMoneyChange::Withdrew(30).log_message("Godmode"),
        "Godmode withdrew 30G"
    );
}

#[test]
fn dirty_flag_is_not_persisted_across_serde_round_trip() {
    let mut registry = ClanRegistry::new();
    registry.found_clan("Dirty", 0).unwrap();
    assert!(registry.dirty());

    let json = serde_json::to_string(&registry).unwrap();
    let reloaded: ClanRegistry = serde_json::from_str(&json).unwrap();
    assert!(
        !reloaded.dirty(),
        "a freshly deserialized registry starts clean, matching what was just saved"
    );
    assert_eq!(reloaded.name(1), Some("Dirty"));
}

#[test]
fn bonus_name_matches_c_table_and_out_of_range_guard() {
    assert_eq!(bonus_name(0), "Pentagram Quest");
    assert_eq!(bonus_name(1), "Military Advisor");
    assert_eq!(bonus_name(2), "Merchant");
    assert_eq!(bonus_name(3), "unassigned");
    assert_eq!(bonus_name(13), "unassigned");
    assert_eq!(bonus_name(-1), "Unknown");
    assert_eq!(bonus_name(14), "Unknown");
}

#[test]
fn found_clan_initializes_economy_to_standard_defaults() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Traders", 1_000).unwrap();
    let economy = registry.identity(nr).unwrap().economy;
    assert_eq!(economy.bonus_level, [0; MAX_BONUS]);
    assert_eq!(economy.depot_money, 0);
    assert_eq!(economy.treasure.jewels, 0);
    assert_eq!(economy.treasure.cost_per_week, 0);
    assert_eq!(economy.treasure.debt, 0);
    // C: `c->treasure.payed_till = realtime;` (`clan.c:92`).
    assert_eq!(economy.treasure.payed_till, 1_000);
    assert_eq!(economy.training_score, 0);
}

#[test]
fn clan_money_defaults_to_zero_for_unknown_clans() {
    let registry = ClanRegistry::new();
    assert_eq!(registry.clan_money(1), 0);
    assert_eq!(registry.clan_money(99), 0);
}

#[test]
fn clan_money_change_applies_diff_and_gates_logging_by_threshold() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Bankers", 0).unwrap();

    // Small deposit (< 100): applied, but not logged.
    assert_eq!(registry.clan_money_change(nr, 50, true), None);
    assert_eq!(registry.clan_money(nr), 50);

    // Large deposit (>= 100): applied and logged.
    assert_eq!(
        registry.clan_money_change(nr, 150, true),
        Some(ClanMoneyChange::Deposited(150))
    );
    assert_eq!(registry.clan_money(nr), 200);

    // Any withdrawal is logged, regardless of size.
    assert_eq!(
        registry.clan_money_change(nr, -30, true),
        Some(ClanMoneyChange::Withdrew(30))
    );
    assert_eq!(registry.clan_money(nr), 170);

    // `log` false suppresses the log event even for a qualifying diff.
    assert_eq!(registry.clan_money_change(nr, -30, false), None);
    assert_eq!(registry.clan_money(nr), 140);
}

#[test]
fn clan_money_change_on_unknown_clan_is_a_no_op() {
    let mut registry = ClanRegistry::new();
    assert_eq!(registry.clan_money_change(5, 500, true), None);
    assert_eq!(registry.clan_money(5), 0);
}

#[test]
fn jewel_count_and_add_jewel() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Jewelers", 0).unwrap();
    assert_eq!(registry.jewel_count(nr), 0);
    registry.add_jewel(nr).unwrap();
    registry.add_jewel(nr).unwrap();
    assert_eq!(registry.jewel_count(nr), 2);
}

#[test]
fn add_jewel_rejects_unknown_clan() {
    let mut registry = ClanRegistry::new();
    assert_eq!(registry.add_jewel(5), Err(ClanIdentityError::NotFound));
}

#[test]
fn swap_jewels_charges_debt_without_removing_source_jewels() {
    let mut registry = ClanRegistry::new();
    let a = registry.found_clan("Raided", 0).unwrap();
    let b = registry.found_clan("Raider", 0).unwrap();
    for _ in 0..5 {
        registry.add_jewel(a).unwrap();
    }

    registry.swap_jewels(a, b, 2);

    // C: `swap_jewels` only adds debt to the source, it never
    // decrements the source's jewel count directly (`clan.c:501-513`).
    assert_eq!(registry.jewel_count(a), 5);
    assert_eq!(registry.jewel_count(b), 2);
    assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 2000);
}

#[test]
fn swap_jewels_clamps_to_available_jewels_and_no_ops_when_empty() {
    let mut registry = ClanRegistry::new();
    let a = registry.found_clan("Poor", 0).unwrap();
    let b = registry.found_clan("Rich", 0).unwrap();

    // No jewels at all: no-op even though a debt-only change would be
    // "harmless" - matches C's `if (cnt_jewels(nr1) < 1) return;` early
    // exit before any state is touched.
    registry.swap_jewels(a, b, 10);
    assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 0);
    assert_eq!(registry.jewel_count(b), 0);

    registry.add_jewel(a).unwrap();
    registry.add_jewel(a).unwrap();
    // Requesting more than available (2) clamps down to 2.
    registry.swap_jewels(a, b, 10);
    assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 2000);
    assert_eq!(registry.jewel_count(b), 2);
}

#[test]
fn dungeon_jewel_steal_applies_defender_debt_and_training_and_attacker_jewels() {
    let mut registry = ClanRegistry::new();
    let cnr = registry.found_clan("Defender", 0).unwrap();
    let onr = registry.found_clan("Attacker", 0).unwrap();

    registry.dungeon_jewel_steal(cnr, onr, 3);

    // C: `clan[cnr].dungeon.training_score += 150;
    // clan[cnr].treasure.debt += cnt*1000+1000;
    // clan[onr].treasure.jewels += cnt;` (`clan.c:1360-1365`) - note
    // the extra flat `+1000` debt term that `swap_jewels` doesn't have.
    assert_eq!(registry.identity(cnr).unwrap().economy.training_score, 150);
    assert_eq!(registry.identity(cnr).unwrap().economy.treasure.debt, 4000);
    assert_eq!(registry.jewel_count(onr), 3);
    // The defender's own jewel count is untouched (jewels are never
    // physically removed from the loser, same as `swap_jewels`).
    assert_eq!(registry.jewel_count(cnr), 0);
}

#[test]
fn dungeon_jewel_steal_is_a_no_op_when_nothing_was_stolen() {
    let mut registry = ClanRegistry::new();
    let cnr = registry.found_clan("Defender", 0).unwrap();
    let onr = registry.found_clan("Attacker", 0).unwrap();

    registry.dungeon_jewel_steal(cnr, onr, 0);
    registry.dungeon_jewel_steal(cnr, onr, -1);

    assert_eq!(registry.identity(cnr).unwrap().economy.training_score, 0);
    assert_eq!(registry.identity(cnr).unwrap().economy.treasure.debt, 0);
    assert_eq!(registry.jewel_count(onr), 0);
}

#[test]
fn dungeon_jewel_steal_ignores_unknown_clan_numbers() {
    let mut registry = ClanRegistry::new();
    let cnr = registry.found_clan("Defender", 0).unwrap();
    // Attacker clan number 9 doesn't exist: the defender side still
    // mutates (matches C's per-field writes with no atomic rollback),
    // the nonexistent attacker side is simply dropped.
    registry.dungeon_jewel_steal(cnr, 9, 2);
    assert_eq!(registry.identity(cnr).unwrap().economy.training_score, 150);
    assert_eq!(registry.jewel_count(9), 0);
}

#[test]
fn bonus_level_get_and_set() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Bonused", 0).unwrap();
    assert_eq!(registry.bonus_level(nr, 2), 0);

    registry.set_bonus_level(nr, 2, 3).unwrap();
    assert_eq!(registry.bonus_level(nr, 2), 3);

    assert_eq!(
        registry.set_bonus_level(nr, MAX_BONUS, 1),
        Err(ClanIdentityError::NotFound)
    );
    assert_eq!(registry.bonus_level(nr, MAX_BONUS), 0);
    assert_eq!(
        registry.set_bonus_level(99, 0, 1),
        Err(ClanIdentityError::NotFound)
    );
}

#[test]
fn update_treasure_charges_flat_clan_hall_rent_with_no_bonuses() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Rentpayers", 0).unwrap();
    registry.update_treasure(0);
    assert_eq!(
        registry
            .identity(nr)
            .unwrap()
            .economy
            .treasure
            .cost_per_week,
        CLAN_HALL_RENT * 1000
    );
}

#[test]
fn update_treasure_reduces_unaffordable_bonus_to_zero() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Overspent", 0).unwrap();
    registry.set_bonus_level(nr, 0, 1).unwrap(); // no jewels to support it
    registry.update_treasure(0);
    assert_eq!(registry.bonus_level(nr, 0), 0);
}

#[test]
fn update_treasure_keeps_affordable_bonus() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Sponsored", 0).unwrap();
    registry.set_bonus_level(nr, 0, 1).unwrap(); // costs 1000/250=4 <= 5 jewels
    for _ in 0..5 {
        registry.add_jewel(nr).unwrap();
    }
    registry.update_treasure(0);
    assert_eq!(registry.bonus_level(nr, 0), 1);
    assert_eq!(
        registry
            .identity(nr)
            .unwrap()
            .economy
            .treasure
            .cost_per_week,
        1000 + CLAN_HALL_RENT * 1000
    );
}

#[test]
fn update_treasure_skips_debt_accrual_before_five_minutes_late() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("OnTime", 0).unwrap();
    registry.update_treasure(300); // exactly 300s: C requires `diff > 300`
    assert_eq!(registry.identity(nr).unwrap().economy.treasure.debt, 0);
}

#[test]
fn update_treasure_accrues_small_debt_once_five_minutes_late() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Late", 0).unwrap();
    // cost = 5000 (rent only) => step = 604800/5000 = 120.
    // diff = 301 => n = 301/120 + 1 = 3.
    registry.update_treasure(301);
    let treasure = registry.identity(nr).unwrap().economy.treasure;
    assert_eq!(treasure.debt, 3);
    assert_eq!(treasure.payed_till, 120 * 3);
}

#[test]
fn update_treasure_pays_off_debt_with_jewels_when_affordable() {
    let mut registry = ClanRegistry::new();
    let a = registry.found_clan("Payer", 0).unwrap();
    let b = registry.found_clan("Other", 0).unwrap();
    for _ in 0..5 {
        registry.add_jewel(a).unwrap();
    }
    registry.swap_jewels(a, b, 2); // a now owes 2000 debt, keeps 5 jewels

    let events = registry.update_treasure(0);
    assert_eq!(
        events,
        vec![ClanTreasuryEvent::PaidDebtWithJewels {
            clan: a,
            jewels_paid: 2,
        }]
    );
    assert_eq!(registry.jewel_count(a), 3);
    assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 0);
    assert!(registry.exists(a), "debt fully paid off, clan survives");
}

#[test]
fn update_treasure_deletes_clan_that_goes_broke_with_no_jewels() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Broke", 0).unwrap();
    let serial_before = registry.serial(nr);

    // cost = 5000, step = 120; diff = 250_000 => n = 250000/120 + 1 = 2084,
    // which lands debt at 2084 (>= 2000) with zero jewels to pay it off.
    let events = registry.update_treasure(250_000);
    assert_eq!(
        events,
        vec![ClanTreasuryEvent::WentBroke {
            clan: nr,
            serial: serial_before,
            name: "Broke".to_string(),
        }]
    );
    assert!(!registry.exists(nr));
    assert!(registry.serial(nr) > serial_before);
}

#[test]
fn update_treasure_pays_partial_debt_then_still_goes_broke() {
    let mut registry = ClanRegistry::new();
    let a = registry.found_clan("AlmostBroke", 0).unwrap();
    let b = registry.found_clan("Other", 0).unwrap();
    let serial_a_before = registry.serial(a);
    registry.add_jewel(a).unwrap(); // only 1 jewel available

    // Four raids, each clamped to the single available jewel, push
    // debt to 4000 while the jewel count itself never drops (`swap_
    // jewels` only ever adds debt to the source, `clan.c:501-513`).
    for _ in 0..4 {
        registry.swap_jewels(a, b, 3);
    }
    assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 4000);

    let events = registry.update_treasure(0);
    // n = debt/1000 = 4, clamped down to the 1 available jewel:
    // jewels -> 0, debt -= 1*1000 = 3000, which is still >= 2000.
    assert_eq!(
        events,
        vec![
            ClanTreasuryEvent::PaidDebtWithJewels {
                clan: a,
                jewels_paid: 1,
            },
            ClanTreasuryEvent::WentBroke {
                clan: a,
                serial: serial_a_before,
                name: "AlmostBroke".to_string(),
            },
        ]
    );
    assert!(!registry.exists(a));
}

#[test]
fn update_training_decays_score_by_five_percent_after_one_hour() {
    let mut registry = ClanRegistry::new();
    let nr = registry.found_clan("Trainers", 0).unwrap();
    registry.identity_mut(nr).unwrap().economy.training_score = 1000;

    registry.update_training(3599);
    assert_eq!(registry.identity(nr).unwrap().economy.training_score, 1000);

    registry.update_training(3600);
    assert_eq!(registry.identity(nr).unwrap().economy.training_score, 950);
    assert_eq!(
        registry.identity(nr).unwrap().economy.last_training_update,
        3600
    );
}
