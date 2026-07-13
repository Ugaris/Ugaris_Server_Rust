//! Thai Pan NPC (`CDR_THAIPAN`), the Arkhata monk who runs "The Ancient
//! Scroll" (quest 74) and the repeatable "Buddah Statue" negative-
//! experience recovery hand-in.
//!
//! Ports `src/area/37/arkhata.c::thaipan_driver` (`:3371-3587`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! jada`/`ramin`: the caller supplies a per-player fact snapshot
//! ([`ThaipanPlayerFacts`]) up front and applies the returned
//! [`ThaipanOutcomeEvent`]s afterwards, since `arkhata_ppd.thai_state`/
//! `last_budda` live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `thaipan_driver`'s ten-state (`0`-`9`) dialogue chain, gated at one
//! point on cross-driver state this file cannot see directly (read via
//! [`ThaipanPlayerFacts`]):
//! - `0` needs `ch[co].level >= 49 && arkhata_ppd.pot_state >= 4`
//!   (`world::npc::area37::potmaker`'s own progress) to advance; C's own
//!   `case 0` falls through into `case 1`'s speech/`questlog_open(74)`/
//!   advance-to-`2` in the same tick - collapsed into one `rs == 0` arm
//!   here, same "fallthrough lands on the next case's action" precedent
//!   as `world::npc::area37::jada`'s own `rs == 0` arm. State `1` itself
//!   is never independently stored (the collapse always lands on `2`),
//!   so no separate arm exists for it.
//! - `8` is a pure wait state: waiting for the Red Scroll
//!   (`IID_ARKHATA_SCROLL2`), handled entirely by this file's own
//!   `NT_GIVE` branch - no cross-driver dependency.
//! - `9` is a pure wait state: quest already completed, nothing left to
//!   say.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`jaz`/`jada`'s identical observation for
//!   that file's shared driver shape).
//! - `NT_GIVE` has two independent turn-in items: the Red Scroll
//!   (`IID_ARKHATA_SCROLL2`, quest-74 completion, speaks on success -
//!   `say`, ported as `npc_quiet_say` for consistency with every other
//!   dialogue line in this file, same precedent as `world::npc::area37::
//!   ramin`'s own module doc comment) and the Buddah Statue
//!   (`IID_ARKHATA_BUDDA`, a repeatable once-per-24h "recover negative
//!   experience" hand-in gated on `arkhata_ppd.thai_state > 0` alone, not
//!   the `<= 8` completion window the scroll uses - reproduced verbatim).
//!   The Buddah branch's own three-way `else` fallback (`arkhata.c:3553-
//!   3567`) is reproduced exactly including its logically dead third
//!   case: if the item is the Buddah Statue, `thai_state > 0`, but
//!   neither "no negative experience" nor "cooldown still active" holds,
//!   C prints no message at all before handing the item back - this can
//!   only happen if the primary success branch's own identical two
//!   conditions were already true, so in practice it never fires, but is
//!   kept as-is rather than "fixed" to match the original code path
//!   exactly.
//! - C's `dlog(co, 0, "got %d exp for thai pain quest 2", v)`
//!   (`arkhata.c:3547`) is dropped - no Rust `dlog` sink exists (same
//!   established gap as `world::npc::area1::james`'s own dropped `dlog`
//!   call, see that module's doc comment).
//! - The Buddah Statue exp grant calls [`World::give_exp`] directly
//!   (`exp`/`exp_used` live on `Character`, inside `World`) rather than
//!   going through an outcome event, same "call it straight from `World`"
//!   precedent as `world::npc::area37::arkhatamonk`'s own dictionary
//!   turn-in reward.
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `thaipan_driver` body at all (matching the `rammy`/`jaz`/`jada`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:3586`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_ARKHATA_BUDDA, IID_ARKHATA_SCROLL2};
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:3420`, sibling drivers' own
/// identical guard).
const THAIPAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const THAIPAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:3403`).
const THAIPAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:3408`).
const THAIPAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:3580`): idle "return to post" threshold.
const THAIPAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `60 * 60 * 24` (`arkhata.c:3534`/`:3557`): the Buddah Statue's
/// once-per-day cooldown, in wall-clock seconds.
const THAIPAN_BUDDA_COOLDOWN_SECONDS: i32 = 60 * 60 * 24;
/// C quest 74, "The Ancient Scroll".
const QLOG_THAIPAN_SCROLL: usize = 74;

/// Per-player facts [`World::process_thaipan_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThaipanPlayerFacts {
    /// `PlayerRuntime::arkhata_thai_state()`.
    pub thai_state: i32,
    /// `PlayerRuntime::arkhata_pot_state()` (`ppd->pot_state`,
    /// `arkhata.c:3431`): gates `rs` `0`.
    pub pot_state: i32,
    /// `PlayerRuntime::arkhata_last_budda()` (`ppd->last_budda`,
    /// `arkhata.c:3534`/`:3540`/`:3557`).
    pub last_budda: i32,
}

/// A side effect [`World::process_thaipan_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThaipanOutcomeEvent {
    /// Write the new `arkhata_ppd.thai_state` back.
    UpdateThaiState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 74)` (`arkhata.c:3439`).
    QuestOpen74 { player_id: CharacterId },
    /// C `questlog_done(co, 74)` (`arkhata.c:3526`), the `NT_GIVE` Red
    /// Scroll turn-in.
    QuestDone74 { player_id: CharacterId },
    /// C `ppd->last_budda = realtime;` (`arkhata.c:3540`), the `NT_GIVE`
    /// Buddah Statue hand-in's cooldown stamp.
    UpdateLastBudda {
        player_id: CharacterId,
        realtime_seconds: i32,
    },
}

impl World {
    /// C `thaipan_driver`'s per-tick body (`arkhata.c:3371-3587`).
    pub fn process_thaipan_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ThaipanPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<ThaipanOutcomeEvent> {
        let thaipan_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_THAIPAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for thaipan_id in thaipan_ids {
            self.process_thaipan_messages(thaipan_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_thaipan_messages(
        &mut self,
        thaipan_id: CharacterId,
        player_facts: &HashMap<CharacterId, ThaipanPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<ThaipanOutcomeEvent>,
    ) {
        let Some(thaipan_name) = self
            .characters
            .get(&thaipan_id)
            .map(|thaipan| thaipan.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Thaipan(mut data)) = self
            .characters
            .get(&thaipan_id)
            .and_then(|thaipan| thaipan.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&thaipan_id)
            .map(|thaipan| std::mem::take(&mut thaipan.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.thaipan_handle_char_message(
                    thaipan_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.thaipan_handle_text_message(
                    thaipan_id,
                    &thaipan_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.thaipan_handle_give_message(thaipan_id, message, now, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(thaipan) = self.characters.get_mut(&thaipan_id) {
            thaipan.driver_state = Some(CharacterDriverState::Thaipan(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:3576-3578`).
        if let (Some(thaipan), Some((tx, ty))) =
            (self.characters.get(&thaipan_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(thaipan.x), i32::from(thaipan.y), tx, ty) {
                if let Some(thaipan_mut) = self.characters.get_mut(&thaipan_id) {
                    let _ = turn(thaipan_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:3580-3584`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(thaipan) = self.characters.get(&thaipan_id) {
            match thaipan.driver_state.as_ref() {
                Some(CharacterDriverState::Thaipan(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + THAIPAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(thaipan) = self.characters.get(&thaipan_id) else {
                return;
            };
            let (post_x, post_y) = (thaipan.rest_x, thaipan.rest_y);
            self.secure_move_driver(
                thaipan_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `thaipan_driver`'s `NT_CHAR` branch (`arkhata.c:3387-3489`).
    #[allow(clippy::too_many_arguments)]
    fn thaipan_handle_char_message(
        &mut self,
        thaipan_id: CharacterId,
        data: &mut ThaipanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ThaipanPlayerFacts>,
        events: &mut Vec<ThaipanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(thaipan) = self.characters.get(&thaipan_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:3391`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:3397`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:3403`).
        if tick < data.last_talk + THAIPAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:3408`).
        if tick < data.last_talk + THAIPAN_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:3414`).
        if thaipan_id == player_id
            || !char_see_char(&thaipan, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:3420`).
        if char_dist(&thaipan, &player) > THAIPAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.thai_state;
        match facts.thai_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 3430-3442`) - see the module doc comment.
            0 if player.level >= 49 && facts.pot_state >= 4 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "Aaaaaaaaaaooooommmm... Oh, hello there friend. I'm Thai Pan, monk in this small place of worship. You are welcome to have a cup of green tea with me.",
                );
                events.push(ThaipanOutcomeEvent::QuestOpen74 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:3443-3447`).
            2 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "I know many stories and there is one in particular I think will interest thee.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:3448-3453`).
            3 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "It is said that the ruins on the shore right north-west of here was once a sanctuary for scholars of my order.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:3454-3459`).
            4 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "And that they were researching some ancient scrolls that somehow should lead them to a higher level of understanding.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:3460-3465`).
            5 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "But their search was powered by greed, not by the will to do good. And as they discovered some of the ancient secrets it corrupted their minds and lead do their downfall.",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` (`arkhata.c:3466-3471`).
            6 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "Now the ruins are occupied by zombies, who have inhabited the place for as long as I have records of.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`arkhata.c:3472-3477`).
            7 => {
                self.npc_quiet_say(
                    thaipan_id,
                    "Very few have ventured there in recent years, but I am told that the place is filled with magic and holds a strange shrine.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8: break;` (`arkhata.c:3478-3479`): waiting for the
            // scroll.
            8 => {}
            // C `case 9: break;` (`arkhata.c:3480-3481`): all done.
            9 => {}
            _ => {}
        }

        if new_state != facts.thai_state {
            events.push(ThaipanOutcomeEvent::UpdateThaiState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:3483-3487`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `thaipan_driver`'s `NT_TEXT` branch (`arkhata.c:3492-3517`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn thaipan_handle_text_message(
        &mut self,
        thaipan_id: CharacterId,
        thaipan_name: &str,
        data: &mut ThaipanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ThaipanPlayerFacts>,
        events: &mut Vec<ThaipanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:3495-3497`).
        let tick = self.tick.0;
        if tick > data.last_talk + THAIPAN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:3499`).
        if data.current_victim.is_some() && data.current_victim != Some(speaker_id) {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(thaipan) = self.characters.get(&thaipan_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if thaipan_id == speaker_id {
            return;
        }
        if char_dist(&thaipan, &speaker) > THAIPAN_QA_DISTANCE
            || !char_see_char(&thaipan, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let thai_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.thai_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, thaipan_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(thaipan_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:3504-3511`): rewind to state
            // 1 while the turn-in window (`1..=8`) is open.
            TextAnalysisOutcome::Matched(2) => {
                if thai_state > 0 && thai_state <= 8 {
                    data.last_talk = 0;
                    events.push(ThaipanOutcomeEvent::UpdateThaiState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by thaipan's
            // own `switch` but still counts as `didsay` (C: `switch
            // (didsay = analyse_text_driver(...))` - any nonzero return
            // is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:3513-3516`) - note this does *not* touch `dat->
        // last_talk` (except the "repeat" branch's own explicit reset
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `thaipan_driver`'s `NT_GIVE` branch (`arkhata.c:3520-3569`).
    fn thaipan_handle_give_message(
        &mut self,
        thaipan_id: CharacterId,
        message: &CharacterDriverMessage,
        now: i32,
        player_facts: &HashMap<CharacterId, ThaipanPlayerFacts>,
        events: &mut Vec<ThaipanOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&thaipan_id)
            .and_then(|thaipan| thaipan.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let is_player = giver.flags.contains(CharacterFlags::PLAYER);
        let facts = player_facts.get(&giver_id).copied();

        // C `if (ppd && it[in].ID == IID_ARKHATA_SCROLL2 && ppd->thai_state
        // > 0 && ppd->thai_state <= 8)` (`arkhata.c:3524`).
        if item.template_id == IID_ARKHATA_SCROLL2
            && is_player
            && facts.is_some_and(|facts| facts.thai_state > 0 && facts.thai_state <= 8)
        {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_SCROLL2);
            events.push(ThaipanOutcomeEvent::QuestDone74 {
                player_id: giver_id,
            });
            events.push(ThaipanOutcomeEvent::UpdateThaiState {
                player_id: giver_id,
                new_state: 9,
            });
            self.npc_quiet_say(
                thaipan_id,
                "So the story is true then, may the secrets within this scroll be yours.",
            );
            self.destroy_item(item_id);
            return;
        }

        // C `else if (ppd && it[in].ID == IID_ARKHATA_BUDDA &&
        // ppd->thai_state > 0 && ch[co].exp_used > ch[co].exp && realtime -
        // ppd->last_budda > 60*60*24)` (`arkhata.c:3533-3534`).
        let budda_ready = item.template_id == IID_ARKHATA_BUDDA
            && is_player
            && facts.is_some_and(|facts| {
                facts.thai_state > 0
                    && giver.exp_used > giver.exp
                    && now.saturating_sub(facts.last_budda) > THAIPAN_BUDDA_COOLDOWN_SECONDS
            });
        if budda_ready {
            self.npc_quiet_say(
                thaipan_id,
                "May you find peace and recover from any discomfort you have had.",
            );
            // C `v = ch[co].exp_used - ch[co].exp;` (`arkhata.c:3538`).
            let v = u64::from(giver.exp_used) - u64::from(giver.exp);
            // C `if (ch[co].flags & CF_HARDCORE) w = ch[co].exp_used / 500;
            // else w = ch[co].exp_used / 200;` (`arkhata.c:3541-3545`).
            let w = if giver.flags.contains(CharacterFlags::HARDCORE) {
                u64::from(giver.exp_used) / 500
            } else {
                u64::from(giver.exp_used) / 200
            };
            // C `v = min(v, w);` (`arkhata.c:3546`).
            let v = v.min(w);
            // C `dlog(co, 0, "got %d exp for thai pain quest 2", v);`
            // (`arkhata.c:3547`) - dropped, see the module doc comment.
            self.give_exp(giver_id, v as i64, u32::from(self.area_id));
            events.push(ThaipanOutcomeEvent::UpdateLastBudda {
                player_id: giver_id,
                realtime_seconds: now,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's `else` fallback (`arkhata.c:3553-3567`) - see the module doc
        // comment for the logically-dead third case this reproduces
        // verbatim.
        let budda_active = item.template_id == IID_ARKHATA_BUDDA
            && is_player
            && facts.is_some_and(|facts| facts.thai_state > 0);
        if budda_active {
            if giver.exp_used <= giver.exp {
                self.npc_say(thaipan_id, "Thou doest not have any negative experience.");
            } else if let Some(facts) = facts {
                if now.saturating_sub(facts.last_budda) <= THAIPAN_BUDDA_COOLDOWN_SECONDS {
                    self.npc_say(thaipan_id, "Thou canst only do this once per day.");
                }
            }
        } else {
            self.npc_say(
                thaipan_id,
                "Thou hast better use for this than I do. Well, if there is a use for it at all.",
            );
        }
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_THAIPAN;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `thaipan_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ThaipanDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_THAIPAN_SCROLL`] to `ugaris-server`'s
/// `apply_thaipan_events`.
pub const fn qlog_thaipan_scroll() -> usize {
    QLOG_THAIPAN_SCROLL
}
