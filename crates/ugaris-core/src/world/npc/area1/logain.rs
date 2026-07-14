//! Retired knight-trainer quest-giver NPC (`CDR_LOGAIN`), area 1's
//! "Knightly Troubles" chain (`QLOG` index 9) - the last driver in
//! `ch_driver`'s dispatch table (`src/area/1/gwendylon.c:6076-6155`).
//!
//! Ports `src/area/1/gwendylon.c::logain_driver` (`:4893-5195`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for every other
//! area-1 NPC file). Follows the same `World`/`PlayerRuntime` split
//! established there: the caller supplies a per-player fact snapshot
//! ([`LogainPlayerFacts`]) up front and applies the returned
//! [`LogainOutcomeEvent`]s afterwards, since `logain_state`/
//! `logain_seen_timer` (`area1_ppd` fields) and `QLOG` index 9 live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - The money reward (`create_money_item(MONEY_AREA1_MADKNIGHT)` +
//!   plain `give_char_item`, `gwendylon.c:5140-5145`) is deferred to
//!   `ugaris-server`'s `apply_logain_events`, gated on
//!   [`LogainOutcomeEvent::QuestDone`]'s `times_done == 1` (C's `if (tmp
//!   == 1)`) - same pattern `world::guiwynn`'s own `QuestDone` handling
//!   already established, since `World` has no `ZoneLoader` template
//!   access and C's plain `give_char_item` never auto-converts an
//!   `IF_MONEY` item to gold the way `give_char_item_smart` does.
//! - `destroy_item_byID(co, ID)` (`gwendylon.c:5120-5122`) sweeps the
//!   player's equipment/inventory/cursor via
//!   [`World::destroy_items_by_template_id`] but not the account depot
//!   (`DRD_DEPOT_PPD`) - same documented gap as every other area-1 NPC's
//!   own `destroy_item_byID` sweep.
//! - The commented-out C dead code granting `V_STR` on quest completion
//!   (`gwendylon.c:5147-5153`, wrapped in `// Removed:` in the C source
//!   itself) is not ported - it is already disabled in C.
//! - `struct logain_driver_data`'s `nighttime` field (`gwendylon.c:4897`)
//!   is never read or written anywhere in `logain_driver`'s body - dead
//!   even in C, same precedent as `world::guiwynn`'s own `nighttime`
//!   field - so it is not ported.
//! - C's own `logain_driver` nests its trailing `NT_NPC`/`NTID_DIDSAY`
//!   self-throttle bump *inside* the `if (msg->type == NT_NPC)` block
//!   (`gwendylon.c:5185-5187`) rather than as a separate top-level check
//!   the way `world::guiwynn`'s own second copy is written - functionally
//!   identical (`NTID_DIDSAY` broadcasts are always `NT_NPC`), so this
//!   port reuses the same "unconditional second check after the match"
//!   shape every sibling area-1 NPC file already established for
//!   readability, not a structural deviation.
//! - `balltrap_skelly_dead` (`gwendylon.c:5197-5199`) - the last death
//!   hook in the C file - is a no-op (`;`) in C itself and needs no port.
//! - The `case 5` reminder line wraps "repeat" in `COL_LIGHT_BLUE`/
//!   `COL_RESET` markers (`gwendylon.c:5020-5021`); restored via
//!   `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels and
//!   `World::npc_quiet_say_bytes`, same mechanism as `world::camhermit`.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, CDR_LOGAIN, CDR_LOSTCON, GWENDYLON_QA, NTID_DIDSAY,
    NTID_TERION,
};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_AREA1_MADKEY6, IID_AREA1_MADKEY7, IID_AREA1_MADKEY8, IID_AREA1_MADKEY9, IID_AREA1_MADNOTE2,
};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 16` (`gwendylon.c:4954`): the `NT_CHAR` greeting
/// range.
const LOGAIN_GREET_DISTANCE: i32 = 16;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const LOGAIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:4937`).
const LOGAIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:4942`).
const LOGAIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 10` (`gwendylon.c:5178`): idle "return to post" threshold.
const LOGAIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `realtime - ppd->logain_seen_timer > 120` (`gwendylon.c:4963`,
/// `4966`): the pre-`switch` auto-reset gate, shared by both reset `if`s.
const LOGAIN_STATE_RESET_SECONDS: i32 = 120;
/// C `realtime - ppd->logain_seen_timer > 60` (`gwendylon.c:5040`,
/// `5089`): the reminder gate shared by states 5 and 9.
const LOGAIN_REMINDER_SECONDS: i32 = 60;
/// C `ppd->guiwynn_state < 11` (`gwendylon.c:4970`): no named `#define`
/// exists in the C source for this threshold, so it is named here purely
/// for readability.
const GUIWYNN_STATE_MAD_MAGES_DONE: i32 = 11;

/// Per-player facts [`World::process_logain_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogainPlayerFacts {
    /// `PlayerRuntime::area1_logain_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_logain_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_guiwynn_state()`: gates state 0's quest
    /// offer ("don't offer quest before mad mages is done").
    pub guiwynn_state: i32,
}

/// A side effect [`World::process_logain_actions`] could not apply
/// directly because it touches `PlayerRuntime` (or, for
/// [`LogainOutcomeEvent::QuestDone`], needs `ZoneLoader`/quest-log
/// access). See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogainOutcomeEvent {
    /// Write the new `area1_ppd.logain_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->logain_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:5075`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, 9)` (`gwendylon.c:4977`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 9)` (`gwendylon.c:5119`) - the caller applies
    /// the exp reward, the questlog resend, and (only on first
    /// completion, C's `if (tmp == 1)`) the `create_money_item`+plain
    /// `give_char_item` reward. See the module doc comment for why the
    /// money reward can't be resolved here.
    QuestDone { player_id: CharacterId },
    /// C's `!has_item(co, IID_AREA1_MADKEY6)` + `create_item("mad_key6")`
    /// + plain `give_char_item` (`gwendylon.c:5009-5015`, `5029-5035`) -
    ///   deferred to `ugaris-server` since `World` has no `ZoneLoader`
    ///   template access.
    GrantMadKey6 { player_id: CharacterId },
    /// C's `!has_item(co, IID_AREA1_MADKEY9)` + `create_item("mad_key9")`
    /// + plain `give_char_item` (`gwendylon.c:5022-5028`) - deferred to
    ///   `ugaris-server` since `World` has no `ZoneLoader` template access.
    GrantMadKey9 { player_id: CharacterId },
}

impl World {
    /// C `logain_driver`'s per-tick body (`gwendylon.c:4893-5195`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_logain_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, LogainPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<LogainOutcomeEvent> {
        let logain_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LOGAIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for logain_id in logain_ids {
            self.process_logain_messages(logain_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_logain_messages(
        &mut self,
        logain_id: CharacterId,
        player_facts: &HashMap<CharacterId, LogainPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<LogainOutcomeEvent>,
    ) {
        let Some(logain_name) = self
            .characters
            .get(&logain_id)
            .map(|logain| logain.name.clone())
        else {
            return;
        };
        let mut data = match self
            .characters
            .get(&logain_id)
            .and_then(|logain| logain.driver_state.clone())
        {
            Some(CharacterDriverState::Logain(data)) => data,
            _ => LogainDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&logain_id)
            .map(|logain| std::mem::take(&mut logain.driver_messages))
            .unwrap_or_default();

        // C's first pass over the (not-yet-removed) message queue
        // (`gwendylon.c:4910-4916`): any `NT_NPC`/`NTID_DIDSAY` broadcast
        // from someone else resets our own talk throttle to "just
        // talked".
        for message in &messages {
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != logain_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.logain_handle_char_message(
                    logain_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.logain_handle_text_message(
                    logain_id,
                    &logain_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.logain_handle_give_message(logain_id, message, player_facts, events)
                }
                NT_NPC if message.dat1 == NTID_TERION => {
                    self.logain_handle_terion_message(
                        logain_id,
                        &mut data,
                        message,
                        &mut face_target,
                    );
                }
                _ => {}
            }

            // C's second, nested check of the same self-throttle bump
            // inside `if (msg->type == NT_NPC)` right before
            // `remove_message` (`gwendylon.c:5185-5187`) - see the module
            // doc comment.
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != logain_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        if let Some(logain) = self.characters.get_mut(&logain_id) {
            logain.driver_state = Some(CharacterDriverState::Logain(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:5175-5177`).
        if let (Some(logain), Some((tx, ty))) =
            (self.characters.get(&logain_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(logain.x), i32::from(logain.y), tx, ty) {
                if let Some(logain_mut) = self.characters.get_mut(&logain_id) {
                    let _ = turn(logain_mut, direction as u8);
                }
            }
        }

        // C `if (ticker - dat->last_talk < TICKS*10) { do_idle(cn, TICKS);
        // return; } if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy,
        // DX_RIGHT, ret, lastact)) return; do_idle(cn, TICKS);`
        // (`gwendylon.c:5178-5194`). The NPC's post position (C's
        // `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same substitution
        // every other stationary area-1 NPC uses.
        let last_talk = if let Some(logain) = self.characters.get(&logain_id) {
            match logain.driver_state.as_ref() {
                Some(CharacterDriverState::Logain(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if self.tick.0 < last_talk + LOGAIN_RETURN_TO_POST_TICKS {
            return;
        }
        let Some(logain) = self.characters.get(&logain_id) else {
            return;
        };
        let (post_x, post_y) = (logain.rest_x, logain.rest_y);
        self.secure_move_driver(
            logain_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `logain_driver`'s `NT_CHAR` branch (`gwendylon.c:4923-5077`).
    #[allow(clippy::too_many_arguments)]
    fn logain_handle_char_message(
        &mut self,
        logain_id: CharacterId,
        data: &mut LogainDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LogainPlayerFacts>,
        now: i32,
        events: &mut Vec<LogainOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(logain) = self.characters.get(&logain_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:4927-4930`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:4933-4936`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:4939-4942`).
        if tick < data.last_talk + LOGAIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`gwendylon.c:4944-
        // 4947`).
        if tick < data.last_talk + LOGAIN_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:4950-4953`).
        if logain_id == player_id || !char_see_char(&logain, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 16) continue;` (`gwendylon.c:4956-
        // 4959`).
        if char_dist(&logain, &player) > LOGAIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (realtime - ppd->logain_seen_timer > 120 &&
        // ppd->logain_state && ppd->logain_state <= 4) { ppd->logain_state
        // = 1; }` / `if (realtime - ppd->logain_seen_timer > 120 &&
        // ppd->logain_state >= 6 && ppd->logain_state <= 8) {
        // ppd->logain_state = 6; }` (`gwendylon.c:4963-4968`).
        let mut state = facts.state;
        let seen_gap = now.saturating_sub(facts.seen_timer);
        if seen_gap > LOGAIN_STATE_RESET_SECONDS && state > 0 && state <= 4 {
            state = 1;
        }
        if seen_gap > LOGAIN_STATE_RESET_SECONDS && (6..=8).contains(&state) {
            state = 6;
        }

        let mut didsay = false;
        let mut new_state = state;

        match state {
            0 => {
                // C `case 0:` (`gwendylon.c:4970-4978`).
                if facts.guiwynn_state >= GUIWYNN_STATE_MAD_MAGES_DONE {
                    self.npc_quiet_say(
                        logain_id,
                        &format!("Hail thee, {}. Canst thou spare a moment?", player.name),
                    );
                    events.push(LogainOutcomeEvent::QuestOpen { player_id });
                    new_state = 1;
                    didsay = true;
                }
            }
            1 => {
                self.npc_quiet_say(
                    logain_id,
                    &format!(
                        "My name is {}. I used to train the young knights from the Brotherhood of Knights on the eastern edge of town. But when I went there a few days ago, they started to behave very strangely.",
                        logain.name
                    ),
                );
                new_state = 2;
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    logain_id,
                    "I gathered they had used some strength potions to increase their abilities. I was about to find out more, but they got aggressive and a fight was about to start, so I left.",
                );
                new_state = 3;
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    logain_id,
                    "One day later, I tried talking to them again, but they attacked me on sight. Even though I teach fighting, I am old. I could not win against a score of young men. All I could do was keep them away and escape.",
                );
                new_state = 4;
                didsay = true;
            }
            4 => {
                self.npc_quiet_say(
                    logain_id,
                    "I suspect these potions have been poisoned. If thou couldst go there and find out where they got them, I'd reward thee.",
                );
                new_state = 5;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY6) {
                    events.push(LogainOutcomeEvent::GrantMadKey6 { player_id });
                    self.npc_quiet_say(logain_id, "Thou willt need this key to gain entry.");
                }
            }
            5 => {
                // C `case 5:` (`gwendylon.c:5037-5045`).
                if now.saturating_sub(facts.seen_timer) > LOGAIN_REMINDER_SECONDS {
                    self.npc_quiet_say_bytes(
                        logain_id,
                        &format!(
                            "Hail thee, {}! Couldst thou find out who is responsible? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine offer?",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            6 => {
                self.npc_quiet_say(
                    logain_id,
                    "Loisan? This is strange indeed. Loisan used to live next door to this tavern. I cannot say I knew him very well, since he hardly ever left his home. He is gone now, anyway.",
                );
                new_state = 7;
                didsay = true;
            }
            7 => {
                self.npc_quiet_say(
                    logain_id,
                    "He left for Aston a few days... Wait, he left for Aston the very day the Knights started to behave strangely. He even left the key to his house with me, asked me to look after it.",
                );
                new_state = 8;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY9) {
                    events.push(LogainOutcomeEvent::GrantMadKey9 { player_id });
                    self.npc_quiet_say(
                        logain_id,
                        "Here. I won't use it. But thou might want to search his house.",
                    );
                }
            }
            8 => {
                self.npc_quiet_say(
                    logain_id,
                    "Well, if you ever get to Aston, pay this Loisan a visit.",
                );
                new_state = 9;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY6) {
                    events.push(LogainOutcomeEvent::GrantMadKey6 { player_id });
                    self.npc_quiet_say(
                        logain_id,
                        "Shouldst thou wish to visit the Brotherhood again, here's the key.",
                    );
                }
            }
            9
                // C `case 9:` (`gwendylon.c:5087-5092`).
                if now.saturating_sub(facts.seen_timer) > LOGAIN_REMINDER_SECONDS => {
                    self.npc_quiet_say(
                        logain_id,
                        &format!("I am pleased to see thee, {}.", player.name),
                    );
                    didsay = true;
                }
            // Every other value: no-op, matching C's `switch` with no
            // matching `case`.
            _ => {}
        }

        // C `ppd->logain_seen_timer = realtime;` (`gwendylon.c:5075`):
        // unconditional.
        events.push(LogainOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });
        if new_state != facts.state {
            events.push(LogainOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(..., NTID_DIDSAY, cn, 0);
        // }` (`gwendylon.c:5076-5081`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            self.notify_area(
                logain.x,
                logain.y,
                NT_NPC,
                NTID_DIDSAY,
                logain_id.0 as i32,
                0,
            );
        }
    }

    /// C `logain_driver`'s `NT_TEXT` branch (`gwendylon.c:5083-5106`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as every other area-1 NPC's own text handler).
    #[allow(clippy::too_many_arguments)]
    fn logain_handle_text_message(
        &mut self,
        logain_id: CharacterId,
        logain_name: &str,
        data: &mut LogainDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LogainPlayerFacts>,
        events: &mut Vec<LogainOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let tick = self.tick.0;
        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:5086-5088`).
        if tick > data.last_talk + LOGAIN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:5090-5093`).
        if let Some(current_victim) = data.current_victim {
            if current_victim != speaker_id {
                return;
            }
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if logain_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(logain) = self.characters.get(&logain_id).cloned() else {
            return;
        };
        if char_dist(&logain, &speaker) > LOGAIN_QA_DISTANCE
            || !char_see_char(&logain, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, logain_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(logain_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:5096-5103`): two disjoint `if`s,
            // each resetting `logain_state` back to a checkpoint - safe
            // since the ranges (0-5, 6-9) partition the full state space
            // without overlap, so at most one applies.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= 5 {
                        Some(0)
                    } else if (6..=9).contains(&facts.state) {
                        Some(6)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(LogainOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by logain's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:5104-5106`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `logain_driver`'s `NT_GIVE` branch (`gwendylon.c:5108-5145`).
    fn logain_handle_give_message(
        &mut self,
        logain_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LogainPlayerFacts>,
        events: &mut Vec<LogainOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&logain_id)
            .and_then(|logain| logain.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let giver_name = self
            .characters
            .get(&giver_id)
            .map(|giver| giver.name.clone())
            .unwrap_or_default();

        if template_id == IID_AREA1_MADNOTE2 && facts.is_some_and(|facts| facts.state <= 5) {
            // C `if (it[in].ID == IID_AREA1_MADNOTE2 && ppd &&
            // ppd->logain_state <= 5)` (`gwendylon.c:5114-5145`).
            self.npc_quiet_say(
                logain_id,
                &format!("Now let's see. Ah. I thank thee, {giver_name}."),
            );
            events.push(LogainOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA1_MADNOTE2);
            self.destroy_items_by_template_id(giver_id, IID_AREA1_MADKEY7);
            self.destroy_items_by_template_id(giver_id, IID_AREA1_MADKEY8);
            events.push(LogainOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: 6,
            });
            // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`gwendylon.c:5131-5132`).
            self.destroy_item(item_id);
        } else {
            // C `else { quiet_say(...); if (!give_char_item(co,
            // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem =
            // 0; }` (`gwendylon.c:5136-5143`) - the plain `give_char_item`,
            // not `give_char_item_smart` (same documented asymmetry as
            // every other area-1 NPC's own `NT_GIVE` handler).
            self.npc_quiet_say(
                logain_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }

    /// C `logain_driver`'s `NT_NPC`/`NTID_TERION` relay branch
    /// (`gwendylon.c:5158-5173`): terion's own "yoakin ruins" broadcast
    /// (`dat3 == 3`) prompts an ambient reply that re-broadcasts
    /// `NTID_TERION` with `dat3 == 4` (picked up by `world::guiwynn`'s own
    /// `NTID_TERION` handler), and the earlier "fools to seek danger"
    /// relay (`dat3 == 2`, originating from `world::guiwynn`'s own
    /// `dat3 == 1` reply) prompts a reply with no further broadcast.
    fn logain_handle_terion_message(
        &mut self,
        logain_id: CharacterId,
        data: &mut LogainDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let co_id = CharacterId(message.dat2.max(0) as u32);
        let Some(logain) = self.characters.get(&logain_id).cloned() else {
            return;
        };
        let Some(co) = self.characters.get(&co_id).cloned() else {
            return;
        };

        if message.dat3 == 2 {
            // C `if (msg->dat3 == 2)` (`gwendylon.c:5161-5166`).
            self.npc_quiet_say(
                logain_id,
                "Fools, yes fools they are. As if we didn't have enough problems already.",
            );
            *face_target = Some((i32::from(co.x), i32::from(co.y)));
            data.last_talk = self.tick.0;
        }
        if message.dat3 == 3 {
            // C `if (msg->dat3 == 3)` (`gwendylon.c:5167-5173`).
            self.npc_quiet_say(
                logain_id,
                "Ah, Terion, thou art right! I remember Yoakin telling me about nightmares he's been having lately. Something about skeletons hunting him in a dark, moist place.",
            );
            self.notify_area(
                logain.x,
                logain.y,
                NT_NPC,
                NTID_TERION,
                logain_id.0 as i32,
                4,
            );
            *face_target = Some((i32::from(co.x), i32::from(co.y)));
            data.last_talk = self.tick.0;
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct logain_driver_data` (`src/area/1/gwendylon.c:4893-4897`): the
/// retired knight-trainer NPC's own driver memory (`CDR_LOGAIN`, distinct
/// from the per-player `logain_state`/`logain_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::logain`'s
/// module doc comment for the split). C's own `nighttime` field is never
/// read or written anywhere in `logain_driver`'s body - dead even in C -
/// so it is not ported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogainDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
