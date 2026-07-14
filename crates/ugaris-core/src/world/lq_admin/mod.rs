//! `CDR_LQPARSER` admin command table (`special_driver`, `src/area/20/
//! lq.c:2505-2742`): the area-gated (`areaID == 20 || areaID == 35`)
//! `#`/`/`-prefixed text-command interceptor every `CF_GOD`/`CF_LQMASTER`
//! character in the Live Quest area can use to author quest content.
//! `special_driver` is dispatched purely by `command()`'s own area gate
//! (`system/command.c:5855-5859`), independent of `CharacterDriverKind`/
//! spawned-character iteration - `CDR_LQPARSER` is never assigned to any
//! spawned character (see [`crate::world::npc::area20::lqnpc`]'s module
//! doc comment for the sibling `CDR_LQNPC` driver this table authors).
//!
//! First slice: the NPC-template CRUD family (`#npc`, `#npcname`,
//! `#npcgold`, `#npcsprite`, `#npcpos`, `#npcdescription`,
//! `#npcgreeting`, `#npcreply`, `#npclist`, `#npcdelete`,
//! `#npcwantitem`, `#npcitem`, `#npcshow`, `#npckillmark`,
//! `#npchurtmark`, `#npcrewarditem`, `#npcmodlevel`, `#npcrespawn` - 18
//! of the ~45-entry table). All mutate only [`crate::world::LqNpcState`]
//! template rows already modeled by `world::lq`; none of them need
//! `ZoneLoader`/`PlayerRuntime`, so the entire slice is pure `World`
//! logic, unlike most of the rest of this port.
//!
//! Second slice: `#doorlist`/`#doorlock` (`lq.c:2443-2503`), operating on
//! [`crate::world::LqDoorState`] (`world::lq`'s `discover_lq_doors_once`,
//! already ported for the `LqTicker` periodic scan). Both commands call
//! `discover_lq_doors_once` themselves first, matching C's own lazy
//! `lq_ticker`-driven `init_done` populate-on-first-use behavior in case
//! no `LqTicker` tick has run yet.
//!
//! Third slice: the live-instance-control family (`#nspawn`/`#nremove`/
//! `#nsay`/`#nimmortal`/`#nemote`/`#nattack`).
//!
//! Fourth slice: `#thrall`/`#killthrall` (`lq.c:427-503`) - the on-the-fly,
//! template-detached NPC spawn/despawn pair, `spawn_npc`'s `isthrall`
//! branch. Like `#nspawn`, `#thrall` needs a fresh character
//! (`World::try_dispatch_lq_thrall`, dispatched ahead of
//! `apply_lq_admin_command` the same way); `#killthrall` needs none (pure
//! `World::lq_admin_cmd_killthrall`, matching every live `CDR_LQNPC`
//! character's `thrallname` directly - a thrall has no `world::LqNpcState`
//! template row to look up by).
//!
//! Fifth slice: `#usurp`/`#follow`/`#stop`/`#exit`, `#wimp`, the
//! possessed-NPC "me"/"emote" relay sub-command, the possessed-NPC plain-
//! speech relay, and the per-tick `domirror` movement mirroring it drives
//! - see [`crate::world::lq_usurp`] (a separate file/module: this one was
//!   already near the ~2,000-line hard cap).
//!
//! Sixth slice: the non-file-I/O half of the quest-lifecycle family -
//! `#questlevel`/`#questreward`/`#questshow`/`#questentrance`/
//! `#queststart`/`#questreset` - operating on the new [`LqData`] (`struct
//! lq_data`, previously not modeled at all). All six are pure `World`
//! logic; `#questreset` additionally reuses `teleport_char_driver`/
//! `destroy_item`/`remove_character` (already ported) plus a new
//! `World::lq_reset_drop_body_item` (the `remove_item_map`+`drop_item`
//! sequence for a body *already* on the map, unlike the fresh-body
//! [`World::drop_body_item`] used by `die_char`). `#npcshow`'s hurt/
//! killmark exp-preview line now reads the real `lq_data.reward[]`/
//! `reward_desc[]` table instead of the previously-always-empty
//! placeholder.
//!
//! Seventh slice: `#questend`/`#xinfo` - see `world::lq_quest_admin`
//! (split into its own file: this one was already over the ~2,000-line
//! hard cap).
//!
//! Eighth (final) slice: `#questsave`/`#questdelete`/`#questload` - see
//! `world::lq_quest_file` (also its own file, for the same reason). This
//! closes every subcommand in the `CDR_LQPARSER` table.
//!
//! Only the `c9`/`mirror` possessed-NPC relay sub-command remains
//! deliberately unported (`lq_usurp`'s own doc comment) - it needs
//! `src/system/chat/chat.c`'s `server_chat`, permanently deferred
//! cross-server chat transport per `AGENTS.md`'s "Not Applicable /
//! Deferred" list.
//!
//! Two C existence checks are permanently deferred rather than ported:
//! `cmd_npc`'s `lookup_char("lq_"+basename)` and `get_lq_item`'s
//! `lookup_item("lq_"+basename)` (used by `#npcitem`/`#npcrewarditem`) -
//! `World` has no character/item-template registry (only
//! `ugaris-server`'s `ZoneLoader` does). A bad basename is silently
//! accepted here and will simply no-op once the (still-unported)
//! `#nspawn` reaches `ZoneLoader::instantiate_character_template`,
//! matching the existing `spawns.rs::spawn_lq_npc_character` precedent
//! for the *scheduled*-respawn path.

use crate::character_driver::CDR_LQNPC;

use super::*;

mod commands;
mod dispatch;
mod instances;
mod npc_crud;
mod quests;

pub use dispatch::{LqNspawnDispatch, LqThrallDispatch};

/// C `cmdcmp(ptr, cmd, minlen)` (`system/command.c:217-234`): `word` is a
/// case-insensitive prefix of `full`, at least `minlen` characters long.
pub(super) fn cmd_word_matches(word: &str, full: &str, minlen: usize) -> bool {
    let lower = word.to_ascii_lowercase();
    lower.len() >= minlen && full.starts_with(&lower)
}

/// C `atoi` (used throughout `lq.c` on already-tokenized single-word
/// strings, e.g. `n = atoi(nick)`): leading-whitespace-then-optional-
/// sign-then-digits, `0` if there are no leading digits at all. `pub(super)`
/// (not just `pub(self)`) since `world::strategy_special` reuses it
/// verbatim for the same C `atoi`-on-tokenized-input idiom in
/// `special_driver` (`src/area/23_24/strategy.c`).
pub(super) fn legacy_atoi(input: &str) -> i64 {
    let trimmed = input.trim_start();
    let mut chars = trimmed.chars().peekable();
    let sign = match chars.peek() {
        Some('-') => {
            chars.next();
            -1
        }
        Some('+') => {
            chars.next();
            1
        }
        _ => 1,
    };
    let mut value: i64 = 0;
    let mut any_digit = false;
    for ch in chars {
        match ch.to_digit(10) {
            Some(digit) => {
                any_digit = true;
                value = value * 10 + i64::from(digit);
            }
            None => break,
        }
    }
    if any_digit {
        value * sign
    } else {
        0
    }
}

/// C `get_str`/`get_int`/`get_chr`/`check_anything` (`lq.c:238-317`): the
/// tiny space/quote-aware argument tokenizer shared by every `cmd_*`
/// handler in `special_driver`'s command table.
pub(super) struct ArgReader<'a> {
    rest: &'a str,
}

impl<'a> ArgReader<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        ArgReader { rest: input }
    }

    fn skip_ws(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// C `get_str`.
    pub(super) fn take_str(&mut self) -> Option<String> {
        self.skip_ws();
        if self.rest.is_empty() {
            return None;
        }
        let quoted = matches!(self.rest.chars().next(), Some('"') | Some('\''));
        let mut consumed = 0usize;
        if quoted {
            consumed += self.rest.chars().next().map(char::len_utf8).unwrap_or(0);
        }
        let mut out = String::new();
        for ch in self.rest[consumed..].chars() {
            if quoted && ch == '"' {
                break;
            }
            if !quoted && ch.is_whitespace() {
                break;
            }
            out.push(ch);
            consumed += ch.len_utf8();
        }
        // C also eats any remaining non-whitespace after the token
        // itself (`lq.c:285-287`) - only relevant for a truncated/
        // unterminated quoted token butchered by the length limit.
        let tail_start = self.rest[consumed..]
            .find(char::is_whitespace)
            .map(|offset| consumed + offset)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[tail_start..];
        Some(out)
    }

    /// C `get_int`.
    fn take_int(&mut self) -> Option<i64> {
        self.skip_ws();
        let negative = self.rest.starts_with('-');
        let digits_start = usize::from(negative);
        let has_digit = self.rest[digits_start..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit());
        if !has_digit {
            return None;
        }
        let mut end = digits_start;
        for ch in self.rest[digits_start..].chars() {
            if ch.is_ascii_digit() {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        let token = &self.rest[..end];
        let value = token.parse::<i64>().unwrap_or(0);
        self.rest = &self.rest[end..];
        Some(value)
    }

    /// C `get_chr`.
    fn take_chr(&mut self) -> Option<char> {
        self.skip_ws();
        let first = self.rest.chars().next()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        let tail_start = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[tail_start..];
        Some(first)
    }

    /// C `check_anything`.
    pub(super) fn has_trailing_garbage(&self) -> bool {
        !self.rest.trim_start().is_empty()
    }

    /// The raw, not-yet-tokenized remainder (C's own `ptr` after however
    /// many `get_str`/`cmdcmp` calls already advanced it) - used by
    /// callers that hand the rest of the line to another function
    /// verbatim instead of tokenizing it further (e.g. `cmd_usurp`'s
    /// name-only argument doubling as `emote`/`c9`'s free-text payload
    /// in `world::lq_usurp`).
    pub(super) fn remaining(&self) -> &'a str {
        self.rest
    }
}
