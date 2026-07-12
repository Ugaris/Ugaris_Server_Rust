//! Judge NPC (`CDR_JUDGE`), the Arkhata fortress judge who writes the
//! formal entrance-pass letters `world::npc::area37::captain`'s own dialogue
//! chain sets in motion.
//!
//! Ports `src/area/37/arkhata.c::judge_driver` (`:2292-2497`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`captain`: the caller supplies a per-player fact snapshot
//! ([`JudgePlayerFacts`]) up front and applies the returned
//! [`JudgeOutcomeEvent`]s afterwards, since `arkhata_ppd.judge_state` and
//! sibling fields live on `crate::player::PlayerRuntime`, not `World`.
//! Unlike `ramin`/`captain`, two variants ([`JudgeOutcomeEvent::
//! GiveEntranceLetters`]/[`JudgeOutcomeEvent::GiveEntrancePass`]) need
//! `ZoneLoader` item creation, same precedent as `world::npc::area37::
//! rammy`'s own `RammyOutcomeEvent::GiveFortressKeyAndLetter`.
//!
//! `judge_driver`'s seven-state (`0`-`6`) dialogue chain has one
//! cross-driver gate at `0`, read via [`JudgePlayerFacts`]: it needs
//! `captain_state > 0` (`world::npc::area37::captain`'s own progress) to
//! advance, then falls through into `case 1`'s speech/advance-to-`2` in
//! the same tick - collapsed into one `rs == 0` arm, same "fallthrough
//! lands on the next case's action" precedent as `world::npc::area37::
//! captain`'s own `rs == 4` arm.
//!
//! Whether letters 2/3/4 are actually handed out at `rs == 3` is decided
//! by C's own `!(ppd->letter_bits & bit) && !has_item(co, ID)` double
//! gate for each of the three letters independently - reproduced here via
//! [`World::character_has_item_template`] plus the already-ported
//! `PlayerRuntime::arkhata_letter_bits` snapshot in [`JudgePlayerFacts`].
//! `rs == 4`'s letter-5 hand-out uses only the `has_item` half of that
//! gate (there is no tracking bit for letter 5 in `struct arkhata_ppd`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`jaz`/`ramin`/`captain`'s identical
//!   observation for that file's shared driver shape).
//! - `rs == 5`'s dialogue is itself conditional on `letter_bits != (2|4|8)`
//!   but the state increment/`didsay` happen unconditionally either way -
//!   reproduced verbatim (same "conditional dialogue, unconditional state
//!   advance" quirk as `world::npc::area37::ramin`'s own `rs == 10` arm).
//! - `NT_GIVE`'s letter-1 turn-in (`:2464-2470`) is a silent turn-in (no
//!   `say`/`quiet_say` call at all in C), matching `world::npc::area37::
//!   captain`'s own identical letter-1 branch precedent. The fallback
//!   branch (wrong item) uses `say`, matching `ramin`/`rammy`/`jaz`/
//!   `captain`'s own fallback precedent exactly.
//! - No self-defense/regen/spell-self cascade exists in C's `judge_driver`
//!   body at all (matching the `rammy`/`jaz`/`ramin`/`captain` "pure
//!   talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2496`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_ARKHATA_LETTER1, IID_ARKHATA_LETTER2, IID_ARKHATA_LETTER3, IID_ARKHATA_LETTER4,
    IID_ARKHATA_LETTER5,
};
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:2341`, sibling drivers' own
/// identical guard).
const JUDGE_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const JUDGE_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:2324`).
const JUDGE_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:2329`).
const JUDGE_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:2490`): idle "return to post" threshold.
const JUDGE_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ppd->letter_bits == (2 | 4 | 8)` (`arkhata.c:2408,2442,2446`).
const ALL_LETTER_BITS: i32 = 2 | 4 | 8;

/// Per-player facts [`World::process_judge_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgePlayerFacts {
    /// `PlayerRuntime::arkhata_judge_state()`.
    pub judge_state: i32,
    /// `PlayerRuntime::arkhata_captain_state()` (`ppd->captain_state`,
    /// `arkhata.c:2352`): gates `rs` `0`.
    pub captain_state: i32,
    /// `PlayerRuntime::arkhata_letter_bits()` (`ppd->letter_bits`,
    /// `arkhata.c:2374-2408`): gates the `rs` `3` letter hand-outs and
    /// `rs` `5`'s dialogue.
    pub letter_bits: i32,
}

/// A side effect [`World::process_judge_actions`] could not apply
/// directly because it touches `PlayerRuntime` or needs `ZoneLoader` item
/// creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgeOutcomeEvent {
    /// Write the new `arkhata_ppd.judge_state` back.
    UpdateJudgeState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `rs == 3`'s three conditional `create_item("letter2"/"letter3"/
    /// "letter4")` calls (`arkhata.c:2374-2391`), each gated on its own
    /// `!(letter_bits & bit) && !has_item(...)` check already evaluated by
    /// [`World::process_judge_actions`] (needs `World::items` access this
    /// event type cannot carry).
    GiveEntranceLetters {
        player_id: CharacterId,
        give_letter2: bool,
        give_letter3: bool,
        give_letter4: bool,
    },
    /// C `rs == 4`'s `create_item("letter5")` (`arkhata.c:2398-2403`),
    /// gated on `!has_item(co, IID_ARKHATA_LETTER5)`.
    GiveEntrancePass { player_id: CharacterId },
}

impl World {
    /// C `judge_driver`'s per-tick body (`arkhata.c:2292-2497`).
    pub fn process_judge_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JudgePlayerFacts>,
        area_id: u16,
    ) -> Vec<JudgeOutcomeEvent> {
        let judge_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JUDGE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for judge_id in judge_ids {
            self.process_judge_messages(judge_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_judge_messages(
        &mut self,
        judge_id: CharacterId,
        player_facts: &HashMap<CharacterId, JudgePlayerFacts>,
        area_id: u16,
        events: &mut Vec<JudgeOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Judge(mut data)) = self
            .characters
            .get(&judge_id)
            .and_then(|judge| judge.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&judge_id)
            .map(|judge| std::mem::take(&mut judge.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.judge_handle_char_message(
                    judge_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.judge_handle_text_message(
                    judge_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.judge_handle_give_message(judge_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(judge) = self.characters.get_mut(&judge_id) {
            judge.driver_state = Some(CharacterDriverState::Judge(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:2486-2488`).
        if let (Some(judge), Some((tx, ty))) =
            (self.characters.get(&judge_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(judge.x), i32::from(judge.y), tx, ty) {
                if let Some(judge_mut) = self.characters.get_mut(&judge_id) {
                    let _ = turn(judge_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:2490-2494`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(judge) = self.characters.get(&judge_id) {
            match judge.driver_state.as_ref() {
                Some(CharacterDriverState::Judge(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + JUDGE_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(judge) = self.characters.get(&judge_id) else {
                return;
            };
            let (post_x, post_y) = (judge.rest_x, judge.rest_y);
            self.secure_move_driver(
                judge_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `judge_driver`'s `NT_CHAR` branch (`arkhata.c:2308-2424`).
    #[allow(clippy::too_many_arguments)]
    fn judge_handle_char_message(
        &mut self,
        judge_id: CharacterId,
        data: &mut JudgeDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JudgePlayerFacts>,
        events: &mut Vec<JudgeOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(judge) = self.characters.get(&judge_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:2312`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:2318`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:2324`).
        if tick < data.last_talk + JUDGE_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:2329`).
        if tick < data.last_talk + JUDGE_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:2335`).
        if judge_id == player_id || !char_see_char(&judge, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:2341`).
        if char_dist(&judge, &player) > JUDGE_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.judge_state;
        match facts.judge_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 2351-2364`) - see the module doc comment.
            0 if facts.captain_state > 0 => {
                let rank_name = army_rank_name(army_rank_for_points(player.military_points));
                self.npc_say(
                    judge_id,
                    &format!(
                        "A, hello {rank_name}! So the captain needs a system of authorization letters for people to pass through the fortress?"
                    ),
                );
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:2365-2370`).
            2 => {
                self.npc_say(
                    judge_id,
                    "I'm not surprised that this would become an issue. I'll write the formal agreements up right away, please wait a moment.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:2371-2394`).
            3 => {
                self.npc_say(
                    judge_id,
                    "Here, take these three agreements, one for Ramin, one for Count Brannington and one for the fortress Captain.",
                );
                let give_letter2 = facts.letter_bits & 2 == 0
                    && !self.character_has_item_template(player_id, IID_ARKHATA_LETTER2);
                let give_letter3 = facts.letter_bits & 4 == 0
                    && !self.character_has_item_template(player_id, IID_ARKHATA_LETTER3);
                let give_letter4 = facts.letter_bits & 8 == 0
                    && !self.character_has_item_template(player_id, IID_ARKHATA_LETTER4);
                events.push(JudgeOutcomeEvent::GiveEntranceLetters {
                    player_id,
                    give_letter2,
                    give_letter3,
                    give_letter4,
                });
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:2395-2406`).
            4 => {
                self.npc_say(
                    judge_id,
                    "And for you, an entrance pass to the fortress, whilst carrying it, no guard will attack you.",
                );
                if !self.character_has_item_template(player_id, IID_ARKHATA_LETTER5) {
                    events.push(JudgeOutcomeEvent::GiveEntrancePass { player_id });
                }
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:2407-2414`): conditional dialogue,
            // unconditional state advance - see the module doc comment.
            5 => {
                if facts.letter_bits != ALL_LETTER_BITS {
                    self.npc_say(
                        judge_id,
                        "Now please deliver those agreements for me, and speak with Rammy again afterwards. I'm sure he will be happy to hear that this problem is solved.",
                    );
                }
                new_state = 6;
                didsay = true;
            }
            // C `case 6: break;` (`arkhata.c:2415-2416`): all done.
            6 => {}
            _ => {}
        }

        if new_state != facts.judge_state {
            events.push(JudgeOutcomeEvent::UpdateJudgeState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:2418-2422`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `judge_driver`'s `NT_TEXT` branch (`arkhata.c:2427-2457`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn judge_handle_text_message(
        &mut self,
        judge_id: CharacterId,
        data: &mut JudgeDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JudgePlayerFacts>,
        events: &mut Vec<JudgeOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:2430-2432`).
        let tick = self.tick.0;
        if tick > data.last_talk + JUDGE_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:2434`).
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
        let Some(judge_name) = self
            .characters
            .get(&judge_id)
            .map(|judge| judge.name.clone())
        else {
            return;
        };
        let Some(judge) = self.characters.get(&judge_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if judge_id == speaker_id {
            return;
        }
        if char_dist(&judge, &speaker) > JUDGE_QA_DISTANCE
            || !char_see_char(&judge, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let judge_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.judge_state)
            .unwrap_or(0);
        let letter_bits = player_facts
            .get(&speaker_id)
            .map(|facts| facts.letter_bits)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, &judge_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(judge_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:2439-2450`).
            TextAnalysisOutcome::Matched(2) => {
                // C `if (ppd->judge_state>0 && ppd->judge_state<=6 &&
                // ppd->letter_bits!=(2|4|8))` (`arkhata.c:2442`).
                if judge_state > 0 && judge_state <= 6 && letter_bits != ALL_LETTER_BITS {
                    data.last_talk = 0;
                    events.push(JudgeOutcomeEvent::UpdateJudgeState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                // C `if (ppd->judge_state>0 && ppd->judge_state<=6 &&
                // ppd->letter_bits==(2|4|8))` (`arkhata.c:2446`).
                if judge_state > 0 && judge_state <= 6 && letter_bits == ALL_LETTER_BITS {
                    data.last_talk = 0;
                    events.push(JudgeOutcomeEvent::UpdateJudgeState {
                        player_id: speaker_id,
                        new_state: 4,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by judge's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:2453-2456`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `judge_driver`'s `NT_GIVE` branch (`arkhata.c:2460-2479`).
    fn judge_handle_give_message(
        &mut self,
        judge_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JudgePlayerFacts>,
        events: &mut Vec<JudgeOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&judge_id)
            .and_then(|judge| judge.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_LETTER1 && ppd->judge_state ==
        // 0)` (`arkhata.c:2464`): silent turn-in, no `say`/`quiet_say`
        // call at all - reproduced verbatim.
        if item.template_id == IID_ARKHATA_LETTER1
            && is_player
            && facts.is_some_and(|facts| facts.judge_state == 0)
        {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_LETTER1);
            events.push(JudgeOutcomeEvent::UpdateJudgeState {
                player_id: giver_id,
                new_state: 1,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:2471-2477`): hand the
        // item back to the giver.
        self.npc_say(
            judge_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_JUDGE;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `judge_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::captain`'s `CaptainDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JudgeDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
