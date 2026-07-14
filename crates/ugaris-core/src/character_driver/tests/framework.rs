use super::*;

#[test]
fn analyse_text_qa_matches_keyword_and_substitutes_names() {
    // C: `quiet_say(cn, "Hello, %s!", ch[co].name, ch[cn].name)`.
    assert_eq!(
        analyse_text_qa("hello", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("Hello, Bob!".to_string())
    );
}

#[test]
fn analyse_text_qa_is_case_insensitive() {
    assert_eq!(
        analyse_text_qa("HELLO", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("Hello, Bob!".to_string())
    );
    assert_eq!(
        analyse_text_qa("HeLLo", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("Hello, Bob!".to_string())
    );
}

#[test]
fn analyse_text_qa_reports_no_match_for_unknown_text() {
    assert_eq!(
        analyse_text_qa("blahblah nonsense", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::NoMatch
    );
    // Empty word list (e.g. only punctuation) is also NoMatch, matching
    // C's `if (w) { ... }` guard around the qa scan.
    assert_eq!(
        analyse_text_qa("...", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::NoMatch
    );
}

#[test]
fn analyse_text_qa_filters_own_name_out_of_wordlist() {
    // C: `strcasecmp(wordlist[w], ch[cn].name)` drops the NPC's own
    // name from the tokenized message before matching, so addressing
    // the merchant by name doesn't break a match.
    assert_eq!(
        analyse_text_qa("Dolf, hello", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("Hello, Bob!".to_string())
    );
    assert_eq!(
        analyse_text_qa("hello Dolf", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("Hello, Bob!".to_string())
    );
}

#[test]
fn analyse_text_qa_requires_exact_word_count_match() {
    // C's inner match loop requires the tokenized message and the qa
    // pattern to run out of words together (`n == w && !qa[q].word[n]`);
    // a longer or shorter phrase around a keyword is not a match.
    assert_eq!(
        analyse_text_qa("well hello there", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::NoMatch
    );
    assert_eq!(
        analyse_text_qa("how are you doing", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::NoMatch
    );
    assert_eq!(
        analyse_text_qa("how are you", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Said("I'm fine!".to_string())
    );
}

#[test]
fn analyse_text_qa_reports_answer_code_when_no_canned_answer() {
    // C: `who are you` -> `answer: NULL, answer_code: 1` -> callers
    // that don't special-case it (like `gwendylon_driver`) get the
    // raw code back to interpret themselves.
    assert_eq!(
        analyse_text_qa("who are you", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Matched(1)
    );
    assert_eq!(
        analyse_text_qa("what is your name", "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::Matched(1)
    );
}

#[test]
fn analyse_text_qa_rejects_oversized_words() {
    // C: `if (n > 250) return 0;` bails out of tokenization entirely.
    let huge_word = "a".repeat(300);
    assert_eq!(
        analyse_text_qa(&huge_word, "Dolf", "Bob", MERCHANT_QA),
        TextAnalysisOutcome::NoMatch
    );
}

const NEEDS_NAME_TEST_QA: &[TextQaEntry] = &[TextQaEntry {
    words: &["yes"],
    answer: None,
    answer_code: 20,
}];

#[test]
fn analyse_text_qa_needs_name_requires_the_target_name_in_the_same_sentence() {
    // C `if (qa[q].needs_name && !name) continue;` - the word pattern
    // alone is not enough for a `needs_name: 1` row.
    assert_eq!(
        analyse_text_qa_needs_name("yes", "Bert", "Hero", NEEDS_NAME_TEST_QA),
        TextAnalysisOutcome::NoMatch
    );
}

#[test]
fn analyse_text_qa_needs_name_matches_once_the_target_is_addressed_by_name() {
    assert_eq!(
        analyse_text_qa_needs_name("Bert, yes", "Bert", "Hero", NEEDS_NAME_TEST_QA),
        TextAnalysisOutcome::Matched(20)
    );
    assert_eq!(
        analyse_text_qa_needs_name("yes Bert", "Bert", "Hero", NEEDS_NAME_TEST_QA),
        TextAnalysisOutcome::Matched(20)
    );
}

#[test]
fn analyse_text_qa_needs_name_is_case_insensitive_for_the_name() {
    assert_eq!(
        analyse_text_qa_needs_name("BERT, yes", "Bert", "Hero", NEEDS_NAME_TEST_QA),
        TextAnalysisOutcome::Matched(20)
    );
}

#[test]
fn mem_check_driver_is_false_until_added() {
    let memory = DriverMemory::default();
    assert!(!mem_check_driver(&memory, 7, 42));
}

#[test]
fn mem_add_then_check_driver_remembers_target() {
    let mut memory = DriverMemory::default();
    assert!(mem_add_driver(&mut memory, 7, 42));
    assert!(mem_check_driver(&memory, 7, 42));
    // C: unrelated slots and unrelated targets stay untouched.
    assert!(!mem_check_driver(&memory, 6, 42));
    assert!(!mem_check_driver(&memory, 7, 99));
}

#[test]
fn mem_add_driver_is_idempotent_for_duplicate_targets() {
    // C: `if (dat->xID[n] == xID) return 1;` - no duplicate entry, and
    // erasing the slot removes the target in one shot either way.
    let mut memory = DriverMemory::default();
    assert!(mem_add_driver(&mut memory, 3, 7));
    assert!(mem_add_driver(&mut memory, 3, 7));
    assert_eq!(memory.slots[3].len(), 1);
}

#[test]
fn mem_add_and_check_driver_reject_out_of_range_slots() {
    // C: `if (nr < 0 || nr > 7) return 0;`.
    let mut memory = DriverMemory::default();
    assert!(!mem_add_driver(&mut memory, DRIVER_MEMORY_SLOTS, 1));
    assert!(!mem_check_driver(&memory, DRIVER_MEMORY_SLOTS, 1));
}

#[test]
fn mem_erase_driver_clears_only_the_requested_slot() {
    let mut memory = DriverMemory::default();
    mem_add_driver(&mut memory, 2, 1);
    mem_add_driver(&mut memory, 7, 2);
    mem_erase_driver(&mut memory, 7);
    assert!(!mem_check_driver(&memory, 7, 2));
    assert!(mem_check_driver(&memory, 2, 1));
}

#[test]
fn mem_erase_driver_out_of_range_slot_is_a_silent_no_op() {
    let mut memory = DriverMemory::default();
    mem_add_driver(&mut memory, 0, 1);
    mem_erase_driver(&mut memory, DRIVER_MEMORY_SLOTS);
    assert!(mem_check_driver(&memory, 0, 1));
}
