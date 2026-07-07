//! Robber-operations quest NPC (`CDR_JESSICA`), area 1's Cameron-village
//! camp dweller running the two-quest robber chain.
//!
//! Ports `src/area/1/gwendylon.c::jessica_driver` (`:1809-2065`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`/`world::gwendylon`).
//! Follows the same `World`/`PlayerRuntime` split established there: the
//! caller supplies a per-player fact snapshot ([`JessicaPlayerFacts`]) up
//! front and applies the returned [`JessicaOutcomeEvent`]s afterwards,
//! since `jessica_state`/`jessica_seen_timer` (`area1_ppd` fields) and the
//! two `QLOG_JESSICA_*` quest-log entries live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - The `JESSICA_STATE_QUEST2_DO` -> `JESSICA_STATE_QUEST2_FINISH`
//!   transition is driven by a separate death hook, `bredel_dead(cn, co)`
//!   (`gwendylon.c:2825-2842`, "the local robber leader" boss monster),
//!   not by anything in `jessica_driver` itself. That hook - and the
//!   `monster_dead`-style death-dispatch table it would need to be wired
//!   from - is not ported anywhere in this codebase yet (same documented
//!   gap as `world::camhermit`'s `monster_dead` bear-kill counter), so a
//!   player can reach `JESSICA_STATE_QUEST2_DO` but the chain cannot yet
//!   advance to `QUEST2_FINISH` on a live server. Once `jessica_state` is
//!   set to `JESSICA_STATE_QUEST2_FINISH` by a future port of that hook,
//!   this driver's own `NT_CHAR` handling already closes out the quest
//!   correctly (tested directly via the fact snapshot).
//! - The `JESSICA_STATE_QUEST1_DO`/`QUEST2_DO` reminder lines wrap
//!   "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers in C (`gwendylon.c:
//!   1925-1926`, `1956`); dropped here for the same reason documented on
//!   `world::camhermit`'s module doc comment (`World::npc_quiet_say`
//!   broadcasts a plain UTF-8 `String`).
//! - The `NT_GIVE` "unwanted item" give-back (`gwendylon.c:2038-2043`)
//!   calls plain `give_char_item`, not `give_char_item_smart` like every
//!   other area-1 NPC in this file - a genuine C behavioral difference
//!   (no ground-drop fallback on a full inventory), preserved here via
//!   `World::give_char_item` rather than "fixed" to match its siblings.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_JESSICA, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA1_ROBBER2NOTE;
use crate::quest::{
    JESSICA_STATE_QUEST1_DO, JESSICA_STATE_QUEST1_FINISH, JESSICA_STATE_QUEST1_GIVE_1,
    JESSICA_STATE_QUEST2_DO, JESSICA_STATE_QUEST2_FINISH, JESSICA_STATE_QUEST2_GIVE_1,
    QLOG_JESSICA_KILL, QLOG_JESSICA_ROBBER_NOTE,
};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`gwendylon.c:1858`): the `NT_CHAR` greeting
/// range.
const JESSICA_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const JESSICA_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:1841`).
const JESSICA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 20` (`gwendylon.c:1846`, `:1992`).
const JESSICA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 20;
/// C `TICKS * 30` (`gwendylon.c:2058`): idle "return to post" threshold.
const JESSICA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `#define JESSICA_EXTEND_WAIT_TIME 60` (`gwendylon.c:1807`): the
/// shared reminder-line gate for every "waiting" state.
const JESSICA_EXTEND_WAIT_TIME: i32 = 60;

/// C's bare `int` state value for `ppd->jessica_state`'s idle entry state
/// (`src/common/npc_states.h:84`) - not itself needed by
/// `crate::quest::init_area1_quests`, so it lives here rather than there.
const JESSICA_STATE_ENTRY: i32 = 0;
const JESSICA_STATE_QUEST1_GIVE_2: i32 = 2;
const JESSICA_STATE_QUEST1_GIVE_3: i32 = 3;
const JESSICA_STATE_QUEST1_GIVE_4: i32 = 4;
const JESSICA_STATE_QUEST1_GIVE_5: i32 = 5;
const JESSICA_STATE_QUEST2_GIVE_2: i32 = 9;
const JESSICA_STATE_DONE: i32 = 12;

/// Per-player facts [`World::process_jessica_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JessicaPlayerFacts {
    /// `PlayerRuntime::area1_jessica_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_jessica_seen_timer()` (C `realtime`
    /// wall-clock seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::quest_log.is_done(QLOG_NOOK)`: the
    /// `JESSICA_STATE_ENTRY` gate on Nook's own quest chain being done
    /// first (`gwendylon.c:1869`).
    pub nook_quest_done: bool,
}

/// A side effect [`World::process_jessica_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JessicaOutcomeEvent {
    /// Write the new `area1_ppd.jessica_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->jessica_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:1978`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, ...)` - the caller must apply
    /// `PlayerRuntime::quest_log.complete_legacy` (exp reward + resend).
    QuestDone {
        player_id: CharacterId,
        quest: usize,
    },
}

impl World {
    /// C `jessica_driver`'s per-tick body (`gwendylon.c:1809-2065`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_jessica_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JessicaPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<JessicaOutcomeEvent> {
        let jessica_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JESSICA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for jessica_id in jessica_ids {
            self.process_jessica_messages(jessica_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_jessica_messages(
        &mut self,
        jessica_id: CharacterId,
        player_facts: &HashMap<CharacterId, JessicaPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<JessicaOutcomeEvent>,
    ) {
        let Some(jessica_name) = self
            .characters
            .get(&jessica_id)
            .map(|jessica| jessica.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Jessica(mut data)) = self
            .characters
            .get(&jessica_id)
            .and_then(|jessica| jessica.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&jessica_id)
            .map(|jessica| std::mem::take(&mut jessica.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.jessica_handle_char_message(
                    jessica_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.jessica_handle_text_message(
                    jessica_id,
                    &jessica_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.jessica_handle_give_message(jessica_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(jessica) = self.characters.get_mut(&jessica_id) {
            jessica.driver_state = Some(CharacterDriverState::Jessica(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:2054-2056`).
        if let (Some(jessica), Some((tx, ty))) =
            (self.characters.get(&jessica_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(jessica.x), i32::from(jessica.y), tx, ty) {
                if let Some(jessica_mut) = self.characters.get_mut(&jessica_id) {
                    let _ = turn(jessica_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:2058-
        // 2064`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::yoakin` already
        // uses for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(jessica) = self.characters.get(&jessica_id) {
            match jessica.driver_state.as_ref() {
                Some(CharacterDriverState::Jessica(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + JESSICA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(jessica) = self.characters.get(&jessica_id) else {
                return;
            };
            let (post_x, post_y) = (jessica.rest_x, jessica.rest_y);
            self.secure_move_driver(
                jessica_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `jessica_driver`'s `NT_CHAR` branch (`gwendylon.c:1825-1986`).
    fn jessica_handle_char_message(
        &mut self,
        jessica_id: CharacterId,
        data: &mut JessicaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JessicaPlayerFacts>,
        now: i32,
        events: &mut Vec<JessicaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(jessica) = self.characters.get(&jessica_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:1829-1832`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:1834-1838`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`gwendylon.c:1840-1844`).
        if tick < data.last_talk + JESSICA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*20 && dat->current_victim
        // != co) continue;` (`gwendylon.c:1846-1849`) - a plain `!=`, so
        // `None` (C's `0`) compares equal to a real `player_id` only if
        // that id itself were `0` (never true for a live character).
        if tick < data.last_talk + JESSICA_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:1851-1855`).
        if jessica_id == player_id
            || !char_see_char(&jessica, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:1857-
        // 1861`).
        if char_dist(&jessica, &player) > JESSICA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        // C `switch (ppd->jessica_state) { ... }` (`gwendylon.c:1867-
        // 1977`).
        if new_state == JESSICA_STATE_ENTRY {
            if !facts.nook_quest_done {
                if now.saturating_sub(facts.seen_timer) > JESSICA_EXTEND_WAIT_TIME {
                    self.npc_quiet_say(
                        jessica_id,
                        &format!(
                            "Hail {}! my name is {}. My cousin Nook has some work for thee. Beware of robbers in the forest, they must have a hideout nearby.",
                            player.name, jessica.name
                        ),
                    );
                    didsay = true;
                }
            } else {
                new_state = JESSICA_STATE_QUEST1_GIVE_1;
            }
        } else if new_state == JESSICA_STATE_QUEST1_GIVE_1 {
            events.push(JessicaOutcomeEvent::QuestOpen {
                player_id,
                quest: QLOG_JESSICA_ROBBER_NOTE,
            });
            self.npc_quiet_say(
                jessica_id,
                &format!(
                    "Hail {}! my name is {}. My cousin Nook has spoken well of you. Spare me a moment of you time, I think we may be able to help each other.",
                    player.name, jessica.name
                ),
            );
            new_state = JESSICA_STATE_QUEST1_GIVE_2;
            didsay = true;
        } else if new_state == JESSICA_STATE_QUEST1_GIVE_2 {
            self.npc_quiet_say(
                jessica_id,
                "It seems the same robbers that stole my cousins hat have set up shop in four other locations East and South of here.",
            );
            new_state = JESSICA_STATE_QUEST1_GIVE_3;
            didsay = true;
        } else if new_state == JESSICA_STATE_QUEST1_GIVE_3 {
            self.npc_quiet_say(
                jessica_id,
                "I see them wander through here every night shortly after midnight...they even had the courage to steal from my camp while I was out hunting at night!",
            );
            new_state = JESSICA_STATE_QUEST1_GIVE_4;
            didsay = true;
        } else if new_state == JESSICA_STATE_QUEST1_GIVE_4 {
            self.npc_quiet_say(
                jessica_id,
                "Maybe you could follow one of them and see if they lead you to one of their other hideouts? I'm sure they've stolen from people other than Nook and myself...who knows what you might find.",
            );
            new_state = JESSICA_STATE_QUEST1_GIVE_5;
            didsay = true;
        } else if new_state == JESSICA_STATE_QUEST1_GIVE_5 {
            self.npc_quiet_say(
                jessica_id,
                "If you manage to find them please take care of those villains for us. Come find me again when you have proof of their operations.",
            );
            didsay = true;
            new_state = JESSICA_STATE_QUEST1_DO;
        } else if new_state == JESSICA_STATE_QUEST1_DO {
            if now.saturating_sub(facts.seen_timer) > JESSICA_EXTEND_WAIT_TIME {
                self.npc_quiet_say(
                    jessica_id,
                    "Hast thou found proof of the robber's operations? Or dost thou want me to repeat mine offer?",
                );
                didsay = true;
            }
            // New state given if note handed to her (see
            // `jessica_handle_give_message`).
        } else if new_state == JESSICA_STATE_QUEST1_FINISH {
            self.npc_quiet_say(
                jessica_id,
                "Thou hast done well in damaging the robber's operations.",
            );
            didsay = true;
            events.push(JessicaOutcomeEvent::QuestDone {
                player_id,
                quest: QLOG_JESSICA_ROBBER_NOTE,
            });
            new_state = JESSICA_STATE_QUEST2_GIVE_1;
        } else if new_state == JESSICA_STATE_QUEST2_GIVE_1 {
            self.npc_quiet_say(
                jessica_id,
                "If thou couldst now slay the leader who issued this order. It would certainly make them think twice about robbing the citizens of Cameron again.",
            );
            didsay = true;
            events.push(JessicaOutcomeEvent::QuestOpen {
                player_id,
                quest: QLOG_JESSICA_KILL,
            });
            new_state = JESSICA_STATE_QUEST2_GIVE_2;
        } else if new_state == JESSICA_STATE_QUEST2_GIVE_2 {
            self.npc_quiet_say(
                jessica_id,
                "Any stolen goods though mayest find shall be yours to keep as a prize.",
            );
            didsay = true;
            new_state = JESSICA_STATE_QUEST2_DO;
        } else if new_state == JESSICA_STATE_QUEST2_DO {
            if now.saturating_sub(facts.seen_timer) > JESSICA_EXTEND_WAIT_TIME {
                self.npc_quiet_say(
                    jessica_id,
                    &format!(
                        "Hello {}, does thou want me to repeat mine offer?",
                        player.name
                    ),
                );
                didsay = true;
            }
            // New state given if boss is killed (see the module doc
            // comment's `bredel_dead` gap).
        } else if new_state == JESSICA_STATE_QUEST2_FINISH {
            self.npc_quiet_say(
                jessica_id,
                "Excellent work, we shall have little trouble with those robbers in the near future.",
            );
            didsay = true;
            events.push(JessicaOutcomeEvent::QuestDone {
                player_id,
                quest: QLOG_JESSICA_KILL,
            });
            new_state = JESSICA_STATE_DONE;
        } else if new_state == JESSICA_STATE_DONE {
            if now.saturating_sub(facts.seen_timer) > JESSICA_EXTEND_WAIT_TIME {
                self.npc_quiet_say(jessica_id, &format!("Hello again {}.", player.name));
                didsay = true;
            }
        }
        // Every other value: no-op, matching C's implicit empty default.

        if new_state != facts.state {
            events.push(JessicaOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->jessica_seen_timer = realtime;` (`gwendylon.c:1978`):
        // unconditional, regardless of `didsay`.
        events.push(JessicaOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:1980-1984`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `jessica_driver`'s `NT_TEXT` branch (`gwendylon.c:1989-2021`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`'s text handler).
    fn jessica_handle_text_message(
        &mut self,
        jessica_id: CharacterId,
        jessica_name: &str,
        data: &mut JessicaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JessicaPlayerFacts>,
        events: &mut Vec<JessicaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*20 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:1992-1994`).
        let tick = self.tick.0;
        if tick > data.last_talk + JESSICA_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:1996-1999`).
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
        if jessica_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(jessica) = self.characters.get(&jessica_id).cloned() else {
            return;
        };
        if char_dist(&jessica, &speaker) > JESSICA_QA_DISTANCE
            || !char_see_char(&jessica, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, jessica_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(jessica_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:2002-2014`).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state >= JESSICA_STATE_QUEST1_GIVE_1
                        && facts.state <= JESSICA_STATE_QUEST1_DO
                    {
                        data.last_talk = 0;
                        events.push(JessicaOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: JESSICA_STATE_QUEST1_GIVE_1,
                        });
                    } else if facts.state >= JESSICA_STATE_QUEST2_GIVE_1
                        && facts.state <= JESSICA_STATE_QUEST2_DO
                    {
                        data.last_talk = 0;
                        events.push(JessicaOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: JESSICA_STATE_QUEST2_GIVE_1,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by jessica's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:2017-2020`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `jessica_driver`'s `NT_GIVE` branch (`gwendylon.c:2025-2046`).
    fn jessica_handle_give_message(
        &mut self,
        jessica_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JessicaPlayerFacts>,
        events: &mut Vec<JessicaOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&jessica_id)
            .and_then(|jessica| jessica.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        if template_id == IID_AREA1_ROBBER2NOTE
            && facts.is_some_and(|facts| {
                facts.state >= JESSICA_STATE_QUEST1_GIVE_1 && facts.state <= JESSICA_STATE_QUEST1_DO
            })
        {
            self.destroy_items_by_template_id(giver_id, IID_AREA1_ROBBER2NOTE);
            self.destroy_item(item_id);
            events.push(JessicaOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: JESSICA_STATE_QUEST1_FINISH,
            });
        } else {
            // C `else { quiet_say(...); if (!give_char_item(co,
            // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem =
            // 0; }` (`gwendylon.c:2037-2043`) - the plain `give_char_item`,
            // not `give_char_item_smart` (see the module doc comment's
            // last bullet).
            self.npc_quiet_say(
                jessica_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct jessica_driver_data` (`src/area/1/gwendylon.c:1802-1805`): the
/// robber-quest NPC's own driver memory (`CDR_JESSICA`, distinct from the
/// per-player `jessica_state`/`jessica_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::jessica`'s
/// module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JessicaDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
