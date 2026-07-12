//! Fiona NPC (`CDR_FIONA`), the Fighting School headmistress: quest 67
//! ("The Missing Ring") giver, student-challenge host, and post-challenge
//! skill-raise vendor.
//!
//! Ports `src/area/37/arkhata.c::fiona_driver` (`:811-1067`) plus its two
//! helpers `fight_student` (`:756-804`, shared with `world::npc::area37::
//! gladiator`'s spawn) and `fiona_raise` (`:806-822`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::jaz`:
//! the caller supplies a per-player fact snapshot ([`FionaPlayerFacts`]) up
//! front and applies the returned [`FionaOutcomeEvent`]s afterwards, since
//! `arkhata_ppd.fiona_state` lives on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! `fight_student`'s actual `Gladiator_<nr>` character creation needs
//! `ZoneLoader`, which `World` cannot see - [`FionaOutcomeEvent::
//! FightStudent`] carries the resolved `nr` (`1..=10`) out to
//! `ugaris-server`'s `area37::spawn_gladiator_student`, which also performs
//! the "is the arena busy" check itself (needs to inspect the map *after*
//! resolving whether to actually spawn, same ordering C's own
//! `fight_student` uses). [`World::arkhata_arena_is_busy`] exposes the pure
//! bounding-box scan so the ugaris-server caller does not have to
//! reimplement it.
//!
//! `fiona_raise`'s doubled `raise_value_exp` call (`:816-818`: C calls it
//! once to test truthiness, then calls it *again* for the actual effect if
//! the first call succeeded) is a real, observable C quirk - a successful
//! raise request actually raises the skill by up to 2 points (the second
//! call can still fail silently, e.g. if the first call already hit the
//! skill's max) while the 10000-gold fee is deducted only once. Reproduced
//! verbatim in [`World::fiona_raise`], not "fixed".
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches `world::npc::
//!   area37::rammy`/`jaz`'s identical observation for that file's shared
//!   driver shape).
//! - No self-defense/regen/spell-self cascade exists in C's `fiona_driver`
//!   body at all (matching the `rammy_driver`/`jaz_driver` "pure talker"
//!   NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1066`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON, NTID_GLADIATOR};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_RING;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::values::full_skill_name;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:337` sibling drivers).
const FIONA_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const FIONA_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:841`).
const FIONA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:846`).
const FIONA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:1061`): idle "return to post" threshold.
const FIONA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level >= 50` (`arkhata.c:878`).
const FIONA_MIN_LEVEL_FOR_TRAINING: u32 = 50;
/// C `ch[co].level > 80` (`arkhata.c:900`, `:934`).
const FIONA_STUDENT_LEVEL_CEILING: u32 = 80;
/// C `ch[co].gold < 10000 * 100` (`fiona_raise`, `arkhata.c:807`).
const FIONA_RAISE_FEE: u32 = 10000 * 100;
/// C quest 67, "The Missing Ring".
const QLOG_FIONA_RING: usize = 67;
/// C `for (x=9;x<=24;x++) for(y=238;y<=252;y++)` (`fight_student`,
/// `arkhata.c:759-760`): the Fighting School arena bounds.
const FIGHT_STUDENT_ARENA_X: std::ops::RangeInclusive<usize> = 9..=24;
const FIGHT_STUDENT_ARENA_Y: std::ops::RangeInclusive<usize> = 238..=252;

/// Per-player facts [`World::process_fiona_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FionaPlayerFacts {
    /// `PlayerRuntime::arkhata_fiona_state()`.
    pub fiona_state: i32,
}

/// A side effect [`World::process_fiona_actions`] could not apply directly
/// because it touches `PlayerRuntime` or `ZoneLoader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FionaOutcomeEvent {
    /// Write the new `arkhata_ppd.fiona_state` back.
    UpdateFionaState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 67)` (`arkhata.c:891`).
    QuestOpen67 { player_id: CharacterId },
    /// C `questlog_done(co, 67)` (`arkhata.c:1042`), the `NT_GIVE` ring
    /// turn-in.
    QuestDone67 { player_id: CharacterId },
    /// C `fight_student(cn, co, ppd->fiona_state - 6)` (`arkhata.c:1014`):
    /// needs `ZoneLoader` to spawn `"Gladiator_<nr>"` - see the module doc
    /// comment.
    FightStudent {
        fiona_id: CharacterId,
        player_id: CharacterId,
        nr: i32,
    },
}

impl World {
    /// C `fiona_driver`'s per-tick body (`arkhata.c:811-1067`).
    pub fn process_fiona_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        area_id: u16,
    ) -> Vec<FionaOutcomeEvent> {
        let fiona_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FIONA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for fiona_id in fiona_ids {
            self.process_fiona_messages(fiona_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_fiona_messages(
        &mut self,
        fiona_id: CharacterId,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        area_id: u16,
        events: &mut Vec<FionaOutcomeEvent>,
    ) {
        let Some(fiona_name) = self
            .characters
            .get(&fiona_id)
            .map(|fiona| fiona.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Fiona(mut data)) = self
            .characters
            .get(&fiona_id)
            .and_then(|fiona| fiona.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&fiona_id)
            .map(|fiona| std::mem::take(&mut fiona.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                // C `if (msg->type==NT_NPC && msg->dat1==NTID_GLADIATOR)`
                // (`arkhata.c:826-836`): a defeated `Gladiator_<nr>`'s own
                // death hook (`world::npc::area37::gladiator`'s `gladiator_
                // dead`) reports the killer back here.
                NT_NPC if message.dat1 == NTID_GLADIATOR => {
                    self.fiona_handle_gladiator_win(fiona_id, message, player_facts, events);
                }
                NT_CHAR => self.fiona_handle_char_message(
                    fiona_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.fiona_handle_text_message(
                    fiona_id,
                    &fiona_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.fiona_handle_give_message(fiona_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(fiona) = self.characters.get_mut(&fiona_id) {
            fiona.driver_state = Some(CharacterDriverState::Fiona(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:1057-1059`).
        if let (Some(fiona), Some((tx, ty))) =
            (self.characters.get(&fiona_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(fiona.x), i32::from(fiona.y), tx, ty) {
                if let Some(fiona_mut) = self.characters.get_mut(&fiona_id) {
                    let _ = turn(fiona_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact))
        // return; }` (`arkhata.c:1061-1065`). The NPC's post position (C's
        // `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same substitution
        // every other stationary NPC in this codebase makes.
        let last_talk = match self
            .characters
            .get(&fiona_id)
            .and_then(|f| f.driver_state.as_ref())
        {
            Some(CharacterDriverState::Fiona(data)) => data.last_talk,
            _ => return,
        };
        if last_talk + FIONA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(fiona) = self.characters.get(&fiona_id) else {
                return;
            };
            let (post_x, post_y) = (fiona.rest_x, fiona.rest_y);
            self.secure_move_driver(
                fiona_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `fiona_driver`'s `NT_NPC`/`NTID_GLADIATOR` branch (`arkhata.c:
    /// 826-836`): a defeated student's `notify_area` report from
    /// `gladiator_dead`. `msg->dat2` (the dead gladiator's own id) and
    /// `msg->dat1` (already matched above) are unused here - only
    /// `msg->dat3` (the killer) matters; `say(cn, ...)` speaks as `fiona_id`
    /// itself (the receiving driver's own character, not any message
    /// field).
    fn fiona_handle_gladiator_win(
        &mut self,
        fiona_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        events: &mut Vec<FionaOutcomeEvent>,
    ) {
        let killer_id = CharacterId(message.dat3.max(0) as u32);
        if killer_id.0 == 0 {
            return;
        }
        let Some(killer) = self.characters.get(&killer_id).cloned() else {
            return;
        };
        if !killer.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(facts) = player_facts.get(&killer_id) else {
            return;
        };
        if !(7..=16).contains(&facts.fiona_state) {
            return;
        }
        events.push(FionaOutcomeEvent::UpdateFionaState {
            player_id: killer_id,
            new_state: facts.fiona_state + 1,
        });
        self.npc_say(fiona_id, &format!("Well done, {}.", killer.name));
        self.teleport_char_driver(killer_id, 15, 235);
    }

    /// C `fiona_driver`'s `NT_CHAR` branch (`arkhata.c:838-919`).
    #[allow(clippy::too_many_arguments)]
    fn fiona_handle_char_message(
        &mut self,
        fiona_id: CharacterId,
        data: &mut FionaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        events: &mut Vec<FionaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(fiona) = self.characters.get(&fiona_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:839`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:845`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:851`).
        if tick < data.last_talk + FIONA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:856`).
        if tick < data.last_talk + FIONA_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:862`).
        if fiona_id == player_id || !char_see_char(&fiona, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:868`).
        if char_dist(&fiona, &player) > FIONA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.fiona_state;
        match facts.fiona_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:877-
            // 887`) - only if the player is high enough level.
            0 if player.level >= FIONA_MIN_LEVEL_FOR_TRAINING => {
                self.npc_say(
                    fiona_id,
                    "Hello there stranger, and welcome to my Academy of the Fighting Arts. I can train your abillities here if you prove thine worth first.",
                );
                events.push(FionaOutcomeEvent::QuestOpen67 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:888-893`).
            2 => {
                self.npc_say(
                    fiona_id,
                    "I ventured into the vampire lair a few days ago, and suddenly my ring disappeared. And my students need me here so I can't retrieve it myself. Please bring it back to me.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3: break;` (`arkhata.c:894-895`): waiting for the ring.
            3 => {}
            // C `case 4:` (`arkhata.c:897-900`).
            4 => {
                self.npc_say(fiona_id, "Thank thee ever so much.");
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:901-913`).
            5 => {
                if player.level > FIONA_STUDENT_LEVEL_CEILING {
                    self.npc_say(
                        fiona_id,
                        "I was going to offer thee a chance to prove thyself against my students, but they are no challenge for thee.",
                    );
                    new_state = 19;
                } else {
                    self.npc_say(
                        fiona_id,
                        "I will now allow you to test your skills against my students. If you defeat them all I will raise one of your skills by 2 points for the price of 10000gold.",
                    );
                    new_state = 6;
                }
                didsay = true;
            }
            // C `case 6:` (`arkhata.c:914-919`).
            6 => {
                self.npc_say(
                    fiona_id,
                    &format!(
                        "To fight my student say {COL_STR_LIGHT_BLUE}enter{COL_STR_RESET}! (Offer valid only for level 80 and below)"
                    ),
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7..16: break;` (`arkhata.c:920-940`): fighting
            // gladiators 1-10.
            7..=16 => {}
            // C `case 17:` (`arkhata.c:941-951`).
            17 => {
                if player.level <= FIONA_STUDENT_LEVEL_CEILING {
                    self.npc_say(
                        fiona_id,
                        &format!(
                            "Well done, {}. So what skill does thou want raised? Please say 'raise skill name' (like, for example, 'raise attack'). This will not increase your skills past the usual maxes.",
                            player.name
                        ),
                    );
                    new_state = 18;
                } else {
                    self.npc_say(
                        fiona_id,
                        &format!(
                            "Nicely done, {}. But then, it wasn't a big challenge, now, was it?",
                            player.name
                        ),
                    );
                    new_state = 19;
                }
                didsay = true;
            }
            // C `case 18: break;` (`arkhata.c:952-953`): waiting for skill
            // choice.
            18 => {}
            // C `case 19:` (`arkhata.c:954-958`).
            19 => {
                self.npc_say(fiona_id, &format!("Fare thee well, {}.", player.name));
                new_state = 20;
                didsay = true;
            }
            // C `case 20: break;` (`arkhata.c:959-960`): all done.
            20 => {}
            _ => {}
        }

        if new_state != facts.fiona_state {
            events.push(FionaOutcomeEvent::UpdateFionaState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:962-966`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `fiona_driver`'s `NT_TEXT` branch (`arkhata.c:968-1025`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn fiona_handle_text_message(
        &mut self,
        fiona_id: CharacterId,
        fiona_name: &str,
        data: &mut FionaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        events: &mut Vec<FionaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`arkhata.c:971-973`).
        let tick = self.tick.0;
        if tick > data.last_talk + FIONA_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:975`).
        if data.current_victim.is_some() && data.current_victim != Some(speaker_id) {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        let Some(fiona) = self.characters.get(&fiona_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if fiona_id == speaker_id {
            return;
        }
        if char_dist(&fiona, &speaker) > FIONA_QA_DISTANCE
            || !char_see_char(&fiona, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let fiona_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.fiona_state)
            .unwrap_or(0);

        let mut didsay = false;
        let mut matched_code = 0;
        match analyse_text_qa(text, fiona_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(fiona_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:986-1006`): rewind to the
            // start of whichever mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if fiona_state <= 3 {
                    data.last_talk = 0;
                    events.push(FionaOutcomeEvent::UpdateFionaState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                if (5..=7).contains(&fiona_state) {
                    data.last_talk = 0;
                    events.push(FionaOutcomeEvent::UpdateFionaState {
                        player_id: speaker_id,
                        new_state: 5,
                    });
                }
                if (17..=18).contains(&fiona_state) {
                    data.last_talk = 0;
                    events.push(FionaOutcomeEvent::UpdateFionaState {
                        player_id: speaker_id,
                        new_state: 17,
                    });
                }
                if (19..=20).contains(&fiona_state) {
                    data.last_talk = 0;
                    events.push(FionaOutcomeEvent::UpdateFionaState {
                        player_id: speaker_id,
                        new_state: 19,
                    });
                }
                didsay = true;
            }
            // "enter" (`arkhata.c:1007-1010`): challenge the next student.
            TextAnalysisOutcome::Matched(5) => {
                if (7..=16).contains(&fiona_state) {
                    events.push(FionaOutcomeEvent::FightStudent {
                        fiona_id,
                        player_id: speaker_id,
                        nr: fiona_state - 6,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(other) => {
                didsay = true;
                matched_code = other;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay > 100 && didsay < 200)` (`arkhata.c:1017-1025`),
        // outside (not part of) the `switch` above.
        if matched_code > 100 && matched_code < 200 {
            if fiona_state == 18 {
                if self.fiona_raise(fiona_id, speaker_id, (matched_code - 100) as usize) {
                    events.push(FionaOutcomeEvent::UpdateFionaState {
                        player_id: speaker_id,
                        new_state: 19,
                    });
                }
            } else {
                self.npc_say(fiona_id, "That offer is not open at the moment.");
            }
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:1023-1025`) - note this does *not* touch `dat->
        // last_talk` (except the explicit "repeat" resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `fiona_driver`'s `NT_GIVE` branch (`arkhata.c:1028-1055`).
    fn fiona_handle_give_message(
        &mut self,
        fiona_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, FionaPlayerFacts>,
        events: &mut Vec<FionaOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&fiona_id)
            .and_then(|fiona| fiona.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        // C `if (it[in].ID == IID_ARKHATA_RING && ppd->fiona_state == 3)`
        // (`arkhata.c:1041`): the ring vanishes silently on success (no
        // `say`/`quiet_say` call at all).
        if item.template_id == IID_ARKHATA_RING && facts.is_some_and(|facts| facts.fiona_state == 3)
        {
            events.push(FionaOutcomeEvent::QuestDone67 {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_RING);
            events.push(FionaOutcomeEvent::UpdateFionaState {
                player_id: giver_id,
                new_state: 4,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:1048-1052`): hand the
        // item back to the giver.
        self.npc_say(
            fiona_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `fiona_raise(cn, co, v)` (`arkhata.c:806-822`) - see the module
    /// doc comment for the doubled-`raise_value_exp`-call quirk.
    fn fiona_raise(&mut self, fiona_id: CharacterId, player_id: CharacterId, value: usize) -> bool {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return false;
        };
        // C `if (ch[co].gold < 10000*100)` (`arkhata.c:807-810`).
        if player.gold < FIONA_RAISE_FEE {
            self.npc_quiet_say(fiona_id, "Sorry, it seems you cannot pay me.");
            return false;
        }
        let Some(character_value) = character_value_from_index(value) else {
            return false;
        };

        let first_raise = self
            .characters
            .get_mut(&player_id)
            .and_then(|character| crate::item_driver::raise_value_exp(character, value));

        if first_raise.is_some() {
            // C calls `raise_value_exp(co, v)` a second time - a real
            // observable quirk, see the module doc comment.
            let _ = self
                .characters
                .get_mut(&player_id)
                .and_then(|character| crate::item_driver::raise_value_exp(character, value));
            self.queue_system_text(
                player_id,
                format!("You gained {}.", full_skill_name(character_value)),
            );
            if let Some(player_mut) = self.characters.get_mut(&player_id) {
                player_mut.gold = player_mut.gold.saturating_sub(FIONA_RAISE_FEE);
                player_mut.flags.insert(CharacterFlags::ITEMS);
            }
            true
        } else {
            self.npc_say(
                fiona_id,
                &format!(
                    "You cannot raise the skill {}. Please choose a different one, {}.",
                    full_skill_name(character_value),
                    player.name
                ),
            );
            false
        }
    }

    /// C `fight_student`'s "is the arena busy" scan (`arkhata.c:759-765`):
    /// pure bounding-box occupancy check, exposed so `ugaris-server`'s
    /// `area37::spawn_gladiator_student` doesn't have to reimplement it.
    pub fn arkhata_arena_is_busy(&self) -> bool {
        for x in FIGHT_STUDENT_ARENA_X {
            for y in FIGHT_STUDENT_ARENA_Y {
                if self.map.tile(x, y).is_some_and(|tile| tile.character != 0) {
                    return true;
                }
            }
        }
        false
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_FIONA;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `fiona_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FionaDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_FIONA_RING`] to `ugaris-server`'s `apply_fiona_events`.
pub const fn qlog_fiona_ring() -> usize {
    QLOG_FIONA_RING
}
