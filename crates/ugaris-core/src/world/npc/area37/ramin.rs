//! Ramin NPC (`CDR_RAMIN`), the Arkhata civil officer who runs "A
//! Shopkeeper's Fright" (quest 68).
//!
//! Ports `src/area/37/arkhata.c::ramin_driver` (`:1338-1582`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! rammy`/`jaz`: the caller supplies a per-player fact snapshot
//! ([`RaminPlayerFacts`]) up front and applies the returned
//! [`RaminOutcomeEvent`]s afterwards, since `arkhata_ppd.ramin_state` and
//! sibling fields live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `ramin_driver`'s seventeen-state (`0`-`16`) dialogue chain, gated at
//! three points on cross-driver state this file cannot see directly (all
//! read via [`RaminPlayerFacts`]):
//! - `0` needs `arkhata_ppd.fiona_state >= 4` (`world::npc::area37::
//!   fiona`'s own progress) to advance; C's own `case 0` falls through
//!   into `case 1`'s speech/`questlog_open(68)`/advance-to-`2` in the
//!   same tick - collapsed into one `rs == 0` arm here, same "fallthrough
//!   lands on the next case's action" precedent as `world::npc::area37::
//!   rammy`'s own `rs == 6`/`13`/`17` arms. State `1` itself is never
//!   independently stored (the collapse always lands on `2`), so no
//!   separate arm exists for it.
//! - `6` is a pure wait state: `arkhataskelly_driver`'s own death hook
//!   (`world_events::death_hooks::apply_arkhataskelly_death_from_hurt_
//!   event`) advances `ramin_state` from `6` to `7` directly once the
//!   player has killed the Fighting School's skeletons - this file never
//!   drives that transition itself.
//! - `9` needs `ch[co].level >= 54 && arkhata_ppd.monk_state >= 20` (the
//!   still-unported `arkhatamonk_driver`'s own progress) to advance, then
//!   falls through into `case 10`'s speech/advance-to-`11` the same way -
//!   collapsed into one `rs == 9` arm.
//! - `10`'s dialogue is itself conditional on `arkhata_ppd.rammy_state <
//!   14` but the state increment/`didsay` happen unconditionally either
//!   way - reproduced verbatim (same "conditional dialogue, unconditional
//!   state advance" quirk as `world::npc::area29::guardbran`'s own `case
//!   0`).
//! - `11` needs `ch[co].level >= 60 && arkhata_ppd.rammy_state >= 18`
//!   (`world::npc::area37::rammy`'s own progress) to advance, then falls
//!   through into `case 12`'s speech/advance-to-`13` the same way -
//!   collapsed into one `rs == 11` arm.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`jaz`'s identical observation for that
//!   file's shared driver shape).
//! - `NT_GIVE`'s letter-2 handling (`:1547-1562`) is the only
//!   `arkhata_ppd` write this driver itself performs directly (every
//!   other field it reads is written by a sibling driver): a successful
//!   turn-in speaks (`quiet_say`, unlike `rammy`/`jaz`'s silent quest-item
//!   success) and sets `letter_bits |= 2`, consumed by the still-unported
//!   `judge_driver`'s own gate and by the now-ported `world::npc::
//!   area37::rammy`'s `letter_bits == (2|4|8)` check at its own `rs`
//!   `17`. The fallback branch (wrong item, or the bit already set) uses
//!   `say`, matching `rammy`/`jaz`'s own fallback precedent exactly.
//! - No self-defense/regen/spell-self cascade exists in C's `ramin_driver`
//!   body at all (matching the `rammy`/`jaz` "pure talker" NPC precedent)
//!   - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1581`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_LETTER2;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:1387`, sibling drivers' own
/// identical guard).
const RAMIN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const RAMIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:1370`).
const RAMIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:1375`).
const RAMIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:1575`): idle "return to post" threshold.
const RAMIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 68, "A Shopkeeper's Fright".
const QLOG_RAMIN_SHOPKEEPER: usize = 68;
/// C `ppd->letter_bits |= 2` / `!(ppd->letter_bits & 2)` (`arkhata.c:
/// 1548-1552`).
const RAMIN_LETTER2_BIT: i32 = 2;

/// Per-player facts [`World::process_ramin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaminPlayerFacts {
    /// `PlayerRuntime::arkhata_ramin_state()`.
    pub ramin_state: i32,
    /// `PlayerRuntime::arkhata_fiona_state()` (`ppd->fiona_state`,
    /// `arkhata.c:1398`): gates `rs` `0`.
    pub fiona_state: i32,
    /// `PlayerRuntime::arkhata_monk_state()` (`ppd->monk_state`,
    /// `arkhata.c:1447`): gates `rs` `9`.
    pub monk_state: i32,
    /// `PlayerRuntime::arkhata_rammy_state()` (`ppd->rammy_state`,
    /// `arkhata.c:1453`/`:1462`): read at `rs` `10`, gates `rs` `11`.
    pub rammy_state: i32,
    /// `PlayerRuntime::arkhata_letter_bits()` (`ppd->letter_bits`,
    /// `arkhata.c:1548`): gates the `NT_GIVE` letter-2 turn-in.
    pub letter_bits: i32,
}

/// A side effect [`World::process_ramin_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaminOutcomeEvent {
    /// Write the new `arkhata_ppd.ramin_state` back.
    UpdateRaminState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 68)` (`arkhata.c:1406`).
    QuestOpen68 { player_id: CharacterId },
    /// C `ppd->letter_bits |= 2` (`arkhata.c:1552`), the `NT_GIVE`
    /// letter-2 turn-in.
    GiveLetter2Bit { player_id: CharacterId },
}

impl World {
    /// C `ramin_driver`'s per-tick body (`arkhata.c:1338-1582`).
    pub fn process_ramin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, RaminPlayerFacts>,
        area_id: u16,
    ) -> Vec<RaminOutcomeEvent> {
        let ramin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_RAMIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for ramin_id in ramin_ids {
            self.process_ramin_messages(ramin_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_ramin_messages(
        &mut self,
        ramin_id: CharacterId,
        player_facts: &HashMap<CharacterId, RaminPlayerFacts>,
        area_id: u16,
        events: &mut Vec<RaminOutcomeEvent>,
    ) {
        let Some(ramin_name) = self
            .characters
            .get(&ramin_id)
            .map(|ramin| ramin.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Ramin(mut data)) = self
            .characters
            .get(&ramin_id)
            .and_then(|ramin| ramin.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&ramin_id)
            .map(|ramin| std::mem::take(&mut ramin.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.ramin_handle_char_message(
                    ramin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.ramin_handle_text_message(
                    ramin_id,
                    &ramin_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.ramin_handle_give_message(ramin_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(ramin) = self.characters.get_mut(&ramin_id) {
            ramin.driver_state = Some(CharacterDriverState::Ramin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:1571-1573`).
        if let (Some(ramin), Some((tx, ty))) =
            (self.characters.get(&ramin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(ramin.x), i32::from(ramin.y), tx, ty) {
                if let Some(ramin_mut) = self.characters.get_mut(&ramin_id) {
                    let _ = turn(ramin_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:1575-1579`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(ramin) = self.characters.get(&ramin_id) {
            match ramin.driver_state.as_ref() {
                Some(CharacterDriverState::Ramin(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + RAMIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(ramin) = self.characters.get(&ramin_id) else {
                return;
            };
            let (post_x, post_y) = (ramin.rest_x, ramin.rest_y);
            self.secure_move_driver(
                ramin_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `ramin_driver`'s `NT_CHAR` branch (`arkhata.c:1354-1499`).
    #[allow(clippy::too_many_arguments)]
    fn ramin_handle_char_message(
        &mut self,
        ramin_id: CharacterId,
        data: &mut RaminDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RaminPlayerFacts>,
        events: &mut Vec<RaminOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(ramin) = self.characters.get(&ramin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:1358`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:1364`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:1370`).
        if tick < data.last_talk + RAMIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:1375`).
        if tick < data.last_talk + RAMIN_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:1381`).
        if ramin_id == player_id || !char_see_char(&ramin, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:1387`).
        if char_dist(&ramin, &player) > RAMIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.ramin_state;
        match facts.ramin_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:1397-
            // 1409`) - see the module doc comment.
            0 if facts.fiona_state >= 4 => {
                self.npc_quiet_say(
                    ramin_id,
                    "Hello Great Adventurer! Tidings of thy deed of returning Queen Fiona's ring have reached me. I believe you could help me too.",
                );
                events.push(RaminOutcomeEvent::QuestOpen68 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:1410-1415`).
            2 => {
                self.npc_quiet_say(
                    ramin_id,
                    "As a direct ancestor of High Counsellor Regnior who brough us here, I am in charge of the civil life of the city.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:1416-1421`).
            3 => {
                self.npc_quiet_say(
                    ramin_id,
                    "One of the traders has reported that he found a strange hole in his bedroom corner a few days ago.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:1422-1426`).
            4 => {
                self.npc_quiet_say(
                    ramin_id,
                    "It seems it appeared out of nowhere and he is now afraid to even enter his own bedroom.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:1427-1432`).
            5 => {
                self.npc_quiet_say(
                    ramin_id,
                    "I would have sent a soldier for this but Rammy says he doesn't have any to spare from the fortress. May you go instead, explore this hole and destroy any dangers within it?",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6: break;` (`arkhata.c:1433-1434`): waiting for the
            // player to kill all the Fighting School's skeletons -
            // advanced by `world_events::death_hooks::
            // apply_arkhataskelly_death_from_hurt_event`, not this file.
            6 => {}
            // C `case 7:` (`arkhata.c:1435-1439`).
            7 => {
                self.npc_quiet_say(
                    ramin_id,
                    "The trader and I thank thee for thy help. Now he may sleep at night again.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`arkhata.c:1440-1445`).
            8 => {
                self.npc_quiet_say(
                    ramin_id,
                    "Also, I hear there is something going on in the library, wouldst thou please visit the monks there?",
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9:` falling through into `case 10:` (`arkhata.c:
            // 1446-1460`) - see the module doc comment.
            9 if player.level >= 54 && facts.monk_state >= 20 => {
                // C `case 10:`'s dialogue is conditional but the state
                // advance/`didsay` are not - see the module doc comment.
                if facts.rammy_state < 14 {
                    self.npc_quiet_say(
                        ramin_id,
                        "Greetings my friend! I hear that you have helped the monks. Now Rammy has sent me news about trouble opening the fortress for a trade route, he is in need of thy help again. Please go and talk to him.",
                    );
                }
                new_state = 11;
                didsay = true;
            }
            9 => {}
            // C `case 11:` falling through into `case 12:` (`arkhata.c:
            // 1461-1471`) - see the module doc comment.
            11 if player.level >= 60 && facts.rammy_state >= 18 => {
                self.npc_quiet_say(
                    ramin_id,
                    &format!("Ah, it is good to see you again, {}!", player.name),
                );
                new_state = 13;
                didsay = true;
            }
            11 => {}
            // C `case 13:` (`arkhata.c:1472-1477`).
            13 => {
                self.npc_quiet_say(
                    ramin_id,
                    "I know from the library books, diaries of the early years here, that when we first arrived it was a peaceful and uninhabited place.",
                );
                new_state = 14;
                didsay = true;
            }
            // C `case 14:` (`arkhata.c:1478-1483`).
            14 => {
                self.npc_quiet_say(
                    ramin_id,
                    "Now we have monsters everywhere, it is odd. Seems almost as if a source of some kind generates evil.",
                );
                new_state = 15;
                didsay = true;
            }
            // C `case 15:` (`arkhata.c:1484-1489`).
            15 => {
                self.npc_quiet_say(
                    ramin_id,
                    "Go speak to Jada in the house next door. She is my most trusted officer in matters of the mystical.",
                );
                new_state = 16;
                didsay = true;
            }
            // C `case 16: break;` (`arkhata.c:1490-1491`): all done.
            16 => {}
            _ => {}
        }

        if new_state != facts.ramin_state {
            events.push(RaminOutcomeEvent::UpdateRaminState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:1493-1497`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `ramin_driver`'s `NT_TEXT` branch (`arkhata.c:1502-1539`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn ramin_handle_text_message(
        &mut self,
        ramin_id: CharacterId,
        ramin_name: &str,
        data: &mut RaminDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RaminPlayerFacts>,
        events: &mut Vec<RaminOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:1505-1507`).
        let tick = self.tick.0;
        if tick > data.last_talk + RAMIN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:1509`).
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
        let Some(ramin) = self.characters.get(&ramin_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if ramin_id == speaker_id {
            return;
        }
        if char_dist(&ramin, &speaker) > RAMIN_QA_DISTANCE
            || !char_see_char(&ramin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let ramin_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.ramin_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, ramin_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(ramin_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:1514-1533`): rewind to the
            // start of whichever mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if ramin_state <= 6 {
                    data.last_talk = 0;
                    events.push(RaminOutcomeEvent::UpdateRaminState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                } else if (7..=9).contains(&ramin_state) {
                    data.last_talk = 0;
                    events.push(RaminOutcomeEvent::UpdateRaminState {
                        player_id: speaker_id,
                        new_state: 7,
                    });
                } else if (10..=11).contains(&ramin_state) {
                    data.last_talk = 0;
                    events.push(RaminOutcomeEvent::UpdateRaminState {
                        player_id: speaker_id,
                        new_state: 10,
                    });
                } else if (12..=16).contains(&ramin_state) {
                    data.last_talk = 0;
                    events.push(RaminOutcomeEvent::UpdateRaminState {
                        player_id: speaker_id,
                        new_state: 12,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by ramin's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:1535-1538`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `ramin_driver`'s `NT_GIVE` branch (`arkhata.c:1542-1563`).
    fn ramin_handle_give_message(
        &mut self,
        ramin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RaminPlayerFacts>,
        events: &mut Vec<RaminOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&ramin_id)
            .and_then(|ramin| ramin.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_LETTER2 && ppd && !(ppd->
        // letter_bits & 2))` (`arkhata.c:1548`).
        if item.template_id == IID_ARKHATA_LETTER2
            && is_player
            && facts.is_some_and(|facts| facts.letter_bits & RAMIN_LETTER2_BIT == 0)
        {
            self.npc_quiet_say(
                ramin_id,
                "You bring comfort and solution. My friend I am most grateful.",
            );
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_LETTER2);
            events.push(RaminOutcomeEvent::GiveLetter2Bit {
                player_id: giver_id,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:1556-1561`): hand the
        // item back to the giver.
        self.npc_say(
            ramin_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_RAMIN;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `ramin_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RaminDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_RAMIN_SHOPKEEPER`] to `ugaris-server`'s
/// `apply_ramin_events`.
pub const fn qlog_ramin_shopkeeper() -> usize {
    QLOG_RAMIN_SHOPKEEPER
}
