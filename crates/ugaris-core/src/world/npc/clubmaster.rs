//! `CDR_CLUBMASTER` club foundations/administration NPC.
//!
//! Ports `src/system/clubmaster.c`'s `clubmaster_driver`: the
//! `found:`/`accept:`/`join:`/`leave!` handshake (club founding is a
//! single-step 10,000-gold payment - no Clan-Jewel two-step handoff, so
//! `found:` alone creates the club and installs the founder), the
//! `deposit:`/`withdraw:` club-treasury commands, the `rank:`/`fire:`
//! leader rank-management commands (including their `lookup_name`/
//! `task_set_clan_rank`/`task_fire_from_clan` offline-player fallback,
//! same [`super::clanmaster::ClanmasterEvent::OfflineRankLookup`]/
//! `OfflineFire` shape - see [`ClubmasterEvent::OfflineRankLookup`]/
//! [`ClubmasterEvent::OfflineFire`]), the generic small-talk qa table
//! ([`CLUBMASTER_QA`]), the periodic greeting, the idle-murmur table, and
//! the 12h driver-memory clear timer.
//!
//! Deviations from C (documented here, not silent):
//! - The `NT_CHAR` greeting handler's membership check is a genuine C bug
//!   preserved verbatim: `if (!get_char_club(cn) && !get_char_clan(cn))`
//!   (`clubmaster.c:269`) checks the *clubmaster NPC's own* `cn`
//!   membership, not the visiting player `co`'s (contrast
//!   `clanmaster_driver`'s equivalent check, `clanmaster.c:352`, which
//!   correctly uses `co`). Since NPCs never have a clan/club, this
//!   condition is always true in practice, so the clubmaster greets
//!   every single visitor every time (unlike the clanmaster, which stops
//!   greeting existing clan/club members) - ported as
//!   `char_is_clan_or_club_member(clubmaster_id)` to match exactly.
//! - The `NT_GIVE` "try to give the item back to the sender first"
//!   fallback (`give_char_item`) is simplified to an unconditional
//!   [`World::destroy_item`], matching the precedent already established
//!   by `world/bank.rs`/`world/clanmaster.rs`.
//! - `dat->dir`'s "return to rest position, then face `dat->dir`"
//!   (`secure_move_driver`) is ported via the same `setup_walk_toward`/
//!   `turn` fallback `world/clanmaster.rs` already established.
//! - C's bare `dlog(...)` calls on club founding/deposit/withdraw
//!   (`clubmaster.c:304,496,515`) are server-debug-only, not a persisted
//!   log (unlike `add_clanlog`'s clan-log table) - there is no `club_log`
//!   equivalent in this codebase, matching the precedent
//!   `world/clanclerk.rs`'s module doc comment already documents for
//!   clan's own bare `dlog` calls.
//! - Achievement awards (`ACHIEVEMENT_CLUB_MEMBER`/`ACHIEVEMENT_CLUB_
//!   MASTER`) need `ServerRuntime`/DB handles `World` doesn't have, so
//!   they're queued as [`ClubmasterEvent`] and applied in
//!   `ugaris-server`'s `world_events.rs::apply_clubmaster_events`, same
//!   shape as `world/clanmaster.rs::ClanmasterEvent`.
use crate::character_driver::{mem_add_driver, mem_check_driver, mem_erase_driver};
use crate::world::*;

const CLUBMASTER_GREET_DISTANCE: i32 = 10;
const CLUBMASTER_QA_DISTANCE: i32 = 12;
const CLUBMASTER_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
/// C `TICKS * 60` in `clubmaster_driver`'s idle-murmur throttle
/// (`clubmaster.c:551`).
const CLUBMASTER_TALK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)` in
/// `clubmaster_driver`'s greeting handler (`clubmaster.c:264,272`).
const CLUBMASTER_GREET_MEMORY_SLOT: usize = 7;
/// C's weekly club founding fee, `10000 * 100` in 1/100 gold
/// (`clubmaster.c:284,298`).
const CLUB_FOUNDING_FEE: i32 = 10_000 * 100;

/// C `clubmaster_driver`'s `switch (RANDOM(8))` idle-murmur table
/// (`clubmaster.c:552-579`). Every case is `murmur` except case `1`
/// (`whisper`). Unlike `CLANMASTER_MUTTERINGS` (`RANDOM(13)`, 13
/// entries), this driver only ever rolls `RANDOM(8)`, and its first 8
/// entries happen to be byte-for-byte identical to `clanmaster.c`'s own
/// first 8 (both NPCs share this same copy-pasted idle-murmur table) -
/// duplicated here rather than shared, matching how C itself duplicates
/// the table per source file.
const CLUBMASTER_MUTTERINGS: [(bool, &str); 8] = [
    (false, "My back itches."),
    (true, "There's something stuck between your teeth."),
    (false, "Oh yeah, those were the days."),
    (false, "Now where did I put it?"),
    (false, "Oh my, life is hard but unfair."),
    (false, "Beware of the fire snails!"),
    (false, "I love the clicking of coins."),
    (false, "Gold and Silver, Silver and Gold."),
];

/// A `clubmaster_driver` outcome that needs `ServerRuntime`'s
/// `PlayerRuntime`/DB handles (achievement awards) to finish - see the
/// module doc comment for why this split exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClubmasterEvent {
    /// C `clubmaster_driver`'s `found:` success branch
    /// (`clubmaster.c:297-309`): `ACHIEVEMENT_CLUB_MEMBER` and
    /// `ACHIEVEMENT_CLUB_MASTER`, both awarded to the founder.
    ClubFounded { founder_id: CharacterId },
    /// C `clubmaster_driver`'s `join:` success branch
    /// (`clubmaster.c:360-368`): `ACHIEVEMENT_CLUB_MEMBER`, awarded to
    /// the new member.
    MemberAdded { member_id: CharacterId },
    /// C `rank:`'s offline-name fallback (`clubmaster.c:420-432`):
    /// `lookup_name` resolves `target_name` against the DB rather than an
    /// online character, so C schedules `task_set_clan_rank` (the same
    /// shared task-queue worker `clanmaster.c` uses, dispatching on
    /// `set->clan < CLUBOFFSET` internally - `clubmaster.c` always passes
    /// `get_char_club(co) + CLUBOFFSET`, taking its `else` branch). No
    /// task queue exists in this codebase, so `ugaris-server`'s
    /// `apply_clubmaster_events` resolves the DB lookup, validation,
    /// mutation, and feedback synchronously instead - see
    /// `ClanmasterEvent::OfflineRankLookup`'s doc comment for the same
    /// shape applied to clans.
    OfflineRankLookup {
        clubmaster_id: CharacterId,
        club_nr: u16,
        target_name: String,
        rank: u8,
        setter_name: String,
    },
    /// Same shape as [`ClubmasterEvent::OfflineRankLookup`] but for
    /// `fire:`'s offline fallback (`clubmaster.c:468-481`,
    /// `task_fire_from_clan`/`fire_from_clan`'s `else` branch,
    /// `task.c:133-168`).
    OfflineFire {
        clubmaster_id: CharacterId,
        club_nr: u16,
        target_name: String,
        setter_name: String,
    },
}

impl World {
    pub fn drain_pending_clubmaster_events(&mut self) -> Vec<ClubmasterEvent> {
        std::mem::take(&mut self.pending_clubmaster_events)
    }

    // `char_is_clan_or_club_member` (C's `get_char_clan(x) ||
    // get_char_club(x)` idiom) is shared with `world/clanmaster.rs` -
    // used here with `x == cn`, the clubmaster NPC itself, in the
    // greeting handler, and `x == co`, the speaking player, everywhere
    // else (see the module doc comment for that first case's C-bug
    // caveat).

    /// C `get_char_club(co)` combined with a minimum `ch[co].clan_rank`
    /// gate, the shared precondition of `accept:`/`rank:`/`fire:`/
    /// `withdraw:`.
    fn char_club_if_rank(&mut self, character_id: CharacterId, min_rank: u8) -> Option<u16> {
        let character = self.characters.get_mut(&character_id)?;
        let club_nr = self.club_registry.get_char_club(character)?;
        (character.clan_rank >= min_rank).then_some(club_nr)
    }

    /// Clubmaster NPC tick: process messages, greet nearby visitors,
    /// walk/turn back to post, clear expired driver memory, and roll idle
    /// mutterings. Ports the per-tick body of C `clubmaster_driver`.
    pub fn process_clubmaster_actions(&mut self, area_id: u16, _now_unix: i64) {
        let clubmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == crate::character_driver::CDR_CLUBMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for clubmaster_id in clubmaster_ids {
            self.process_clubmaster_messages(clubmaster_id);
            self.greet_nearby_club_founders(clubmaster_id);
            self.clubmaster_tick_action(clubmaster_id, area_id);
            self.clear_expired_clubmaster_memory(clubmaster_id);
            self.clubmaster_idle_chatter(clubmaster_id);
        }
    }

    /// C `clubmaster_driver`'s message loop (`clubmaster.c:243-542`).
    fn process_clubmaster_messages(&mut self, clubmaster_id: CharacterId) {
        let Some(clubmaster_name) = self.characters.get(&clubmaster_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Clubmaster(mut data)) = self
            .characters
            .get(&clubmaster_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&clubmaster_id)
            .map(|clubmaster| std::mem::take(&mut clubmaster.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_TEXT => self.clubmaster_handle_text_message(
                    clubmaster_id,
                    &clubmaster_name,
                    &mut data,
                    message,
                ),
                NT_GIVE => self.clubmaster_handle_give_message(clubmaster_id, message),
                _ => {}
            }
        }

        if let Some(clubmaster) = self.characters.get_mut(&clubmaster_id) {
            clubmaster.driver_state = Some(CharacterDriverState::Clubmaster(data));
        }
    }

    /// C `clubmaster_driver`'s `NT_TEXT` branch (`clubmaster.c:276-523`).
    fn clubmaster_handle_text_message(
        &mut self,
        clubmaster_id: CharacterId,
        clubmaster_name: &str,
        data: &mut ClubmasterDriverData,
        message: &CharacterDriverMessage,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        if speaker_id == clubmaster_id {
            return;
        }
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C's small-talk qa table, called unconditionally first
        // (`analyse_text_driver(cn, msg->dat1, ..., msg->dat3)` -
        // `clubmaster_driver` never even reads the return value).
        if let Some(reply) =
            self.clubmaster_qa_reply(clubmaster_id, clubmaster_name, speaker_id, text)
        {
            self.npc_quiet_say(clubmaster_id, &reply);
        }

        if !self
            .characters
            .get(&speaker_id)
            .is_some_and(|speaker| speaker.flags.contains(CharacterFlags::PLAYER))
        {
            return;
        }

        let lower = text.to_ascii_lowercase();

        if let Some(pos) = lower.find("found:") {
            let rest = text[pos + 6..].to_string();
            self.clubmaster_handle_found_command(clubmaster_id, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("accept:") {
            let rest = text[pos + 7..].to_string();
            self.clubmaster_handle_accept_command(clubmaster_id, data, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("join:") {
            let rest = text[pos + 5..].to_string();
            self.clubmaster_handle_join_command(clubmaster_id, data, speaker_id, &rest);
        }
        if lower.contains("leave!") {
            self.clubmaster_handle_leave_command(clubmaster_id, speaker_id);
        }
        if let Some(pos) = lower.find("deposit:") {
            let rest = text[pos + 8..].to_string();
            self.clubmaster_handle_deposit_command(clubmaster_id, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("withdraw:") {
            let rest = text[pos + 9..].to_string();
            self.clubmaster_handle_withdraw_command(clubmaster_id, speaker_id, &rest);
        }
        // C: `ptr += 6` after the `strcasestr` match (one past "rank:"/
        // "fire:"'s own 5 characters) - see `world/clanmaster.rs`'s
        // sibling dispatch for why this is *not* the same `+= <keyword
        // length>` offset `found:`/`accept:`/`join:` use above.
        if let Some(pos) = lower.find("rank:") {
            let rest = text.get(pos + 6..).unwrap_or("").to_string();
            self.clubmaster_handle_rank_command(clubmaster_id, speaker_id, &rest);
        }
        if let Some(pos) = lower.find("fire:") {
            let rest = text.get(pos + 6..).unwrap_or("").to_string();
            self.clubmaster_handle_fire_command(clubmaster_id, speaker_id, &rest);
        }
    }

    fn clubmaster_qa_reply(
        &self,
        clubmaster_id: CharacterId,
        clubmaster_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) -> Option<String> {
        let clubmaster = self.characters.get(&clubmaster_id)?;
        let speaker = self.characters.get(&speaker_id)?;
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if char_dist(clubmaster, speaker) > CLUBMASTER_QA_DISTANCE {
            return None;
        }
        if !char_see_char(clubmaster, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(
            text,
            clubmaster_name,
            &speaker.name,
            CLUBMASTER_QA,
        ) {
            crate::character_driver::TextAnalysisOutcome::Said(reply) => Some(reply),
            // C: `answer_code == 1` -> `quiet_say(cn, "I'm %s.", ch[cn].name)`.
            crate::character_driver::TextAnalysisOutcome::Matched(1) => {
                Some(format!("I'm {clubmaster_name}."))
            }
            _ => None,
        }
    }

    /// C `clubmaster_driver`'s `found:` handler (`clubmaster.c:280-316`):
    /// unlike `clanmaster_driver`'s `name:`, this is a single-step
    /// founding - no Clan-Jewel handoff, the club is created and the
    /// speaker installed as its founder (`clan_rank == 2`) immediately.
    fn clubmaster_handle_found_command(
        &mut self,
        clubmaster_id: CharacterId,
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
                clubmaster_id,
                &format!("I'm sorry, {speaker_name}, but only paying players may found clubs."),
            );
            return;
        }
        if self.char_is_clan_or_club_member(speaker_id) {
            self.npc_quiet_say(
                clubmaster_id,
                "You are already a member of a clan or club. You cannot found a new one.",
            );
            return;
        }
        let speaker_gold = self
            .characters
            .get(&speaker_id)
            .map(|c| c.gold)
            .unwrap_or(0);
        if (speaker_gold as i64) < CLUB_FOUNDING_FEE as i64 {
            self.npc_quiet_say(clubmaster_id, "You cannot pay the fee of 10,000 gold.");
            return;
        }
        // C: `for (n = 0; n < 79; n++) { if (!(isalpha(*ptr) || *ptr == '
        // ')) break; name[n] = *ptr++; }` - stops at the first character
        // that is neither alpha nor space (including end-of-string),
        // which already enforces `create_club`'s own name-validity rule,
        // so `create_club` can only still fail here on `NameTaken`/
        // `ClubListFull`.
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|&c| c.is_ascii_alphabetic() || c == ' ')
            .take(79)
            .collect();
        match self.club_registry.create_club(&name, 0) {
            Ok(club_nr) => {
                let Some(speaker) = self.characters.get_mut(&speaker_id) else {
                    return;
                };
                speaker.gold -= CLUB_FOUNDING_FEE as u32;
                speaker.flags.insert(CharacterFlags::ITEMS);
                speaker.clan = crate::clan::CLUB_OFFSET + club_nr;
                speaker.clan_serial = self.club_registry.serial(club_nr);
                speaker.clan_rank = 2;
                let club_name = self
                    .club_registry
                    .name(club_nr)
                    .unwrap_or_default()
                    .to_string();
                self.npc_quiet_say(
                    clubmaster_id,
                    &format!(
                        "Congratulations, {speaker_name}, you are now the leader of the club \
                         {club_name}."
                    ),
                );
                self.pending_clubmaster_events
                    .push(ClubmasterEvent::ClubFounded {
                        founder_id: speaker_id,
                    });
            }
            Err(_) => {
                self.npc_quiet_say(clubmaster_id, "Something's wrong with the name.");
            }
        }
    }

    /// C `clubmaster_driver`'s `accept:` handler (`clubmaster.c:317-338`).
    /// Unlike `clanmaster_driver`'s `accept:` (leader rank `>= 2`), a
    /// club's own `accept:` only requires rank `>= 1`.
    fn clubmaster_handle_accept_command(
        &mut self,
        clubmaster_id: CharacterId,
        data: &mut ClubmasterDriverData,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(club_nr) = self.char_club_if_rank(speaker_id, 1) else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club leader, {speaker_name}."),
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
        data.accept_clan = club_nr;
        data.accept_cn = Some(speaker_id);
        self.npc_quiet_say(
            clubmaster_id,
            &format!("To join {speaker_name}'s club {target_name}, say: 'join: {speaker_name}'"),
        );
    }

    /// C `clubmaster_driver`'s `join:` handler (`clubmaster.c:339-370`).
    fn clubmaster_handle_join_command(
        &mut self,
        clubmaster_id: CharacterId,
        data: &mut ClubmasterDriverData,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        if self.char_is_clan_or_club_member(speaker_id) {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are already a clan or club member, {speaker_name}."),
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
                clubmaster_id,
                &format!("You have not been invited, {speaker_name}."),
            );
            return;
        }
        if !data.join.eq_ignore_ascii_case(&typed) {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("{typed} has not invited you, {speaker_name}."),
            );
            return;
        }
        let club_nr = data.accept_clan;
        let master_name = data.join.clone();
        let serial = self.club_registry.serial(club_nr);
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        // C: `ch[co].clan = dat->accept_clan + CLUBOFFSET;` etc. - set
        // directly rather than via `add_member` (which is clan-only and
        // rejects `cnr >= CLUB_OFFSET`), matching `clubmaster.c`'s own
        // commented-out `// add_member(co,dat->accept_clan,dat->join);`
        // line.
        speaker.clan = crate::clan::CLUB_OFFSET + club_nr;
        speaker.clan_serial = serial;
        speaker.clan_rank = 0;
        self.npc_quiet_say(
            clubmaster_id,
            &format!("{speaker_name}, you are now a member of {master_name}'s club."),
        );
        data.accept.clear();
        data.accept_clan = 0;
        data.join.clear();
        self.pending_clubmaster_events
            .push(ClubmasterEvent::MemberAdded {
                member_id: speaker_id,
            });
    }

    /// C `clubmaster_driver`'s `leave!` handler (`clubmaster.c:371-378`).
    /// `remove_member` is the same shared clan/club function
    /// `world/clanmaster.rs::clanmaster_handle_leave_command` already
    /// calls - it clears `clan`/`clan_rank`/`clan_serial` unconditionally,
    /// with no club-specific validation of its own. C's shared
    /// `remove_member` (`clan.c:1208-1221`) only writes a clan-log entry
    /// for `clan < CLUBOFFSET`, so a club departure logs nothing here,
    /// matching that gate exactly (no `ClubmasterEvent` needed).
    fn clubmaster_handle_leave_command(
        &mut self,
        clubmaster_id: CharacterId,
        speaker_id: CharacterId,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        if self.club_registry.get_char_club(speaker).is_none() {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club member, {speaker_name}."),
            );
            return;
        }
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        self.clan_registry.remove_member(speaker);
        self.npc_quiet_say(
            clubmaster_id,
            &format!("You are no longer a member of any club, {speaker_name}"),
        );
    }

    /// C `clubmaster_driver`'s `deposit:` handler (`clubmaster.c:485-503`).
    fn clubmaster_handle_deposit_command(
        &mut self,
        clubmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        let Some(club_nr) = self.club_registry.get_char_club(speaker) else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club member, {speaker_name}."),
            );
            return;
        };
        // C: `val = atoi(ptr + 8) * 100;` - a plain-gold amount, scaled to
        // 1/100 gold to match `ch[co].gold`/`club[n].money`'s own unit.
        let val = super::clanclerk::parse_int_atoi(rest).saturating_mul(100);
        let Some(speaker) = self.characters.get_mut(&speaker_id) else {
            return;
        };
        if val > 0 && speaker.gold as i64 >= i64::from(val) {
            speaker.gold -= val as u32;
            speaker.flags.insert(CharacterFlags::ITEMS);
            self.club_registry.club_money_change(club_nr, val);
            let total = self.club_registry.club_money(club_nr);
            self.npc_quiet_say(
                clubmaster_id,
                &format!(
                    "You have deposited {}G, for a total of {}G, {speaker_name}.",
                    val / 100,
                    total / 100
                ),
            );
        } else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You do not have that much gold, {speaker_name}."),
            );
        }
    }

    /// C `clubmaster_driver`'s `withdraw:` handler (`clubmaster.c:504-522`).
    /// Unlike `deposit:` (any member), `withdraw:` requires `clan_rank ==
    /// 2` (the club founder, `get_char_club(co)` combined with `ch[co].
    /// clan_rank < 2` in C).
    fn clubmaster_handle_withdraw_command(
        &mut self,
        clubmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(club_nr) = self.char_club_if_rank(speaker_id, 2) else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club founder, {speaker_name}."),
            );
            return;
        };
        let val = super::clanclerk::parse_int_atoi(rest).saturating_mul(100);
        let club_money = self.club_registry.club_money(club_nr);
        if val > 0 && club_money >= val {
            self.club_registry.club_money_change(club_nr, -val);
            if let Some(speaker) = self.characters.get_mut(&speaker_id) {
                speaker.gold += val as u32;
                speaker.flags.insert(CharacterFlags::ITEMS);
            }
            let remaining = self.club_registry.club_money(club_nr);
            self.npc_quiet_say(
                clubmaster_id,
                &format!(
                    "You have withdrawn {}G, money left in club {}G, {speaker_name}.",
                    val / 100,
                    remaining / 100
                ),
            );
        } else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("The club does not have that much gold, {speaker_name}."),
            );
        }
    }

    /// C `clubmaster_driver`'s `rank:` handler (`clubmaster.c:379-435`).
    /// Unlike `clanmaster_driver`'s `rank:` (leader rank `>= 4`, target
    /// range 0-4), a club's own `rank:` only requires rank `>= 2` (the
    /// founder) and the target range is 0-1, plus a founder-can't-be-
    /// retargeted guard C's clan `rank:` doesn't have (a clan's own rank 4
    /// is unique to the leader by construction of `add_member`, but a
    /// club founder (`clan_rank == 2`) is a distinct, protected rank a
    /// club's `rank:` must explicitly reject retargeting). An unmatched
    /// online name is queued as [`ClubmasterEvent::OfflineRankLookup`]
    /// for `ugaris-server` to resolve against the DB - see the module doc
    /// comment.
    fn clubmaster_handle_rank_command(
        &mut self,
        clubmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(club_nr) = self.char_club_if_rank(speaker_id, 2) else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club founder, {speaker_name}."),
            );
            return;
        };
        let (target_name, remainder) = super::clanmaster::take_name_token(rest);
        if target_name.is_empty() {
            return;
        }
        // C: `rank = atoi(ptr)`, `ptr` being whatever followed the parsed
        // name (`clubmaster.c:395`).
        let rank = super::clanclerk::parse_int_atoi(remainder);
        if !(0..=1).contains(&rank) {
            self.npc_quiet_say(clubmaster_id, "You must use a rank between 0 and 1.");
            return;
        }
        let rank = rank as u8;

        let Some(target_id) = self.find_online_player_by_name(&target_name) else {
            // C: falls through to `lookup_name`/`task_set_clan_rank`
            // (`clubmaster.c:420-432`) - see
            // `ClubmasterEvent::OfflineRankLookup`'s doc comment.
            self.pending_clubmaster_events
                .push(ClubmasterEvent::OfflineRankLookup {
                    clubmaster_id,
                    club_nr,
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
        if !target_is_paid && rank > 0 {
            self.npc_quiet_say(
                clubmaster_id,
                &format!(
                    "{target_display_name} is not a paying player, you cannot set the rank higher than 0."
                ),
            );
            return;
        }
        // C: `else if (ch[cc].clan_rank == 2)` (`clubmaster.c:412`) -
        // checked before, not as part of, the same-club membership test.
        if self
            .characters
            .get(&target_id)
            .is_some_and(|c| c.clan_rank == 2)
        {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("{target_display_name} is the club's founder, cannot change rank."),
            );
            return;
        }
        if self.char_club_if_rank(target_id, 0) == Some(club_nr) {
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.clan_rank = rank;
            }
            self.npc_quiet_say(
                clubmaster_id,
                &format!("Set {target_display_name}'s rank to {rank}."),
            );
        } else {
            self.npc_quiet_say(
                clubmaster_id,
                "You cannot change the rank of those not belonging to your club.",
            );
        }
    }

    /// C `clubmaster_driver`'s `fire:` handler (`clubmaster.c:436-483`).
    /// Unlike `clanmaster_driver`'s `fire:` (leader rank `>= 4`), a
    /// club's own `fire:` only requires rank `>= 1`, and rejects firing
    /// the founder (`clan_rank == 2`) rather than clan's implicit
    /// "leader is always unique" invariant. An unmatched online name is
    /// queued as [`ClubmasterEvent::OfflineFire`] for `ugaris-server` to
    /// resolve against the DB - see the module doc comment.
    fn clubmaster_handle_fire_command(
        &mut self,
        clubmaster_id: CharacterId,
        speaker_id: CharacterId,
        rest: &str,
    ) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(club_nr) = self.char_club_if_rank(speaker_id, 1) else {
            self.npc_quiet_say(
                clubmaster_id,
                &format!("You are not a club leader, {speaker_name}."),
            );
            return;
        };
        let (target_name, _remainder) = super::clanmaster::take_name_token(rest);
        if target_name.is_empty() {
            return;
        }
        let Some(target_id) = self.find_online_player_by_name(&target_name) else {
            // C: falls through to `lookup_name`/`task_fire_from_clan`
            // (`clubmaster.c:468-481`) - see
            // `ClubmasterEvent::OfflineFire`'s doc comment.
            self.pending_clubmaster_events
                .push(ClubmasterEvent::OfflineFire {
                    clubmaster_id,
                    club_nr,
                    target_name,
                    setter_name: speaker_name,
                });
            return;
        };
        if self.char_club_if_rank(target_id, 0) != Some(club_nr) {
            self.npc_quiet_say(
                clubmaster_id,
                "You cannot fire those not belonging to your club.",
            );
            return;
        }
        let Some(target_display_name) = self.characters.get(&target_id).map(|c| c.name.clone())
        else {
            return;
        };
        // C: `if (ch[cc].clan_rank < 2) { remove_member(...) } else {
        // "cannot fire the founder" }` (`clubmaster.c:459-464`).
        if self
            .characters
            .get(&target_id)
            .is_some_and(|c| c.clan_rank >= 2)
        {
            self.npc_quiet_say(clubmaster_id, "You cannot fire the founder of the club.");
            return;
        }
        let Some(target) = self.characters.get_mut(&target_id) else {
            return;
        };
        self.clan_registry.remove_member(target);
        self.npc_quiet_say(clubmaster_id, &format!("Fired: {target_display_name}."));
    }

    /// C `clubmaster_driver`'s `NT_GIVE` branch (`clubmaster.c:526-540`):
    /// there is no special item this driver ever wants (unlike
    /// `clanmaster_driver`'s Clan Jewel), so every handed-over item is
    /// simply destroyed - see the module doc comment for the "give it
    /// back first" simplification.
    fn clubmaster_handle_give_message(
        &mut self,
        clubmaster_id: CharacterId,
        _message: &CharacterDriverMessage,
    ) {
        if let Some(item_id) = self
            .characters
            .get_mut(&clubmaster_id)
            .and_then(|clubmaster| clubmaster.cursor_item.take())
        {
            self.destroy_item(item_id);
        }
    }

    /// C `clubmaster_driver`'s `NT_CHAR` greeting branch
    /// (`clubmaster.c:247-273`), ported as a periodic nearby-player scan
    /// matching the simplification `world/clanmaster.rs` already
    /// established. See the module doc comment for the `cn`-vs-`co` C-bug
    /// this preserves.
    fn greet_nearby_club_founders(&mut self, clubmaster_id: CharacterId) {
        let Some(clubmaster) = self.characters.get(&clubmaster_id).cloned() else {
            return;
        };

        let mut candidates: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == clubmaster_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(
                    &clubmaster.driver_memory,
                    CLUBMASTER_GREET_MEMORY_SLOT,
                    character.id.0,
                )
            {
                continue;
            }
            if char_dist(&clubmaster, character) > CLUBMASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&clubmaster, character, &self.map, self.date.daylight) {
                continue;
            }
            candidates.push(character.id);
        }

        // C: `if (!get_char_club(cn) && !get_char_clan(cn))` - the
        // clubmaster NPC's own membership, not the visitor's (see the
        // module doc comment). Always true in practice (NPCs never have
        // a clan/club), so this greeting fires for every visitor.
        let clubmaster_is_member = self.char_is_clan_or_club_member(clubmaster_id);

        for player_id in candidates {
            if !clubmaster_is_member {
                let Some(name) = self.characters.get(&player_id).map(|c| c.name.clone()) else {
                    continue;
                };
                self.npc_quiet_say(
                    clubmaster_id,
                    &format!("Hello {name}! Would you like to found a club?"),
                );
            }
            if let Some(clubmaster_mut) = self.characters.get_mut(&clubmaster_id) {
                mem_add_driver(
                    &mut clubmaster_mut.driver_memory,
                    CLUBMASTER_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
        }
    }

    /// C `clubmaster_driver`'s `secure_move_driver(cn, ch[cn].tmpx,
    /// ch[cn].tmpy, dat->dir, ret, lastact)` (`clubmaster.c:547-549`),
    /// ported via the same `setup_walk_toward`/`turn` fallback
    /// `world/clanmaster.rs::clanmaster_tick_action` already established.
    fn clubmaster_tick_action(&mut self, clubmaster_id: CharacterId, area_id: u16) {
        let Some(clubmaster) = self.characters.get(&clubmaster_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::Clubmaster(data)) = clubmaster.driver_state.clone() else {
            return;
        };
        if self.setup_walk_toward(
            clubmaster_id,
            usize::from(clubmaster.rest_x),
            usize::from(clubmaster.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if clubmaster.dir != data.dir as u8 {
            if let Some(clubmaster_mut) = self.characters.get_mut(&clubmaster_id) {
                let _ = turn(clubmaster_mut, data.dir as u8);
            }
        }
    }

    /// C `clubmaster_driver`'s memory-clear timer (`clubmaster.c:584-587`).
    fn clear_expired_clubmaster_memory(&mut self, clubmaster_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(clubmaster) = self.characters.get_mut(&clubmaster_id) {
            let memcleartimer = match clubmaster.driver_state.as_ref() {
                Some(CharacterDriverState::Clubmaster(data)) => data.memcleartimer,
                _ => return,
            };
            if tick > memcleartimer {
                mem_erase_driver(&mut clubmaster.driver_memory, CLUBMASTER_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Clubmaster(data)) =
                    clubmaster.driver_state.as_mut()
                {
                    data.memcleartimer = tick + CLUBMASTER_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }

    /// C `clubmaster_driver`'s idle-murmur block (`clubmaster.c:551-582`).
    fn clubmaster_idle_chatter(&mut self, clubmaster_id: CharacterId) {
        let tick = self.tick.0;
        let Some(clubmaster) = self.characters.get(&clubmaster_id) else {
            return;
        };
        let last_talk = match clubmaster.driver_state.as_ref() {
            Some(CharacterDriverState::Clubmaster(data)) => data.last_talk,
            _ => return,
        };
        if tick <= last_talk + CLUBMASTER_TALK_INTERVAL_TICKS {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            return;
        }

        let index = legacy_random_below_from_seed(&mut self.legacy_random_seed, 8) as usize;
        let (is_whisper, text) = CLUBMASTER_MUTTERINGS[index];
        if is_whisper {
            self.npc_whisper(clubmaster_id, text);
        } else {
            self.npc_murmur(clubmaster_id, text);
        }

        if let Some(CharacterDriverState::Clubmaster(data)) = self
            .characters
            .get_mut(&clubmaster_id)
            .and_then(|clubmaster| clubmaster.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct clubmaster_driver_data` (`src/system/clubmaster.c:198-213`):
/// the club foundations/administration NPC's own driver memory
/// (`CDR_CLUBMASTER`). Unlike [`ClanmasterDriverData`], club founding
/// (`found:`) is a single-step gold payment - there is no per-player
/// "name chosen, waiting for a Clan Jewel" state, so there is no club
/// counterpart to [`ClanFoundData`]. C's own `new_name[80]`/`new_co`/
/// `new_ID`/`new_timeout` fields are declared but never read *or* written
/// anywhere in `clubmaster_driver` (genuinely dead struct members, unlike
/// `ClanmasterDriverData::accept_cn`, which is at least written once) -
/// dropped here rather than kept for fidelity, since there is nothing to
/// be faithful to.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClubmasterDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub dir: i32,
    /// C `dat->accept[80]`: the name of the player a club leader has
    /// invited (`accept: <name>`).
    pub accept: String,
    pub accept_clan: u16,
    /// C `dat->accept_cn`: set by the `accept:` handler but never read
    /// again anywhere in `clubmaster.c` either - kept for the same
    /// fidelity reason `ClanmasterDriverData::accept_cn` documents.
    pub accept_cn: Option<CharacterId>,
    /// C `dat->join[80]`: the inviting leader's own name, echoed back by
    /// the invitee via `join: <leader name>` to confirm the invite.
    pub join: String,
    #[serde(default)]
    pub memcleartimer: u64,
}

/// C `clubmaster_driver_parse` (`src/system/clubmaster.c:215-225`): same
/// `dir=N;` zone-file arg shape as [`parse_clanmaster_driver_args`].
pub fn parse_clubmaster_driver_args(args: &str) -> ClubmasterDriverData {
    let mut data = ClubmasterDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "dir" {
            data.dir = value.parse::<i32>().unwrap_or(0);
        }
        rest = next;
    }
    data
}

/// C `struct qa qa[]` from `src/system/clubmaster.c:70-83`. Like
/// `CLANMASTER_QA`, C's own caller (`clubmaster_driver`) never reads
/// `analyse_text_driver`'s return value either, so `answer_code == 1`
/// ("what's your name"/"who are you") is the only observable special
/// case, handled by `crate::world::World::clubmaster_qa_reply` the same
/// way `clanmaster_qa_reply` handles it. Unlike `CLANMASTER_QA`, this
/// table has no "jewels"/"repeat"/"raid"/"scout"/"info" entries at all -
/// `clubmaster.c`'s own table genuinely stops after `"club"`.
pub const CLUBMASTER_QA: &[TextQaEntry] = &[
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
        words: &["help"],
        answer: Some("Sorry, I'm just a merchant, %s!"),
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
        words: &["club"],
        answer: Some(
            "Say 'found: <club name>' to found a club. The first weekly payment of 10000g is \
             due immediately.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["what", "is", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["who", "are", "you"],
        answer: None,
        answer_code: 1,
    },
];
