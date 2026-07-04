//! `CDR_CLANMASTER` clan foundations NPC.
//!
//! Ports `src/area/30/clanmaster.c`'s `clanmaster_driver`: the
//! found/invite/join/leave text-command handshake (`name:`/`accept:`/
//! `join:`/`leave!`), the Clan Jewel `NT_GIVE` handoff that completes
//! founding, the generic small-talk qa table ([`CLANMASTER_QA`]), the
//! periodic greeting, the idle-murmur table, and the 12h driver-memory
//! clear timer. This is the first live call site for
//! [`crate::clan::ClanRegistry::found_clan`]/`add_member`/`remove_member`
//! (previously only reachable from the `/joinclan`/`/killclan`/`/renclan`
//! GM cheats - see `PORTING_TODO.md`'s "Clan system" task).
//!
//! `rank:`/`fire:` (leader rank-management text commands, `clanmaster.c:
//! 446-547`) are fully ported, including C's offline-name fallback
//! (`task_set_clan_rank`/`task_fire_from_clan`, an async DB-task queue
//! that schedules the same mutation for whenever that player next logs
//! in): since `World` has no DB handle, an unmatched online name is
//! queued as [`ClanmasterEvent::OfflineRankLookup`]/[`ClanmasterEvent::
//! OfflineFire`] and resolved against the DB synchronously in
//! `ugaris-server`'s `apply_clanmaster_events` instead of via a real task
//! queue - see those variants' doc comments and
//! `clanmaster_handle_rank_command`/`clanmaster_handle_fire_command`.
//!
//! Deliberately out of scope for this slice (documented in that task's
//! REMAINING notes, not silently dropped):
//! - `CDR_CLANCLERK` (`clanclerk_driver`, the members-only economy driver:
//!   deposit/withdraw/bonus/relation/rank-name/website/message/raiding
//!   commands - needs 8+ new `clan.rs` functions - money, jewel-in-vault
//!   count, bonuses, relation-by-command, raid flags - that don't exist
//!   yet).
//! - Club founding/joining: `get_char_club` is approximated here as a
//!   bare `clan >= CLUB_OFFSET` range check (`is_club_member`) since
//!   `club.c` itself isn't ported (no club registry to validate against);
//!   this driver never actually joins anyone to a club (clanmaster.c
//!   itself doesn't either - that's `clubmaster`/driver 113, a different
//!   NPC entirely), it only *gates* on "already in a clan or club".
//!
//! Like `world/trader.rs`/`world/bank.rs`, achievement awards and clan-log
//! persistence need `ServerRuntime`/DB handles `World` doesn't have, so
//! they're queued as [`ClanmasterEvent`] and applied in `ugaris-server`'s
//! `world_events.rs::apply_clanmaster_events`.
//!
//! Deviations from C (documented here, not silent):
//! - The `NT_GIVE` "try to give the item back to the sender first"
//!   fallback (`give_driver`/`dat->give_try`) is simplified to an
//!   unconditional [`World::destroy_item`], matching the precedent
//!   already established by `world/bank.rs`/`world/merchant.rs` (no
//!   generic "give item back" driver helper exists yet). `give_try`/
//!   `accept_cn` are kept as dead fields on [`ClanmasterDriverData`] for
//!   struct fidelity only.
//! - `dat->dir`'s "return to rest position, then face `dat->dir`"
//!   (`secure_move_driver`) is ported via the same `setup_walk_toward`/
//!   `turn` fallback `world/bank.rs::process_bank_tick_action` already
//!   established, rather than porting `secure_move_driver` itself.
//! - COL_LIGHT_BLUE/COL_RESET color markers around "clan" in the greeting
//!   are dropped (same simplification as every other ported NPC greeting
//!   in this codebase); wording stays byte-for-byte identical otherwise.
use super::clanclerk::parse_int_atoi;
use super::*;
use crate::character_driver::{
    mem_add_driver, mem_check_driver, mem_erase_driver, ClanFoundData, ClanmasterDriverData,
    CLANMASTER_QA,
};
use crate::clan::CLUB_OFFSET;

const CLANMASTER_GREET_DISTANCE: i32 = 10;
const CLANMASTER_QA_DISTANCE: i32 = 12;
const CLANMASTER_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
/// C `TICKS * 60` in `clanmaster_driver`'s idle-murmur throttle
/// (`clanmaster.c:600`).
const CLANMASTER_TALK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)` in
/// `clanmaster_driver`'s greeting handler (`clanmaster.c:346,352`).
const CLANMASTER_GREET_MEMORY_SLOT: usize = 7;

/// C `clanmaster_driver`'s `switch (RANDOM(13))` idle-murmur table
/// (`clanmaster.c:601-635`). Every case is `murmur` except case `1`
/// (`whisper`), tracked via the bool.
const CLANMASTER_MUTTERINGS: [(bool, &str); 13] = [
    (false, "My back itches."),
    (true, "There's something stuck between your teeth."),
    (false, "Oh yeah, those were the days."),
    (false, "Now where did I put it?"),
    (false, "Oh my, life is hard but unfair."),
    (false, "Beware of the fire snails!"),
    (false, "I love the clicking of coins."),
    (false, "Gold and Silver, Silver and Gold."),
    (false, "I could really use a nap."),
    (false, "I think I saw a dragonfly."),
    (false, "Who moved my cheese?"),
    (false, "Time for a coffee break."),
    (false, "Eddow at it again?"),
];

/// A `clanmaster_driver` outcome that needs `ServerRuntime`'s
/// `PlayerRuntime`/DB handles (achievement awards, clan-log persistence)
/// to finish - see the module doc comment for why this split exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClanmasterEvent {
    /// C `clanmaster_driver`'s `NT_GIVE` Clan Jewel success branch
    /// (`clanmaster.c:558-573`): `found_clan`'s own "Clan was founded by
    /// %s" clan-log entry (`clan.c:489`, prio 1), `add_member`'s "%s was
    /// added to clan by %s" entry (`clan.c:1192`, prio 15, master = the
    /// founder's own name) and `ACHIEVEMENT_CLAN_MEMBER` award, plus
    /// `clanmaster_driver`'s own explicit `ACHIEVEMENT_CLAN_MASTER` award.
    ClanFounded {
        founder_id: CharacterId,
        clan_nr: u16,
    },
    /// C `add_member` (`clan.c:1186-1206`) via the `accept:`/`join:`
    /// handshake (`clanmaster.c:404`): the "%s was added to clan by %s"
    /// clan-log entry (master = the inviting leader's name) and the
    /// `ACHIEVEMENT_CLAN_MEMBER` award.
    MemberAdded {
        member_id: CharacterId,
        clan_nr: u16,
        master_name: String,
    },
    /// C `remove_member` (`clan.c:1208-1221`) via `leave!`
    /// (`clanmaster.c:435-441`): the "%s was fired from clan by %s"
    /// clan-log entry (master = the leaving member themself, since C
    /// calls `remove_member(co, co)`).
    MemberLeft {
        member_id: CharacterId,
        clan_nr: u16,
    },
    /// C `clanmaster_driver`'s `rank:` handler's `add_clanlog`
    /// (`clanmaster.c:493-494`, prio 30): "%s rank was set to %d by %s".
    RankSet {
        clan_nr: u16,
        target_id: CharacterId,
        rank: u8,
        setter_name: String,
    },
    /// C `remove_member` (`clan.c:1208-1221`) via `fire:`
    /// (`clanmaster.c:539`, `remove_member(cc, co)`): same clan-log shape
    /// as [`ClanmasterEvent::MemberLeft`], but master = the firing
    /// leader, not the fired member themself.
    MemberFired {
        member_id: CharacterId,
        clan_nr: u16,
        firer_name: String,
    },
    /// C `rank:`'s offline-name fallback (`clanmaster.c:481-499`):
    /// `lookup_name` resolves `target_name` against the DB rather than an
    /// online character, so C schedules `task_set_clan_rank` for its
    /// async DB-task queue worker (`set_clan_rank`, `task.c:87-115,
    /// 333-345`). This codebase has no task queue, so `ugaris-server`'s
    /// `apply_clanmaster_events` resolves the DB lookup, validation,
    /// mutation, and feedback synchronously instead (same tick, just
    /// after `World`'s own online-target branch) - meaning, unlike C,
    /// this always resolves definitively (found-and-updated, found-
    /// but-rejected, or genuinely no such player), never leaving C's
    /// ambiguous "still resolving" `uID == 0` case unaddressed.
    OfflineRankLookup {
        clanmaster_id: CharacterId,
        clan_nr: u16,
        target_name: String,
        rank: u8,
        setter_name: String,
    },
    /// Same shape as [`ClanmasterEvent::OfflineRankLookup`] but for
    /// `fire:`'s offline fallback (`clanmaster.c:525-546`,
    /// `task_fire_from_clan`/`fire_from_clan`, `task.c:117-133,347-356`).
    OfflineFire {
        clanmaster_id: CharacterId,
        clan_nr: u16,
        target_name: String,
        setter_name: String,
    },
}

/// C `get_char_club` (`src/system/club.c:29-51`) approximated as a bare
/// range check, since `club.c` itself isn't ported (see the module doc
/// comment).
fn is_club_member(character: &Character) -> bool {
    character.clan >= CLUB_OFFSET
}

/// C `rank:`/`fire:`'s shared name-token parser (`clanmaster.c:472-480,
/// 517-525`): skip leading whitespace, then take up to 79 bytes stopping
/// at the first quote/whitespace/end. Returns `(name, remainder)`, where
/// `remainder` starts at the stopping byte (matching C's `ptr` after the
/// loop, still pointing *at* the delimiter rather than past it - `rank:`
/// feeds this straight into `atoi`, which skips leading whitespace
/// itself).
fn take_name_token(text: &str) -> (String, &str) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && i - start < 79 {
        let c = bytes[i] as char;
        if c == '"' || c.is_ascii_whitespace() {
            break;
        }
        i += 1;
    }
    (text[start..i].to_string(), &text[i..])
}

impl World {
    pub fn drain_pending_clanmaster_events(&mut self) -> Vec<ClanmasterEvent> {
        std::mem::take(&mut self.pending_clanmaster_events)
    }

    /// Whether `character_id` currently belongs to a clan or club, per
    /// C's `get_char_clan(co) || get_char_club(co)` idiom. Uses the real
    /// (not cloned) character so `get_char_clan`'s stale-reference
    /// self-heal (clearing `clan`/`clan_rank`/`clan_serial` on a mismatch)
    /// actually sticks, matching C.
    fn char_is_clan_or_club_member(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        self.clan_registry.get_char_clan(character).is_some() || is_club_member(character)
    }

    fn char_clan_if_leader(&mut self, character_id: CharacterId, min_rank: u8) -> Option<u16> {
        let character = self.characters.get_mut(&character_id)?;
        let clan_nr = self.clan_registry.get_char_clan(character)?;
        (character.clan_rank >= min_rank).then_some(clan_nr)
    }

    /// Clanmaster NPC tick: process messages, greet nearby non-members,
    /// walk/turn back to post, clear expired driver memory, and roll idle
    /// mutterings. Ports the per-tick body of C `clanmaster_driver`.
    pub fn process_clanmaster_actions(&mut self, area_id: u16, now_unix: i64) {
        let clanmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == crate::character_driver::CDR_CLANMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for clanmaster_id in clanmaster_ids {
            self.process_clanmaster_messages(clanmaster_id, now_unix);
            self.greet_nearby_clan_founders(clanmaster_id);
            self.clanmaster_tick_action(clanmaster_id, area_id);
            self.clear_expired_clanmaster_memory(clanmaster_id);
            self.clanmaster_idle_chatter(clanmaster_id);
        }
    }

    /// C `clanmaster_driver`'s message loop (`clanmaster.c:326-593`).
    fn process_clanmaster_messages(&mut self, clanmaster_id: CharacterId, now_unix: i64) {
        let Some(clanmaster_name) = self.characters.get(&clanmaster_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Clanmaster(mut data)) = self
            .characters
            .get(&clanmaster_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&clanmaster_id)
            .map(|clanmaster| std::mem::take(&mut clanmaster.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_TEXT => self.clanmaster_handle_text_message(
                    clanmaster_id,
                    &clanmaster_name,
                    &mut data,
                    message,
                ),
                NT_GIVE => self.clanmaster_handle_give_message(clanmaster_id, message, now_unix),
                _ => {}
            }
        }

        if let Some(clanmaster) = self.characters.get_mut(&clanmaster_id) {
            clanmaster.driver_state = Some(CharacterDriverState::Clanmaster(data));
        }
    }

    /// C `clanmaster_driver`'s `NT_TEXT` branch (`clanmaster.c:356-529`).
    fn clanmaster_handle_text_message(
        &mut self,
        clanmaster_id: CharacterId,
        clanmaster_name: &str,
        data: &mut ClanmasterDriverData,
        message: &CharacterDriverMessage,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        if speaker_id == clanmaster_id {
            return;
        }
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C's small-talk qa table, called unconditionally first
        // (`analyse_text_driver(cn, msg->dat1, ..., msg->dat3)` -
        // clanmaster.c never even reads the return value).
        if let Some(reply) =
            self.clanmaster_qa_reply(clanmaster_id, clanmaster_name, speaker_id, text)
        {
            self.npc_quiet_say(clanmaster_id, &reply);
        }

        if !self
            .characters
            .get(&speaker_id)
            .is_some_and(|speaker| speaker.flags.contains(CharacterFlags::PLAYER))
        {
            return;
        }

        let lower = text.to_ascii_lowercase();

        if let Some(pos) = lower.find("name:") {
            let rest = text[pos + 5..].to_string();
            self.clanmaster_handle_name_command(clanmaster_id, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("accept:") {
            let rest = text[pos + 7..].to_string();
            self.clanmaster_handle_accept_command(clanmaster_id, data, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("join:") {
            let rest = text[pos + 5..].to_string();
            self.clanmaster_handle_join_command(clanmaster_id, data, speaker_id, &rest);
        }
        if lower.contains("leave!") {
            self.clanmaster_handle_leave_command(clanmaster_id, speaker_id);
        }
        // C: `ptr += 6` after the `strcasestr` match (one past "rank:"/
        // "fire:"'s own 5 characters) - see `clanmaster_handle_rank_command`/
        // `clanmaster_handle_fire_command`'s doc comments for why this is
        // *not* the same `+= <keyword length>` offset `name:`/`accept:`/
        // `join:` use above.
        if let Some(pos) = lower.find("rank:") {
            let rest = text.get(pos + 6..).unwrap_or("").to_string();
            self.clanmaster_handle_rank_command(clanmaster_id, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("fire:") {
            let rest = text.get(pos + 6..).unwrap_or("").to_string();
            self.clanmaster_handle_fire_command(clanmaster_id, speaker_id, &rest);
        }
    }

    fn clanmaster_qa_reply(
        &self,
        clanmaster_id: CharacterId,
        clanmaster_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) -> Option<String> {
        let clanmaster = self.characters.get(&clanmaster_id)?;
        let speaker = self.characters.get(&speaker_id)?;
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if char_dist(clanmaster, speaker) > CLANMASTER_QA_DISTANCE {
            return None;
        }
        if !char_see_char(clanmaster, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(
            text,
            clanmaster_name,
            &speaker.name,
            CLANMASTER_QA,
        ) {
            crate::character_driver::TextAnalysisOutcome::Said(reply) => Some(reply),
            // C: `answer_code == 1` -> `quiet_say(cn, "I'm %s.", ch[cn].name)`.
            crate::character_driver::TextAnalysisOutcome::Matched(1) => {
                Some(format!("I'm {clanmaster_name}."))
            }
            // Every other `answer_code` (2/3/4) is genuinely dead in C -
            // see the module doc comment/`CLANMASTER_QA`'s doc comment.
            _ => None,
        }
    }

    /// C `clanmaster_driver`'s `name:` handler (`clanmaster.c:358-382`).
    fn clanmaster_handle_name_command(
        &mut self,
        clanmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let is_paid = self
            .characters
            .get(&speaker_id)
            .is_some_and(|c| c.flags.contains(CharacterFlags::PAID));
        if !is_paid {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("I'm sorry, {speaker_name}, but only paying players may found clans."),
            );
            return;
        }
        if self.char_is_clan_or_club_member(speaker_id) {
            self.npc_quiet_say(
                clanmaster_id,
                "You are already a member of a clan or club. You cannot found a new one.",
            );
            return;
        }
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|&c| c != '"')
            .take(79)
            .collect();
        self.npc_quiet_say(
            clanmaster_id,
            &format!(
                "Your clan, {speaker_name}, will be named '{name}'. Try again if that is not \
                 what you want. Or hand me a Clan Jewel to proceed. You can buy them at Jeremy's"
            ),
        );
        if let Some(speaker) = self.characters.get_mut(&speaker_id) {
            speaker.driver_state = Some(CharacterDriverState::ClanFound(ClanFoundData {
                state: 1,
                nr: 0,
                name,
            }));
        }
    }

    /// C `clanmaster_driver`'s `accept:` handler (`clanmaster.c:384-402`).
    fn clanmaster_handle_accept_command(
        &mut self,
        clanmaster_id: CharacterId,
        data: &mut ClanmasterDriverData,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(clan_nr) = self.char_clan_if_leader(speaker_id, 2) else {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You are not a clan leader, {speaker_name}."),
            );
            return;
        };
        let target_name: String = rest
            .trim_start()
            .chars()
            .take_while(|&c| c != '"')
            .take(79)
            .collect();
        data.accept = target_name.clone();
        data.join = speaker_name.clone();
        data.accept_clan = clan_nr;
        data.accept_cn = Some(speaker_id);
        self.npc_quiet_say(
            clanmaster_id,
            &format!("To join {speaker_name}'s clan {target_name}, say: 'join: {speaker_name}'"),
        );
    }

    /// C `clanmaster_driver`'s `join:` handler (`clanmaster.c:404-424`).
    fn clanmaster_handle_join_command(
        &mut self,
        clanmaster_id: CharacterId,
        data: &mut ClanmasterDriverData,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        if self.char_is_clan_or_club_member(speaker_id) {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You are already a clan member, {speaker_name}."),
            );
            return;
        }
        let typed: String = rest
            .trim_start()
            .chars()
            .take_while(|&c| c != '"')
            .take(79)
            .collect();
        if !data.accept.eq_ignore_ascii_case(&speaker_name) {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You have not been invited, {speaker_name}."),
            );
            return;
        }
        if !data.join.eq_ignore_ascii_case(&typed) {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("{typed} has not invited you, {speaker_name}."),
            );
            return;
        }
        let clan_nr = data.accept_clan;
        let master_name = data.join.clone();
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        if self.clan_registry.add_member(speaker, clan_nr).is_ok() {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("{speaker_name}, you are now a member of {master_name}'s clan."),
            );
            data.accept.clear();
            data.accept_clan = 0;
            data.join.clear();
            self.pending_clanmaster_events
                .push(ClanmasterEvent::MemberAdded {
                    member_id: speaker_id,
                    clan_nr,
                    master_name,
                });
        }
    }

    /// C `clanmaster_driver`'s `leave!` handler (`clanmaster.c:426-433`).
    fn clanmaster_handle_leave_command(
        &mut self,
        clanmaster_id: CharacterId,
        speaker_id: CharacterId,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        let Some(clan_nr) = self.clan_registry.get_char_clan(speaker) else {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You are not a clan member, {speaker_name}."),
            );
            return;
        };
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        self.clan_registry.remove_member(speaker);
        self.npc_quiet_say(
            clanmaster_id,
            &format!("You are no longer a member of any clan, {speaker_name}"),
        );
        self.pending_clanmaster_events
            .push(ClanmasterEvent::MemberLeft {
                member_id: speaker_id,
                clan_nr,
            });
    }

    /// C `find_char_byname` (`base.c:4189-4201`) as used by the `rank:`/
    /// `fire:` handlers' own `getfirst_char`/`getnext_char` search loop
    /// (`clanmaster.c:465-467,522-524`): first `CF_PLAYER` character
    /// whose name case-insensitively matches. See `world/trader.rs`'s
    /// sibling helper/module doc comment for the iteration-order caveat.
    fn find_online_player_by_name(&self, name: &str) -> Option<CharacterId> {
        let mut candidates: Vec<&Character> = self
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        candidates.first().map(|character| character.id)
    }

    /// C `clanmaster_driver`'s `rank:` handler (`clanmaster.c:446-500`).
    /// An unmatched online name is queued as
    /// [`ClanmasterEvent::OfflineRankLookup`] for `ugaris-server` to
    /// resolve against the DB - see the module doc comment.
    fn clanmaster_handle_rank_command(
        &mut self,
        clanmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(clan_nr) = self.char_clan_if_leader(speaker_id, 4) else {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You are not a clan leader, {speaker_name}."),
            );
            return;
        };
        let (target_name, remainder) = take_name_token(rest);
        if target_name.is_empty() {
            return;
        }
        // C: `rank = atoi(ptr)`, `ptr` being whatever followed the parsed
        // name (`clanmaster.c:490`).
        let rank = parse_int_atoi(remainder);
        if !(0..=4).contains(&rank) {
            self.npc_quiet_say(clanmaster_id, "You must use a rank between 0 and 4.");
            return;
        }
        let rank = rank as u8;

        let Some(target_id) = self.find_online_player_by_name(&target_name) else {
            // C: falls through to `lookup_name`/`task_set_clan_rank`
            // (`clanmaster.c:487-499`) - see
            // `ClanmasterEvent::OfflineRankLookup`'s doc comment.
            self.pending_clanmaster_events
                .push(ClanmasterEvent::OfflineRankLookup {
                    clanmaster_id,
                    clan_nr,
                    target_name,
                    rank,
                    setter_name: speaker_name,
                });
            return;
        };
        let target_display_name = self
            .characters
            .get(&target_id)
            .map(|c| c.name.clone())
            .unwrap_or(target_name);
        let target_is_paid = self
            .characters
            .get(&target_id)
            .is_some_and(|c| c.flags.contains(CharacterFlags::PAID));
        if !target_is_paid && rank > 1 {
            self.npc_quiet_say(
                clanmaster_id,
                &format!(
                    "{target_display_name} is not a paying player, you cannot set the rank higher than 1."
                ),
            );
            return;
        }
        if self.char_clan_if_leader(target_id, 0) == Some(clan_nr) {
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.clan_rank = rank;
            }
            self.npc_quiet_say(
                clanmaster_id,
                &format!("Set {target_display_name}'s rank to {rank}."),
            );
            self.pending_clanmaster_events
                .push(ClanmasterEvent::RankSet {
                    clan_nr,
                    target_id,
                    rank,
                    setter_name: speaker_name,
                });
        } else {
            self.npc_quiet_say(
                clanmaster_id,
                "You cannot change the rank of those not belonging to your clan.",
            );
        }
    }

    /// C `clanmaster_driver`'s `fire:` handler (`clanmaster.c:503-547`).
    /// An unmatched online name is queued as
    /// [`ClanmasterEvent::OfflineFire`] for `ugaris-server` to resolve
    /// against the DB - see the module doc comment.
    fn clanmaster_handle_fire_command(
        &mut self,
        clanmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(clan_nr) = self.char_clan_if_leader(speaker_id, 4) else {
            self.npc_quiet_say(
                clanmaster_id,
                &format!("You are not a clan leader, {speaker_name}."),
            );
            return;
        };
        let (target_name, _remainder) = take_name_token(rest);
        if target_name.is_empty() {
            return;
        }
        let Some(target_id) = self.find_online_player_by_name(&target_name) else {
            // C: falls through to `lookup_name`/`task_fire_from_clan`
            // (`clanmaster.c:530-545`) - see
            // `ClanmasterEvent::OfflineFire`'s doc comment.
            self.pending_clanmaster_events
                .push(ClanmasterEvent::OfflineFire {
                    clanmaster_id,
                    clan_nr,
                    target_name,
                    setter_name: speaker_name,
                });
            return;
        };
        if self.char_clan_if_leader(target_id, 0) != Some(clan_nr) {
            self.npc_quiet_say(
                clanmaster_id,
                "You cannot fire those not belonging to your clan.",
            );
            return;
        }
        let Some(target_display_name) = self.characters.get(&target_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(target) = self.characters.get_mut(&target_id) else {
            return;
        };
        self.clan_registry.remove_member(target);
        self.npc_quiet_say(clanmaster_id, &format!("Fired: {target_display_name}."));
        self.pending_clanmaster_events
            .push(ClanmasterEvent::MemberFired {
                member_id: target_id,
                clan_nr,
                firer_name: speaker_name,
            });
    }

    /// C `clanmaster_driver`'s `NT_GIVE` branch (`clanmaster.c:531-591`).
    fn clanmaster_handle_give_message(
        &mut self,
        clanmaster_id: CharacterId,
        message: &CharacterDriverMessage,
        now_unix: i64,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&clanmaster_id)
            .and_then(|clanmaster| clanmaster.cursor_item.take())
        else {
            return;
        };
        let is_clan_jewel = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver == IDR_CLANJEWEL);

        if !is_clan_jewel {
            self.destroy_item(item_id);
            return;
        }

        let mut fnd = match self
            .characters
            .get(&giver_id)
            .and_then(|c| c.driver_state.clone())
        {
            Some(CharacterDriverState::ClanFound(f)) => f,
            _ => ClanFoundData::default(),
        };

        if fnd.state != 1 {
            self.npc_quiet_say(
                clanmaster_id,
                "You must name your clan first. Say: 'name: <clan-name>'.",
            );
            self.destroy_item(item_id);
            return;
        }

        match self.clan_registry.found_clan(&fnd.name, now_unix) {
            Ok(clan_nr) => {
                let Some(giver_name) = self.characters.get(&giver_id).map(|c| c.name.clone())
                else {
                    self.destroy_item(item_id);
                    return;
                };
                self.npc_quiet_say(
                    clanmaster_id,
                    &format!(
                        "So be it. There will be a new clan, named '{}', and you, {giver_name}, \
                         shall be its new master. Good luck, young master!",
                        fnd.name
                    ),
                );
                fnd.state = 0;
                fnd.nr = clan_nr;
                if let Some(giver) = self.characters.get_mut(&giver_id) {
                    let _ = self.clan_registry.add_member(giver, clan_nr);
                }
                if let Some(giver) = self.characters.get_mut(&giver_id) {
                    giver.clan_rank = 4;
                    giver.driver_state = Some(CharacterDriverState::ClanFound(fnd));
                }
                self.pending_clanmaster_events
                    .push(ClanmasterEvent::ClanFounded {
                        founder_id: giver_id,
                        clan_nr,
                    });
                self.destroy_item(item_id);
            }
            Err(_) => {
                self.npc_quiet_say(
                    clanmaster_id,
                    "There was an error creating your clan. Please try again.",
                );
                self.destroy_item(item_id);
            }
        }
    }

    /// C `clanmaster_driver`'s `NT_CHAR` greeting branch
    /// (`clanmaster.c:329-354`), ported as a periodic nearby-player scan
    /// matching the simplification `world/bank.rs`/`world/merchant.rs`/
    /// `world/trader.rs` already established.
    fn greet_nearby_clan_founders(&mut self, clanmaster_id: CharacterId) {
        let Some(clanmaster) = self.characters.get(&clanmaster_id).cloned() else {
            return;
        };

        let mut candidates: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == clanmaster_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(
                    &clanmaster.driver_memory,
                    CLANMASTER_GREET_MEMORY_SLOT,
                    character.id.0,
                )
            {
                continue;
            }
            if char_dist(&clanmaster, character) > CLANMASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&clanmaster, character, &self.map, self.date.daylight) {
                continue;
            }
            candidates.push(character.id);
        }

        for player_id in candidates {
            // C: `if (!get_char_club(co) && !get_char_clan(co))
            // quiet_say(...)`; the greeting is skipped for existing
            // members, but the memory slot is added either way
            // (`clanmaster.c:329-354`).
            if !self.char_is_clan_or_club_member(player_id) {
                let Some(name) = self.characters.get(&player_id).map(|c| c.name.clone()) else {
                    continue;
                };
                self.npc_quiet_say(
                    clanmaster_id,
                    &format!("Hello {name}! Would you like to found a clan?"),
                );
            }
            if let Some(clanmaster_mut) = self.characters.get_mut(&clanmaster_id) {
                mem_add_driver(
                    &mut clanmaster_mut.driver_memory,
                    CLANMASTER_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
        }
    }

    /// C `clanmaster_driver`'s `secure_move_driver(cn, ch[cn].tmpx,
    /// ch[cn].tmpy, dat->dir, ret, lastact)` (`clanmaster.c:596-598`),
    /// ported via the same `setup_walk_toward`/`turn` fallback
    /// `world/bank.rs::process_bank_tick_action` already established.
    fn clanmaster_tick_action(&mut self, clanmaster_id: CharacterId, area_id: u16) {
        let Some(clanmaster) = self.characters.get(&clanmaster_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::Clanmaster(data)) = clanmaster.driver_state.clone() else {
            return;
        };
        if self.setup_walk_toward(
            clanmaster_id,
            usize::from(clanmaster.rest_x),
            usize::from(clanmaster.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if clanmaster.dir != data.dir as u8 {
            if let Some(clanmaster_mut) = self.characters.get_mut(&clanmaster_id) {
                let _ = turn(clanmaster_mut, data.dir as u8);
            }
        }
    }

    /// C `clanmaster_driver`'s memory-clear timer (`clanmaster.c:637-640`).
    fn clear_expired_clanmaster_memory(&mut self, clanmaster_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(clanmaster) = self.characters.get_mut(&clanmaster_id) {
            let memcleartimer = match clanmaster.driver_state.as_ref() {
                Some(CharacterDriverState::Clanmaster(data)) => data.memcleartimer,
                _ => return,
            };
            if tick > memcleartimer {
                mem_erase_driver(&mut clanmaster.driver_memory, CLANMASTER_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Clanmaster(data)) =
                    clanmaster.driver_state.as_mut()
                {
                    data.memcleartimer = tick + CLANMASTER_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }

    /// C `clanmaster_driver`'s idle-murmur block (`clanmaster.c:599-636`).
    fn clanmaster_idle_chatter(&mut self, clanmaster_id: CharacterId) {
        let tick = self.tick.0;
        let Some(clanmaster) = self.characters.get(&clanmaster_id) else {
            return;
        };
        let last_talk = match clanmaster.driver_state.as_ref() {
            Some(CharacterDriverState::Clanmaster(data)) => data.last_talk,
            _ => return,
        };
        if tick <= last_talk + CLANMASTER_TALK_INTERVAL_TICKS {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            return;
        }

        let index = legacy_random_below_from_seed(&mut self.legacy_random_seed, 13) as usize;
        let (is_whisper, text) = CLANMASTER_MUTTERINGS[index];
        if is_whisper {
            self.npc_whisper(clanmaster_id, text);
        } else {
            self.npc_murmur(clanmaster_id, text);
        }

        if let Some(CharacterDriverState::Clanmaster(data)) = self
            .characters
            .get_mut(&clanmaster_id)
            .and_then(|clanmaster| clanmaster.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }
}
