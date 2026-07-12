//! Jaz NPC (`CDR_JAZ`), the Arkhata townsman who runs "Ishtar's Bracelet"
//! (quest 66).
//!
//! Ports `src/area/37/arkhata.c::jaz_driver` (`:571-763`) plus the shared
//! `analyse_text_driver`/`qa[]` table (`:109-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! rammy`: the caller supplies a per-player fact snapshot
//! ([`JazPlayerFacts`]) up front and applies the returned
//! [`JazOutcomeEvent`]s afterwards, since `arkhata_ppd.jaz_state` lives on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `jaz_driver`'s eight-state (`0`-`7`) dialogue chain is entirely local to
//! this file except for its single greeting gate: `0` needs
//! `arkhata_ppd.rammy_state >= 12` (`world::npc::area37::rammy`'s own
//! progress, `case 12` of `rammy_driver`) to advance, then falls through
//! into `case 1`'s speech/`questlog_open(66)`/advance-to-`2` in the same
//! tick - collapsed into one `rs == 0` arm here, same "fallthrough lands on
//! the next case's action" precedent as `world::npc::area37::rammy`'s own
//! `rs == 6`/`13`/`17` arms. State `1` itself is never independently
//! stored (the collapse always lands on `2`), so no separate arm exists for
//! it.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`'s identical observation for that file's
//!   shared driver shape).
//! - `NT_GIVE`'s successful bracelet turn-in (`:719-726`) is silent in C -
//!   no `say`/`quiet_say` call at all. Reproduced verbatim: only the
//!   gold/item fallback branch speaks.
//! - No self-defense/regen/spell-self cascade exists in C's `jaz_driver`
//!   body at all (matching the `rammy_driver`/`brannington.c` "pure
//!   talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:762`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_BRACELET;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:337` sibling drivers).
const JAZ_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const JAZ_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:601`).
const JAZ_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:606`).
const JAZ_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:757`): idle "return to post" threshold.
const JAZ_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 66, "Ishtar's Bracelet".
const QLOG_JAZ_BRACELET: usize = 66;

/// Per-player facts [`World::process_jaz_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JazPlayerFacts {
    /// `PlayerRuntime::arkhata_jaz_state()`.
    pub jaz_state: i32,
    /// `PlayerRuntime::arkhata_rammy_state()` (`ppd->rammy_state`,
    /// `arkhata.c:628`): gates `rs` `0`.
    pub rammy_state: i32,
}

/// A side effect [`World::process_jaz_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JazOutcomeEvent {
    /// Write the new `arkhata_ppd.jaz_state` back.
    UpdateJazState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 66)` (`arkhata.c:635`).
    QuestOpen66 { player_id: CharacterId },
    /// C `questlog_done(co, 66)` (`arkhata.c:720`), the `NT_GIVE` bracelet
    /// turn-in.
    QuestDone66 { player_id: CharacterId },
}

impl World {
    /// C `jaz_driver`'s per-tick body (`arkhata.c:571-763`).
    pub fn process_jaz_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JazPlayerFacts>,
        area_id: u16,
    ) -> Vec<JazOutcomeEvent> {
        let jaz_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JAZ
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for jaz_id in jaz_ids {
            self.process_jaz_messages(jaz_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_jaz_messages(
        &mut self,
        jaz_id: CharacterId,
        player_facts: &HashMap<CharacterId, JazPlayerFacts>,
        area_id: u16,
        events: &mut Vec<JazOutcomeEvent>,
    ) {
        let Some(jaz_name) = self.characters.get(&jaz_id).map(|jaz| jaz.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Jaz(mut data)) = self
            .characters
            .get(&jaz_id)
            .and_then(|jaz| jaz.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&jaz_id)
            .map(|jaz| std::mem::take(&mut jaz.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.jaz_handle_char_message(
                    jaz_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.jaz_handle_text_message(
                    jaz_id,
                    &jaz_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.jaz_handle_give_message(jaz_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(jaz) = self.characters.get_mut(&jaz_id) {
            jaz.driver_state = Some(CharacterDriverState::Jaz(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:753-755`).
        if let (Some(jaz), Some((tx, ty))) = (self.characters.get(&jaz_id).cloned(), face_target) {
            if let Some(direction) = offset2dx(i32::from(jaz.x), i32::from(jaz.y), tx, ty) {
                if let Some(jaz_mut) = self.characters.get_mut(&jaz_id) {
                    let _ = turn(jaz_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:757-761`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(jaz) = self.characters.get(&jaz_id) {
            match jaz.driver_state.as_ref() {
                Some(CharacterDriverState::Jaz(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + JAZ_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(jaz) = self.characters.get(&jaz_id) else {
                return;
            };
            let (post_x, post_y) = (jaz.rest_x, jaz.rest_y);
            self.secure_move_driver(jaz_id, post_x, post_y, Direction::Down as u8, 0, 0, area_id);
        }
    }

    /// C `jaz_driver`'s `NT_CHAR` branch (`arkhata.c:588-668`).
    #[allow(clippy::too_many_arguments)]
    fn jaz_handle_char_message(
        &mut self,
        jaz_id: CharacterId,
        data: &mut JazDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JazPlayerFacts>,
        events: &mut Vec<JazOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(jaz) = self.characters.get(&jaz_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:589`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:595`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:601`).
        if tick < data.last_talk + JAZ_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:606`).
        if tick < data.last_talk + JAZ_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:612`).
        if jaz_id == player_id || !char_see_char(&jaz, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:618`).
        if char_dist(&jaz, &player) > JAZ_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.jaz_state;
        match facts.jaz_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:625-
            // 636`) - see the module doc comment.
            0 if facts.rammy_state >= 12 => {
                self.npc_quiet_say(
                    jaz_id,
                    "Welcome to my home, fellow adventurer. Couldst thou please spare a moment?",
                );
                events.push(JazOutcomeEvent::QuestOpen66 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:637-641`).
            2 => {
                self.npc_quiet_say(
                    jaz_id,
                    "While I was traveling, I found a bug house. It was full of strong Knogers. When I reached the main room, the Knogers attacked me.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:642-647`).
            3 => {
                self.npc_quiet_say(
                    jaz_id,
                    "I had to run for my life, but accidently, my bracelet fell off my hand. It holds the insignia of Ishtar and has been passed down through generations in my family.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:648-652`).
            4 => {
                self.npc_quiet_say(
                    jaz_id,
                    "Now, dear fellow, I'll ask thee to cross bridge to the north-east and go into the hut. Defeat the Knoger who has my bracelet and return it to me.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break;` (`arkhata.c:653-654`): waiting for the
            // bracelet, handled by `NT_GIVE`.
            5 => {}
            // C `case 6:` (`arkhata.c:655-659`).
            6 => {
                self.npc_quiet_say(
                    jaz_id,
                    &format!(
                        "Thank thee so much, I will call thee my {} from now on.",
                        if player.flags.contains(CharacterFlags::MALE) {
                            "brother"
                        } else {
                            "sister"
                        }
                    ),
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7: break;` (`arkhata.c:660-661`): all done.
            7 => {}
            _ => {}
        }

        if new_state != facts.jaz_state {
            events.push(JazOutcomeEvent::UpdateJazState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:663-667`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `jaz_driver`'s `NT_TEXT` branch (`arkhata.c:670-707`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn jaz_handle_text_message(
        &mut self,
        jaz_id: CharacterId,
        jaz_name: &str,
        data: &mut JazDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JazPlayerFacts>,
        events: &mut Vec<JazOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:673-675`).
        let tick = self.tick.0;
        if tick > data.last_talk + JAZ_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:677`).
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
        let Some(jaz) = self.characters.get(&jaz_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if jaz_id == speaker_id {
            return;
        }
        if char_dist(&jaz, &speaker) > JAZ_QA_DISTANCE
            || !char_see_char(&jaz, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let jaz_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.jaz_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, jaz_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(jaz_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:682-701`): rewind to the start
            // of whichever mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if jaz_state <= 5 {
                    data.last_talk = 0;
                    events.push(JazOutcomeEvent::UpdateJazState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                } else if (6..=7).contains(&jaz_state) {
                    data.last_talk = 0;
                    events.push(JazOutcomeEvent::UpdateJazState {
                        player_id: speaker_id,
                        new_state: 6,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by jaz's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:703-706`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `jaz_driver`'s `NT_GIVE` branch (`arkhata.c:710-732`).
    fn jaz_handle_give_message(
        &mut self,
        jaz_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JazPlayerFacts>,
        events: &mut Vec<JazOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&jaz_id)
            .and_then(|jaz| jaz.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_BRACELET && ppd->jaz_state == 5)`
        // (`arkhata.c:719`): silent on success - see the module doc
        // comment.
        if item.template_id == IID_ARKHATA_BRACELET
            && is_player
            && facts.is_some_and(|facts| facts.jaz_state == 5)
        {
            events.push(JazOutcomeEvent::QuestDone66 {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_BRACELET);
            events.push(JazOutcomeEvent::UpdateJazState {
                player_id: giver_id,
                new_state: 6,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:729-732`): hand the item
        // back to the giver.
        self.npc_say(
            jaz_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_JAZ;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `jaz_driver` itself - no field for it here, same "only port
/// fields the driver actually uses" precedent as `world::npc::area37::
/// rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JazDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_JAZ_BRACELET`] to `ugaris-server`'s `apply_jaz_events`.
pub const fn qlog_jaz_bracelet() -> usize {
    QLOG_JAZ_BRACELET
}
