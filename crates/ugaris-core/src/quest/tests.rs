use super::*;

#[test]
fn quest_constants_match_c_header() {
    assert_eq!(MAX_QUESTS, 100);
    assert_eq!(QF_OPEN, 1);
    assert_eq!(QF_DONE, 2);
    assert_eq!(QLF_REPEATABLE, 1);
    assert_eq!(QLOG_JESSICA_KILL, 84);
}

#[test]
fn quest_exp_constants_match_c_header() {
    use quest_exp::*;

    // `src/common/quest_exp.h`, copied digit for digit.
    assert_eq!(EXP_AREA1_SKULL1, 75);
    assert_eq!(EXP_AREA1_SKULL2, 150);
    assert_eq!(EXP_AREA1_SKULL3, 300);
    assert_eq!(EXP_AREA1_JESTER, 400);
    assert_eq!(EXP_AREA1_SKULL4, 800);
    assert_eq!(EXP_AREA1_BEARTOOTH, 600);
    assert_eq!(EXP_AREA1_MADMAGE1, 800);
    assert_eq!(EXP_AREA1_MADMAGE2, 900);
    assert_eq!(EXP_AREA1_MADKNIGHT, 1200);
    assert_eq!(EXP_AREA1_GUILD, 1250);
    assert_eq!(EXP_AREA3_SKULL1, 850);
    assert_eq!(EXP_AREA3_SKULL2, 1000);
    assert_eq!(EXP_AREA3_SKULL3, 1250);
    assert_eq!(EXP_AREA3_LOISAN, 1500);
    assert_eq!(EXP_AREA3_CREEPER, 1850);
    assert_eq!(EXP_AREA3_SHRINE, 1500);
    assert_eq!(EXP_AREA3_MOONIES, 5000);
    assert_eq!(EXP_AREA2_VAMPIRE1, 5000);
    assert_eq!(EXP_AREA2_VAMPIRE2, 12000);
    assert_eq!(EXP_AREA3_REACHCLARA, 2500);
    assert_eq!(EXP_AREA15_HARDKILL, 7500);
    assert_eq!(EXP_AREA15_DIDKILL, 22500);
    assert_eq!(EXP_AREA16_BEARKILL, 12500);
    assert_eq!(EXP_AREA16_MANTIS, 15000);
    assert_eq!(EXP_AREA16_SPIDERKILL, 25000);

    assert_eq!(MONEY_AREA1_SKULL1, 125);
    assert_eq!(MONEY_AREA1_SKULL2, 250);
    assert_eq!(MONEY_AREA1_SKULL3, 400);
    assert_eq!(MONEY_AREA1_SKULL4, 600);
    assert_eq!(MONEY_AREA1_BEARTOOTH, 500);
    assert_eq!(MONEY_AREA1_MADMAGE1, 250);
    assert_eq!(MONEY_AREA1_MADMAGE2, 500);
    assert_eq!(MONEY_AREA1_MADKNIGHT, 550);
    assert_eq!(MONEY_AREA3_MOONIES, 2500);
    assert_eq!(MONEY_AREA3_VAMPIRE1, 2500);
}

#[test]
fn quest_done_count_is_six_bit_like_c_bitfield() {
    let mut log = QuestLog::default();
    for _ in 0..70 {
        log.mark_done(QLOG_LYDIA);
    }
    assert_eq!(log.count(QLOG_LYDIA), 0x3f);
    assert!(log.is_done(QLOG_LYDIA));
}

#[test]
fn entries_expose_fixed_legacy_quest_count() {
    let log = QuestLog::default();

    assert_eq!(log.entries().len(), MAX_QUESTS);
}

#[test]
fn reopen_legacy_allows_done_repeatable_quests() {
    let mut log = QuestLog::default();
    log.mark_done(QLOG_LYDIA);

    assert_eq!(
        log.try_reopen_legacy(QLOG_LYDIA),
        QuestReopenResult::Reopened
    );
    let entry = log.entries()[QLOG_LYDIA];
    assert_eq!(entry.done, 1);
    assert_eq!(entry.flags, QF_OPEN);
}

#[test]
fn reopen_legacy_rejects_non_repeatable_and_not_done_quests() {
    let mut log = QuestLog::default();
    assert_eq!(
        log.try_reopen_legacy(QLOG_NOOK),
        QuestReopenResult::CannotOpenAgain
    );
    assert_eq!(
        log.try_reopen_legacy(QLOG_GWENDY_FIRST_SKULL),
        QuestReopenResult::CannotOpenNow
    );
}

#[test]
fn reopen_legacy_rejects_after_ten_completions_like_c() {
    let mut log = QuestLog::default();
    for _ in 0..10 {
        log.mark_done(QLOG_LYDIA);
    }

    assert_eq!(
        log.try_reopen_legacy(QLOG_LYDIA),
        QuestReopenResult::CannotOpenAgain
    );
}

/// C `level_value(level)` (`src/system/tool.c:1282`), duplicated here
/// only for test expectations (this leaf module doesn't depend on
/// `world::exp` - see `taper_exp_by_level`'s doc comment).
fn level_value(level: u32) -> u32 {
    let next = level + 1;
    next.pow(4) - level.pow(4)
}

#[test]
fn quest_table_has_85_entries_matching_c_array() {
    assert_eq!(QUEST_TABLE.len(), 85);
    assert_eq!(quest_meta(85), None);
    assert_eq!(quest_meta(MAX_QUESTS - 1), None);
}

#[test]
fn quest_table_entries_match_c_source_digit_for_digit() {
    let lydia = quest_meta(QLOG_LYDIA).unwrap();
    assert_eq!(lydia.name, "Lydia's Potion");
    assert_eq!(lydia.min_level, 1);
    assert_eq!(lydia.max_level, 2);
    assert_eq!(lydia.giver, "James");
    assert_eq!(lydia.area, "Cameron");
    assert_eq!(lydia.exp, 15);
    assert_eq!(lydia.flags, QLF_REPEATABLE);

    // Trailing-space quest names copied verbatim from the C table.
    assert_eq!(quest_meta(40).unwrap().name, "The Jewels of Brannington ");
    assert_eq!(quest_meta(42).unwrap().name, "A Thief's Loot ");

    // QLF_XREPEAT-only entries (not QLF_REPEATABLE).
    for qnr in [25, 26, 27, 28] {
        let meta = quest_meta(qnr).unwrap();
        assert_eq!(meta.flags, QLF_XREPEAT);
        assert_eq!(meta.flags & QLF_REPEATABLE, 0);
    }

    // Highest-value quest in the table.
    let sarkilar = quest_meta(33).unwrap();
    assert_eq!(sarkilar.name, "Searching Sarkilar");
    assert_eq!(sarkilar.exp, 450000);

    assert_eq!(
        quest_meta(QLOG_JESSICA_KILL).unwrap().name,
        "Defeating the Robber Leader"
    );
}

#[test]
fn quest_table_flags_stay_in_sync_with_reopen_repeatability_table() {
    // Every quest previously hand-marked repeatable in QUESTLOG_FLAGS
    // must have QLF_REPEATABLE set in the ported metadata table too.
    let repeatable_indices = [
        0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 20, 30, 31, 35, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        79, 83, 84,
    ];
    for (qnr, quest) in QUEST_TABLE.iter().enumerate() {
        let expects_repeatable = repeatable_indices.contains(&qnr);
        let is_repeatable = (quest.flags & QLF_REPEATABLE) != 0;
        assert_eq!(
            is_repeatable, expects_repeatable,
            "quest {qnr} repeatability mismatch"
        );
    }
}

#[test]
fn scale_exp_matches_c_questlog_scale_curve() {
    assert_eq!(scale_exp(0, 1000), 1000);
    assert_eq!(scale_exp(1, 1000), 820);
    assert_eq!(scale_exp(2, 1000), 680);
    assert_eq!(scale_exp(3, 1000), 560);
    assert_eq!(scale_exp(4, 1000), 460);
    assert_eq!(scale_exp(5, 1000), 380);
    assert_eq!(scale_exp(6, 1000), 320);
    assert_eq!(scale_exp(7, 1000), 260);
    assert_eq!(scale_exp(8, 1000), 210);
    assert_eq!(scale_exp(9, 1000), 180);
    assert_eq!(scale_exp(10, 1000), 150);
    assert_eq!(scale_exp(200, 1000), 150);
}

#[test]
fn taper_exp_by_level_matches_c_bands() {
    // level <= 4: min(level_value(level), val)
    assert_eq!(
        taper_exp_by_level(1, level_value(1), 1_000_000),
        level_value(1) as i64
    );
    assert_eq!(taper_exp_by_level(1, level_value(1), 1), 1);

    // 4 < level <= 19: min(level_value(level)/2, val)
    assert_eq!(
        taper_exp_by_level(10, level_value(10), 1_000_000_000),
        (level_value(10) / 2) as i64
    );

    // 19 < level <= 44: min(level_value(level)/4, val)
    assert_eq!(
        taper_exp_by_level(30, level_value(30), 1_000_000_000),
        (level_value(30) / 4) as i64
    );

    // level > 44: min(level_value(level)/6, val)
    assert_eq!(
        taper_exp_by_level(50, level_value(50), 1_000_000_000),
        (level_value(50) / 6) as i64
    );
}

#[test]
fn complete_legacy_ports_questlog_done_first_completion() {
    let mut log = QuestLog::default();
    log.open(QLOG_LYDIA);

    let result = log
        .complete_legacy(QLOG_LYDIA, 1, level_value(1))
        .expect("Lydia's Potion has metadata");

    assert_eq!(result.times_done, 1);
    assert_eq!(result.nominal_exp, 15);
    // scale_exp(0, 15) = 15, tapered by min(level_value(1), 15) = 15
    // (level_value(1) is far bigger than 15 for level 1).
    assert_eq!(result.granted_exp, 15);

    let entry = log.entries()[QLOG_LYDIA];
    assert_eq!(entry.done, 1);
    assert_eq!(entry.flags, QF_DONE);
}

#[test]
fn complete_legacy_scales_repeat_completions_and_increments_done() {
    let mut log = QuestLog::default();
    // Complete Lydia's Potion (exp 15, repeatable) three times.
    for expected_prior in 0..3u8 {
        let result = log.complete_legacy(QLOG_LYDIA, 1, level_value(1)).unwrap();
        assert_eq!(result.times_done, expected_prior + 1);
    }
    assert_eq!(log.count(QLOG_LYDIA), 3);

    // Now complete a high-level, high-exp quest at a high level to
    // exercise the taper.
    let mut log2 = QuestLog::default();
    let result = log2
        .complete_legacy(20, 50, level_value(50))
        .expect("Wanted: Occult Staff has metadata");
    assert_eq!(result.nominal_exp, 40000);
    // level 50 > 44, so granted = min(level_value(50)/6, 40000)
    let expected = (level_value(50) as i64 / 6).min(40000);
    assert_eq!(result.granted_exp, expected);
}

#[test]
fn complete_legacy_returns_none_for_indices_without_metadata() {
    let mut log = QuestLog::default();
    assert_eq!(log.complete_legacy(85, 1, level_value(1)), None);
    assert_eq!(log.complete_legacy(MAX_QUESTS, 1, level_value(1)), None);
}

#[test]
fn open_matches_c_unconditional_assignment() {
    let mut log = QuestLog::default();
    log.mark_done(QLOG_LYDIA);
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);

    // C `questlog_open` assigns flags = QF_OPEN outright, clearing
    // QF_DONE, without touching `done`.
    log.open(QLOG_LYDIA);
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_LYDIA].done, 1);
}

#[test]
fn close_only_transitions_from_exactly_open_like_c() {
    let mut log = QuestLog::default();

    // Closed (flags = 0): no-op.
    log.close(QLOG_LYDIA);
    assert_eq!(log.entries()[QLOG_LYDIA].flags, 0);

    // Open -> Done.
    log.open(QLOG_LYDIA);
    log.close(QLOG_LYDIA);
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);

    // Already done: closing again is a no-op (flags stay QF_DONE, not
    // reset to 0 or anything else).
    log.close(QLOG_LYDIA);
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);
}

#[test]
fn init_area1_quests_lydia_branches_match_c() {
    let mut log = QuestLog::default();

    // done > 0, no flag transition into open until >=6.
    init_area1_quests(&mut log, &Area1QuestState::default());
    assert_eq!(log.entries()[QLOG_LYDIA].flags, 0);

    init_area1_quests(
        &mut log,
        &Area1QuestState {
            lydia_state: 3,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_OPEN);

    init_area1_quests(
        &mut log,
        &Area1QuestState {
            lydia_state: 6,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_LYDIA].done, 1);

    // Calling again with the same state must not bump `done` past 1
    // (C only seeds `done = 1` when it was previously 0).
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            lydia_state: 6,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_LYDIA].done, 1);
}

#[test]
fn init_area1_quests_gwendy_series_matches_c_ladder() {
    let mut log = QuestLog::default();

    // Entry: all four closed.
    init_area1_quests(&mut log, &Area1QuestState::default());
    for quest in [
        QLOG_GWENDY_FIRST_SKULL,
        QLOG_GWENDY_SECOND_SKULL,
        QLOG_GWENDY_THIRD_SKULL,
        QLOG_GWENDY_FOUL_MAGICIAN,
    ] {
        assert_eq!(log.entries()[quest].flags, 0);
    }

    // In progress on first skull.
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            gwendy_state: 3,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, 0);

    // First skull done (>=6), second open.
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            gwendy_state: GWENDYLON_STATE_FIRST_SKULL_DONE,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, 0);

    // Second skull done (>=10), third open.
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            gwendy_state: GWENDYLON_STATE_SECOND_SKULL_DONE,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_GWENDY_FOUL_MAGICIAN].flags, 0);

    // Third skull done (>=14), foul magician open.
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            gwendy_state: GWENDYLON_STATE_THIRD_SKULL_DONE,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_GWENDY_FOUL_MAGICIAN].flags, QF_OPEN);

    // Foul magician done (>=18): whole series done, `done` seeded to 1.
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            gwendy_state: GWENDYLON_STATE_FOUL_MAGICIAN_DONE,
            ..Default::default()
        },
    );
    for quest in [
        QLOG_GWENDY_FIRST_SKULL,
        QLOG_GWENDY_SECOND_SKULL,
        QLOG_GWENDY_THIRD_SKULL,
        QLOG_GWENDY_FOUL_MAGICIAN,
    ] {
        assert_eq!(log.entries()[quest].flags, QF_DONE);
        assert_eq!(log.entries()[quest].done, 1);
    }
}

#[test]
fn init_area1_quests_yoakin_nook_guiwynn_logain_reskin_match_c() {
    let mut log = QuestLog::default();
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            yoakin_state: 5,
            nook_state: 12,
            guiwynn_state: 9,
            logain_state: 6,
            reskin_state: 8,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[5].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_NOOK].flags, QF_DONE);
    assert_eq!(log.entries()[7].flags, QF_DONE);
    assert_eq!(log.entries()[8].flags, QF_DONE);
    assert_eq!(log.entries()[9].flags, QF_DONE);
    assert_eq!(log.entries()[17].flags, QF_DONE);

    let mut log = QuestLog::default();
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            yoakin_state: 2,
            nook_state: 4,
            guiwynn_state: 7,
            logain_state: 3,
            reskin_state: 5,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[5].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_NOOK].flags, QF_OPEN);
    assert_eq!(log.entries()[7].flags, QF_DONE);
    assert_eq!(log.entries()[8].flags, QF_OPEN);
    assert_eq!(log.entries()[9].flags, QF_OPEN);
    // reskin_state=5 is >=4 but <8: open, not done.
    assert_eq!(log.entries()[17].flags, QF_OPEN);
}

#[test]
fn init_area1_quests_jessica_brithildie_camhermit_match_c() {
    let mut log = QuestLog::default();
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            jessica_state: JESSICA_STATE_QUEST1_FINISH,
            brithildie_state: BRITHILDIE_STATE_NOMORETALES_QOPEN,
            camhermit_state: CAMHERMIT_STATE_QUEST1DO,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_JESSICA_ROBBER_NOTE].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_JESSICA_KILL].flags, 0);
    assert_eq!(log.entries()[QLOG_BRITHILDIE].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_HERMIT_QUEST1].flags, QF_OPEN);
    assert_eq!(log.entries()[QLOG_HERMIT_QUEST2].flags, 0);

    let mut log = QuestLog::default();
    init_area1_quests(
        &mut log,
        &Area1QuestState {
            jessica_state: JESSICA_STATE_QUEST2_FINISH,
            brithildie_state: BRITHILDIE_STATE_NOMORETALES_QDONE,
            camhermit_state: CAMHERMIT_STATE_DONE,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[QLOG_JESSICA_ROBBER_NOTE].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_JESSICA_KILL].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_BRITHILDIE].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_HERMIT_QUEST1].flags, QF_DONE);
    assert_eq!(log.entries()[QLOG_HERMIT_QUEST2].flags, QF_DONE);
}

#[test]
fn init_nomad_quests_matches_c_thresholds() {
    let mut log = QuestLog::default();
    init_nomad_quests(&mut log, &NomadQuestState::default());
    assert_eq!(log.entries()[32].flags, 0);
    assert_eq!(log.entries()[33].flags, 0);
    assert_eq!(log.entries()[34].flags, 0);

    let mut state = NomadQuestState::default();
    state.nomad_state[1] = 5;
    state.nomad_state[4] = 2;
    state.nomad_state[5] = 1;
    init_nomad_quests(&mut log, &state);
    assert_eq!(log.entries()[32].flags, QF_OPEN);
    assert_eq!(log.entries()[33].flags, QF_OPEN);
    assert_eq!(log.entries()[34].flags, QF_OPEN);

    let mut state = NomadQuestState::default();
    state.nomad_state[1] = 9;
    state.nomad_state[4] = 4;
    state.nomad_state[5] = 4;
    init_nomad_quests(&mut log, &state);
    assert_eq!(log.entries()[32].flags, QF_DONE);
    assert_eq!(log.entries()[32].done, 1);
    assert_eq!(log.entries()[33].flags, QF_DONE);
    assert_eq!(log.entries()[34].flags, QF_DONE);
}

#[test]
fn init_area3_quests_seymour_and_kelly_ladders_match_c() {
    let mut log = QuestLog::default();
    init_area3_quests(&mut log, &Area3QuestState::default());
    for quest in [10, 11, 12, 13, 14, 15] {
        assert_eq!(log.entries()[quest].flags, 0);
    }

    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            seymour_state: 1,
            kelly_state: 2,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[10].flags, QF_OPEN);
    assert_eq!(log.entries()[11].flags, 0);
    assert_eq!(log.entries()[13].flags, QF_OPEN);
    assert_eq!(log.entries()[14].flags, 0);

    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            seymour_state: 12,
            kelly_state: 14,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[10].flags, QF_DONE);
    assert_eq!(log.entries()[11].flags, QF_DONE);
    assert_eq!(log.entries()[12].flags, QF_OPEN);
    assert_eq!(log.entries()[13].flags, QF_DONE);
    assert_eq!(log.entries()[14].flags, QF_DONE);
    assert_eq!(log.entries()[15].flags, QF_OPEN);

    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            seymour_state: 16,
            kelly_state: 16,
            ..Default::default()
        },
    );
    for quest in [10, 11, 12, 13, 14, 15] {
        assert_eq!(log.entries()[quest].flags, QF_DONE);
        assert_eq!(log.entries()[quest].done, 1);
    }
}

#[test]
fn init_area3_quests_astro2_crypt_clara_hermit_match_c() {
    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            astro2_state: 5,
            crypt_state: 12,
            clara_state: 15,
            hermit_state: 5,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[16].flags, QF_DONE);
    assert_eq!(log.entries()[18].flags, QF_DONE);
    assert_eq!(log.entries()[19].flags, QF_OPEN);
    assert_eq!(log.entries()[21].flags, QF_DONE);
    assert_eq!(log.entries()[24].flags, QF_DONE);

    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            astro2_state: 1,
            crypt_state: 1,
            clara_state: 6,
            hermit_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[16].flags, QF_OPEN);
    assert_eq!(log.entries()[18].flags, QF_OPEN);
    assert_eq!(log.entries()[19].flags, 0);
    assert_eq!(log.entries()[21].flags, QF_OPEN);
    assert_eq!(log.entries()[24].flags, QF_OPEN);
}

#[test]
fn init_area3_quests_william_ladder_has_no_final_else_like_c() {
    // Prime quests 22/23 to a non-zero flag, then confirm
    // `william_state <= 0` leaves them untouched (C has no final
    // `else` branch in this ladder, unlike every other one).
    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            william_state: 7,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[22].flags, QF_DONE);
    assert_eq!(log.entries()[23].flags, QF_DONE);

    init_area3_quests(&mut log, &Area3QuestState::default());
    assert_eq!(log.entries()[22].flags, QF_DONE);
    assert_eq!(log.entries()[23].flags, QF_DONE);

    // A fresh log with william_state=1 opens quest 22 only.
    let mut log = QuestLog::default();
    init_area3_quests(
        &mut log,
        &Area3QuestState {
            william_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[22].flags, QF_OPEN);
    assert_eq!(log.entries()[23].flags, 0);
}

#[test]
fn init_staff_quests_carlos_smugglecom_countbran_match_c() {
    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            carlos_state: 6,
            smugglecom_state: 10,
            countbran_bits: 1 | 2 | 4,
            countbran_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[20].flags, QF_DONE);
    assert_eq!(log.entries()[35].flags, QF_DONE);
    assert_eq!(log.entries()[36].flags, QF_DONE);
    assert_eq!(log.entries()[37].flags, QF_DONE);
    assert_eq!(log.entries()[40].flags, QF_DONE);

    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            carlos_state: 1,
            smugglecom_state: 5,
            countbran_bits: 1 | 2,
            countbran_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[20].flags, QF_OPEN);
    assert_eq!(log.entries()[35].flags, QF_DONE);
    assert_eq!(log.entries()[36].flags, QF_OPEN);
    assert_eq!(log.entries()[37].flags, 0);
    // Missing bit 4: not all bits set, but state>0 so open.
    assert_eq!(log.entries()[40].flags, QF_OPEN);
}

#[test]
fn init_staff_quests_yoatin_ladder_reproduces_c_copy_paste_bug() {
    // C bug: the "open" branch for quest 39 tests `aristocrat_state`,
    // not `yoatin_state` (`src/system/questlog.c:1284-1290`). With
    // yoatin_state=0 but aristocrat_state>0, quest 39 still opens.
    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            yoatin_state: 0,
            aristocrat_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[39].flags, QF_OPEN);

    // With both zero, quest 39 stays closed.
    let mut log = QuestLog::default();
    init_staff_quests(&mut log, &StaffQuestState::default());
    assert_eq!(log.entries()[39].flags, 0);

    // yoatin_state alone (aristocrat_state=0) does NOT open it either,
    // per the same bug.
    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            yoatin_state: 3,
            aristocrat_state: 0,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[39].flags, 0);

    // yoatin_state>=9 always marks it done regardless of aristocrat.
    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            yoatin_state: 9,
            aristocrat_state: 0,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[39].flags, QF_DONE);
}

#[test]
fn init_staff_quests_brenneth_broklin_dwarf_ladders_match_c() {
    let mut log = QuestLog::default();
    init_staff_quests(
        &mut log,
        &StaffQuestState {
            brennethbran_state: 9,
            spiritbran_state: 5,
            broklin_state: 11,
            dwarfchief_state: 11,
            dwarfshaman_state: 6,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[41].flags, QF_DONE);
    assert_eq!(log.entries()[42].flags, QF_DONE);
    assert_eq!(log.entries()[43].flags, QF_OPEN);
    assert_eq!(log.entries()[44].flags, QF_DONE);
    assert_eq!(log.entries()[45].flags, QF_DONE);
    assert_eq!(log.entries()[46].flags, QF_DONE);
    assert_eq!(log.entries()[47].flags, QF_DONE);
    assert_eq!(log.entries()[48].flags, QF_DONE);
    assert_eq!(log.entries()[49].flags, QF_DONE);
    assert_eq!(log.entries()[50].flags, QF_OPEN);
    assert_eq!(log.entries()[51].flags, QF_DONE);
    assert_eq!(log.entries()[52].flags, QF_DONE);
    assert_eq!(log.entries()[53].flags, QF_OPEN);

    let mut log = QuestLog::default();
    init_staff_quests(&mut log, &StaffQuestState::default());
    for quest in [41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53] {
        assert_eq!(log.entries()[quest].flags, 0);
    }
}

#[test]
fn init_twocity_quests_thief_ladder_matches_c() {
    let mut log = QuestLog::default();
    init_twocity_quests(&mut log, &TwocityQuestState::default());
    for quest in [25, 26, 27, 28] {
        assert_eq!(log.entries()[quest].flags, 0);
    }

    let mut log = QuestLog::default();
    init_twocity_quests(
        &mut log,
        &TwocityQuestState {
            thief_state: 18,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[25].flags, QF_DONE);
    assert_eq!(log.entries()[26].flags, QF_DONE);
    assert_eq!(log.entries()[27].flags, QF_DONE);
    assert_eq!(log.entries()[28].flags, QF_OPEN);

    let mut log = QuestLog::default();
    init_twocity_quests(
        &mut log,
        &TwocityQuestState {
            thief_state: 20,
            ..Default::default()
        },
    );
    for quest in [25, 26, 27, 28] {
        assert_eq!(log.entries()[quest].flags, QF_DONE);
        assert_eq!(log.entries()[quest].done, 1);
    }
}

#[test]
fn init_twocity_quests_sanwyn_skelly_alchemist_match_c() {
    let mut log = QuestLog::default();
    init_twocity_quests(
        &mut log,
        &TwocityQuestState {
            sanwyn_state: 8,
            skelly_state: 3,
            alchemist_state: 5,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[29].flags, QF_DONE);
    assert_eq!(log.entries()[30].flags, QF_DONE);
    assert_eq!(log.entries()[31].flags, QF_DONE);

    let mut log = QuestLog::default();
    init_twocity_quests(
        &mut log,
        &TwocityQuestState {
            sanwyn_state: 1,
            skelly_state: 1,
            alchemist_state: 1,
            ..Default::default()
        },
    );
    assert_eq!(log.entries()[29].flags, QF_OPEN);
    assert_eq!(log.entries()[30].flags, QF_OPEN);
    assert_eq!(log.entries()[31].flags, QF_OPEN);
}
