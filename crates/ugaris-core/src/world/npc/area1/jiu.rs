//! Riverbeast quest-giving pilgrim NPC (`CDR_JIU`), area 1's forest
//! sanctuary hermit.
//!
//! Ports `src/area/1/gwendylon.c::jiu_driver` (`:2074-2247`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`). Follows the same
//! `World`/`PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`JiuPlayerFacts`]) up front and applies the
//! returned [`JiuOutcomeEvent`]s afterwards, since `jiu_state`/
//! `jiu_seen_timer` (both `area1_ppd` fields) and the `QLOG_JIU`
//! quest-log state live on `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `riverbeast_dead(cn, co)` (`gwendylon.c:2255-2272`), the death hook
//!   that advances `JIU_STATE_WAIT_FOR_KILL` -> `JIU_STATE_BEAST_KILLED`
//!   when a player kills the riverbeast (`CDR_RIVERBEAST`, itself just
//!   `CDR_SIMPLEBADDY` under the hood per C's own `ch_driver` dispatch,
//!   `gwendylon.c:6127-6128`), is **not ported**. This needs the same
//!   still-missing generic per-species death-dispatch table (C's
//!   `ch_died_driver`, `gwendylon.c:6160-`) that gates `world::jessica`'s
//!   `bredel_dead` gap and `world::camhermit`'s `monster_dead` bear-kill
//!   counter - see those modules' doc comments for the same documented
//!   precedent. Until that's wired, a player who kills the riverbeast
//!   will never see their `jiu_state` advance past
//!   `JIU_STATE_WAIT_FOR_KILL` on a live server, so the quest cannot yet
//!   be completed end-to-end; the dialogue chain up to and including the
//!   `JIU_STATE_BEAST_KILLED`/`JIU_STATE_DONE` turn-in text is otherwise
//!   fully ported and independently testable by directly setting
//!   `state` in [`JiuPlayerFacts`], matching this codebase's existing
//!   precedent for documenting such gaps rather than silently dropping
//!   them.
//! - C's own `jiu_driver` `NT_CHAR` throttle has a redundant second
//!   `if` (`gwendylon.c:2104-2108`, `ticker < dat->last_talk + TICKS*10 &&
//!   dat->current_victim && dat->current_victim != co`) that is
//!   unreachable dead code: the immediately preceding `if` (`:2099-2102`)
//!   already `continue`s on the exact same `ticker < dat->last_talk +
//!   TICKS*10` condition (both use the same `TICKS*10` threshold, unlike
//!   `world::yoakin`'s genuinely distinct `TICKS*5`/`TICKS*10` pair), so
//!   the second check's body can never execute. Ported as the single
//!   effective check, preserving C's *behavior* rather than reproducing
//!   its dead branch.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_JIU, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::world::*;

/// C `char_dist(cn, co) > 15` (`gwendylon.c:2120`): the `NT_CHAR` greeting
/// range.
const JIU_GREET_DISTANCE: i32 = 15;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const JIU_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:2099`, `:2104`, `:2173`): the single
/// effective talk throttle - see the module doc comment's last bullet for
/// why C's second, seemingly-distinct `TICKS*10` check is dead code here.
const JIU_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:2242`): idle "return to post" threshold.
const JIU_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `#define JIU_STATE_ENTRY 0` (`src/common/npc_states.h:76`).
const JIU_STATE_ENTRY: i32 = 0;
/// C `#define JIU_STATE_STORY1 1` (`src/common/npc_states.h:77`).
const JIU_STATE_STORY1: i32 = 1;
/// C `#define JIU_STATE_WAIT_FOR_KILL 2` (`src/common/npc_states.h:78`).
const JIU_STATE_WAIT_FOR_KILL: i32 = 2;
/// C `#define JIU_STATE_BEAST_KILLED 3` (`src/common/npc_states.h:79`).
const JIU_STATE_BEAST_KILLED: i32 = 3;
/// C `#define JIU_STATE_DONE 4` (`src/common/npc_states.h:80`).
const JIU_STATE_DONE: i32 = 4;

/// Per-player facts [`World::process_jiu_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JiuPlayerFacts {
    /// `PlayerRuntime::area1_jiu_state()`.
    pub state: i32,
}

/// A side effect [`World::process_jiu_actions`] could not apply directly
/// because it touches `PlayerRuntime`. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JiuOutcomeEvent {
    /// Write the new `area1_ppd.jiu_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->jiu_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:2168`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, QLOG_JIU)` (`gwendylon.c:2145`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, QLOG_JIU)` (`gwendylon.c:2159`) - the caller
    /// must apply `PlayerRuntime::quest_log.complete_legacy`.
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `jiu_driver`'s per-tick body (`gwendylon.c:2074-2247`). `now` is
    /// C's wall-clock `realtime` (seconds).
    pub fn process_jiu_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JiuPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<JiuOutcomeEvent> {
        let jiu_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JIU
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for jiu_id in jiu_ids {
            self.process_jiu_messages(jiu_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_jiu_messages(
        &mut self,
        jiu_id: CharacterId,
        player_facts: &HashMap<CharacterId, JiuPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<JiuOutcomeEvent>,
    ) {
        let Some(jiu_name) = self.characters.get(&jiu_id).map(|jiu| jiu.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Jiu(mut data)) = self
            .characters
            .get(&jiu_id)
            .and_then(|jiu| jiu.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&jiu_id)
            .map(|jiu| std::mem::take(&mut jiu.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.jiu_handle_char_message(
                    jiu_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.jiu_handle_text_message(
                    jiu_id,
                    &jiu_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.jiu_handle_give_message(jiu_id, message),
                _ => {}
            }
        }

        if let Some(jiu) = self.characters.get_mut(&jiu_id) {
            jiu.driver_state = Some(CharacterDriverState::Jiu(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:2239-2241`).
        if let (Some(jiu), Some((tx, ty))) = (self.characters.get(&jiu_id).cloned(), face_target) {
            if let Some(direction) = offset2dx(i32::from(jiu.x), i32::from(jiu.y), tx, ty) {
                if let Some(jiu_mut) = self.characters.get_mut(&jiu_id) {
                    let _ = turn(jiu_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; } do_idle(cn, TICKS);`
        // (`gwendylon.c:2242-2247`). The NPC's post position (C's
        // `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same substitution
        // `world::camhermit`/`world::yoakin`/`world::terion` already use
        // for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(jiu) = self.characters.get(&jiu_id) {
            match jiu.driver_state.as_ref() {
                Some(CharacterDriverState::Jiu(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + JIU_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(jiu) = self.characters.get(&jiu_id) else {
                return;
            };
            let (post_x, post_y) = (jiu.rest_x, jiu.rest_y);
            self.secure_move_driver(
                jiu_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `jiu_driver`'s `NT_CHAR` branch (`gwendylon.c:2091-2173`).
    #[allow(clippy::too_many_arguments)]
    fn jiu_handle_char_message(
        &mut self,
        jiu_id: CharacterId,
        data: &mut JiuDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JiuPlayerFacts>,
        now: i32,
        events: &mut Vec<JiuOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(jiu) = self.characters.get(&jiu_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:2093-2096`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:2098-2102`... actually `:2098-2101`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C's two throttle `if`s collapse to this single effective check
        // - see the module doc comment's last bullet.
        if tick < data.last_talk + JIU_TALK_MIN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:2115-2118`).
        if jiu_id == player_id || !char_see_char(&jiu, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 15) continue;` (`gwendylon.c:2120-
        // 2123`).
        if char_dist(&jiu, &player) > JIU_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        match facts.state {
            JIU_STATE_ENTRY => {
                // C `case JIU_STATE_ENTRY:` (`gwendylon.c:2128-2138`).
                if player.level >= 39 {
                    self.npc_quiet_say(
                        jiu_id,
                        "Hail thee, traveler! I have a difficult impediment thou mayst be able to aid me with. I have traveled far to reach the holy sanctuary across the river.",
                    );
                    new_state = JIU_STATE_STORY1;
                } else {
                    self.npc_quiet_say(jiu_id, "Hail thee, traveler.");
                }
                didsay = true;
            }
            JIU_STATE_STORY1 => {
                // C `case JIU_STATE_STORY1:` (`gwendylon.c:2140-2147`).
                self.npc_quiet_say(
                    jiu_id,
                    "To meditate in this holy place would be an honour. Unfortunately, this large beast has settled nearby the sanctuary. I can no longer venture there safely. If thou couldst kill the beast, I would be ever grateful to thee!",
                );
                didsay = true;
                events.push(JiuOutcomeEvent::QuestOpen { player_id });
                new_state = JIU_STATE_WAIT_FOR_KILL;
            }
            JIU_STATE_WAIT_FOR_KILL => {
                // C `case JIU_STATE_WAIT_FOR_KILL:` (`gwendylon.c:2149-
                // 2151`): next state gets triggered from monster death -
                // no-op here.
            }
            JIU_STATE_BEAST_KILLED => {
                // C `case JIU_STATE_BEAST_KILLED:` (`gwendylon.c:2153-
                // 2160`).
                self.npc_quiet_say(
                    jiu_id,
                    &format!(
                        "Thou art an honourable fighter my friend. I shall always remember thine good deed {}.",
                        player.name
                    ),
                );
                didsay = true;
                events.push(JiuOutcomeEvent::QuestDone { player_id });
                new_state = JIU_STATE_DONE;
            }
            // `JIU_STATE_DONE` and any other value: no-op, matching C's
            // empty `case JIU_STATE_DONE: break;`.
            _ => {}
        }

        if new_state != facts.state {
            events.push(JiuOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `ppd->jiu_seen_timer = realtime;` (`gwendylon.c:2168`):
        // unconditional whenever `ppd` was valid (i.e. whenever we got
        // this far).
        events.push(JiuOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:2170-2174`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `jiu_driver`'s `NT_TEXT` branch (`gwendylon.c:2177-2211`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::yoakin`'s text handler).
    fn jiu_handle_text_message(
        &mut self,
        jiu_id: CharacterId,
        jiu_name: &str,
        data: &mut JiuDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JiuPlayerFacts>,
        events: &mut Vec<JiuOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:2181-2183`).
        let tick = self.tick.0;
        if tick > data.last_talk + JIU_TALK_MIN_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:2185-2188`).
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
        if jiu_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(jiu) = self.characters.get(&jiu_id).cloned() else {
            return;
        };
        if char_dist(&jiu, &speaker) > JIU_QA_DISTANCE
            || !char_see_char(&jiu, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, jiu_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(jiu_id, &reply);
                didsay = true;
            }
            // C `case 2:  // repeat` (`gwendylon.c:2191-2205`). `didsay`
            // is unconditionally true here (already set to `2` by the
            // outer `switch ((didsay = analyse_text_driver(...)))`
            // assignment), regardless of which branch below fires -
            // matching `world::terion`/`world::yoakin`'s identical
            // `Matched(2)` shape.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state < JIU_STATE_BEAST_KILLED {
                        // C `if (ppd->jiu_state < JIU_STATE_BEAST_KILLED)`
                        // (`gwendylon.c:2196-2199`).
                        events.push(JiuOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: JIU_STATE_ENTRY,
                        });
                        data.last_talk = 0;
                    } else if facts.state == JIU_STATE_DONE || facts.state == JIU_STATE_BEAST_KILLED
                    {
                        // C `else if (ppd->jiu_state == JIU_STATE_DONE ||
                        // ppd->jiu_state == JIU_STATE_BEAST_KILLED)`
                        // (`gwendylon.c:2200-2204`).
                        self.npc_quiet_say(
                            jiu_id,
                            &format!(
                                "Thou hast done me a great favor, I have no more requests for thee. May Ishtar's blessing be upon thee {}.",
                                speaker.name
                            ),
                        );
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by jiu's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:2207-2210`).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `jiu_driver`'s `NT_GIVE` branch (`gwendylon.c:2214-2225`).
    fn jiu_handle_give_message(&mut self, jiu_id: CharacterId, message: &CharacterDriverMessage) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&jiu_id)
            .and_then(|jiu| jiu.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            jiu_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        self.give_char_item_smart(giver_id, item_id, true);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct jiu_driver_data` (`src/area/1/gwendylon.c:2069-2072`): the
/// riverbeast-quest pilgrim NPC's own driver memory (`CDR_JIU`, distinct
/// from the per-player `jiu_state`/`jiu_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::jiu`'s
/// module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JiuDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
