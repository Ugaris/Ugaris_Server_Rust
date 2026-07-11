//! Smugglecom NPC (`CDR_SMUGGLECOM`), the Imperial Commander below Aston 2
//! who runs the Contraband quest chain (quests 35-37).
//!
//! Ports `src/area/26/staffer.c::smugglecom_driver` (`:403-656`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:84-101`, ported as
//! [`super::AREA26_QA`] in `world::npc::area26`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area3::
//! thomas`/`sir_jones`: the caller supplies a per-player fact snapshot
//! ([`SmuggleComPlayerFacts`]) up front and applies the returned
//! [`SmuggleComOutcomeEvent`]s afterwards, since `staffer_ppd.
//! smugglecom_state`/`smugglecom_bits` and the `QLOG` 35/36/37 quest-log
//! entries live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `smugglecom_driver`'s eleven-state (`0`-`10`) dialogue chain: greeting
//! -> "find the contraband book" -> (`NT_GIVE`: hand in `IID_STAFF_
//! SMUGGLEBOOK`, quest 35 done, state jumps to `5`) -> "here are the four
//! items to find" -> (`NT_GIVE` x4: each of pearls/ring/cape/necklace
//! grants a manual scaled-exp reward and sets one `SMUGGLEBIT_*`; once all
//! four bits are set, quest 36 completes) -> "kill the smuggler's leader"
//! -> (external: a player kills `CDR_SMUGGLELEAD`,
//! `world_events::death_hooks::apply_smugglelead_death_from_hurt_event`
//! advances `smugglecom_state` from `8` to `9`) -> "thank you", quest 37
//! done.
//!
//! Deviations/gaps (documented, not silent):
//! - Unlike `world::thomas`/`world::sir_jones`'s `NT_TEXT` branch,
//!   `smugglecom_driver`'s own C body has no `dat->current_victim`
//!   staleness-reset preamble and no "ignore text from anyone but the
//!   current victim" early-out (`area3.c:1763-1770` has no equivalent
//!   here) - reproduced verbatim: this driver replies to *any* nearby
//!   player's matched small talk, not just its currently tracked victim.
//! - C `case 2`/`3` (`:474-477`) is an intentional fallthrough with no
//!   code of its own (`// fall through intended for now`) - reproduced by
//!   matching `2 | 3` directly to `case 3`'s body.
//! - C `case 6`'s `ppd->smugglecom_state++` (`:499`) never sets `didsay =
//!   1` (no dialogue line is spoken for this transition) - reproduced by
//!   not touching `last_talk`/`current_victim`/`face_target` for it, same
//!   "silent state transition" precedent as `world::sir_jones`'s `case
//!   12`/`13`.
//! - C `case 7`'s "Thank you..." line (`:504`) is spoken unconditionally,
//!   *before* the `questlog_isdone(co, 37)` check that decides whether
//!   `didsay` ever becomes true - reproduced verbatim: the already-done
//!   sub-branch (jumping straight to state `10`) still speaks that first
//!   line but does not refresh `last_talk`/`current_victim`, same
//!   "spoken but didsay unset" quirk as `world::sir_jones`'s `case 12`.
//! - C's `dlog(cn, 0, "Received %d exp for doing quest Contraband ...")`
//!   calls in the `NT_GIVE` item branches are admin-only debug audit log
//!   lines, not player-visible - not ported, matching the established
//!   `dlog` skip precedent (see `world::npc::area17::thiefmaster`'s own
//!   silent skip of the same C function).
//! - No self-defense/regen/spell-self cascade exists in C's `smugglecom_
//!   driver` body at all (matching `world::astro1`'s identical
//!   observation for other area "pure talker" NPCs) - this port omits it
//!   too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:655`) is not
//!   ported, matching the established `world::thomas`/`world::sir_jones`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::quest::scale_exp;
use crate::world::exp::level_value;
use crate::world::*;

use super::AREA26_QA;

/// C `char_dist(cn, co) > 10` (`staffer.c:452`).
const SMUGGLECOM_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`staffer.c:129`, the shared
/// `analyse_text_driver` copy's own guard).
const SMUGGLECOM_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`staffer.c:435`).
const SMUGGLECOM_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`staffer.c:440`).
const SMUGGLECOM_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`staffer.c:649`): idle "return to post" threshold.
const SMUGGLECOM_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `questlog_open(co, 35)` (`staffer.c:464`).
const QLOG_SMUGGLECOM_BOOK: usize = 35;
/// C `questlog_open(co, 36)` (`staffer.c:493`).
const QLOG_SMUGGLECOM_CONTRABAND: usize = 36;
/// C `questlog_open(co, 37)` (`staffer.c:510`).
const QLOG_SMUGGLECOM_LEADER: usize = 37;
/// C `#define SMUGGLEBIT_PEARLS 1` (`staffer.c:55`).
const SMUGGLEBIT_PEARLS: i32 = 1;
/// C `#define SMUGGLEBIT_RING 2` (`staffer.c:56`).
const SMUGGLEBIT_RING: i32 = 2;
/// C `#define SMUGGLEBIT_CAPE 4` (`staffer.c:57`).
const SMUGGLEBIT_CAPE: i32 = 4;
/// C `#define SMUGGLEBIT_NECKLACE 8` (`staffer.c:58`).
const SMUGGLEBIT_NECKLACE: i32 = 8;
/// C `ppd->smugglecom_bits == 15` (`staffer.c:498`): all four bits set.
const SMUGGLEBITS_ALL: i32 = 15;
/// C `level_value(ch[co].level) / 4` (`staffer.c:590` et al.).
const SMUGGLECOM_ITEM_REWARD_LEVEL_DIVISOR: i64 = 4;

/// Per-player facts [`World::process_smugglecom_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmuggleComPlayerFacts {
    /// `PlayerRuntime::staffer_smugglecom_state()`.
    pub smugglecom_state: i32,
    /// `PlayerRuntime::staffer_smugglecom_bits()`.
    pub smugglecom_bits: i32,
    /// `PlayerRuntime::quest_log.count(QLOG_SMUGGLECOM_CONTRABAND)` (C
    /// `questlog_count(co, 36)`), used to scale each contraband item's
    /// manual exp reward.
    pub quest36_count: u8,
    /// `PlayerRuntime::quest_log.is_done(QLOG_SMUGGLECOM_CONTRABAND)` (C
    /// `questlog_isdone(co, 36)`).
    pub quest36_done: bool,
    /// `PlayerRuntime::quest_log.is_done(QLOG_SMUGGLECOM_LEADER)` (C
    /// `questlog_isdone(co, 37)`).
    pub quest37_done: bool,
}

/// A side effect [`World::process_smugglecom_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmuggleComOutcomeEvent {
    /// Write the new `staffer_ppd.smugglecom_state` back.
    UpdateSmugglecomState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, ...)` - applied via the standard `complete_
    /// legacy` flow (quest 35/37 grant real table exp; quest 36's table
    /// entry is `exp: 0`, "exp awarded in driver" - its real reward is
    /// the manual per-item grants below).
    QuestDone {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `ppd->smugglecom_bits |= SMUGGLEBIT_*;`.
    SetSmugglecomBit { player_id: CharacterId, bit: i32 },
    /// C `case 3:` (`staffer.c:555-559`): the god-only "reset me" wipe.
    ResetSmugglecom { player_id: CharacterId },
}

impl World {
    /// C `smugglecom_driver`'s per-tick body (`staffer.c:403-656`).
    pub fn process_smugglecom_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, SmuggleComPlayerFacts>,
        area_id: u16,
    ) -> Vec<SmuggleComOutcomeEvent> {
        let smugglecom_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SMUGGLECOM
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for smugglecom_id in smugglecom_ids {
            self.process_smugglecom_messages(smugglecom_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_smugglecom_messages(
        &mut self,
        smugglecom_id: CharacterId,
        player_facts: &HashMap<CharacterId, SmuggleComPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SmuggleComOutcomeEvent>,
    ) {
        let Some(smugglecom_name) = self
            .characters
            .get(&smugglecom_id)
            .map(|smugglecom| smugglecom.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::SmuggleCom(mut data)) = self
            .characters
            .get(&smugglecom_id)
            .and_then(|smugglecom| smugglecom.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&smugglecom_id)
            .map(|smugglecom| std::mem::take(&mut smugglecom.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.smugglecom_handle_char_message(
                    smugglecom_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.smugglecom_handle_text_message(
                    smugglecom_id,
                    &smugglecom_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.smugglecom_handle_give_message(
                    smugglecom_id,
                    message,
                    player_facts,
                    area_id,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(smugglecom) = self.characters.get_mut(&smugglecom_id) {
            smugglecom.driver_state = Some(CharacterDriverState::SmuggleCom(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`staffer.c:645-647`).
        if let (Some(smugglecom), Some((tx, ty))) =
            (self.characters.get(&smugglecom_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(smugglecom.x), i32::from(smugglecom.y), tx, ty)
            {
                if let Some(smugglecom_mut) = self.characters.get_mut(&smugglecom_id) {
                    let _ = turn(smugglecom_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`staffer.c:649-653`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::thomas`/`world::sir_jones` already use.
        let last_talk = if let Some(smugglecom) = self.characters.get(&smugglecom_id) {
            match smugglecom.driver_state.as_ref() {
                Some(CharacterDriverState::SmuggleCom(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + SMUGGLECOM_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(smugglecom) = self.characters.get(&smugglecom_id) else {
                return;
            };
            let (post_x, post_y) = (smugglecom.rest_x, smugglecom.rest_y);
            self.secure_move_driver(
                smugglecom_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `smugglecom_driver`'s `NT_CHAR` branch (`staffer.c:419-532`).
    #[allow(clippy::too_many_arguments)]
    fn smugglecom_handle_char_message(
        &mut self,
        smugglecom_id: CharacterId,
        data: &mut SmuggleComDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SmuggleComPlayerFacts>,
        events: &mut Vec<SmuggleComOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(smugglecom) = self.characters.get(&smugglecom_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`staffer.c:422-426`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`staffer.c:428-432`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`staffer.c:434-438`).
        if tick < data.last_talk + SMUGGLECOM_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`staffer.c:440-443`) - note this has no
        // `dat->current_victim &&` truthy guard, unlike `world::thomas`/
        // `world::sir_jones`; a direct `Option` inequality reproduces the
        // same "0 != co" C semantics.
        if tick < data.last_talk + SMUGGLECOM_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`staffer.c:446-449`).
        if smugglecom_id == player_id
            || !char_see_char(&smugglecom, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`staffer.c:452-455`).
        if char_dist(&smugglecom, &player) > SMUGGLECOM_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.smugglecom_state;
        match facts.smugglecom_state {
            // C `case 0:` (`staffer.c:462-467`).
            0 => {
                self.npc_quiet_say(smugglecom_id, &format!("Greetings, {}!", player.name));
                events.push(SmuggleComOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_SMUGGLECOM_BOOK,
                });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`staffer.c:468-473`).
            1 => {
                self.npc_quiet_say(
                    smugglecom_id,
                    "I want you to find a book for me called 'the contraband book', which contains the names of four of the smuggler's most precious items.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` falls through into `case 3`'s body with no code
            // of its own (`staffer.c:474-482`, `// fall through intended
            // for now`).
            2 | 3 => {
                self.npc_quiet_say(smugglecom_id, "Go now, and may Ishtar be with you.");
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`staffer.c:483-484`): waiting for the
            // player to hand in the contraband book.
            4 => {}
            // C `case 5:` (`staffer.c:485-496`).
            5 => {
                if facts.quest36_done {
                    new_state = 7;
                } else {
                    self.npc_quiet_say(
                        smugglecom_id,
                        "It lists four important items which I want you to retrieve: the Rainbow Pearls, the Crimson Ring, the Leopard Cape, and the Emerald Necklace. Find them, and bring them to me.",
                    );
                    events.push(SmuggleComOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_SMUGGLECOM_CONTRABAND,
                    });
                    new_state = 6;
                    didsay = true;
                }
            }
            // C `case 6:` (`staffer.c:497-502`) - never sets `didsay = 1`
            // (see the module doc comment).
            6 => {
                if facts.smugglecom_bits == SMUGGLEBITS_ALL {
                    events.push(SmuggleComOutcomeEvent::QuestDone {
                        player_id,
                        quest: QLOG_SMUGGLECOM_CONTRABAND,
                    });
                    new_state = 7;
                }
            }
            // C `case 7:` (`staffer.c:503-513`) - the first line is
            // spoken unconditionally, but `didsay` is only set in the
            // not-yet-done sub-branch (see the module doc comment).
            7 => {
                self.npc_quiet_say(
                    smugglecom_id,
                    "Thank you, you are of great help in hurting the smuggler's operations.",
                );
                if facts.quest37_done {
                    new_state = 10;
                } else {
                    self.npc_quiet_say(
                        smugglecom_id,
                        "Now, as a final task, I want you to kill the smuggler's leader. Good luck!",
                    );
                    events.push(SmuggleComOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_SMUGGLECOM_LEADER,
                    });
                    new_state = 8;
                    didsay = true;
                }
            }
            // C `case 8: break;` (`staffer.c:514-515`): waiting for the
            // player to kill the smuggler leader.
            8 => {}
            // C `case 9:` (`staffer.c:516-521`).
            9 => {
                self.npc_quiet_say(
                    smugglecom_id,
                    &format!(
                        "Thank you for helping us, {}, you have been of great value.",
                        player.name
                    ),
                );
                events.push(SmuggleComOutcomeEvent::QuestDone {
                    player_id,
                    quest: QLOG_SMUGGLECOM_LEADER,
                });
                new_state = 10;
                didsay = true;
            }
            // C `case 10: break;` (`staffer.c:522-523`): quest chain done.
            10 => {}
            _ => {}
        }

        if new_state != facts.smugglecom_state {
            events.push(SmuggleComOutcomeEvent::UpdateSmugglecomState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(...); }`
        // (`staffer.c:525-530`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `smugglecom_driver`'s `NT_TEXT` branch (`staffer.c:535-566`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::thomas`'s text handler). Unlike `world::thomas`/
    /// `world::sir_jones`, this branch has no victim-staleness-reset
    /// preamble and no victim-mismatch early-out (see the module doc
    /// comment).
    #[allow(clippy::too_many_arguments)]
    fn smugglecom_handle_text_message(
        &mut self,
        smugglecom_id: CharacterId,
        smugglecom_name: &str,
        data: &mut SmuggleComDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SmuggleComPlayerFacts>,
        events: &mut Vec<SmuggleComOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`staffer.c:538`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`staffer.c:121-
        // 135`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if smugglecom_id == speaker_id {
            return;
        }
        let Some(smugglecom) = self.characters.get(&smugglecom_id).cloned() else {
            return;
        };
        if char_dist(&smugglecom, &speaker) > SMUGGLECOM_QA_DISTANCE
            || !char_see_char(&smugglecom, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let smugglecom_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.smugglecom_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, smugglecom_name, &speaker.name, AREA26_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(smugglecom_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) (`staffer.c:541-554`).
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                let new_state = if smugglecom_state <= 4 {
                    Some(0)
                } else if (5..=6).contains(&smugglecom_state) {
                    Some(5)
                } else if (7..=8).contains(&smugglecom_state) {
                    Some(7)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    events.push(SmuggleComOutcomeEvent::UpdateSmugglecomState {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (reset me, god-only) (`staffer.c:555-559`).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    events.push(SmuggleComOutcomeEvent::ResetSmugglecom {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by smugglecom's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`staffer.c:561-564`) - note this does *not* touch
        // `dat->last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `smugglecom_driver`'s `NT_GIVE` branch (`staffer.c:569-637`).
    fn smugglecom_handle_give_message(
        &mut self,
        smugglecom_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SmuggleComPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SmuggleComOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&smugglecom_id)
            .and_then(|smugglecom| smugglecom.cursor_item.take())
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

        // C `if (it[in].ID == IID_STAFF_SMUGGLEBOOK && ppd &&
        // ppd->smugglecom_state <= 4 && (ch[co].flags & CF_PLAYER))`
        // (`staffer.c:577-582`).
        if item.template_id == IID_STAFF_SMUGGLEBOOK
            && is_player
            && facts.is_some_and(|facts| facts.smugglecom_state <= 4)
        {
            self.npc_quiet_say(
                smugglecom_id,
                &format!("Thank you for the book, {}.", giver.name),
            );
            events.push(SmuggleComOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: QLOG_SMUGGLECOM_BOOK,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_SMUGGLEBOOK);
            events.push(SmuggleComOutcomeEvent::UpdateSmugglecomState {
                player_id: giver_id,
                new_state: 5,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's four identically-shaped contraband-piece branches
        // (`staffer.c:583-623`).
        let contraband_piece = [
            (IID_STAFF_SMUGGLEPEARLS, SMUGGLEBIT_PEARLS),
            (IID_STAFF_SMUGGLERING, SMUGGLEBIT_RING),
            (IID_STAFF_SMUGGLECAPE, SMUGGLEBIT_CAPE),
            (IID_STAFF_SMUGGLENECKLACE, SMUGGLEBIT_NECKLACE),
        ]
        .into_iter()
        .find(|(template_id, bit)| {
            item.template_id == *template_id
                && is_player
                && facts.is_some_and(|facts| facts.smugglecom_bits & bit == 0)
        });

        if let Some((_, bit)) = contraband_piece {
            self.npc_quiet_say(
                smugglecom_id,
                &format!(
                    "Thank you for bringing back the {}, {}.",
                    item.name, giver.name
                ),
            );
            let prior_completions = facts.map(|facts| facts.quest36_count).unwrap_or(0);
            let scaled = scale_exp(prior_completions, 1000);
            let level = giver.level;
            let cap = i64::from(level_value(level)) / SMUGGLECOM_ITEM_REWARD_LEVEL_DIVISOR;
            self.give_exp(giver_id, scaled.min(cap), u32::from(area_id));
            events.push(SmuggleComOutcomeEvent::SetSmugglecomBit {
                player_id: giver_id,
                bit,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`staffer.c:624-629`): hand the item
        // back to the giver.
        self.npc_quiet_say(
            smugglecom_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_SMUGGLECOM};
use crate::item_driver::{
    IID_STAFF_SMUGGLEBOOK, IID_STAFF_SMUGGLECAPE, IID_STAFF_SMUGGLENECKLACE,
    IID_STAFF_SMUGGLEPEARLS, IID_STAFF_SMUGGLERING,
};

/// C `struct smugglecom_data` (`src/area/26/staffer.c:397-401`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SmuggleComDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
