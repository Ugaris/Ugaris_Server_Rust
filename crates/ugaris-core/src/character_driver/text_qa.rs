//! Generic NPC small-talk keyword matcher (text-QA framework) and the
//! gatekeeper QA table.

//-----------------------
// Generic NPC small-talk keyword matcher.
//
// C `analyse_text_driver` is duplicated near-verbatim across
// `src/module/merchants/merchant.c`, `src/area/1/gwendylon.c`,
// `src/module/bank.c`, `src/module/base.c`, `src/module/military.c`,
// `src/area/16/forest.c`, `src/area/3/area3.c`, `src/area/37/arkhata.c` and
// `src/module/orbbank/orb_bank_npc.c`. Every copy shares the same core:
// tokenize the spoken text into lowercase words (splitting on
// `' ' ',' ':' '?' '!' '"' '.'`), drop any word equal to the NPC's own
// name (`strcasecmp(wordlist[w], ch[cn].name)`), then scan a `struct qa`
// table in order for the first entry whose word pattern matches the
// tokenized message *exactly* (same word count, same words in order -
// the C inner loop only reports a hit when `n == w && !qa[q].word[n]`,
// i.e. both the message and the pattern run out of words together).
//
// C's tokenizer is fed the *full* formatted log line (`"Name says:
// \"text\""`) and skips a leading `alpha+space+alpha+':'+space+'"'`
// prefix to strip the speaker name/verb before splitting into words; the
// Rust driver messages (`push_driver_text_message`) already carry only
// the bare spoken text, so that prefix-skip has no equivalent here.
// C also never flushes the last accumulated word unless a delimiter
// follows it - harmless in C because the trailing `'"'` of the quoted
// log line always supplies one. Since our `text` has no such trailing
// quote, we flush the final word unconditionally to keep the same
// user-visible matching behavior.

/// One `struct qa` row shared by every `analyse_text_driver` copy.
#[derive(Debug, Clone, Copy)]
pub struct TextQaEntry {
    /// Lowercase word pattern (`qa[q].word[..]`), matched for an exact
    /// (same length, same order) hit against the tokenized message.
    pub words: &'static [&'static str],
    /// `qa[q].answer`: a canned reply template fed to
    /// `quiet_say(cn, answer, ch[co].name, ch[cn].name)`. `%s` placeholders
    /// are substituted in order: speaker name, then the NPC's own name.
    pub answer: Option<&'static str>,
    /// `qa[q].answer_code`: reported back to the caller when `answer` is
    /// `None`, for area-specific dialogue branches to interpret.
    pub answer_code: i32,
}
/// Result of [`analyse_text_qa`], mirroring the two ways C
/// `analyse_text_driver` reports a qa-table hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextAnalysisOutcome {
    /// Matched an entry with a canned `answer` template; text already has
    /// `%s` placeholders substituted - the caller should `quiet_say` it.
    Said(String),
    /// Matched an entry with `answer: None`; carries `answer_code` for the
    /// caller to interpret.
    Matched(i32),
    /// No qa entry matched the tokenized message (including the case of
    /// an empty word list, matching C's `if (w) { ... }` guard).
    NoMatch,
}
/// Tokenizes spoken `text` into lowercase words the way every
/// `analyse_text_driver` copy does: split on `' ' ',' ':' '?' '!' '"'
/// '.'`, drop words equal to `own_name` (`strcasecmp`), cap at 20 words
/// (`if (w < 20) w++`), and bail out (returning `None`) if any single
/// word exceeds 250 bytes (`if (n > 250) return 0;`).
pub fn tokenize_text_words(text: &str, own_name: &str) -> Option<Vec<String>> {
    tokenize_text_words_with_name_flag(text, own_name).map(|(words, _name_seen)| words)
}
/// [`tokenize_text_words`], plus whether `own_name` was seen as one of the
/// dropped words (C's `name = 1` flag, set when a word equals `ch[cn].name`
/// via `strcasecmp`, `fdemon.c:229-237`). Used by [`analyse_text_qa_needs_name`]
/// for qa rows with `needs_name: 1` (`struct qa::needs_name`,
/// `fdemon.c:83-88`), which only match when the speaker also addressed the
/// target by name in the same sentence.
fn tokenize_text_words_with_name_flag(text: &str, own_name: &str) -> Option<(Vec<String>, bool)> {
    let mut words: Vec<String> = Vec::new();
    let mut name_seen = false;
    let mut current = String::new();
    let flush = |current: &mut String, words: &mut Vec<String>, name_seen: &mut bool| {
        if !current.is_empty() {
            let lower = current.to_ascii_lowercase();
            if lower.eq_ignore_ascii_case(own_name) {
                *name_seen = true;
            } else if words.len() < 20 {
                words.push(lower);
            }
            current.clear();
        }
    };
    for c in text.chars() {
        match c {
            ' ' | ',' | ':' | '?' | '!' | '"' | '.' => {
                flush(&mut current, &mut words, &mut name_seen)
            }
            _ => {
                current.push(c);
                if current.len() > 250 {
                    return None;
                }
            }
        }
    }
    flush(&mut current, &mut words, &mut name_seen);
    Some((words, name_seen))
}
/// Substitutes `%s` placeholders in a qa `answer` template: the first
/// with `speaker_name`, the second with `own_name`, matching C's
/// `quiet_say(cn, qa[q].answer, ch[co].name, ch[cn].name)`.
fn format_qa_answer(template: &str, speaker_name: &str, own_name: &str) -> String {
    let mut args = [speaker_name, own_name].into_iter();
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' && chars.peek() == Some(&'s') {
            chars.next();
            out.push_str(args.next().unwrap_or(""));
        } else {
            out.push(c);
        }
    }
    out
}
/// C `analyse_text_driver`'s shared tokenize-and-match core. Callers are
/// responsible for the guard clauses that precede tokenization in C
/// (ignore system/info log messages, ignore our own talk, ignore
/// non-players, distance and visibility checks) since those need access
/// to `World` state this module does not have.
pub fn analyse_text_qa(
    text: &str,
    own_name: &str,
    speaker_name: &str,
    qa: &[TextQaEntry],
) -> TextAnalysisOutcome {
    let Some(words) = tokenize_text_words(text, own_name) else {
        return TextAnalysisOutcome::NoMatch;
    };
    if words.is_empty() {
        return TextAnalysisOutcome::NoMatch;
    }
    for entry in qa {
        if entry.words.len() == words.len()
            && entry
                .words
                .iter()
                .zip(words.iter())
                .all(|(pattern, word)| pattern.eq_ignore_ascii_case(word))
        {
            return match entry.answer {
                Some(template) => {
                    TextAnalysisOutcome::Said(format_qa_answer(template, speaker_name, own_name))
                }
                None => TextAnalysisOutcome::Matched(entry.answer_code),
            };
        }
    }
    TextAnalysisOutcome::NoMatch
}
/// [`analyse_text_qa`], but only for qa rows whose C `struct qa::needs_name`
/// is `1` - i.e. the speaker must also have addressed the target by its own
/// name somewhere in the same sentence (C's `if (qa[q].needs_name && !name)
/// continue;` guard, `fdemon.c:262-264`). Every row of `CDR_FDEMON_ARMY`'s
/// emote-reaction table (`QA_YES`..`QA_COWARD`, `fdemon.c:139-183`) has
/// `needs_name: 1`, so - unlike C, which stores the flag per-row in the same
/// shared `qa[]` array as the `needs_name: 0` small-talk rows - this crate
/// keeps needs-name-gated entries in their own dedicated `qa` slice/function
/// pair rather than adding a `needs_name` field to every one of
/// [`TextQaEntry`]'s ~300 other call sites across every other `analyse_text_
/// driver` table in this codebase (all of which are `needs_name: 0`).
pub fn analyse_text_qa_needs_name(
    text: &str,
    own_name: &str,
    speaker_name: &str,
    qa: &[TextQaEntry],
) -> TextAnalysisOutcome {
    let Some((words, name_seen)) = tokenize_text_words_with_name_flag(text, own_name) else {
        return TextAnalysisOutcome::NoMatch;
    };
    if words.is_empty() || !name_seen {
        return TextAnalysisOutcome::NoMatch;
    }
    for entry in qa {
        if entry.words.len() == words.len()
            && entry
                .words
                .iter()
                .zip(words.iter())
                .all(|(pattern, word)| pattern.eq_ignore_ascii_case(word))
        {
            return match entry.answer {
                Some(template) => {
                    TextAnalysisOutcome::Said(format_qa_answer(template, speaker_name, own_name))
                }
                None => TextAnalysisOutcome::Matched(entry.answer_code),
            };
        }
    }
    TextAnalysisOutcome::NoMatch
}
/// C `struct qa qa[]` from `src/system/gatekeeper.c:83-112`
/// (`gate_welcome_driver`'s small-talk plus the class-choice answer
/// codes). Unlike [`MERCHANT_QA`]/[`TRADER_QA`], every row past `"nay"`
/// carries `answer: NULL` and a distinct `answer_code` the caller must
/// interpret: `2` repeat/restart (resets `welcome_state` to `0`), `3`
/// aye, `4` nay, `5`/`6`/`7`/`8` the Arch-Warrior/Arch-Mage/Arch-Seyan'Du/
/// Seyan'Du class choice fed to `enter_test`, and `9` reset (deletes
/// `DRD_LAB_PPD` for `CF_GOD` speakers). Word patterns are copied
/// verbatim; C's tokenizer only splits on `' ' ',' ':' '?' '!' '"' '.'`
/// so `"arch-warrior"`/`"seyan'du"` stay single tokens (hyphen and
/// apostrophe are not delimiters).
pub const GATEKEEPER_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["aye"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["nay"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["arch", "warrior"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["arch-warrior"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["arch", "mage"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["arch-mage"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["arch-seyan", "du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan", "du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch-seyan'du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan'du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch-seyan"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["seyan", "du"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["seyan'du"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["seyan"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["reset"],
        answer: None,
        answer_code: 9,
    },
];
