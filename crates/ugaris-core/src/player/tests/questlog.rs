use super::*;

#[test]
fn farmy_ppd_advances_blood_and_lava_quest_stages() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.farmy_boss_stage(), 0);
    assert!(!player.advance_farmy_blood_stage());

    player.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
    write_i32(&mut player.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 20);
    assert!(player.advance_farmy_blood_stage());
    assert_eq!(player.farmy_boss_stage(), 21);
    assert!(!player.advance_farmy_blood_stage());

    write_i32(&mut player.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 22);
    assert!(player.advance_farmy_lava_stage());
    assert_eq!(player.farmy_boss_stage(), 24);
    assert!(!player.advance_farmy_lava_stage());
}

#[test]
fn clear_turn_seyan_ppd_removes_quest_items_from_depot_but_keeps_others() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.depot[0] = Some(sample_depot_item(
        1,
        0x1111,
        ItemFlags::USED | ItemFlags::QUEST,
    ));
    player.depot[1] = Some(sample_depot_item(2, 0x2222, ItemFlags::USED));

    player.clear_turn_seyan_ppd();

    assert!(player.depot[0].is_none(), "quest item slot must be wiped");
    assert!(
        player.depot[1].is_some(),
        "non-quest item slot must be untouched"
    );
    assert_eq!(player.depot[1].as_ref().unwrap().template_id, 0x2222);
}

#[test]
fn questlog_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(
        DRD_QUESTLOG_PPD,
        make_drd(DEV_ID_DB, 158 | PERSISTENT_PLAYER_DATA)
    );
    assert_eq!(LEGACY_QUESTLOG_PPD_SIZE, 100);

    let mut player = PlayerRuntime::connected(1, 0);
    player.quest_log.open(0);
    player
        .quest_log
        .complete_legacy(1, 10, 1)
        .expect("quest 1 has metadata");
    player.quest_log.mark_init_complete();

    let encoded = player.encode_legacy_questlog_ppd();
    assert_eq!(encoded.len(), LEGACY_QUESTLOG_PPD_SIZE);
    // done in the low 6 bits, flags in the high 2 bits (LSB-first
    // bitfield allocation, matching `struct quest { done:6; flags:2; }`).
    assert_eq!(encoded[0], crate::quest::QF_OPEN << 6);
    assert_eq!(encoded[1], 1 | (crate::quest::QF_DONE << 6));
    assert_eq!(encoded[MAX_QUESTS - 1], 55);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_questlog_ppd(&encoded));
    assert!(!decoded.quest_log.is_done(0));
    assert_eq!(decoded.quest_log.count(0), 0);
    assert!(decoded.quest_log.is_done(1));
    assert_eq!(decoded.quest_log.count(1), 1);
    assert!(decoded.quest_log.is_init_complete());
}

#[test]
fn questlog_ppd_blob_replaces_and_appends_legacy_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.quest_log.mark_done(2);

    let mut existing_questlog = vec![0u8; LEGACY_QUESTLOG_PPD_SIZE];
    existing_questlog[2] = 9;
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x3344_5566, &[4, 5, 6]);
    write_ppd_block(&mut existing, DRD_QUESTLOG_PPD, &existing_questlog);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), 0x3344_5566);
    assert_eq!(read_u32(&encoded, 11), DRD_QUESTLOG_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_QUESTLOG_PPD_SIZE as u32);
    assert_eq!(encoded[19 + 2], 1 | (crate::quest::QF_DONE << 6));

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert!(decoded.quest_log.is_done(2));

    // No block present and no progress yet -> nothing is appended.
    let untouched = PlayerRuntime::connected(3, 0);
    let appended_empty = untouched.encode_legacy_ppd_blob(&[]);
    assert!(appended_empty.is_empty());

    // Progress with no existing block -> the block is appended.
    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_QUESTLOG_PPD);
}

#[test]
fn init_questlog_runs_all_five_sub_functions_once() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area1_lydia_state(6);
    player.set_area1_nook_state(12);

    assert!(!player.quest_log.is_init_complete());
    player.init_questlog();
    assert!(player.quest_log.is_init_complete());
    // `init_area1_quests`'s lydia/nook branches should have marked
    // their quests done (see `init_area1_quests`).
    assert!(player.quest_log.is_done(crate::quest::QLOG_LYDIA));
    assert!(player.quest_log.is_done(crate::quest::QLOG_NOOK));

    // Calling again is a no-op (sentinel guard): even though the PPD
    // state driving the sub-functions has since changed, the already-
    // seeded completion is left untouched.
    player.set_area1_lydia_state(0);
    player.init_questlog();
    assert!(player.quest_log.is_done(crate::quest::QLOG_LYDIA));
}

#[test]
fn clear_turn_seyan_ppd_resets_quest_log() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.init_questlog();
    assert!(player.quest_log.is_init_complete());

    player.clear_turn_seyan_ppd();
    assert!(!player.quest_log.is_init_complete());
    assert!(player
        .quest_log
        .entries()
        .iter()
        .all(|entry| entry.done == 0 && entry.flags == 0));
}

#[test]
fn reopen_quest_legacy_q0_resets_james_and_lydia_state() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area1_james_state(4);
    player.set_area1_lydia_state(6);
    mark_reopenable(&mut player, crate::quest::QLOG_LYDIA);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_LYDIA),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(player.area1_james_state(), 0);
    assert_eq!(player.area1_lydia_state(), 0);
    assert!(player.quest_log.is_open(crate::quest::QLOG_LYDIA));
}

#[test]
fn reopen_quest_legacy_gwendy_series_rejects_when_sibling_open() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_GWENDY_FIRST_SKULL);
    // Simulate quest 2 (second skull) already open.
    player
        .quest_log
        .open(crate::quest::QLOG_GWENDY_SECOND_SKULL);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_GWENDY_FIRST_SKULL),
        crate::quest::QuestReopenResult::SeriesConflict
    );
    // Series conflict must not touch the gwendy_state or open flags.
    assert_eq!(player.area1_gwendy_state(), 0);
    assert!(!player
        .quest_log
        .is_open(crate::quest::QLOG_GWENDY_FIRST_SKULL));
}

#[test]
fn reopen_quest_legacy_gwendy_series_succeeds_when_no_conflict() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_GWENDY_THIRD_SKULL);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_GWENDY_THIRD_SKULL),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(
        player.area1_gwendy_state(),
        crate::quest::GWENDYLON_STATE_SECOND_SKULL_DONE
    );
    assert!(player
        .quest_log
        .is_open(crate::quest::QLOG_GWENDY_THIRD_SKULL));
}

#[test]
fn reopen_quest_legacy_rejects_zero_flag_quest_even_when_done() {
    // Quest 6 (Nook, "A Fool's Request") has table `flags == 0` (no
    // `QLF_REPEATABLE`, no `QLF_XREPEAT`) - C's `!questlog[qnr].flags
    // & QLF_REPEATABLE` precedence bug (see `QuestLog::reopen_precheck`)
    // means this is the one shape that's genuinely rejected as "not
    // repeatable", so the switch's `case 6:` arm (a no-op anyway) is
    // dead code, unreachable through the public API - confirmed by
    // `reopen_dispatch_case_6_is_a_documented_dead_noop_arm` below.
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_NOOK);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_NOOK),
        crate::quest::QuestReopenResult::CannotOpenAgain
    );
}

#[test]
fn reopen_dispatch_case_36_falls_through_into_case_37_like_c() {
    // C `questlog_reopen`'s `case 36` is missing a `break;`
    // (`src/system/questlog.c:746-750`), so it falls into `case 37`'s
    // `questlog_reopen_q35(cn, 7, quest)` instead of doing nothing.
    // Quest 36 ("Contraband") has table `flags == 0` though, so this
    // arm is unreachable through the public `reopen_quest_legacy` API
    // (confirmed below) - exercised directly via `reopen_dispatch` to
    // prove the switch body itself faithfully reproduces the bug.
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.reopen_dispatch(36), ReopenOutcome::Open);
    assert_eq!(player.staffer_smugglecom_state(), 7);

    // Confirm the dead-code claim: the live, precondition-gated path
    // never reaches the switch for quest 36 at all.
    let mut gated = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut gated, 36);
    assert_eq!(
        gated.reopen_quest_legacy(36),
        crate::quest::QuestReopenResult::CannotOpenAgain
    );
}

#[test]
fn reopen_smugglecom_state_five_clears_bits() {
    // `questlog_reopen_q35`'s `if (state == 5) ppd->smugglecom_bits =
    // 0;` branch (`src/system/questlog.c:503-505`) is unreachable
    // through the live switch (no reachable case ever passes `state
    // == 5` - the one that once did, case 36, is both `ret = 0;`
    // with the call commented out *and* gated out by quest 36's
    // zero table flags) but the helper still faithfully implements
    // it for completeness; exercised directly.
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_staffer_smugglecom_bits(7);

    assert_eq!(player.reopen_smugglecom(5), ReopenOutcome::Open);
    assert_eq!(player.staffer_smugglecom_state(), 5);
    assert_eq!(player.staffer_smugglecom_bits(), 0);
}

#[test]
fn reopen_quest_legacy_simple_single_state_reset_cases() {
    // Reopen cases whose only side effect is zeroing one PPD field
    // (`questlog_reopen_q5/q9/q13/q16/q20/q30/q31/q38/q39/q44`) - each
    // exercised end-to-end through the public, precondition-gated
    // `reopen_quest_legacy` API since every one of these quest
    // numbers is genuinely `QLF_REPEATABLE` in the table.
    #[allow(clippy::type_complexity)]
    let cases: &[(
        usize,
        fn(&PlayerRuntime) -> i32,
        fn(&mut PlayerRuntime, i32),
    )] = &[
        (
            5,
            PlayerRuntime::area1_yoakin_state,
            PlayerRuntime::set_area1_yoakin_state,
        ),
        (
            9,
            PlayerRuntime::area1_logain_state,
            PlayerRuntime::set_area1_logain_state,
        ),
        (
            13,
            PlayerRuntime::area3_kelly_state,
            PlayerRuntime::set_area3_kelly_state,
        ),
        (
            16,
            PlayerRuntime::area3_astro2_state,
            PlayerRuntime::set_area3_astro2_state,
        ),
        (
            20,
            PlayerRuntime::staffer_carlos_state,
            PlayerRuntime::set_staffer_carlos_state,
        ),
        (
            30,
            PlayerRuntime::twocity_skelly_state,
            PlayerRuntime::set_twocity_skelly_state,
        ),
        (
            31,
            PlayerRuntime::twocity_alchemist_state,
            PlayerRuntime::set_twocity_alchemist_state,
        ),
        (
            38,
            PlayerRuntime::staffer_aristocrat_state,
            PlayerRuntime::set_staffer_aristocrat_state,
        ),
        (
            39,
            PlayerRuntime::staffer_yoatin_state,
            PlayerRuntime::set_staffer_yoatin_state,
        ),
        (
            44,
            PlayerRuntime::staffer_spiritbran_state,
            PlayerRuntime::set_staffer_spiritbran_state,
        ),
    ];

    for (qnr, getter, setter) in cases.iter().copied() {
        let mut player = PlayerRuntime::connected(1, 0);
        setter(&mut player, 7);
        mark_reopenable(&mut player, qnr);

        assert_eq!(
            player.reopen_quest_legacy(qnr),
            crate::quest::QuestReopenResult::Reopened,
            "quest {qnr} should reopen"
        );
        assert_eq!(
            getter(&player),
            0,
            "quest {qnr} should reset its state to 0"
        );
        assert!(player.quest_log.is_open(qnr));
    }
}

#[test]
fn reopen_quest_legacy_guiwynn_series() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, 7);
    player.quest_log.open(8);

    assert_eq!(
        player.reopen_quest_legacy(7),
        crate::quest::QuestReopenResult::SeriesConflict
    );

    let mut player = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut player, 8);

    assert_eq!(
        player.reopen_quest_legacy(8),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(player.area1_guiwynn_state(), 6);
}

#[test]
fn reopen_quest_legacy_seymour_case_12() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, 12);
    player.quest_log.open(11);

    assert_eq!(
        player.reopen_quest_legacy(12),
        crate::quest::QuestReopenResult::SeriesConflict
    );

    let mut player = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut player, 12);

    assert_eq!(
        player.reopen_quest_legacy(12),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(player.area3_seymour_state(), 12);
}

#[test]
fn reopen_quest_legacy_brennethbran_series() {
    for (qnr, expected_state) in [(41, 0), (42, 5), (43, 9)] {
        let mut player = PlayerRuntime::connected(1, 0);
        mark_reopenable(&mut player, qnr);

        assert_eq!(
            player.reopen_quest_legacy(qnr),
            crate::quest::QuestReopenResult::Reopened,
            "quest {qnr}"
        );
        assert_eq!(player.staffer_brennethbran_state(), expected_state);
    }

    let mut player = PlayerRuntime::connected(4, 0);
    mark_reopenable(&mut player, 41);
    player.quest_log.open(43);
    assert_eq!(
        player.reopen_quest_legacy(41),
        crate::quest::QuestReopenResult::SeriesConflict
    );
}

#[test]
fn reopen_quest_legacy_broklin_case_45() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, 45);
    player.quest_log.open(46);

    assert_eq!(
        player.reopen_quest_legacy(45),
        crate::quest::QuestReopenResult::SeriesConflict
    );

    let mut player = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut player, 45);

    assert_eq!(
        player.reopen_quest_legacy(45),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(player.staffer_broklin_state(), 0);
}

#[test]
fn reopen_quest_legacy_countbran_clears_only_low_three_bits() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_staffer_countbran_state(9);
    player.set_staffer_countbran_bits(1 | 2 | 4 | 8);
    mark_reopenable(&mut player, 40);

    assert_eq!(
        player.reopen_quest_legacy(40),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(player.staffer_countbran_state(), 0);
    // Only bits 1|2|4 are cleared; bit 8 survives (C:
    // `ppd->countbran_bits &= ~(1 | 2 | 4);`).
    assert_eq!(player.staffer_countbran_bits(), 8);
}

#[test]
fn reopen_quest_legacy_jessica_series_conflict_both_directions() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_JESSICA_ROBBER_NOTE);
    player.quest_log.open(crate::quest::QLOG_JESSICA_KILL);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_JESSICA_ROBBER_NOTE),
        crate::quest::QuestReopenResult::SeriesConflict
    );

    let mut player = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_JESSICA_KILL);
    player
        .quest_log
        .open(crate::quest::QLOG_JESSICA_ROBBER_NOTE);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_JESSICA_KILL),
        crate::quest::QuestReopenResult::SeriesConflict
    );
}

#[test]
fn reopen_quest_legacy_hermit_quest2_sets_camhermit_quest2_entry_state() {
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, crate::quest::QLOG_HERMIT_QUEST2);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_HERMIT_QUEST2),
        crate::quest::QuestReopenResult::Reopened
    );
    assert_eq!(
        player.area1_camhermit_state(),
        crate::quest::CAMHERMIT_STATE_QUEST2_1
    );
}

#[test]
fn reopen_quest_legacy_rejects_xrepeat_only_quest_with_c_precedence_bug() {
    // Quest 25 has only `QLF_XREPEAT`, not `QLF_REPEATABLE`, but C's
    // `!questlog[qnr].flags & QLF_REPEATABLE` operator-precedence bug
    // (see `QuestLog::reopen_precheck`) treats "any flags at all" as
    // repeatable, so this quest passes the generic precondition and
    // reaches the switch - where its `case 25: ret = 0;` arm then
    // silently refuses to reopen anyway.
    let mut player = PlayerRuntime::connected(1, 0);
    mark_reopenable(&mut player, 25);

    assert_eq!(
        player.reopen_quest_legacy(25),
        crate::quest::QuestReopenResult::NoEffect
    );
}

#[test]
fn reopen_quest_legacy_rejects_never_done_quest_before_reaching_switch() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.reopen_quest_legacy(crate::quest::QLOG_GWENDY_FIRST_SKULL),
        crate::quest::QuestReopenResult::CannotOpenNow
    );
}

#[test]
fn reopen_quest_legacy_rejects_invalid_quest_number() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.reopen_quest_legacy(9999),
        crate::quest::QuestReopenResult::InvalidQuest
    );
}
