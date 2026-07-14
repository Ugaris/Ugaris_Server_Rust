use super::*;

#[test]
fn clara_dialogue_ports_initial_report_state_machine() {
    let outcome = clara_dialogue_step(clara_context(0, 0));
    assert_eq!(outcome.clara_state, 1);
    assert_eq!(
        outcome.text.as_deref(),
        Some(
            "Greetings, Hero! I am Clara, First Sergeant of the Seyan'Du and commander of this outpost."
        )
    );

    let blocked = clara_dialogue_step(clara_context(1, 14));
    assert_eq!(blocked.clara_state, 1);
    assert_eq!(blocked.text, None);

    let report = clara_dialogue_step(clara_context(1, 15));
    assert_eq!(report.clara_state, 3);
    assert_eq!(
        report.text.as_deref(),
        Some(
            "I assume thou hast been sent from Aston, Private, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp."
        )
    );

    let dismissed = clara_dialogue_step(clara_context(4, 15));
    assert_eq!(dismissed.clara_state, 5);
    assert_eq!(
        dismissed.text.as_deref(),
        Some(
            "Afterwards come back here, I have more work for thee. That will be all, Private. Dismissed!"
        )
    );
}

#[test]
fn clara_dialogue_ports_hardkill_quest_gates_and_rewards() {
    let blocked = clara_dialogue_step(clara_context(5, 17));
    assert_eq!(blocked.clara_state, 5);
    assert_eq!(blocked.text, None);

    let mission = clara_dialogue_step(clara_context(5, 18));
    assert_eq!(mission.clara_state, 7);
    assert_eq!(mission.open_questlog, Some(21));
    assert_eq!(
        mission.text.as_deref(),
        Some(
            "I have a difficult mission for thee, Hero. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks."
        )
    );

    let no_hardkill = clara_dialogue_step(clara_context(9, 18));
    assert_eq!(no_hardkill.clara_state, 9);
    assert_eq!(no_hardkill.text, None);

    let mut context = clara_context(9, 18);
    context.has_hardkill_item = true;
    context.hardkill_ritual_progress = 24;
    let partial_ritual = clara_dialogue_step(context);
    assert_eq!(partial_ritual.clara_state, 11);
    assert_eq!(partial_ritual.military_points, 4);
    assert_eq!(partial_ritual.military_exp, EXP_AREA15_HARDKILL);
    assert_eq!(
        partial_ritual.text.as_deref(),
        Some(
            "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, Hero."
        )
    );

    let mut context = clara_context(11, 18);
    context.has_hardkill_item = true;
    context.hardkill_ritual_progress = 36;
    let ready_to_kill = clara_dialogue_step(context);
    assert_eq!(ready_to_kill.clara_state, 13);
    assert_eq!(
        ready_to_kill.text.as_deref(),
        Some("Now that thou knowest how to kill that beast, please go and do it.")
    );

    let mut context = clara_context(14, 18);
    context.questlog_21_count = 1;
    let done = clara_dialogue_step(context);
    assert_eq!(done.clara_state, 15);
    assert_eq!(done.complete_questlog, Some(21));
    assert_eq!(done.military_points, 8);
    assert_eq!(done.military_exp, 1);
    assert_eq!(done.text.as_deref(), Some("Well done indeed, Hero!"));
}

#[test]
fn clara_replay_and_monster_death_match_c_state_boundaries() {
    assert_eq!(clara_replay_state_after_text_analysis(5, 2), 0);
    assert_eq!(clara_replay_state_after_text_analysis(9, 2), 6);
    assert_eq!(clara_replay_state_after_text_analysis(11, 2), 10);
    assert_eq!(clara_replay_state_after_text_analysis(13, 2), 12);
    assert_eq!(clara_replay_state_after_text_analysis(16, 2), 15);
    assert_eq!(clara_replay_state_after_text_analysis(14, 2), 14);
    assert_eq!(clara_replay_state_after_text_analysis(13, 1), 13);

    assert_eq!(clara_state_after_swamp_monster_death(12, true, true), 14);
    assert_eq!(clara_state_after_swamp_monster_death(13, true, true), 14);
    assert_eq!(clara_state_after_swamp_monster_death(11, true, true), 11);
    assert_eq!(clara_state_after_swamp_monster_death(12, false, true), 12);
    assert_eq!(clara_state_after_swamp_monster_death(12, true, false), 12);
}

#[test]
fn gatekeeper_qa_matches_c_table_words_and_codes() {
    assert_eq!(
        analyse_text_qa("how are you", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Said("I'm fine!".to_string())
    );
    assert_eq!(
        analyse_text_qa("hello", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Said("Hello, Hero!".to_string())
    );
    assert_eq!(
        analyse_text_qa("repeat", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Matched(2)
    );
    assert_eq!(
        analyse_text_qa("please restart", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Matched(2)
    );
    assert_eq!(
        analyse_text_qa("aye", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Matched(3)
    );
    assert_eq!(
        analyse_text_qa("nay", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Matched(4)
    );
    // Every accepted class-choice spelling variant maps to the same
    // `answer_code` C's table does (`gatekeeper.c:97-109`).
    for phrase in ["arch warrior", "arch-warrior"] {
        assert_eq!(
            analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(5),
            "phrase={phrase}"
        );
    }
    for phrase in ["arch mage", "arch-mage"] {
        assert_eq!(
            analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(6),
            "phrase={phrase}"
        );
    }
    for phrase in [
        "arch-seyan du",
        "arch seyan du",
        "arch-seyan'du",
        "arch seyan'du",
        "arch seyan",
        "arch-seyan",
    ] {
        assert_eq!(
            analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(7),
            "phrase={phrase}"
        );
    }
    for phrase in ["seyan du", "seyan'du", "seyan"] {
        assert_eq!(
            analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(8),
            "phrase={phrase}"
        );
    }
    assert_eq!(
        analyse_text_qa("reset", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Matched(9)
    );
    assert_eq!(
        analyse_text_qa("what's up", "Gatekeeper", "Hero", GATEKEEPER_QA),
        TextAnalysisOutcome::Said("Everything that isn't nailed down.".to_string())
    );
}

#[test]
fn gate_welcome_dialogue_greets_then_explains_the_test() {
    let flags = CharacterFlags::USED;
    let greet = gate_welcome_dialogue_step(gate_context(0, false, flags));
    assert_eq!(greet.welcome_state, 1);
    assert_eq!(
        greet.text.as_deref(),
        Some(
            "Be greeted, Hero. These are the halls of Ishtar. Only the greatest fighters and magic users come here, to take the final test and fight the Gatekeeper."
        )
    );

    let explain = gate_welcome_dialogue_step(gate_context(1, false, flags));
    assert_eq!(explain.welcome_state, 2);
    assert!(explain
        .text
        .unwrap()
        .starts_with("Those who succeed in this test"));
}

#[test]
fn gate_welcome_dialogue_sends_to_labyrinth_when_needed_and_waits() {
    let flags = CharacterFlags::USED;
    let sent = gate_welcome_dialogue_step(gate_context(2, true, flags));
    assert_eq!(sent.welcome_state, 3);
    assert_eq!(
        sent.text.as_deref(),
        Some(
            "Before thou mayest engage the Gatekeeper, thou must solve the Labyrinth built by Ishtar. Thou canst enter the labyrinth through the door to the east."
        )
    );

    // Re-entering at state 3 while the labyrinth is still unsolved:
    // C `case 3`'s `else break;` - no text, no state change.
    let waiting = gate_welcome_dialogue_step(gate_context(3, true, flags));
    assert_eq!(waiting.welcome_state, 3);
    assert_eq!(waiting.text, None);
}

#[test]
fn gate_welcome_dialogue_offers_class_choice_when_lab_already_solved() {
    // Fast path: state 2 with no labyrinth requirement falls through
    // case 3 into case 4 in the same call, ending at state 6 and
    // skipping the `case 5` "name the class" message entirely
    // (`gatekeeper.c`'s undocumented quirk - see `gate_case4` doc).
    let single_class = gate_welcome_dialogue_step(gate_context(
        2,
        false,
        CharacterFlags::USED | CharacterFlags::WARRIOR,
    ));
    assert_eq!(single_class.welcome_state, 6);
    assert_eq!(
        single_class.text.as_deref(),
        Some(
            "The choice is hard, and so is the test. If thou wishest to take the test, decide which path to follow. That of the Arch-Warrior, or that of the Seyan'Du."
        )
    );

    let seyan_already = gate_welcome_dialogue_step(gate_context(
        2,
        false,
        CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::MAGE,
    ));
    assert_eq!(seyan_already.welcome_state, 6);
    assert_eq!(
        seyan_already.text.as_deref(),
        Some("Since thou art already a Seyan'Du, thy only choice is to become Arch-Seyan'Du.")
    );

    let arch_already = gate_welcome_dialogue_step(gate_context(
        2,
        false,
        CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::ARCH,
    ));
    assert_eq!(arch_already.welcome_state, 6);
    assert_eq!(
        arch_already.text.as_deref(),
        Some("There is nothing I can do for thee, Hero, though, since thou art already an Arch-Warrior.")
    );
}

#[test]
fn gate_welcome_dialogue_slow_path_ends_one_state_lower_than_fast_path() {
    // Slow path: entering directly at state 3 (labyrinth requirement
    // just got satisfied since the last call) falls through case 3
    // into case 4 with `state == 4` on entry, so the non-arch
    // branches' `welcome_state++` lands on `5`, not `6` - the next
    // call will show the `case 5` "name the class" message that the
    // fast path (`gate_welcome_dialogue_offers_class_choice_when_
    // lab_already_solved`) never shows.
    let slow = gate_welcome_dialogue_step(gate_context(
        3,
        false,
        CharacterFlags::USED | CharacterFlags::WARRIOR,
    ));
    assert_eq!(slow.welcome_state, 5);
    assert_eq!(
        slow.text.as_deref(),
        Some(
            "The choice is hard, and so is the test. If thou wishest to take the test, decide which path to follow. That of the Arch-Warrior, or that of the Seyan'Du."
        )
    );

    let name_class = gate_welcome_dialogue_step(gate_context(
        5,
        false,
        CharacterFlags::USED | CharacterFlags::WARRIOR,
    ));
    assert_eq!(name_class.welcome_state, 6);
    assert_eq!(
        name_class.text.as_deref(),
        Some("Name the class thou wishest to become to begin the test. Each try will cost thee 100 gold coins.")
    );
}

#[test]
fn gate_welcome_dialogue_waits_silently_at_state_six() {
    let waiting = gate_welcome_dialogue_step(gate_context(6, false, CharacterFlags::USED));
    assert_eq!(waiting.welcome_state, 6);
    assert_eq!(waiting.text, None);
}

#[test]
fn gate_welcome_state_after_repeat_resets_only_below_state_seven() {
    assert_eq!(gate_welcome_state_after_repeat(0), 0);
    assert_eq!(gate_welcome_state_after_repeat(6), 0);
    assert_eq!(gate_welcome_state_after_repeat(7), 7);
}

#[test]
fn needs_next_lab_is_true_until_every_checkpoint_is_solved() {
    // Nothing solved: level 10 is the first checkpoint bit checked.
    assert!(needs_next_lab(0));
    // All five checkpoints solved: no lab needed anymore.
    let all_solved = (1_u64 << 10) | (1 << 15) | (1 << 20) | (1 << 25) | (1 << 30);
    assert!(!needs_next_lab(all_solved));
    // Missing just the last checkpoint still counts as needing a lab.
    let all_but_last = (1_u64 << 10) | (1 << 15) | (1 << 20) | (1 << 25);
    assert!(needs_next_lab(all_but_last));
    // Bits outside the known checkpoints (e.g. bit 0) never matter.
    assert!(needs_next_lab(1));
    assert!(!needs_next_lab(all_solved | 1));
}

#[test]
fn gate_enter_test_precheck_orders_preconditions_like_c() {
    let base = GateEnterTestPrecheck {
        is_paid: true,
        needs_lab: false,
        is_god: false,
        is_noexp: false,
        flags: CharacterFlags::USED | CharacterFlags::WARRIOR,
        carried_item_count: 0,
        class: 5,
    };

    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            is_paid: false,
            ..base
        }),
        GateEnterTestOutcome::NotPaid
    );
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            needs_lab: true,
            ..base
        }),
        GateEnterTestOutcome::LabNotSolved
    );
    // CF_GOD bypasses the labyrinth check but not CF_PAID/CF_NOEXP.
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            needs_lab: true,
            is_god: true,
            ..base
        }),
        GateEnterTestOutcome::Ready
    );
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            is_noexp: true,
            ..base
        }),
        GateEnterTestOutcome::NoExpMode
    );
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            flags: CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::MAGE,
            ..base
        }),
        GateEnterTestOutcome::InvalidClass
    );
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            carried_item_count: 2,
            ..base
        }),
        GateEnterTestOutcome::CarryingItems { count: 2 }
    );
    assert_eq!(gate_enter_test_precheck(base), GateEnterTestOutcome::Ready);

    // Seyan'Du (class 8) tolerates up to three carried items.
    let seyan = GateEnterTestPrecheck {
        flags: CharacterFlags::USED,
        class: 8,
        carried_item_count: 3,
        ..base
    };
    assert_eq!(gate_enter_test_precheck(seyan), GateEnterTestOutcome::Ready);
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            carried_item_count: 4,
            ..seyan
        }),
        GateEnterTestOutcome::CarryingTooManyItems { count: 4 }
    );

    // CF_GOD also bypasses class/item-count validation entirely.
    assert_eq!(
        gate_enter_test_precheck(GateEnterTestPrecheck {
            is_god: true,
            flags: CharacterFlags::USED | CharacterFlags::ARCH,
            carried_item_count: 99,
            ..base
        }),
        GateEnterTestOutcome::Ready
    );
}

#[test]
fn gate_class_choice_validation_matches_c_flag_checks() {
    use CharacterFlags as F;
    // Arch-Warrior (5): blocked if already MAGE or ARCH.
    assert!(gate_class_choice_is_valid(F::USED | F::WARRIOR, 5));
    assert!(!gate_class_choice_is_valid(F::USED | F::MAGE, 5));
    assert!(!gate_class_choice_is_valid(F::USED | F::ARCH, 5));

    // Arch-Mage (6): blocked if already WARRIOR or ARCH.
    assert!(gate_class_choice_is_valid(F::USED | F::MAGE, 6));
    assert!(!gate_class_choice_is_valid(F::USED | F::WARRIOR, 6));

    // Arch-Seyan'Du (7): requires both WARRIOR and MAGE, not ARCH.
    assert!(gate_class_choice_is_valid(
        F::USED | F::WARRIOR | F::MAGE,
        7
    ));
    assert!(!gate_class_choice_is_valid(F::USED | F::WARRIOR, 7));
    assert!(!gate_class_choice_is_valid(
        F::USED | F::WARRIOR | F::MAGE | F::ARCH,
        7
    ));

    // Seyan'Du (8): blocked if already ARCH or already both WARRIOR+MAGE.
    assert!(gate_class_choice_is_valid(F::USED | F::WARRIOR, 8));
    assert!(gate_class_choice_is_valid(F::USED, 8));
    assert!(!gate_class_choice_is_valid(
        F::USED | F::WARRIOR | F::MAGE,
        8
    ));
    assert!(!gate_class_choice_is_valid(F::USED | F::ARCH, 8));

    // Unknown class values are always invalid (C's `default: return 0;`).
    assert!(!gate_class_choice_is_valid(F::USED, 0));
    assert!(!gate_class_choice_is_valid(F::USED, 99));
}
