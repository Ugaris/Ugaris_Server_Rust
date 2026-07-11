//! Dwarf Chief NPC (`CDR_DWARFCHIEF`), Grimroot's leader, who runs "A
//! Miner's Misery"/"A Miner's Bane"/"A Miner's Anguish"/"A Miner Lost"
//! (quests 47-50), handing out `dwarf_recallNN` scrolls that the player
//! carries to each of `world::npc::area31::lostdwarf`'s four lost-miner
//! templates.
//!
//! Ports `src/area/31/warrmines.c::dwarfchief_driver` (`:203-460`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:70-194`, ported as
//! [`super::AREA31_QA`] in `world::npc::area31`, the same table every other
//! `warrmines.c` NPC driver shares). Follows the same `World`/
//! `PlayerRuntime` split established by `world::npc::area29::brennethbran`:
//! the caller supplies a per-player fact snapshot ([`DwarfchiefPlayerFacts`])
//! up front and applies the returned [`DwarfchiefOutcomeEvent`]s afterwards,
//! since `staffer_ppd.dwarfchief_state` and the `QLOG` 47-50 quest-log
//! entries live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `dwarfchief_driver`'s sixteen-state (`0`-`15`) dialogue chain is four
//! back-to-back mini quests sharing one state counter: greeting (opens
//! quest 47) -> "four of our miners have gone missing" -> "here's a
//! scroll" (grants `dwarf_recall90` unless already carried, opens nothing
//! yet) (waiting: state `3`) -> (external: `lostdwarf_driver`'s
//! `dwarfchief_state<=3 && nr==1` branch jumps this to `4` on recall1
//! turn-in) -> quest 47 done -> if quest 48 already done, fast-forward to
//! `8`; else "not too bad..." (opens quest 48, grants `dwarf_recall100`)
//! (waiting: state `6`) -> (external jump to `7`) -> quest 48 done -> if
//! quest 49 already done, fast-forward to `11`; else "a job well done!"
//! (opens quest 49, grants `dwarf_recall110`) (waiting: state `9`) ->
//! (external jump to `10`) -> quest 49 done -> if quest 50 already done,
//! fast-forward to `14`; else "just in time!" (opens quest 50, grants
//! `dwarf_recall120`) (waiting: state `12`) -> (external jump to `13`) ->
//! quest 50 done -> "thank you for saving the last one!" -> done (state
//! `15`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `warrmines.c`/`brannington.c` driver's own `NT_TEXT`
//!   branch, this driver's own C body has no `dat->current_victim`
//!   staleness-reset preamble and no victim-mismatch early-out at all -
//!   reproduced verbatim: replies to *any* nearby player's matched small
//!   talk, not just its tracked victim.
//! - C `case 2:` (`:394-414`) resets to whichever of the four mini quests'
//!   greeting state the player is currently mid-way through (five separate
//!   range checks: `<=3` -> `0`, `5..=6` -> `5`, `8..=9` -> `8`,
//!   `11..=12` -> `11`, `14..=15` -> `14`), ported as
//!   [`DwarfchiefOutcomeEvent::ResetToMiniQuestStart`] with the resolved
//!   target state computed in `World` itself.
//! - C `case 3:` (`:416-421`) speaks a visible `say(cn, "reset done")` line
//!   (not `quiet_say`) before wiping the state to `0` - only if the
//!   speaker is `CF_GOD`, matching `world::npc::area29::brennethbran`'s own
//!   `case 3` precedent exactly.
//! - C's `case 8`/`case 11` (`:319-336`/`:344-361`) both guard their
//!   `dwarf_recallNN` grant with `!has_item(co, IID_DWARFRECALL2)` - a
//!   verbatim-preserved C copy/paste oddity (they should plausibly check
//!   `IID_DWARFRECALL3`/`IID_DWARFRECALL4` respectively). Since each state
//!   is visited only once per playthrough (the state counter advances past
//!   it immediately), this has no observable effect beyond the literal
//!   check performed.
//! - No self-defense/regen/spell-self cascade exists in C's `dwarfchief_
//!   driver` body at all (matching every other `warrmines.c`/
//!   `brannington.c` "pure talker" NPC) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:459`) is not
//!   ported, matching the established `world::npc::area29::brennethbran`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA31_QA;

/// C `char_dist(cn, co) > 10` (`warrmines.c:252`).
const DWARFCHIEF_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`warrmines.c:115`, the shared
/// `analyse_text_driver` copy's own guard).
const DWARFCHIEF_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`warrmines.c:235`).
const DWARFCHIEF_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`warrmines.c:240`).
const DWARFCHIEF_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`warrmines.c:453`): idle "return to post" threshold.
const DWARFCHIEF_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `create_item("dwarf_recallNN")` (`warrmines.c:281`/`306`/`331`/`356`):
/// which recall scroll template `ugaris-server` should instantiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfRecallScroll {
    /// `dwarf_recall90` (`IID_DWARFRECALL1`).
    Recall90,
    /// `dwarf_recall100` (`IID_DWARFRECALL2`).
    Recall100,
    /// `dwarf_recall110` (`IID_DWARFRECALL2`-tagged item, template
    /// `dwarf_recall110`).
    Recall110,
    /// `dwarf_recall120` (`IID_DWARFRECALL2`-tagged item, template
    /// `dwarf_recall120`).
    Recall120,
}

/// Per-player facts [`World::process_dwarfchief_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwarfchiefPlayerFacts {
    /// `PlayerRuntime::staffer_dwarfchief_state()`.
    pub dwarfchief_state: i32,
    /// `PlayerRuntime::quest_log.is_done(48)` (C `questlog_isdone(co,
    /// 48)`, `warrmines.c:295`): `case 5`'s fast-forward guard.
    pub quest48_is_done: bool,
    /// `PlayerRuntime::quest_log.is_done(49)` (C `questlog_isdone(co,
    /// 49)`, `warrmines.c:320`): `case 8`'s fast-forward guard.
    pub quest49_is_done: bool,
    /// `PlayerRuntime::quest_log.is_done(50)` (C `questlog_isdone(co,
    /// 50)`, `warrmines.c:345`): `case 11`'s fast-forward guard.
    pub quest50_is_done: bool,
}

/// A side effect [`World::process_dwarfchief_actions`] could not apply
/// directly because it touches `PlayerRuntime`, or because it needs the
/// zone loader's item-template table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfchiefOutcomeEvent {
    /// Write the new `staffer_ppd.dwarfchief_state` back.
    UpdateDwarfchiefState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 47/48/49/50)`.
    QuestOpen { player_id: CharacterId, quest: u32 },
    /// C `questlog_done(co, 47/48/49/50)`.
    QuestDone { player_id: CharacterId, quest: u32 },
    /// C `create_item("dwarf_recallNN")` + `give_char_item` (`warrmines.c:
    /// 281-284`/`306-309`/`331-334`/`356-359`): `World` has already
    /// confirmed the player doesn't carry the matching recall scroll.
    GrantRecallScroll {
        player_id: CharacterId,
        scroll: DwarfRecallScroll,
    },
    /// C `case 2:` (`warrmines.c:394-414`): reset back to the start of
    /// whichever of the four mini quests the player is currently mid-way
    /// through. `new_state` is already resolved to the target state.
    ResetToMiniQuestStart {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`warrmines.c:416-421`): the god-only "reset me" full
    /// state wipe.
    ResetDwarfchief { player_id: CharacterId },
}

impl World {
    /// C `dwarfchief_driver`'s per-tick body (`warrmines.c:203-460`).
    pub fn process_dwarfchief_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, DwarfchiefPlayerFacts>,
        area_id: u16,
    ) -> Vec<DwarfchiefOutcomeEvent> {
        let dwarfchief_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DWARFCHIEF
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for dwarfchief_id in dwarfchief_ids {
            self.process_dwarfchief_messages(dwarfchief_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_dwarfchief_messages(
        &mut self,
        dwarfchief_id: CharacterId,
        player_facts: &HashMap<CharacterId, DwarfchiefPlayerFacts>,
        area_id: u16,
        events: &mut Vec<DwarfchiefOutcomeEvent>,
    ) {
        let Some(dwarfchief_name) = self
            .characters
            .get(&dwarfchief_id)
            .map(|dwarfchief| dwarfchief.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::DwarfChief(mut data)) = self
            .characters
            .get(&dwarfchief_id)
            .and_then(|dwarfchief| dwarfchief.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&dwarfchief_id)
            .map(|dwarfchief| std::mem::take(&mut dwarfchief.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.dwarfchief_handle_char_message(
                    dwarfchief_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.dwarfchief_handle_text_message(
                    dwarfchief_id,
                    &dwarfchief_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.dwarfchief_handle_give_message(dwarfchief_id, message),
                _ => {}
            }
        }

        if let Some(dwarfchief) = self.characters.get_mut(&dwarfchief_id) {
            dwarfchief.driver_state = Some(CharacterDriverState::DwarfChief(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`warrmines.c:449-451`).
        if let (Some(dwarfchief), Some((tx, ty))) =
            (self.characters.get(&dwarfchief_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(dwarfchief.x), i32::from(dwarfchief.y), tx, ty)
            {
                if let Some(dwarfchief_mut) = self.characters.get_mut(&dwarfchief_id) {
                    let _ = turn(dwarfchief_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`warrmines.c:453-457`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::brennethbran` already uses.
        let last_talk = if let Some(dwarfchief) = self.characters.get(&dwarfchief_id) {
            match dwarfchief.driver_state.as_ref() {
                Some(CharacterDriverState::DwarfChief(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + DWARFCHIEF_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(dwarfchief) = self.characters.get(&dwarfchief_id) else {
                return;
            };
            let (post_x, post_y) = (dwarfchief.rest_x, dwarfchief.rest_y);
            self.secure_move_driver(
                dwarfchief_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `dwarfchief_driver`'s `NT_CHAR` branch (`warrmines.c:218-385`).
    #[allow(clippy::too_many_arguments)]
    fn dwarfchief_handle_char_message(
        &mut self,
        dwarfchief_id: CharacterId,
        data: &mut DwarfChiefDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfchiefPlayerFacts>,
        events: &mut Vec<DwarfchiefOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(dwarfchief) = self.characters.get(&dwarfchief_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`warrmines.c:222-226`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`warrmines.c:228-232`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`warrmines.c:234-238`).
        if tick < data.last_talk + DWARFCHIEF_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`warrmines.c:240-243`).
        if tick < data.last_talk + DWARFCHIEF_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`warrmines.c:245-249`).
        if dwarfchief_id == player_id
            || !char_see_char(&dwarfchief, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`warrmines.c:251-
        // 255`).
        if char_dist(&dwarfchief, &player) > DWARFCHIEF_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.dwarfchief_state;
        match facts.dwarfchief_state {
            // C `case 0:` (`warrmines.c:262-268`).
            0 => {
                self.npc_quiet_say(
                    dwarfchief_id,
                    "Welcome, stranger, to Grimroot, home of the dwarves. I would introduce you to our town further, but I have urgent matters to attend to.",
                );
                events.push(DwarfchiefOutcomeEvent::QuestOpen {
                    player_id,
                    quest: 47,
                });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`warrmines.c:269-274`).
            1 => {
                self.npc_quiet_say(
                    dwarfchief_id,
                    "Four of our miners have gone missing, one each in one of the 4 mine areas, and we can only think it's because of those bothersome golems...",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`warrmines.c:275-286`).
            2 => {
                self.npc_quiet_say(
                    dwarfchief_id,
                    "If you wish, here's a scroll so you can help one of them. Give this one to the miner in the first section, then come back for another scroll for the next miner.",
                );
                new_state = 3;
                didsay = true;
                if !self.character_has_item_template(player_id, IID_DWARFRECALL1) {
                    events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                        player_id,
                        scroll: DwarfRecallScroll::Recall90,
                    });
                }
            }
            // C `case 3: break;` (`warrmines.c:287-288`): waiting for
            // player to save first miner.
            3 => {}
            // C `case 4:` (`warrmines.c:290-293`): `questlog_done(co, 47)`
            // then fall-through to `case 5`.
            4 => {
                events.push(DwarfchiefOutcomeEvent::QuestDone {
                    player_id,
                    quest: 47,
                });
                if facts.quest48_is_done {
                    new_state = 8;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "Not too bad for a human, you people are sturdier than I thought... Don't cheer up though, the miner in the next section is surrounded by stronger golems. Don't let them hurt your precious nails!",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 48,
                    });
                    new_state = 6;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall100,
                        });
                    }
                }
            }
            // C `case 5:` (`warrmines.c:294-311`): fast-forward if quest 48
            // is already done, else the same body as `case 4`'s
            // fall-through.
            5 => {
                if facts.quest48_is_done {
                    new_state = 8;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "Not too bad for a human, you people are sturdier than I thought... Don't cheer up though, the miner in the next section is surrounded by stronger golems. Don't let them hurt your precious nails!",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 48,
                    });
                    new_state = 6;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall100,
                        });
                    }
                }
            }
            // C `case 6: break;` (`warrmines.c:312-313`): waiting for
            // player to save second miner.
            6 => {}
            // C `case 7:`/`case 8:` (`warrmines.c:315-336`): `questlog_done
            // (co, 48)` fall-through, then fast-forward if quest 49 is
            // already done, else opens quest 49. Note the C oddity: both
            // grant checks use `IID_DWARFRECALL2` (see module doc comment).
            7 => {
                events.push(DwarfchiefOutcomeEvent::QuestDone {
                    player_id,
                    quest: 48,
                });
                if facts.quest49_is_done {
                    new_state = 11;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "A job well done! It's that we have enough hands already, otherwise I'd ask you to go out there and mine for us. Anyway, back to business. Go and find the next miner, and you will be rewarded again.",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 49,
                    });
                    new_state = 9;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall110,
                        });
                    }
                }
            }
            8 => {
                if facts.quest49_is_done {
                    new_state = 11;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "A job well done! It's that we have enough hands already, otherwise I'd ask you to go out there and mine for us. Anyway, back to business. Go and find the next miner, and you will be rewarded again.",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 49,
                    });
                    new_state = 9;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall110,
                        });
                    }
                }
            }
            // C `case 9: break;` (`warrmines.c:337-338`): waiting for
            // player to save third miner.
            9 => {}
            // C `case 10:`/`case 11:` (`warrmines.c:340-361`):
            // `questlog_done(co, 49)` fall-through, then fast-forward if
            // quest 50 is already done, else opens quest 50.
            10 => {
                events.push(DwarfchiefOutcomeEvent::QuestDone {
                    player_id,
                    quest: 49,
                });
                if facts.quest50_is_done {
                    new_state = 14;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "Just in time! If you had been any later, he wouldn't have had his dinner, and trust me, you don't want to see a dwarf hungry, it's not a pretty sight. The fourth miner should be ok, he always packs more than the others, but do hurry and find him.",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 50,
                    });
                    new_state = 12;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall120,
                        });
                    }
                }
            }
            11 => {
                if facts.quest50_is_done {
                    new_state = 14;
                } else {
                    self.npc_quiet_say(
                        dwarfchief_id,
                        "Just in time! If you had been any later, he wouldn't have had his dinner, and trust me, you don't want to see a dwarf hungry, it's not a pretty sight. The fourth miner should be ok, he always packs more than the others, but do hurry and find him.",
                    );
                    events.push(DwarfchiefOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 50,
                    });
                    new_state = 12;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_DWARFRECALL2) {
                        events.push(DwarfchiefOutcomeEvent::GrantRecallScroll {
                            player_id,
                            scroll: DwarfRecallScroll::Recall120,
                        });
                    }
                }
            }
            // C `case 12: break;` (`warrmines.c:362-363`): waiting for
            // player to save fourth miner.
            12 => {}
            // C `case 13:`/`case 14:` (`warrmines.c:365-375`):
            // `questlog_done(co, 50)` fall-through into the closing line.
            13 => {
                events.push(DwarfchiefOutcomeEvent::QuestDone {
                    player_id,
                    quest: 50,
                });
                self.npc_quiet_say(
                    dwarfchief_id,
                    "Thank you for saving the last one! You have been of great help to us. Now let's hope they can stay out of the hands of those golems once and for all. Those recall scrolls aren't cheap you know!",
                );
                new_state = 15;
                didsay = true;
            }
            14 => {
                self.npc_quiet_say(
                    dwarfchief_id,
                    "Thank you for saving the last one! You have been of great help to us. Now let's hope they can stay out of the hands of those golems once and for all. Those recall scrolls aren't cheap you know!",
                );
                new_state = 15;
                didsay = true;
            }
            // C `case 15: break;` (`warrmines.c:376-377`): all done.
            15 => {}
            _ => {}
        }

        if new_state != facts.dwarfchief_state {
            events.push(DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`warrmines.c:379-383`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `dwarfchief_driver`'s `NT_TEXT` branch (`warrmines.c:388-428`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area29::brennethbran`'s text handler). This branch has
    /// no victim-staleness-reset preamble and no victim-mismatch early-out
    /// (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn dwarfchief_handle_text_message(
        &mut self,
        dwarfchief_id: CharacterId,
        dwarfchief_name: &str,
        data: &mut DwarfChiefDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfchiefPlayerFacts>,
        events: &mut Vec<DwarfchiefOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`warrmines.c:391`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`warrmines.c:101-
        // 121`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if dwarfchief_id == speaker_id {
            return;
        }
        let Some(dwarfchief) = self.characters.get(&dwarfchief_id).cloned() else {
            return;
        };
        if char_dist(&dwarfchief, &speaker) > DWARFCHIEF_QA_DISTANCE
            || !char_see_char(&dwarfchief, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let dwarfchief_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.dwarfchief_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, dwarfchief_name, &speaker.name, AREA31_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(dwarfchief_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`warrmines.c:394-414`): reset back to the start
            // of whichever of the four mini quests is in progress.
            TextAnalysisOutcome::Matched(2) => {
                let new_state = if dwarfchief_state <= 3 {
                    Some(0)
                } else if (5..=6).contains(&dwarfchief_state) {
                    Some(5)
                } else if (8..=9).contains(&dwarfchief_state) {
                    Some(8)
                } else if (11..=12).contains(&dwarfchief_state) {
                    Some(11)
                } else if (14..=15).contains(&dwarfchief_state) {
                    Some(14)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    data.last_talk = 0;
                    events.push(DwarfchiefOutcomeEvent::ResetToMiniQuestStart {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`warrmines.c:416-421`): the god-only "reset me"
            // wipe, which speaks a visible `say(cn, "reset done")` line
            // first.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(dwarfchief_id, "reset done");
                    events.push(DwarfchiefOutcomeEvent::ResetDwarfchief {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`warrmines.c:423-426`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `dwarfchief_driver`'s `NT_GIVE` branch (`warrmines.c:430-441`): no
    /// item is ever accepted, so the giver always gets it back (or it's
    /// destroyed if the inventory is full).
    fn dwarfchief_handle_give_message(
        &mut self,
        dwarfchief_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&dwarfchief_id)
            .and_then(|dwarfchief| dwarfchief.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            dwarfchief_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_DWARFCHIEF, CDR_LOSTCON};
use crate::item_driver::{IID_DWARFRECALL1, IID_DWARFRECALL2};

/// C `struct dwarfchief_data` (`src/area/31/warrmines.c:198-201`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DwarfChiefDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
