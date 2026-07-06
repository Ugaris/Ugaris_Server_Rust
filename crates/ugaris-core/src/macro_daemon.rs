//! Pure decision logic for the anti-macro/anti-bot "Macro Daemon" NPC
//! driver (`CDR_MACRO` = 37, C `src/module/base.c`'s `macro_driver` and
//! its static helpers, `base.c:236-1263`).
//!
//! # Scope of this slice
//!
//! [`crate::player::MacroPpd`] (the persistent per-player state this
//! driver reads/writes, C `struct macro_ppd`, `command.c:585-626`) has
//! lived on `PlayerRuntime` since long before this slice, with a full set
//! of GOD/staff admin debug commands
//! (`/macrostats`/`/macrohistory`/`/summonmacro`/`/macroimmune`/
//! `/macrosuspicion`/`/macrokarma`/`/macrofailures`/`/macroreset`/
//! `/macrolist`/`/macrohelp` in `ugaris-server/src/commands_admin.rs`)
//! already reading and mutating it - but until now nothing actually
//! *drove* it, since `macro_driver` itself was never ported (`CDR_MACRO`
//! resolved to a permanent `CharacterDriverOutcome::HandledStub`).
//!
//! `World` cannot reach `PlayerRuntime` (same constraint documented on
//! `world/military.rs` and `world/bank.rs` for their own PPD-backed
//! drivers), so a full live wiring of `macro_driver` needs an
//! `apply_macro_events`-shaped bridge in `ugaris-server` (mirroring
//! `apply_military_master_events`) that has both a `&mut World` (for the
//! NPC's own message loop/state machine/teleports, which *is*
//! `CharacterDriverState`-backed data) and a `&mut ServerRuntime` (for
//! `PlayerRuntime::macro_ppd`). That bridge, the `CharacterDriverState::
//! Macro` NPC-side state machine itself (struct `macro_data`), and the
//! `World::process_macro_actions` driver loop are **not** part of this
//! slice - only the pure, `MacroPpd`-and-plain-values-in/out "brain" a
//! future wiring slice will call is ported here, exactly the kind of
//! `PlayerRuntime`-decoupled logic `military.rs`'s doc comment describes
//! as the natural first cut for a PPD-heavy driver. Ported in this slice:
//!
//! - Challenge generation/asking/checking (`macro_generate_challenge`/
//!   `macro_ask_challenge`/`macro_check_answer`, `base.c:507-585`) as
//!   [`macro_generate_challenge`]/[`macro_ask_challenge_lines`]/
//!   [`macro_check_answer`].
//! - The activity/AFK gate (`macro_is_player_active`, `base.c:471-486`)
//!   as [`macro_is_player_active`].
//! - History recording (`macro_record_history`, `base.c:1244-1263`) and
//!   the correct-answer/failure `MacroPpd` mutations (`base.c:854-873`,
//!   `728-773`) as [`macro_record_history`]/
//!   [`macro_apply_correct_answer`]/[`macro_apply_failure`], operating
//!   directly on a real `&mut MacroPpd` - a future live-driver slice can
//!   call these verbatim once it has one in hand via
//!   `PlayerRuntime::macro_ppd`.
//! - The reward-type roll (`macro_give_reward`, `base.c:611-651`,
//!   `isxmas` case at `640-648`) as [`macro_roll_reward`]/
//!   [`macro_xmas_reward_message`] plus the message/template lookups -
//!   *classification* only, since actually creating/giving the item
//!   needs a `ZoneLoader` (`World` has none either, see
//!   `ugaris-server/src/area_apply.rs::grant_template_item_smart`, the
//!   existing precedent for this exact create-item-give-or-destroy
//!   shape) - a future slice resolves the `Gold`/`Item` variants'
//!   remaining `RANDOM`/`create_item` outcomes and calls
//!   `grant_template_item_smart`.
//! - The area/section victim-search exclusions (`base.c:967-975`) as
//!   [`macro_is_area_excluded`].
//!
//! Explicitly **not** ported yet (left for the future live-driver
//! wiring slice):
//!
//! - The message loop, state machine (`MACRO_STATE_*`), and NPC
//!   teleportation itself.
//! - The cross-server "challenge room" teleport-and-restore flow
//!   (needs the same `attempt_cross_area_transfer` hand-off
//!   `world/jail.rs`/`world/dungeon_master.rs` already use for their own
//!   cross-area cases).
//! - `macro_track_exp_gain`/`macro_track_combat`/`macro_track_gold_change`
//!   (`src/system/tool.c:385-426`, called from many scattered gold/exp/
//!   combat call sites in `tool.c`/`death.c`): these update
//!   `MacroPpd::last_exp_gain`/`last_combat`/`last_gold_change`, which
//!   [`macro_is_player_active`] reads - until a future slice wires them
//!   in at each call site (all of which need `PlayerRuntime`, hence
//!   `ugaris-server`, not `World`), those three fields simply never
//!   update, so every player looks permanently "just became active" from
//!   the moment their `MacroPpd` was created (`Default`'s `0`), which
//!   happens to make [`macro_is_player_active`] return `false` forever
//!   for a `MacroPpd::default()` player (since `now - 0` is never `<
//!   300` for any real Unix timestamp) - i.e. safely fails toward "does
//!   not look active" rather than the reverse.
//! - The pentagram-progress save/restore fields (`saved_pent_*`): no-op
//!   until `pents.c` gameplay itself is ported (see
//!   `PlayerRuntime::pentagram_debug`'s own doc comment).
//! - The `isxmas` NPC name/sprite reskin ("Saint Nick"): cosmetic,
//!   `ugaris-core` has no xmas-event awareness (`ServerRuntime::
//!   xmas_special_override` lives in `ugaris-server`, see
//!   `ugaris-server/src/xmas.rs`).

use crate::player::MacroPpd;
use crate::world::level_value;

/// C `#define MACRO_CHALLENGE_MATH 0` (`base.c:236`): simple addition.
pub const MACRO_CHALLENGE_MATH: i32 = 0;
/// C `#define MACRO_CHALLENGE_WORD 1` (`base.c:237`): type a word.
pub const MACRO_CHALLENGE_WORD: i32 = 1;
/// C `#define MACRO_CHALLENGE_REVERSE 2` (`base.c:238`): type backwards.
pub const MACRO_CHALLENGE_REVERSE: i32 = 2;
/// C `#define MACRO_CHALLENGE_CHOICE 3` (`base.c:239`): multiple choice -
/// currently unreachable from [`macro_generate_challenge`] (C's own
/// comment: "multiple choice disabled for now"), but
/// [`macro_check_answer`]/[`macro_ask_challenge_lines`] still handle it
/// for parity, matching the C oracle's own dead-but-present branches.
pub const MACRO_CHALLENGE_CHOICE: i32 = 3;

/// C `#define MACRO_ACTIVITY_TIMEOUT 300` (`base.c:262`): 5 minutes.
pub const MACRO_ACTIVITY_TIMEOUT: i64 = 300;
/// C `#define MACRO_CHALLENGE_TIME 180` (`base.c:263`): 3 minutes to
/// answer.
pub const MACRO_CHALLENGE_TIME: i64 = 180;
/// C `#define MACRO_REPEAT_INTERVAL 45` (`base.c:264`): repeat the
/// question every 45 seconds.
pub const MACRO_REPEAT_INTERVAL: i64 = 45;

/// C `#define CHALLENGE_ROOM_X 178` (`base.c:255`).
pub const CHALLENGE_ROOM_X: u16 = 178;
/// C `#define CHALLENGE_ROOM_Y 248` (`base.c:256`).
pub const CHALLENGE_ROOM_Y: u16 = 248;
/// C `#define CHALLENGE_ROOM_AREA 3` (`base.c:257`).
pub const CHALLENGE_ROOM_AREA: u16 = 3;

/// C `macro_challenge_words[]` (`base.c:311-313`), `MACRO_WORD_COUNT` = 20.
pub const MACRO_CHALLENGE_WORDS: &[&str] = &[
    "GUARDIAN",
    "WARRIOR",
    "MAGE",
    "SEYAN",
    "DEMON",
    "ASTON",
    "CAMERON",
    "PORTAL",
    "QUEST",
    "ADVENTURE",
    "SHIELD",
    "SWORD",
    "MAGIC",
    "BLESS",
    "HEAL",
    "DRAGON",
    "KNIGHT",
    "CASTLE",
    "DUNGEON",
    "TREASURE",
];

/// C `struct macro_choice_question` (`base.c:319-323`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroChoiceQuestion {
    pub question: &'static str,
    pub answers: [&'static str; 4],
    pub correct: i32,
}

/// C `macro_choice_questions[]` (`base.c:326-334`), `MACRO_CHOICE_COUNT` = 8.
pub const MACRO_CHOICE_QUESTIONS: &[MacroChoiceQuestion] = &[
    MacroChoiceQuestion {
        question: "Which skill lets you heal others?",
        answers: ["Bless", "Flash", "Freeze", "Pulse"],
        correct: 0,
    },
    MacroChoiceQuestion {
        question: "What is the starting town called?",
        answers: ["Aston", "Cameron", "Exkordon", "Pents"],
        correct: 1,
    },
    MacroChoiceQuestion {
        question: "Which class uses both melee and magic?",
        answers: ["Warrior", "Mage", "Seyan'Du", "Archer"],
        correct: 2,
    },
    MacroChoiceQuestion {
        question: "What do you need to create a clan?",
        answers: ["Gold", "A Clan Jewel", "Staff approval", "Level 50"],
        correct: 1,
    },
    MacroChoiceQuestion {
        question: "Which NPC teaches basic combat?",
        answers: ["Lydia", "Yoakin", "Gwendylon", "Seymour"],
        correct: 2,
    },
    MacroChoiceQuestion {
        question: "What happens when you die?",
        answers: [
            "Lose all items",
            "Lose some experience",
            "Nothing",
            "Game over",
        ],
        correct: 1,
    },
    MacroChoiceQuestion {
        question: "Which color text indicates a clickable keyword?",
        answers: ["Red", "Green", "Light Blue", "Yellow"],
        correct: 2,
    },
    MacroChoiceQuestion {
        question: "What is the maximum level?",
        answers: ["50", "75", "100", "No limit"],
        correct: 3,
    },
];

/// C `struct macro_data`'s challenge-relevant fields (`base.c:242-254`):
/// `victim`/`v_ID`/`state`/`start`/`last`/`teleported_to_jail` are NPC-
/// side (`CharacterDriverState`-backed) bookkeeping for the not-yet-
/// ported live driver, omitted here.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MacroChallenge {
    pub challenge_type: i32,
    pub val1: i32,
    pub val2: i32,
    pub word: String,
    pub expected_answer: String,
    pub choice_answer: i32,
}

impl MacroChallenge {
    fn math(val1: i32, val2: i32) -> Self {
        MacroChallenge {
            challenge_type: MACRO_CHALLENGE_MATH,
            val1,
            val2,
            word: String::new(),
            expected_answer: (val1 + val2).to_string(),
            choice_answer: 0,
        }
    }

    fn word(word: &str) -> Self {
        MacroChallenge {
            challenge_type: MACRO_CHALLENGE_WORD,
            val1: 0,
            val2: 0,
            word: word.to_string(),
            expected_answer: word.to_string(),
            choice_answer: 0,
        }
    }

    fn reverse(word: &str) -> Self {
        MacroChallenge {
            challenge_type: MACRO_CHALLENGE_REVERSE,
            val1: 0,
            val2: 0,
            word: word.to_string(),
            expected_answer: word.chars().rev().collect(),
            choice_answer: 0,
        }
    }
}

/// C `macro_is_player_active` (`base.c:471-486`): `true` if the player
/// gained exp, fought, or gained gold within the last
/// [`MACRO_ACTIVITY_TIMEOUT`] seconds - see this module's doc comment for
/// why `last_exp_gain`/`last_combat`/`last_gold_change` never update yet.
pub fn macro_is_player_active(ppd: &MacroPpd, now: i64) -> bool {
    now - ppd.last_exp_gain < MACRO_ACTIVITY_TIMEOUT
        || now - ppd.last_combat < MACRO_ACTIVITY_TIMEOUT
        || now - ppd.last_gold_change < MACRO_ACTIVITY_TIMEOUT
}

/// C `macro_driver`'s IDLE-state victim-search area/section exclusions
/// (`base.c:967-975`): `true` when a candidate at `(x, y)` in `area_id`
/// should be skipped.
pub fn macro_is_area_excluded(area_id: u16, x: usize, y: usize) -> bool {
    let section_id = crate::area_section::section_at(area_id, x, y).map(|section| section.id);
    if area_id == 3 && section_id != Some(20) {
        return true;
    }
    if section_id == Some(114) {
        return true;
    }
    if area_id == 22 && (76..=94).contains(&x) && (19..=37).contains(&y) {
        return true;
    }
    false
}

/// C `macro_generate_challenge` (`base.c:507-566`): higher `suspicion`/
/// any prior `challenge_failures` raises the difficulty tier, which picks
/// a harder challenge type. `seed` is threaded through
/// `crate::world::legacy_random_below_from_seed` in the exact call order
/// C's own `RANDOM(n)` sequence uses, so a caller sharing `World::
/// legacy_random_seed` reproduces the identical roll sequence a live
/// driver would.
pub fn macro_generate_challenge(
    seed: &mut u32,
    suspicion: i32,
    challenge_failures: i32,
) -> MacroChallenge {
    use crate::world::legacy_random_below_from_seed as random_below;

    let mut difficulty = 0;
    if suspicion > 30 {
        difficulty = 1;
    }
    if suspicion > 60 {
        difficulty = 2;
    }
    if challenge_failures > 0 {
        difficulty += 1;
    }

    if difficulty < 1 {
        let val1 = random_below(seed, 50) as i32 + 1;
        let val2 = random_below(seed, 20) as i32 + 1;
        MacroChallenge::math(val1, val2)
    } else if difficulty < 2 {
        if random_below(seed, 2) == 0 {
            let idx = random_below(seed, MACRO_CHALLENGE_WORDS.len() as u32) as usize;
            MacroChallenge::word(MACRO_CHALLENGE_WORDS[idx])
        } else {
            let val1 = random_below(seed, 100) as i32 + 10;
            let val2 = random_below(seed, 50) as i32 + 5;
            MacroChallenge::math(val1, val2)
        }
    } else if random_below(seed, 2) == 0 {
        let idx = random_below(seed, MACRO_CHALLENGE_WORDS.len() as u32) as usize;
        MacroChallenge::reverse(MACRO_CHALLENGE_WORDS[idx])
    } else {
        let val1 = random_below(seed, 200) as i32 + 50;
        let val2 = random_below(seed, 100) as i32 + 20;
        MacroChallenge::math(val1, val2)
    }
}

/// C `macro_ask_challenge` (`base.c:568-585`): the `say(cn, ...)` line(s)
/// the daemon speaks to pose `challenge`. C's inline `COL_YELLOW`/
/// `COL_LIGHT_RED`/`COL_KEYWORD` color escapes are dropped, matching this
/// codebase's established plain-text `npc_say` convention (e.g.
/// `world/bank.rs`'s "Hello {}! Would you like to open an account"
/// dropping C's own `COL_LIGHT_BLUE`/`COL_RESET` around "account").
pub fn macro_ask_challenge_lines(challenge: &MacroChallenge, victim_name: &str) -> Vec<String> {
    match challenge.challenge_type {
        MACRO_CHALLENGE_MATH => vec![format!(
            "{victim_name}, answer me this: What is {} plus {}?",
            challenge.val1, challenge.val2
        )],
        MACRO_CHALLENGE_WORD => {
            vec![format!("{victim_name}, type the word: {}", challenge.word)]
        }
        MACRO_CHALLENGE_REVERSE => vec![format!(
            "{victim_name}, type this word backwards: {}",
            challenge.word
        )],
        MACRO_CHALLENGE_CHOICE => {
            let idx = challenge.val1.max(0) as usize;
            match MACRO_CHOICE_QUESTIONS.get(idx) {
                Some(q) => vec![
                    format!("{victim_name}: {}", q.question),
                    format!("  A) {}  B) {}", q.answers[0], q.answers[1]),
                    format!("  C) {}  D) {}", q.answers[2], q.answers[3]),
                ],
                None => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

/// C `atoi` used by `macro_check_answer`'s `MACRO_CHALLENGE_MATH` branch:
/// leading whitespace/sign, then leading digits, `0` if none - a small
/// local copy of the same shape as `world::clanclerk::parse_int_atoi`
/// (kept `pub(super)` there, not reachable from this crate-top-level
/// module without a visibility ripple through a private `mod clanclerk;`
/// declaration).
fn macro_atoi(text: &str) -> i32 {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let negative = matches!(bytes.get(i), Some(b'-'));
    if matches!(bytes.get(i), Some(b'-') | Some(b'+')) {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return 0;
    }
    let value: i64 = text[start..i].parse().unwrap_or(0);
    let value = if negative { -value } else { value };
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// C `macro_check_answer` (`base.c:587-608`): skips leading whitespace/
/// `"` first, matching C's own guard.
pub fn macro_check_answer(challenge: &MacroChallenge, raw_answer: &str) -> bool {
    let answer = raw_answer.trim_start_matches(|c: char| c.is_whitespace() || c == '"');
    match challenge.challenge_type {
        MACRO_CHALLENGE_MATH => macro_atoi(answer) == challenge.val1 + challenge.val2,
        MACRO_CHALLENGE_WORD | MACRO_CHALLENGE_REVERSE => {
            answer.eq_ignore_ascii_case(&challenge.expected_answer)
        }
        MACRO_CHALLENGE_CHOICE => {
            let choice = match answer.chars().next().map(|c| c.to_ascii_uppercase()) {
                Some('A') | Some('1') | Some('0') => Some(0),
                Some('B') | Some('2') => Some(1),
                Some('C') | Some('3') => Some(2),
                Some('D') | Some('4') => Some(3),
                _ => None,
            };
            choice == Some(challenge.choice_answer)
        }
        _ => false,
    }
}

/// C `macro_record_history` (`base.c:1244-1263`): writes into the
/// circular `history` buffer and bumps the lifetime `total_passed`/
/// `total_failed` counters.
pub fn macro_record_history(
    ppd: &mut MacroPpd,
    now: i64,
    challenge_type: i32,
    passed: bool,
    response_time: i32,
) {
    let idx = ppd.history_index as usize % crate::player::MACRO_HISTORY_SIZE;
    ppd.history[idx].timestamp = now;
    ppd.history[idx].challenge_type = challenge_type;
    ppd.history[idx].passed = passed;
    ppd.history[idx].response_time = response_time;

    ppd.history_index = (idx as i32 + 1) % crate::player::MACRO_HISTORY_SIZE as i32;
    ppd.history_count += 1;

    if passed {
        ppd.total_passed += 1;
    } else {
        ppd.total_failed += 1;
    }
}

/// C `macro_driver`'s post-correct-answer `nextcheck` scheduling
/// (`base.c:864-871`): base 10-40 minutes, plus up to 1/2/3 more hours as
/// `karma` crosses 25/50/75. Returns a delta (seconds) to add to
/// `realtime`; `seed` rolls are consumed in the exact same order C's own
/// `RANDOM(60*30)`, then conditionally `RANDOM(60*60)`/`RANDOM(60*60*2)`/
/// `RANDOM(60*60*3)`, does.
pub fn macro_next_check_delay(karma: i32, seed: &mut u32) -> i64 {
    use crate::world::legacy_random_below_from_seed as random_below;

    let mut delay = 60 * 10 + i64::from(random_below(seed, 60 * 30));
    if karma > 25 {
        delay += i64::from(random_below(seed, 60 * 60));
    }
    if karma > 50 {
        delay += i64::from(random_below(seed, 60 * 60 * 2));
    }
    if karma > 75 {
        delay += i64::from(random_below(seed, 60 * 60 * 3));
    }
    delay
}

/// C `macro_driver`'s correct-answer `MacroPpd` mutation (`base.c:854-
/// 873`): records history, raises karma (`(int)(karma*0.9)+10`, clamped
/// at 100 - no lower clamp, matching C exactly), resets
/// `challenge_failures`, lowers `suspicion` by 10 (floored at 0), and
/// reschedules `nextcheck`. The challenge-room return-teleport
/// (`dat->teleported_to_jail && ppd->in_challenge_room`) and the reward
/// roll (needs a fresh `RANDOM(100)` and the target's level - see
/// [`macro_roll_reward`]) are **not** included; both are real `World`/
/// `ZoneLoader` side effects for the future live-driver slice to apply
/// after calling this.
pub fn macro_apply_correct_answer(
    ppd: &mut MacroPpd,
    now: i64,
    response_time: i32,
    challenge_type: i32,
    seed: &mut u32,
) {
    macro_record_history(ppd, now, challenge_type, true, response_time);
    ppd.karma = (((ppd.karma as f64) * 0.9) as i32 + 10).min(100);
    ppd.challenge_failures = 0;
    ppd.suspicion = (ppd.suspicion - 10).max(0);
    ppd.nextcheck = now + macro_next_check_delay(ppd.karma, seed);
}

/// The outcome of [`macro_apply_failure`]: the two player-facing/log
/// messages and whether this failure crossed the "temporary logout"
/// threshold (C `ch[co].flags |= CF_KICKED`, a real `World`/`Character`
/// mutation the future live-driver slice applies on `kicked == true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroFailureUpdate {
    pub victim_message: String,
    pub log_message: &'static str,
    pub kicked: bool,
}

/// C `macro_handle_failure` (`base.c:728-773`): records the failed
/// history entry (`response_time = 0`, matching a timeout), increments
/// `challenge_failures`/raises `suspicion` by 20 (clamped at 100)
/// unconditionally, then branches on the *new* `challenge_failures`
/// count - 1st/2nd failures are gentle warnings (`nextcheck` rescheduled
/// 10-20/5-10 minutes out); the 3rd+ resets `challenge_failures` to `0`,
/// lowers `suspicion` by a further 30 (floored at 0), and reschedules 15
/// minutes out, returning `kicked: true` for the caller to apply
/// `CF_KICKED`.
pub fn macro_apply_failure(
    ppd: &mut MacroPpd,
    victim_name: &str,
    now: i64,
    challenge_type: i32,
    seed: &mut u32,
) -> MacroFailureUpdate {
    use crate::world::legacy_random_below_from_seed as random_below;

    macro_record_history(ppd, now, challenge_type, false, 0);
    ppd.challenge_failures += 1;
    ppd.suspicion = (ppd.suspicion + 20).min(100);

    let update = match ppd.challenge_failures {
        1 => {
            ppd.nextcheck = now + 600 + i64::from(random_below(seed, 600));
            MacroFailureUpdate {
                victim_message: format!(
                    "No answer? That's okay, but please respond next time, {victim_name}."
                ),
                log_message: "The Macro Daemon gave you a warning. Please respond to future \
                               challenges.",
                kicked: false,
            }
        }
        2 => {
            ppd.nextcheck = now + 300 + i64::from(random_below(seed, 300));
            MacroFailureUpdate {
                victim_message: format!(
                    "Still no response, {victim_name}? I'm getting concerned..."
                ),
                log_message: "Warning: You've failed to respond twice. One more failure will \
                               result in a timeout.",
                kicked: false,
            }
        }
        _ => {
            ppd.challenge_failures = 0;
            ppd.suspicion = (ppd.suspicion - 30).max(0);
            ppd.nextcheck = now + 900;
            MacroFailureUpdate {
                victim_message: format!("I'm sorry, {victim_name}, but you need to take a break."),
                log_message: "You've been temporarily logged out for not responding to \
                               challenges. Please take a short break and return when you can \
                               actively play.",
                kicked: true,
            }
        }
    };
    update
}

/// C `macro_give_reward`'s reward-type roll (`base.c:611-651`, minus the
/// `isxmas` special case - see [`macro_xmas_reward_message`]), classified
/// only - actually creating/giving the item needs a `ZoneLoader` (see
/// this module's doc comment). `reward_roll` is C's own `RANDOM(100)`
/// (`0..99`); `karma` mirrors C's `ppd && ppd->karma > N` guards (`None`
/// when no `MacroPpd` exists, matching C's `ppd` possibly being `NULL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroReward {
    /// `reward_roll < 30`: 30% chance. `exp` already includes the
    /// `karma > 50` 50% bonus.
    Experience { exp: u32 },
    /// `reward_roll < 55 && level >= 20`: 25% chance. Actual gold =
    /// `base + RANDOM(random_span)`, rolled by the caller (only it has
    /// the RNG seed at execution time).
    Gold { base: u32, random_span: u32 },
    /// `reward_roll < 75 && level >= 10`: 20% chance (`healing_potion1`).
    HealingPotion { fallback_exp: u32 },
    /// `reward_roll < 90 && level >= 30`: 15% chance (`combo_potion1`).
    ComboPotion { fallback_exp: u32 },
    /// `reward_roll < 100 && karma > 75`: 10% chance (`lollipop`) - no
    /// fallback reward at all if the item can't be created/given (C's
    /// `else if (in) { destroy_item(in); }` gives nothing).
    Lollipop,
    /// Every other case (the `reward_roll < 100` fallthrough when
    /// `karma <= 75`, or absent): the "Thank you" consolation exp.
    Consolation { exp: u32 },
}

pub fn macro_roll_reward(reward_roll: i32, level: u32, karma: Option<i32>) -> MacroReward {
    if reward_roll < 30 {
        let mut exp = level_value(level) / 20 + 1;
        if karma.is_some_and(|k| k > 50) {
            exp = exp * 3 / 2;
        }
        MacroReward::Experience { exp }
    } else if reward_roll < 55 && level >= 20 {
        MacroReward::Gold {
            base: level * 10,
            random_span: level * 5,
        }
    } else if reward_roll < 75 && level >= 10 {
        MacroReward::HealingPotion {
            fallback_exp: level_value(level) / 25 + 1,
        }
    } else if reward_roll < 90 && level >= 30 {
        MacroReward::ComboPotion {
            fallback_exp: level_value(level) / 20 + 1,
        }
    } else if reward_roll < 100 && karma.is_some_and(|k| k > 75) {
        MacroReward::Lollipop
    } else {
        MacroReward::Consolation {
            exp: level_value(level) / 30 + 1,
        }
    }
}

/// The `create_item` template a [`MacroReward`] needs, if any (`None` for
/// `Experience`/`Consolation`, which never touch an item).
pub fn macro_reward_item_template(reward: &MacroReward) -> Option<&'static str> {
    match reward {
        MacroReward::HealingPotion { .. } => Some("healing_potion1"),
        MacroReward::ComboPotion { .. } => Some("combo_potion1"),
        MacroReward::Lollipop => Some("lollipop"),
        MacroReward::Experience { .. }
        | MacroReward::Gold { .. }
        | MacroReward::Consolation { .. } => None,
    }
}

/// The `say(cn, ...)` line on a successful reward (`base.c:653-703`'s
/// first branch of each `if`). `gold` must be `Some` (the just-rolled
/// amount) exactly when `reward` is [`MacroReward::Gold`].
pub fn macro_reward_success_message(reward: &MacroReward, gold: Option<u32>) -> String {
    match reward {
        MacroReward::Experience { .. } => {
            "Excellent! Here's some experience for your trouble.".to_string()
        }
        MacroReward::Gold { .. } => {
            format!(
                "Well done! Here's {} gold for you.",
                gold.unwrap_or_default()
            )
        }
        MacroReward::HealingPotion { .. } => {
            "Good job! Take this potion, it may come in handy.".to_string()
        }
        MacroReward::ComboPotion { .. } => "Impressive! This combo potion is yours.".to_string(),
        MacroReward::Lollipop => "You've been so helpful! Here's something special.".to_string(),
        MacroReward::Consolation { .. } => "Thank you for your cooperation!".to_string(),
    }
}

/// The `(exp, message)` consolation said when an item-template reward
/// fails to create/give (`base.c:660-703`'s `else if (in) { destroy_item
/// (in); give_exp(...); say(...); }` branches). [`MacroReward::Lollipop`]
/// has no fallback at all (C's own `else if (in) { destroy_item(in); }`
/// gives nothing); non-item rewards never reach this path.
pub fn macro_reward_fallback(reward: &MacroReward) -> Option<(u32, &'static str)> {
    match *reward {
        MacroReward::HealingPotion { fallback_exp } => Some((fallback_exp, "Excellent work!")),
        MacroReward::ComboPotion { fallback_exp } => Some((fallback_exp, "Well answered!")),
        _ => None,
    }
}

/// C `macro_give_reward`'s `isxmas` special case (`base.c:640-648`):
/// always `xmaspop`, regardless of `reward_roll`, said only on success
/// (silent `destroy_item` on failure, nothing at all if `create_item`
/// itself fails).
pub fn macro_xmas_reward_message(victim_name: &str) -> String {
    format!("Merry Christmas, {victim_name}! Here's a little gift.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::MacroPpd;

    fn ppd_with(karma: i32, suspicion: i32, challenge_failures: i32) -> MacroPpd {
        MacroPpd {
            karma,
            suspicion,
            challenge_failures,
            ..MacroPpd::default()
        }
    }

    #[test]
    fn activity_gate_matches_c_five_minute_window() {
        let mut ppd = MacroPpd::default();
        ppd.last_exp_gain = 1_000;
        assert!(macro_is_player_active(&ppd, 1_000));
        assert!(macro_is_player_active(&ppd, 1_000 + 299));
        assert!(!macro_is_player_active(&ppd, 1_000 + 300));

        let mut ppd2 = MacroPpd::default();
        ppd2.last_combat = 5_000;
        assert!(macro_is_player_active(&ppd2, 5_299));
        assert!(!macro_is_player_active(&ppd2, 5_300));

        let mut ppd3 = MacroPpd::default();
        ppd3.last_gold_change = 9_000;
        assert!(macro_is_player_active(&ppd3, 9_100));
        assert!(!macro_is_player_active(&ppd3, 9_400));
    }

    #[test]
    fn default_ppd_never_looks_active_for_realistic_unix_time() {
        let ppd = MacroPpd::default();
        // C: `now - 0 < 300` only for `now` near the Unix epoch; any real
        // wall-clock `realtime` value fails every branch.
        assert!(!macro_is_player_active(&ppd, 1_700_000_000));
    }

    #[test]
    fn area_exclusions_match_c_section_and_rectangle_checks() {
        // Area 3: only section 20 is eligible; everything else excluded.
        // No section data mapped at (0, 0) in area 3 -> `None != Some(20)`.
        assert!(macro_is_area_excluded(3, 0, 0));
        // Area 22's hardcoded rectangle exclusion.
        assert!(macro_is_area_excluded(22, 80, 25));
        assert!(!macro_is_area_excluded(22, 10, 10));
        // An ordinary area/position combination with no section mapped
        // and outside any special rectangle is not excluded.
        assert!(!macro_is_area_excluded(1, 5, 5));
    }

    #[test]
    fn generate_challenge_easy_tier_is_math_with_c_ranges() {
        let mut seed = 42u32;
        let challenge = macro_generate_challenge(&mut seed, 0, 0);
        assert_eq!(challenge.challenge_type, MACRO_CHALLENGE_MATH);
        assert!((1..=50).contains(&challenge.val1));
        assert!((1..=20).contains(&challenge.val2));
        assert_eq!(
            challenge.expected_answer,
            (challenge.val1 + challenge.val2).to_string()
        );
    }

    #[test]
    fn generate_challenge_medium_tier_picks_word_or_math() {
        let mut seed = 7u32;
        for _ in 0..50 {
            let challenge = macro_generate_challenge(&mut seed, 35, 0);
            match challenge.challenge_type {
                MACRO_CHALLENGE_WORD => {
                    assert!(MACRO_CHALLENGE_WORDS.contains(&challenge.word.as_str()));
                    assert_eq!(challenge.expected_answer, challenge.word);
                }
                MACRO_CHALLENGE_MATH => {
                    assert!((10..=109).contains(&challenge.val1));
                    assert!((5..=54).contains(&challenge.val2));
                }
                other => panic!("unexpected medium-tier challenge type {other}"),
            }
        }
    }

    #[test]
    fn generate_challenge_hard_tier_picks_reverse_or_harder_math() {
        let mut seed = 99u32;
        for _ in 0..50 {
            let challenge = macro_generate_challenge(&mut seed, 65, 0);
            match challenge.challenge_type {
                MACRO_CHALLENGE_REVERSE => {
                    let expected: String = challenge.word.chars().rev().collect();
                    assert_eq!(challenge.expected_answer, expected);
                }
                MACRO_CHALLENGE_MATH => {
                    assert!((50..=249).contains(&challenge.val1));
                    assert!((20..=119).contains(&challenge.val2));
                }
                other => panic!("unexpected hard-tier challenge type {other}"),
            }
        }
    }

    #[test]
    fn prior_failure_bumps_difficulty_tier_by_one() {
        // suspicion 0 alone -> tier 0 (math, easy range); with a prior
        // failure it should behave like tier 1 (medium).
        let mut seed_easy = 1u32;
        let mut seed_bumped = 1u32;
        let easy = macro_generate_challenge(&mut seed_easy, 0, 0);
        let bumped = macro_generate_challenge(&mut seed_bumped, 0, 1);
        assert_eq!(easy.challenge_type, MACRO_CHALLENGE_MATH);
        assert!((1..=50).contains(&easy.val1));
        // Medium tier's math range starts at 10, easy tier's at 1 - a
        // bumped-tier math roll must fall in the medium range even
        // though both start from the same seed value.
        if bumped.challenge_type == MACRO_CHALLENGE_MATH {
            assert!(bumped.val1 >= 10);
        }
    }

    #[test]
    fn check_answer_math_parses_leading_int_and_ignores_quotes() {
        let challenge = MacroChallenge::math(20, 5);
        assert!(macro_check_answer(&challenge, "25"));
        assert!(macro_check_answer(&challenge, "\"25\""));
        assert!(macro_check_answer(&challenge, "  25 is my answer"));
        assert!(!macro_check_answer(&challenge, "24"));
        assert!(!macro_check_answer(&challenge, "not a number"));
    }

    #[test]
    fn check_answer_word_and_reverse_are_case_insensitive() {
        let word = MacroChallenge::word("GUARDIAN");
        assert!(macro_check_answer(&word, "guardian"));
        assert!(macro_check_answer(&word, "GUARDIAN"));
        assert!(!macro_check_answer(&word, "guardin"));

        let reverse = MacroChallenge::reverse("HEAL");
        assert_eq!(reverse.expected_answer, "LAEH");
        assert!(macro_check_answer(&reverse, "laeh"));
        assert!(!macro_check_answer(&reverse, "heal"));
    }

    #[test]
    fn check_answer_choice_accepts_letter_or_digit_forms() {
        let mut challenge = MacroChallenge::math(0, 0);
        challenge.challenge_type = MACRO_CHALLENGE_CHOICE;
        challenge.choice_answer = 2;
        assert!(macro_check_answer(&challenge, "C"));
        assert!(macro_check_answer(&challenge, "c"));
        assert!(macro_check_answer(&challenge, "3"));
        assert!(!macro_check_answer(&challenge, "A"));
        assert!(!macro_check_answer(&challenge, "z"));
    }

    #[test]
    fn ask_challenge_lines_match_c_wording_per_type() {
        let math = MacroChallenge::math(47, 8);
        assert_eq!(
            macro_ask_challenge_lines(&math, "Bob"),
            vec!["Bob, answer me this: What is 47 plus 8?".to_string()]
        );

        let word = MacroChallenge::word("GUARDIAN");
        assert_eq!(
            macro_ask_challenge_lines(&word, "Bob"),
            vec!["Bob, type the word: GUARDIAN".to_string()]
        );

        let reverse = MacroChallenge::reverse("HEAL");
        assert_eq!(
            macro_ask_challenge_lines(&reverse, "Bob"),
            vec!["Bob, type this word backwards: HEAL".to_string()]
        );

        let mut choice = MacroChallenge::math(0, 0);
        choice.challenge_type = MACRO_CHALLENGE_CHOICE;
        choice.val1 = 0;
        let lines = macro_ask_challenge_lines(&choice, "Bob");
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("Bob: Which skill lets you heal others?"));
    }

    #[test]
    fn record_history_wraps_circular_buffer_and_tallies_totals() {
        let mut ppd = MacroPpd::default();
        for i in 0..(crate::player::MACRO_HISTORY_SIZE as i32 + 3) {
            macro_record_history(
                &mut ppd,
                1_000 + i as i64,
                MACRO_CHALLENGE_MATH,
                i % 2 == 0,
                5,
            );
        }
        assert_eq!(
            ppd.history_count,
            crate::player::MACRO_HISTORY_SIZE as i32 + 3
        );
        // Wrapped back around to index 3 after writing HISTORY_SIZE+3
        // entries into a HISTORY_SIZE-slot ring buffer.
        assert_eq!(ppd.history_index, 3);
        assert_eq!(ppd.total_passed + ppd.total_failed, ppd.history_count);
    }

    #[test]
    fn correct_answer_raises_karma_resets_failures_and_lowers_suspicion() {
        let mut ppd = ppd_with(40, 60, 2);
        let mut seed = 3u32;
        macro_apply_correct_answer(&mut ppd, 1_000, 12, MACRO_CHALLENGE_MATH, &mut seed);
        assert_eq!(ppd.karma, (40.0_f64 * 0.9) as i32 + 10);
        assert_eq!(ppd.challenge_failures, 0);
        assert_eq!(ppd.suspicion, 50);
        assert!(ppd.nextcheck > 1_000);
        assert_eq!(ppd.total_passed, 1);
    }

    #[test]
    fn correct_answer_karma_clamps_at_one_hundred_with_no_lower_clamp() {
        let mut ppd = ppd_with(100, 0, 0);
        let mut seed = 3u32;
        macro_apply_correct_answer(&mut ppd, 1_000, 12, MACRO_CHALLENGE_MATH, &mut seed);
        assert_eq!(ppd.karma, 100);

        let mut ppd_low = ppd_with(0, 5, 0);
        macro_apply_correct_answer(&mut ppd_low, 1_000, 12, MACRO_CHALLENGE_MATH, &mut seed);
        assert_eq!(ppd_low.karma, 10);
        // suspicion floored at 0, not negative.
        assert_eq!(ppd_low.suspicion, 0);
    }

    #[test]
    fn first_failure_is_a_gentle_warning_with_ten_to_twenty_minute_reschedule() {
        let mut ppd = ppd_with(50, 10, 0);
        let mut seed = 11u32;
        let update = macro_apply_failure(&mut ppd, "Bob", 1_000, MACRO_CHALLENGE_MATH, &mut seed);
        assert!(!update.kicked);
        assert_eq!(ppd.challenge_failures, 1);
        assert_eq!(ppd.suspicion, 30);
        assert!(update.victim_message.contains("Bob"));
        assert!((1_000 + 600..=1_000 + 1199).contains(&ppd.nextcheck));
        assert_eq!(ppd.total_failed, 1);
    }

    #[test]
    fn second_failure_is_a_sterner_warning() {
        let mut ppd = ppd_with(50, 10, 1);
        let mut seed = 11u32;
        let update = macro_apply_failure(&mut ppd, "Bob", 2_000, MACRO_CHALLENGE_MATH, &mut seed);
        assert!(!update.kicked);
        assert_eq!(ppd.challenge_failures, 2);
        assert_eq!(ppd.suspicion, 30);
        assert!(update.victim_message.contains("concerned"));
        assert!((2_000 + 300..=2_000 + 599).contains(&ppd.nextcheck));
    }

    #[test]
    fn third_failure_kicks_resets_failures_and_reduces_suspicion() {
        let mut ppd = ppd_with(50, 80, 2);
        let mut seed = 11u32;
        let update = macro_apply_failure(&mut ppd, "Bob", 3_000, MACRO_CHALLENGE_MATH, &mut seed);
        assert!(update.kicked);
        assert_eq!(ppd.challenge_failures, 0);
        // suspicion: 80 + 20 = 100 (clamped), then -30 = 70.
        assert_eq!(ppd.suspicion, 70);
        assert_eq!(ppd.nextcheck, 3_000 + 900);
        assert!(update.victim_message.contains("take a break"));
    }

    #[test]
    fn suspicion_clamps_at_one_hundred_before_third_failure_subtracts() {
        let mut ppd = ppd_with(50, 95, 2);
        let mut seed = 11u32;
        macro_apply_failure(&mut ppd, "Bob", 3_000, MACRO_CHALLENGE_MATH, &mut seed);
        // 95 + 20 clamps to 100, then -30 = 70 (not 85).
        assert_eq!(ppd.suspicion, 70);
    }

    #[test]
    fn reward_roll_matches_c_thresholds_and_level_gates() {
        assert!(matches!(
            macro_roll_reward(0, 5, None),
            MacroReward::Experience { .. }
        ));
        assert!(matches!(
            macro_roll_reward(29, 100, None),
            MacroReward::Experience { .. }
        ));
        // Gold needs level >= 20; below that, falls through toward the
        // next eligible branch (here, no potion eligibility either at
        // level 5, so it lands on Consolation).
        assert!(matches!(
            macro_roll_reward(40, 5, None),
            MacroReward::Consolation { .. }
        ));
        assert!(matches!(
            macro_roll_reward(40, 20, None),
            MacroReward::Gold { .. }
        ));
        assert!(matches!(
            macro_roll_reward(60, 10, None),
            MacroReward::HealingPotion { .. }
        ));
        assert!(matches!(
            macro_roll_reward(80, 30, None),
            MacroReward::ComboPotion { .. }
        ));
        assert!(matches!(
            macro_roll_reward(95, 100, Some(80)),
            MacroReward::Lollipop
        ));
        // karma <= 75 at reward_roll 95 falls through to Consolation.
        assert!(matches!(
            macro_roll_reward(95, 100, Some(50)),
            MacroReward::Consolation { .. }
        ));
        assert!(matches!(
            macro_roll_reward(95, 100, None),
            MacroReward::Consolation { .. }
        ));
    }

    #[test]
    fn reward_experience_gets_fifty_percent_karma_bonus() {
        let level = 40;
        let base = level_value(level) / 20 + 1;
        match macro_roll_reward(0, level, Some(51)) {
            MacroReward::Experience { exp } => assert_eq!(exp, base * 3 / 2),
            other => panic!("expected Experience, got {other:?}"),
        }
        match macro_roll_reward(0, level, Some(50)) {
            MacroReward::Experience { exp } => assert_eq!(exp, base),
            other => panic!("expected Experience, got {other:?}"),
        }
    }

    #[test]
    fn reward_item_templates_and_messages_match_c_text() {
        let healing = MacroReward::HealingPotion { fallback_exp: 7 };
        assert_eq!(
            macro_reward_item_template(&healing),
            Some("healing_potion1")
        );
        assert_eq!(
            macro_reward_success_message(&healing, None),
            "Good job! Take this potion, it may come in handy."
        );
        assert_eq!(
            macro_reward_fallback(&healing),
            Some((7, "Excellent work!"))
        );

        let combo = MacroReward::ComboPotion { fallback_exp: 9 };
        assert_eq!(macro_reward_item_template(&combo), Some("combo_potion1"));
        assert_eq!(macro_reward_fallback(&combo), Some((9, "Well answered!")));

        assert_eq!(
            macro_reward_item_template(&MacroReward::Lollipop),
            Some("lollipop")
        );
        assert_eq!(macro_reward_fallback(&MacroReward::Lollipop), None);

        let gold = MacroReward::Gold {
            base: 10,
            random_span: 5,
        };
        assert_eq!(macro_reward_item_template(&gold), None);
        assert_eq!(
            macro_reward_success_message(&gold, Some(12)),
            "Well done! Here's 12 gold for you."
        );

        assert_eq!(
            macro_reward_success_message(&MacroReward::Consolation { exp: 3 }, None),
            "Thank you for your cooperation!"
        );
    }

    #[test]
    fn xmas_reward_message_matches_c_text() {
        assert_eq!(
            macro_xmas_reward_message("Bob"),
            "Merry Christmas, Bob! Here's a little gift."
        );
    }
}
