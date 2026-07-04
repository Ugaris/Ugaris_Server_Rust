//! `CDR_CLANCLERK` clan administration/treasury NPC.
//!
//! Ports `src/area/30/clanmaster.c`'s `clanclerk_driver`: the `help` text
//! command, `deposit`/`withdraw` treasury commands, the leader-only
//! `set bonus`/`relation`/`rank name`/`website`/`message`/`raiding on`/
//! `raiding off`/`raiding god on`/`raiding god off` commands, and the
//! `NT_GIVE` Clan Jewel handoff (`add_jewel`). The generic small-talk qa
//! table hit (`analyse_text_driver`'s `case 2` - "Our clan has %d
//! jewels.") is also ported, reusing the same
//! [`crate::character_driver::CLANMASTER_QA`] table `clanmaster.c` itself
//! shares between both drivers.
//!
//! Deliberately out of scope for this slice (documented here, not
//! silently dropped - see the "Clan system" task in `PORTING_TODO.md`):
//! - `add potions`/`NT_GIVE`'s `IDR_FLASK` branch (`add_simple_potion`/
//!   `add_alc_potion`) - the alchemy-potion economy these call into has
//!   no Rust port at all.
//! - `buy`/`use` (the dungeon-guard purchase/configuration commands) -
//!   `buy` is C dead code anyway (`say("Buying has been disabled...");
//!   continue;` unconditionally, matching the module's own drop-through
//!   here), but `use` needs `get_clan_dungeon_cost`/`set_clan_dungeon_use`
//!   and the training-points budget, none of which exist without the
//!   dungeon/raid system.
//! - The `doraid` auto-enable inside `ClanRelations::update` stays
//!   unported (see that module's doc comment), so
//!   [`crate::clan::ClanRegistry::get_clan_raid`] only ever becomes
//!   `true` via the `raiding god on` GM override here - matching this
//!   codebase's existing "pure logic first, wiring later" precedent.
//!
//! Like `world/clanmaster.rs`, clan-log persistence needs `ServerRuntime`/
//! DB handles `World` doesn't have, so it's queued as [`ClanclerkEvent`]
//! and applied in `ugaris-server`'s `world_events.rs::apply_clanclerk_events`.
//! C's own `dlog(co, 0, "deposited %dG to clan %d.", ...)` (a
//! server-audit log, not a clan-log entry) has no Rust equivalent and is
//! intentionally skipped, matching every other bare `dlog`/`xlog` call
//! site already skipped elsewhere in this codebase.
//!
//! Deviations from C (documented here, not silent):
//! - `set_clan_website`/`set_clan_message`'s trailing-character-strip
//!   quirk (`website[strlen(website)-1] = 0`) genuinely *does* apply at
//!   this call site (this driver is the only caller of either function in
//!   the entire C tree - not a hypothetical future `/clan` command as a
//!   stale note on [`crate::clan::ClanRegistry::set_website`] suggested),
//!   so it is ported here, at the driver layer, rather than inside the
//!   pure `ClanRegistry` setter: the raw command text (not a
//!   command-line-parser-appended delimiter) loses its own last
//!   character. Guarded against the empty-string case, which would be a
//!   buffer underflow in C (`website[-1]`) - Rust leaves an empty string
//!   as empty instead of guessing at undefined behavior.
//! - `secure_move_driver`'s "return to rest position, then face
//!   `DX_RIGHTDOWN`" is ported via the same `setup_walk_toward`/`turn`
//!   fallback `world/bank.rs`/`world/clanmaster.rs` already established.
use super::*;
use crate::character_driver::{ClanclerkDriverData, CLANMASTER_QA};
use crate::clan::{ClanMoneyChange, ClanRaidError, ClanRelation};

/// C `DX_RIGHTDOWN` (`common/direction.h:19`): the clerk's fixed resting
/// facing, unlike `clanmaster_driver`'s per-instance `dat->dir`.
const CLANCLERK_REST_DIRECTION: u8 = 2;

/// C `clanclerk_driver`'s `analyse_text_driver`/qa-table distance and
/// visibility gate (`clanmaster.c:187,192`), shared with
/// `clanmaster_driver`.
const CLANCLERK_QA_DISTANCE: i32 = 12;

/// A `clanclerk_driver` outcome that needs `ServerRuntime`/DB handles
/// (clan-log persistence) to finish - see the module doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClanclerkEvent {
    /// C `clan_money_change`'s clan-log branch (`clan.c:1243-1249`), hit
    /// by both `deposit` and `withdraw`.
    MoneyChanged {
        clan_nr: u16,
        actor_id: CharacterId,
        change: ClanMoneyChange,
    },
    /// C `set_clan_rankname`'s `add_clanlog` (`clan.c:875`, prio 33):
    /// `"%s set rank name %d to %s"`.
    RankNameSet {
        clan_nr: u16,
        actor_id: CharacterId,
        rank: usize,
        name: String,
    },
    /// C `set_clan_website`'s `add_clanlog` (`clan.c:590`, prio 35):
    /// `"%s set website %s"`.
    WebsiteSet {
        clan_nr: u16,
        actor_id: CharacterId,
        site: String,
    },
    /// C `set_clan_message`'s `add_clanlog` (`clan.c:601`, prio 35):
    /// `"%s set message %s"`.
    MessageSet {
        clan_nr: u16,
        actor_id: CharacterId,
        message: String,
    },
    /// C `add_jewel`'s `add_clanlog` (`clan.c:495`, prio 1):
    /// `"%s added a jewel"`.
    JewelAdded { clan_nr: u16, actor_id: CharacterId },
    /// C `set_clan_raid`'s `add_clanlog` (`clan.c:550,557`, prio 1):
    /// `"%s set raiding to ON"`/`"%s canceled raiding"`.
    RaidToggled {
        clan_nr: u16,
        actor_id: CharacterId,
        enabled: bool,
    },
    /// C `set_clan_raid_god`'s `add_clanlog` (`clan.c:568,575`, prio 1):
    /// same two message shapes as [`ClanclerkEvent::RaidToggled`], kept
    /// separate only so a future caller can distinguish a GM override
    /// from the member-facing command if it ever needs to.
    RaidGodToggled {
        clan_nr: u16,
        actor_id: CharacterId,
        enabled: bool,
    },
}

/// C's hand-rolled `while (*ptr && !isdigit(*ptr) [&& *ptr != '-']) ptr++;`
/// skip-to-next-token scan, shared shape between `"set bonus"` (allows a
/// leading `-` to stay attached to the number it skips to) and
/// `"relation"`/`"rank name"` (no `-` handling, so a `-` is skipped over
/// like any other non-digit and negative numbers are never produced).
fn skip_to_token(bytes: &[u8], mut i: usize, allow_sign: bool) -> usize {
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() || (allow_sign && b == b'-') {
            break;
        }
        i += 1;
    }
    i
}

fn skip_digits(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

/// C `atoi` on an already-isolated digit-only (optionally sign-prefixed)
/// slice.
fn parse_int(text: &str) -> i32 {
    text.parse::<i32>().unwrap_or(0)
}

/// Ports the two-number extraction shared by `"set bonus <a> <b>"` and
/// `"relation <a> <b>"` (`clanmaster.c:906-926,963-976`): skip to the
/// first number (honoring a leading `-` only when `allow_sign`), parse
/// it, skip its digits, skip to the second number the same way, parse it.
fn parse_two_ints(text: &str, allow_sign: bool) -> (i32, i32) {
    let bytes = text.as_bytes();
    let start1 = skip_to_token(bytes, 0, allow_sign);
    let mut i = start1;
    if allow_sign && bytes.get(i) == Some(&b'-') {
        i += 1;
    }
    i = skip_digits(bytes, i);
    let first = parse_int(&text[start1..i]);
    let start2 = skip_to_token(bytes, i, allow_sign);
    let mut j = start2;
    if allow_sign && bytes.get(j) == Some(&b'-') {
        j += 1;
    }
    j = skip_digits(bytes, j);
    let second = parse_int(&text[start2..j]);
    (first, second)
}

/// C `"rank name"` handler's number-then-quoted-name extraction
/// (`clanmaster.c:1024-1042`): skip to the rank digit(s), parse, skip
/// whitespace, then copy up to 39 bytes stopping at the first `"` or end.
fn parse_rank_name(text: &str) -> (i32, String) {
    let bytes = text.as_bytes();
    let digit_start = skip_to_token(bytes, 0, false);
    let digit_end = skip_digits(bytes, digit_start);
    let rank = parse_int(&text[digit_start..digit_end]);
    let mut i = digit_end;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut name = String::new();
    for &b in &bytes[i..] {
        if name.len() >= 39 || b == b'"' {
            break;
        }
        name.push(b as char);
    }
    (rank, name)
}

/// C's `website[strlen(website)-1] = 0` trailing-strip quirk (see the
/// module doc comment for why this is a real, reachable behavior at this
/// call site). A no-op on an empty string (C would underflow the buffer
/// index; there is nothing sane to strip from nothing).
fn strip_trailing_char(text: &str) -> &str {
    if text.is_empty() {
        text
    } else {
        &text[..text.len() - 1]
    }
}

impl World {
    pub fn drain_pending_clanclerk_events(&mut self) -> Vec<ClanclerkEvent> {
        std::mem::take(&mut self.pending_clanclerk_events)
    }

    /// Clan clerk NPC tick: process messages, walk/turn back to post.
    /// Ports the per-tick body of C `clanclerk_driver`. `now_unix` mirrors
    /// C's `realtime` seconds, used by the `relation`/`raiding on` command
    /// handlers (`set_clan_relation`'s `want_date`, `set_clan_raid`'s
    /// `raid_on_start`), matching `process_clanmaster_actions`'s own
    /// `now_unix` parameter.
    pub fn process_clanclerk_actions(&mut self, area_id: u16, now_unix: i64) {
        let clanclerk_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == crate::character_driver::CDR_CLANCLERK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for clanclerk_id in clanclerk_ids {
            self.process_clanclerk_messages(clanclerk_id, now_unix);
            self.clanclerk_tick_action(clanclerk_id, area_id);
        }
    }

    fn process_clanclerk_messages(&mut self, clanclerk_id: CharacterId, now_unix: i64) {
        let Some(clanclerk_name) = self.characters.get(&clanclerk_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Clanclerk(data)) = self
            .characters
            .get(&clanclerk_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&clanclerk_id)
            .map(|clanclerk| std::mem::take(&mut clanclerk.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_TEXT => self.clanclerk_handle_text_message(
                    clanclerk_id,
                    &clanclerk_name,
                    &data,
                    message,
                    now_unix,
                ),
                NT_GIVE => self.clanclerk_handle_give_message(clanclerk_id, &data, message),
                _ => {}
            }
        }
    }

    /// C `clanclerk_driver`'s `NT_TEXT` branch (`clanmaster.c:685-1211`).
    fn clanclerk_handle_text_message(
        &mut self,
        clanclerk_id: CharacterId,
        clanclerk_name: &str,
        data: &ClanclerkDriverData,
        message: &CharacterDriverMessage,
        now_unix: i64,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        if speaker_id == clanclerk_id {
            return;
        }
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C's `analyse_text_driver`'s `case 2` ("jewels"): unlike
        // `clanmaster_qa_reply`, this is the one qa answer_code this
        // driver actually reacts to.
        if let Some(reply) =
            self.clanclerk_qa_jewels_reply(clanclerk_id, clanclerk_name, speaker_id, text)
        {
            self.npc_say(clanclerk_id, &reply);
        }

        if !self
            .characters
            .get(&speaker_id)
            .is_some_and(|speaker| speaker.flags.contains(CharacterFlags::PLAYER))
        {
            return;
        }

        let lower = text.to_ascii_lowercase();

        // C: help short-circuits with `continue` unconditionally.
        if lower.contains("help") {
            self.clanclerk_handle_help(clanclerk_id, speaker_id, data.clan);
            return;
        }

        // C: `deposit` works for anyone nearby, member or not.
        if let Some(pos) = lower.find("deposit") {
            self.clanclerk_handle_deposit(clanclerk_id, speaker_id, data.clan, &text[pos + 7..]);
        }

        // C: `if (get_char_clan(co) != dat->clan) { continue; }` -
        // members only past this point.
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        if self.clan_registry.get_char_clan(speaker) != Some(data.clan) {
            return;
        }
        // C: `if (ch[co].clan_rank < 3) { continue; }` - treasurer+.
        if speaker.clan_rank < 3 {
            return;
        }

        if let Some(pos) = lower.find("withdraw") {
            self.clanclerk_handle_withdraw(clanclerk_id, speaker_id, data.clan, &text[pos + 8..]);
        }

        if lower.contains("buy") {
            // C: unconditionally disabled, the rest of that branch is
            // dead code (`say(...); continue;` before any dungeon logic
            // runs).
            self.npc_say(
                clanclerk_id,
                "Buying has been disabled, you have infinite stock.",
            );
            return;
        }

        // C: `if (ch[co].clan_rank < 4) { continue; }` - leader only.
        let Some(speaker) = self.characters.get(&speaker_id) else {
            return;
        };
        if speaker.clan_rank < 4 {
            return;
        }
        let speaker_is_god = speaker.flags.contains(CharacterFlags::GOD);

        if let Some(pos) = lower.find("set bonus") {
            self.clanclerk_handle_set_bonus(clanclerk_id, speaker_id, data.clan, &text[pos + 9..]);
        }
        if let Some(pos) = lower.find("relation") {
            self.clanclerk_handle_relation(
                clanclerk_id,
                speaker_id,
                data.clan,
                &text[pos + 8..],
                now_unix,
            );
        }
        if let Some(pos) = lower.find("rank name") {
            self.clanclerk_handle_rank_name(clanclerk_id, speaker_id, data.clan, &text[pos + 9..]);
        }
        if let Some(pos) = lower.find("website") {
            self.clanclerk_handle_website(
                clanclerk_id,
                speaker_id,
                data.clan,
                text[pos + 7..].trim_start(),
            );
        }
        if let Some(pos) = lower.find("message") {
            self.clanclerk_handle_message(
                clanclerk_id,
                speaker_id,
                data.clan,
                text[pos + 7..].trim_start(),
            );
        }
        if lower.contains("raiding on") {
            self.clanclerk_handle_raiding(clanclerk_id, speaker_id, data.clan, true, now_unix);
        }
        if lower.contains("raiding off") {
            self.clanclerk_handle_raiding(clanclerk_id, speaker_id, data.clan, false, now_unix);
        }
        if speaker_is_god && lower.contains("raiding god on") {
            self.clanclerk_handle_raiding_god(clanclerk_id, speaker_id, data.clan, true);
        }
        if speaker_is_god && lower.contains("raiding god off") {
            self.clanclerk_handle_raiding_god(clanclerk_id, speaker_id, data.clan, false);
        }
    }

    fn clanclerk_qa_jewels_reply(
        &self,
        clanclerk_id: CharacterId,
        clanclerk_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) -> Option<String> {
        let clanclerk = self.characters.get(&clanclerk_id)?;
        let speaker = self.characters.get(&speaker_id)?;
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if char_dist(clanclerk, speaker) > CLANCLERK_QA_DISTANCE {
            return None;
        }
        if !char_see_char(clanclerk, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(
            text,
            clanclerk_name,
            &speaker.name,
            CLANMASTER_QA,
        ) {
            // C: `case 2: say(cn, "Our clan has %d jewels.",
            // cnt_jewels(dat->clan)); break;`.
            crate::character_driver::TextAnalysisOutcome::Matched(2) => {
                let clan = match self
                    .characters
                    .get(&clanclerk_id)
                    .and_then(|c| c.driver_state.clone())
                {
                    Some(CharacterDriverState::Clanclerk(data)) => data.clan,
                    _ => return None,
                };
                Some(format!(
                    "Our clan has {} jewels.",
                    self.clan_registry.jewel_count(clan)
                ))
            }
            _ => None,
        }
    }

    /// C `clanclerk_driver`'s `help` block (`clanmaster.c:696-727`), color
    /// macros stripped (matching this codebase's house style for
    /// multi-line NPC/command output).
    fn clanclerk_handle_help(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        self.npc_say(
            clanclerk_id,
            &format!("Greetings, {speaker_name}. Here is what I can assist you with:"),
        );

        let (is_member, rank) = match self.characters.get_mut(&speaker_id) {
            Some(speaker) => {
                let member = self.clan_registry.get_char_clan(speaker) == Some(clan);
                (member, speaker.clan_rank)
            }
            None => (false, 0),
        };

        let mut lines = vec![
            " ".to_string(),
            "=== Clan Clerk Commands ===".to_string(),
            "deposit <amount> - Deposit gold into the clan treasury".to_string(),
        ];
        if is_member && rank >= 3 {
            lines.push(
                "withdraw <amount> - Withdraw gold from the treasury (Treasurer+)".to_string(),
            );
        }
        if is_member && rank >= 4 {
            lines.push(" ".to_string());
            lines.push("=== Leader Commands ===".to_string());
            lines.push("set bonus <0-2> <0-20> - Set a clan bonus level".to_string());
            lines.push("  0=Pentagram Quest, 1=Military Advisor, 2=Merchant".to_string());
            lines.push("relation <clan#> <1-5> - Set diplomatic relations".to_string());
            lines.push(
                "  Relations: 1=Alliance, 2=Peace-Treaty, 3=Neutral, 4=War, 5=Feud".to_string(),
            );
            lines.push("rank name <0-4> <name> - Rename a clan rank".to_string());
            lines.push("website <url> - Set clan website".to_string());
            lines.push("message <text> - Set clan message".to_string());
            lines.push("raiding on/off - Enable or disable raiding".to_string());
        }
        lines.push(" ".to_string());

        for line in lines {
            self.queue_system_text(speaker_id, line);
        }
    }

    /// C `clanclerk_driver`'s `deposit` branch (`clanmaster.c:729-751`).
    fn clanclerk_handle_deposit(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        rest: &str,
    ) {
        let nr = parse_int_atoi(rest);
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        let speaker_name = speaker.name.clone();
        if nr < 1 {
            self.npc_say(
                clanclerk_id,
                &format!(
                    "I'm sorry, {speaker_name}, but you must specify a positive amount to deposit."
                ),
            );
            return;
        }
        // C's overflow guard: `nr >= 21474836 || ch[co].gold < nr * 100`.
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        if nr >= 21_474_836 || i64::from(speaker.gold) < i64::from(nr) * 100 {
            self.npc_say(
                clanclerk_id,
                &format!("I'm afraid you don't have {nr}G to deposit, {speaker_name}."),
            );
            return;
        }
        speaker.gold -= (nr * 100) as u32;
        speaker.flags.insert(CharacterFlags::ITEMS);
        if let Some(change) = self.clan_registry.clan_money_change(clan, nr, true) {
            self.pending_clanclerk_events
                .push(ClanclerkEvent::MoneyChanged {
                    clan_nr: clan,
                    actor_id: speaker_id,
                    change,
                });
        }
        self.npc_say(
            clanclerk_id,
            &format!("Thank you, {speaker_name}. I have deposited {nr}G into the clan treasury."),
        );
    }

    /// C `clanclerk_driver`'s `withdraw` branch (`clanmaster.c:790-810`).
    fn clanclerk_handle_withdraw(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        rest: &str,
    ) {
        let nr = parse_int_atoi(rest);
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        if nr < 1 {
            self.npc_say(
                clanclerk_id,
                &format!(
                    "I'm sorry, {speaker_name}, but you must specify a positive amount to withdraw."
                ),
            );
            return;
        }
        let clan_money = self.clan_registry.clan_money(clan);
        if clan_money < nr {
            self.npc_say(
                clanclerk_id,
                &format!("I'm afraid the clan treasury only holds {clan_money}G, {speaker_name}."),
            );
            return;
        }
        if let Some(change) = self.clan_registry.clan_money_change(clan, -nr, true) {
            self.pending_clanclerk_events
                .push(ClanclerkEvent::MoneyChanged {
                    clan_nr: clan,
                    actor_id: speaker_id,
                    change,
                });
        }
        self.gate_give_money_silent(speaker_id, (nr * 100) as u32);
        self.npc_say(
            clanclerk_id,
            &format!(
                "Here you are, {speaker_name}. I have withdrawn {nr}G from the clan treasury for you."
            ),
        );
    }

    /// C `clanclerk_driver`'s `set bonus` branch (`clanmaster.c:905-957`).
    fn clanclerk_handle_set_bonus(
        &mut self,
        clanclerk_id: CharacterId,
        _speaker_id: CharacterId,
        clan: u16,
        rest: &str,
    ) {
        let (nr, level) = parse_two_ints(rest, true);
        if !(0..=2).contains(&nr) {
            self.npc_say(
                clanclerk_id,
                "Invalid bonus number. Available bonuses: 0=Pentagram Quest, 1=Military Advisor, 2=Merchant.",
            );
            return;
        }
        if !(0..=20).contains(&level) {
            self.npc_say(clanclerk_id, "The bonus level must be between 0 and 20.");
            return;
        }
        let bonus_name = crate::clan::bonus_name(nr);
        match self.clan_registry.set_bonus_level(clan, nr as usize, level) {
            Ok(()) => {
                if level == 0 {
                    self.npc_say(
                        clanclerk_id,
                        &format!(
                            "Very well. I have disabled the {bonus_name} bonus for your clan."
                        ),
                    );
                } else {
                    self.npc_say(
                        clanclerk_id,
                        &format!(
                            "Very well. I have set the {bonus_name} bonus to level {level} for your clan."
                        ),
                    );
                }
            }
            Err(_) => {
                self.npc_say(
                    clanclerk_id,
                    &format!(
                        "I'm sorry, I was unable to set the {bonus_name} bonus. Perhaps your clan lacks the requirements."
                    ),
                );
            }
        }
    }

    /// C `clanclerk_driver`'s `relation` branch (`clanmaster.c:963-1021`).
    fn clanclerk_handle_relation(
        &mut self,
        clanclerk_id: CharacterId,
        _speaker_id: CharacterId,
        clan: u16,
        rest: &str,
        now_unix: i64,
    ) {
        let (nr, level) = parse_two_ints(rest, false);
        if !(1..=31).contains(&nr) {
            self.npc_say(
                clanclerk_id,
                "The clan number must be between 1 and 31. Use /clan to see the list.",
            );
            return;
        }
        if !(1..=5).contains(&level) {
            self.npc_say(
                clanclerk_id,
                "The relation must be: 1=Alliance, 2=Peace-Treaty, 3=Neutral, 4=War, 5=Feud.",
            );
            return;
        }
        let target_clan = nr as u16;
        let relation = match level {
            1 => ClanRelation::Alliance,
            2 => ClanRelation::PeaceTreaty,
            3 => ClanRelation::Neutral,
            4 => ClanRelation::War,
            _ => ClanRelation::Feud,
        };
        let relation_name = relation.display_name();

        if relation > ClanRelation::Neutral && !self.clan_registry.get_clan_raid(clan) {
            self.npc_say(
                clanclerk_id,
                &format!(
                    "Your clan cannot declare {relation_name} unless you first say 'raiding on'."
                ),
            );
            return;
        }
        if relation > ClanRelation::Neutral && !self.clan_registry.get_clan_raid(target_clan) {
            let target_name = self
                .clan_registry
                .name(target_clan)
                .unwrap_or("")
                .to_string();
            self.npc_say(
                clanclerk_id,
                &format!(
                    "You cannot declare {relation_name} on {target_name} unless they also have raiding enabled."
                ),
            );
            return;
        }

        match self
            .clan_registry
            .relations_mut()
            .set_relation(clan, target_clan, relation, now_unix)
        {
            Ok(()) => {
                let target_name = self
                    .clan_registry
                    .name(target_clan)
                    .unwrap_or("")
                    .to_string();
                self.npc_say(
                    clanclerk_id,
                    &format!(
                        "Very well. I have requested {relation_name} status with {target_name}. The change may take time to process."
                    ),
                );
            }
            Err(_) => {
                self.npc_say(
                    clanclerk_id,
                    "I'm sorry, I was unable to change the diplomatic relation with that clan.",
                );
            }
        }
    }

    /// C `clanclerk_driver`'s `rank name` branch (`clanmaster.c:1023-1051`).
    fn clanclerk_handle_rank_name(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        rest: &str,
    ) {
        let (nr, name) = parse_rank_name(rest);
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        if !(0..=4).contains(&nr) {
            self.npc_say(
                clanclerk_id,
                &format!("The rank number must be between 0 and 4, {speaker_name}."),
            );
            return;
        }
        match self.clan_registry.set_rankname(clan, nr as usize, &name) {
            Ok(()) => {
                self.npc_say(
                    clanclerk_id,
                    &format!("Very well. Rank {nr} shall now be known as {name}."),
                );
                self.pending_clanclerk_events
                    .push(ClanclerkEvent::RankNameSet {
                        clan_nr: clan,
                        actor_id: speaker_id,
                        rank: nr as usize,
                        name,
                    });
            }
            Err(_) => {
                self.npc_say(
                    clanclerk_id,
                    "I'm sorry, I was unable to change that rank name.",
                );
            }
        }
    }

    /// C `clanclerk_driver`'s `website` branch (`clanmaster.c:1084-1094`).
    fn clanclerk_handle_website(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        site: &str,
    ) {
        let stripped = strip_trailing_char(site);
        match self.clan_registry.set_website(clan, stripped) {
            Ok(()) => {
                self.npc_say(
                    clanclerk_id,
                    &format!("Very well. I have updated your clan's website to: {stripped}"),
                );
                self.pending_clanclerk_events
                    .push(ClanclerkEvent::WebsiteSet {
                        clan_nr: clan,
                        actor_id: speaker_id,
                        site: stripped.to_string(),
                    });
            }
            Err(_) => {
                self.npc_say(
                    clanclerk_id,
                    "I'm sorry, I was unable to update the clan website.",
                );
            }
        }
    }

    /// C `clanclerk_driver`'s `message` branch (`clanmaster.c:1098-1108`).
    fn clanclerk_handle_message(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        message: &str,
    ) {
        let stripped = strip_trailing_char(message);
        match self.clan_registry.set_message(clan, stripped) {
            Ok(()) => {
                self.npc_say(
                    clanclerk_id,
                    "Very well. I have updated your clan's message.",
                );
                self.pending_clanclerk_events
                    .push(ClanclerkEvent::MessageSet {
                        clan_nr: clan,
                        actor_id: speaker_id,
                        message: stripped.to_string(),
                    });
            }
            Err(_) => {
                self.npc_say(
                    clanclerk_id,
                    "I'm sorry, I was unable to update the clan message.",
                );
            }
        }
    }

    /// C `clanclerk_driver`'s `raiding on`/`raiding off` branches
    /// (`clanmaster.c:1110-1132`).
    fn clanclerk_handle_raiding(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        enabled: bool,
        now_unix: i64,
    ) {
        match self.clan_registry.set_clan_raid(clan, enabled, now_unix) {
            Ok(()) => {
                if enabled {
                    self.npc_say(
                        clanclerk_id,
                        "Understood. Raiding has been enabled for your clan. Be prepared for battle!",
                    );
                } else {
                    self.npc_say(
                        clanclerk_id,
                        "Understood. Raiding has been disabled for your clan. May peace be with you.",
                    );
                }
                self.pending_clanclerk_events
                    .push(ClanclerkEvent::RaidToggled {
                        clan_nr: clan,
                        actor_id: speaker_id,
                        enabled,
                    });
            }
            Err(ClanRaidError::NoOp) | Err(ClanRaidError::NotFound) => {
                if enabled {
                    self.npc_say(
                        clanclerk_id,
                        "I'm sorry, I was unable to enable raiding for your clan.",
                    );
                } else {
                    self.npc_say(
                        clanclerk_id,
                        "I'm sorry, I was unable to disable raiding for your clan.",
                    );
                }
            }
        }
    }

    /// C `clanclerk_driver`'s `raiding god on`/`raiding god off` branches
    /// (`clanmaster.c:1134-1155`), `CF_GOD`-gated by the caller.
    fn clanclerk_handle_raiding_god(
        &mut self,
        clanclerk_id: CharacterId,
        speaker_id: CharacterId,
        clan: u16,
        enabled: bool,
    ) {
        match self.clan_registry.set_clan_raid_god(clan, enabled) {
            Ok(()) => {
                self.npc_say(clanclerk_id, "Done.");
                self.pending_clanclerk_events
                    .push(ClanclerkEvent::RaidGodToggled {
                        clan_nr: clan,
                        actor_id: speaker_id,
                        enabled,
                    });
            }
            Err(_) => {
                self.npc_say(clanclerk_id, "Failed.");
            }
        }
    }

    /// C `clanclerk_driver`'s `NT_GIVE` branch, Clan Jewel case only
    /// (`clanmaster.c:1157-1168`) - see the module doc comment for why
    /// the `IDR_FLASK` potion branch is out of scope.
    fn clanclerk_handle_give_message(
        &mut self,
        clanclerk_id: CharacterId,
        data: &ClanclerkDriverData,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&clanclerk_id)
            .and_then(|clanclerk| clanclerk.cursor_item.take())
        else {
            return;
        };
        let is_clan_jewel = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver == IDR_CLANJEWEL);

        if !is_clan_jewel {
            // Out of scope: the `IDR_FLASK` potion branch and the
            // "try to give it back" fallback (same simplification as
            // `world/clanmaster.rs`'s own `NT_GIVE` handler).
            self.destroy_item(item_id);
            return;
        }

        self.clan_registry.add_jewel(data.clan).ok();
        self.destroy_item(item_id);
        self.npc_quiet_say(clanclerk_id, "Added Jewel.");
        self.pending_clanclerk_events
            .push(ClanclerkEvent::JewelAdded {
                clan_nr: data.clan,
                actor_id: giver_id,
            });
    }

    /// C `clanclerk_driver`'s tail movement (`clanmaster.c:1209-1212`):
    /// walk back to the rest position, then face `DX_RIGHTDOWN`.
    fn clanclerk_tick_action(&mut self, clanclerk_id: CharacterId, area_id: u16) {
        let Some(clanclerk) = self.characters.get(&clanclerk_id).cloned() else {
            return;
        };
        if self.setup_walk_toward(
            clanclerk_id,
            usize::from(clanclerk.rest_x),
            usize::from(clanclerk.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if clanclerk.dir != CLANCLERK_REST_DIRECTION {
            if let Some(clanclerk_mut) = self.characters.get_mut(&clanclerk_id) {
                let _ = turn(clanclerk_mut, CLANCLERK_REST_DIRECTION);
            }
        }
    }
}

/// C `atoi(ptr)` on the remainder of the message after a keyword prefix
/// (`deposit`/`withdraw`'s own parsing, `clanmaster.c:730,792`) - no
/// token-skipping, just a direct `atoi` of whatever follows. `pub(super)`
/// so `world::clanmaster`'s `rank:` handler (`clanmaster.c:446-500`, its
/// own `atoi(ptr)` call after parsing the target name) can reuse it
/// instead of duplicating the same C-`atoi` semantics.
pub(super) fn parse_int_atoi(text: &str) -> i32 {
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
