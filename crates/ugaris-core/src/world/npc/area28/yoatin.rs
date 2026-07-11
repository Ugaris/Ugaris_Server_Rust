//! Yoatin NPC (`CDR_YOATIN`), the timid hunter in Brannington Forest who
//! runs "Bear Hunt - Again" (quest 39).
//!
//! Ports `src/area/28/brannington_forest.c::yoatin_driver` (`:432-632`)
//! plus its shared `analyse_text_driver`/`qa[]` table (`:75-199`), ported
//! as [`super::AREA28_QA`] in `world::npc::area28` (the same table
//! `world::npc::area28::aristocrat` shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area26::
//! smugglecom`/`rouven`: the caller supplies a per-player fact snapshot
//! ([`YoatinPlayerFacts`]) up front and applies the returned
//! [`YoatinOutcomeEvent`]s afterwards, since `staffer_ppd.yoatin_state` and
//! the `QLOG` 39 quest-log entry live on `crate::player::PlayerRuntime`,
//! not `World`.
//!
//! `yoatin_driver`'s ten-state (`0`-`9`) dialogue chain: greeting -> "you
//! must be [player], my brother Yoakin told me of you" -> "he mentioned you
//! slew the bears of Cameron" -> "could you help me?" -> "a family asked me
//! to hunt down a bear" -> "bears scare me" -> "bring me proof and I'll
//! reward thee" -> "take care, the forest is full of bears" -> (`NT_GIVE`:
//! hand in `IID_STAFF_BEARHEAD`, quest 39 done, state jumps to `9`,
//! unconditionally grants a `WS_Hunter_Belt` - unlike `world::npc::area28::
//! aristocrat`'s gold, C never gates this reward on a first-completion
//! count) -> done.
//!
//! Deviations/gaps (documented, not silent):
//! - Unlike `world::thomas`/`world::sir_jones`'s `NT_TEXT` branch (but like
//!   `world::npc::area26::smugglecom`/`rouven`'s and `world::npc::area28::
//!   aristocrat`'s), this driver's own C body has no `dat->current_victim`
//!   staleness-reset preamble and no victim-mismatch early-out at all -
//!   reproduced verbatim: replies to *any* nearby player's matched small
//!   talk, not just its tracked victim.
//! - Like `world::npc::area28::aristocrat`'s own `case 3` (but unlike
//!   `world::npc::area26::smugglecom`'s silent one), this driver's `case 3`
//!   (`:562-567`) speaks a visible `say(cn, "reset done")` line (not
//!   `quiet_say`) before wiping the state - ported via
//!   [`crate::world::World::npc_say`].
//! - No self-defense/regen/spell-self cascade exists in C's `yoatin_
//!   driver` body at all (matching `world::astro1`/`world::npc::area26::
//!   smugglecom`/`world::npc::area28::aristocrat`'s identical observation
//!   for other "pure talker" NPCs) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:631`) is not
//!   ported, matching the established `world::thomas`/`world::sir_jones`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA28_QA;

/// C `char_dist(cn, co) > 10` (`brannington_forest.c:481`).
const YOATIN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington_forest.c:120`, the shared
/// `analyse_text_driver` copy's own guard).
const YOATIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington_forest.c:464`).
const YOATIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington_forest.c:469`).
const YOATIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington_forest.c:625`): idle "return to post"
/// threshold.
const YOATIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_yoatin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YoatinPlayerFacts {
    /// `PlayerRuntime::staffer_yoatin_state()`.
    pub yoatin_state: i32,
}

/// A side effect [`World::process_yoatin_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoatinOutcomeEvent {
    /// Write the new `staffer_ppd.yoatin_state` back.
    UpdateYoatinState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 39)`.
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 39); ... if ((in = create_item("WS_Hunter_
    /// Belt"))) { give_char_item(co, in); }` (`brannington_forest.c:589-
    /// 595`) - applied via the standard `complete_legacy` flow (real
    /// quest-table exp) plus an unconditional belt grant on every
    /// completion (unlike `world::npc::area28::aristocrat`'s `times_done ==
    /// 1`-gated gold, C never gates this one).
    QuestDone { player_id: CharacterId },
    /// C `case 3:` (`brannington_forest.c:562-567`): the god-only "reset
    /// me" state wipe.
    ResetYoatin { player_id: CharacterId },
}

impl World {
    /// C `yoatin_driver`'s per-tick body (`brannington_forest.c:432-632`).
    pub fn process_yoatin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, YoatinPlayerFacts>,
        area_id: u16,
    ) -> Vec<YoatinOutcomeEvent> {
        let yoatin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_YOATIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for yoatin_id in yoatin_ids {
            self.process_yoatin_messages(yoatin_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_yoatin_messages(
        &mut self,
        yoatin_id: CharacterId,
        player_facts: &HashMap<CharacterId, YoatinPlayerFacts>,
        area_id: u16,
        events: &mut Vec<YoatinOutcomeEvent>,
    ) {
        let Some(yoatin_name) = self
            .characters
            .get(&yoatin_id)
            .map(|yoatin| yoatin.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Yoatin(mut data)) = self
            .characters
            .get(&yoatin_id)
            .and_then(|yoatin| yoatin.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&yoatin_id)
            .map(|yoatin| std::mem::take(&mut yoatin.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.yoatin_handle_char_message(
                    yoatin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.yoatin_handle_text_message(
                    yoatin_id,
                    &yoatin_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.yoatin_handle_give_message(yoatin_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(yoatin) = self.characters.get_mut(&yoatin_id) {
            yoatin.driver_state = Some(CharacterDriverState::Yoatin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington_forest.c:621-
        // 623`).
        if let (Some(yoatin), Some((tx, ty))) =
            (self.characters.get(&yoatin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(yoatin.x), i32::from(yoatin.y), tx, ty) {
                if let Some(yoatin_mut) = self.characters.get_mut(&yoatin_id) {
                    let _ = turn(yoatin_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`brannington_forest.c:625-629`). The NPC's
        // post position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the
        // same substitution `world::npc::area26::smugglecom` already uses.
        let last_talk = if let Some(yoatin) = self.characters.get(&yoatin_id) {
            match yoatin.driver_state.as_ref() {
                Some(CharacterDriverState::Yoatin(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + YOATIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(yoatin) = self.characters.get(&yoatin_id) else {
                return;
            };
            let (post_x, post_y) = (yoatin.rest_x, yoatin.rest_y);
            self.secure_move_driver(
                yoatin_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `yoatin_driver`'s `NT_CHAR` branch (`brannington_forest.c:447-547`).
    #[allow(clippy::too_many_arguments)]
    fn yoatin_handle_char_message(
        &mut self,
        yoatin_id: CharacterId,
        data: &mut YoatinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoatinPlayerFacts>,
        events: &mut Vec<YoatinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(yoatin) = self.characters.get(&yoatin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington_forest.c:451-455`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington_forest.c:457-461`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington_forest.c:463-467`).
        if tick < data.last_talk + YOATIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington_forest.c:469-472`).
        if tick < data.last_talk + YOATIN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington_forest.c:475-478`).
        if yoatin_id == player_id || !char_see_char(&yoatin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;`
        // (`brannington_forest.c:481-484`).
        if char_dist(&yoatin, &player) > YOATIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.yoatin_state;
        match facts.yoatin_state {
            // C `case 0:` (`brannington_forest.c:491-496`).
            0 => {
                self.npc_quiet_say(yoatin_id, "Greetings stranger!");
                events.push(YoatinOutcomeEvent::QuestOpen { player_id });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington_forest.c:497-502`).
            1 => {
                self.npc_quiet_say(
                    yoatin_id,
                    &format!(
                        "Wait...I recognize you from the description my brother gave - you must be {}!",
                        player.name
                    ),
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington_forest.c:503-508`).
            2 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "My brother's name is Yoakin. It seems you did him a great service slaying the bears of Cameron.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington_forest.c:509-513`).
            3 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "Mayhap you could assist me with a problem I have?",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`brannington_forest.c:514-519`).
            4 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "A family from the town beyond this forest has asked me to hunt down the bear that killed their son.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`brannington_forest.c:520-525`).
            5 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "I am not quite the hunter my brother is and well... to be frank, bears scare the living daylights out of me.",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` (`brannington_forest.c:526-530`).
            6 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "If you could fetch me proof of the bear being slain, I would reward thee greatly.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`brannington_forest.c:531-535`).
            7 => {
                self.npc_quiet_say(
                    yoatin_id,
                    "Take care as you travel! The whole forest is full of bears and bear caves.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8: break;` (`brannington_forest.c:536-537`): waiting
            // for the player to hand in the bear head.
            8 => {}
            // C `case 9: break;` (`brannington_forest.c:538-539`): quest
            // chain done.
            9 => {}
            _ => {}
        }

        if new_state != facts.yoatin_state {
            events.push(YoatinOutcomeEvent::UpdateYoatinState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington_forest.c:541-545`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `yoatin_driver`'s `NT_TEXT` branch (`brannington_forest.c:550-
    /// 574`), wired through the generic `analyse_text_qa` matcher (same
    /// pattern as `world::npc::area26::smugglecom`/`world::npc::area28::
    /// aristocrat`'s text handlers). This branch has no victim-staleness-
    /// reset preamble and no victim-mismatch early-out (see the module doc
    /// comment).
    #[allow(clippy::too_many_arguments)]
    fn yoatin_handle_text_message(
        &mut self,
        yoatin_id: CharacterId,
        yoatin_name: &str,
        data: &mut YoatinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoatinPlayerFacts>,
        events: &mut Vec<YoatinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington_forest.c:553`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses
        // (`brannington_forest.c:101-126`): ignore our own talk, non-
        // players, distance > 12, not-visible.
        if yoatin_id == speaker_id {
            return;
        }
        let Some(yoatin) = self.characters.get(&yoatin_id).cloned() else {
            return;
        };
        if char_dist(&yoatin, &speaker) > YOATIN_QA_DISTANCE
            || !char_see_char(&yoatin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let yoatin_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.yoatin_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, yoatin_name, &speaker.name, AREA28_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(yoatin_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington_forest.c:556-561`): reset back to
            // the greeting if not yet past it.
            TextAnalysisOutcome::Matched(2) => {
                if yoatin_state <= 8 {
                    data.last_talk = 0;
                    events.push(YoatinOutcomeEvent::UpdateYoatinState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington_forest.c:562-567`): the god-only
            // "reset me" wipe, which speaks a visible `say(cn, "reset
            // done")` line first (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(yoatin_id, "reset done");
                    events.push(YoatinOutcomeEvent::ResetYoatin {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by yoatin's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington_forest.c:569-572`) - note this does *not* touch
        // `dat->last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `yoatin_driver`'s `NT_GIVE` branch (`brannington_forest.c:577-
    /// 611`).
    fn yoatin_handle_give_message(
        &mut self,
        yoatin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoatinPlayerFacts>,
        events: &mut Vec<YoatinOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&yoatin_id)
            .and_then(|yoatin| yoatin.cursor_item.take())
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

        // C `if (it[in].ID == IID_STAFF_BEARHEAD && ppd &&
        // ppd->yoatin_state <= 8)` (`brannington_forest.c:584`).
        if item.template_id == IID_STAFF_BEARHEAD
            && is_player
            && facts.is_some_and(|facts| facts.yoatin_state <= 8)
        {
            self.npc_quiet_say(
                yoatin_id,
                &format!(
                    "Thank you {}! This will be perfect proof. Here, take my belt, you are clearly the greater hunter!",
                    giver.name
                ),
            );
            events.push(YoatinOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BEARHEAD);
            events.push(YoatinOutcomeEvent::UpdateYoatinState {
                player_id: giver_id,
                new_state: 9,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`brannington_forest.c:597-602`): hand
        // the item back to the giver.
        self.npc_say(
            yoatin_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_YOATIN};
use crate::item_driver::IID_STAFF_BEARHEAD;

/// C `struct yoatin_data` (`src/area/28/brannington_forest.c:426-430`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YoatinDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
