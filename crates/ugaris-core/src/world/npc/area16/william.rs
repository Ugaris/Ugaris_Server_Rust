//! William NPC (`CDR_FORESTWILLIAM`), the bear-hunt/mantis-stew quest
//! giver (`QLOG` 22 "Impish Bear Hunt", `QLOG` 23 "Praying Mantis Stew").
//!
//! Ports `src/area/16/forest.c::william_driver` (`:428-628`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:83-202`, ported as
//! [`super::FOREST_QA`] in `world::npc::area16`, the same table `world::
//! npc::area16::hermit` shares). Follows the same `World`/`PlayerRuntime`
//! split established by `world::npc::area3::astro2`: the caller supplies
//! a per-player fact snapshot ([`ForestWilliamPlayerFacts`]) up front and
//! applies the returned [`ForestWilliamOutcomeEvent`]s afterwards, since
//! `area3_ppd.william_state`/`imp_state` (borrowed from `src/area/3/
//! area3.h` - C's own comment: "note: the ppd is borrowed from area3 -
//! the missions interact...") live on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! C's `case 0`/`case 3` have a real, deliberately-reproduced quirk: the
//! greeting text in `case 0` is spoken *before* the `questlog_isdone(co,
//! 22)` early-out check (so the greeting always fires, even on a repeat
//! visit after the whole chain is already done), while `case 3`'s
//! greeting is spoken *after* its own `questlog_isdone(co, 23)` check (so
//! it stays silent on a repeat visit). Neither early-out branch sets
//! `didsay`, so neither one stamps `last_talk`/turns to face the
//! player/sets `current_victim` - see [`Self::forest_william_handle_char_
//! message`]'s own inline comments for the exact split.
//!
//! The mantis turn-in's `give_money(co, 2000, "Imp mantis quest")`
//! reward (`forest.c:594`) is a direct gold credit, not a carried item -
//! entirely applied by `ugaris-server`'s `apply_forest_william_events`
//! (needs `World`/achievement-repository access), gated on `tmp == 1`
//! (first-time completion), same precedent as every other quest-
//! completion gold/item reward in this codebase.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_FORESTWILLIAM};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA16_MANTIS;
use crate::world::*;

use super::FOREST_QA;

/// C `char_dist(cn, co) > 10` (`forest.c:477`).
const WILLIAM_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`forest.c:460`).
const WILLIAM_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`forest.c:465`, `:548`).
const WILLIAM_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`forest.c:615`): idle "return to post" threshold.
const WILLIAM_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `hour >= 20 || hour < 6` (`forest.c:616`): night rest position.
const WILLIAM_NIGHT_REST_X: u16 = 176;
const WILLIAM_NIGHT_REST_Y: u16 = 120;

/// C `struct william_driver_data` (`forest.c:422-425`; C's own `dat`
/// pointer is declared `struct imp_driver_data *` but `set_data` only
/// ever allocates `sizeof(struct william_driver_data)` bytes for it - a
/// harmless C aliasing trick since `william_driver_data`'s two fields are
/// exactly `imp_driver_data`'s first two, and `william_driver` only ever
/// touches those two).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForestWilliamDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_forest_william_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestWilliamPlayerFacts {
    /// `PlayerRuntime::area3_william_state()`.
    pub william_state: i32,
    /// `questlog_isdone(co, 22)` (`forest.c:489`).
    pub quest22_done: bool,
    /// `questlog_isdone(co, 23)` (`forest.c:506`).
    pub quest23_done: bool,
}

/// A side effect [`World::process_forest_william_actions`] could not
/// apply directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestWilliamOutcomeEvent {
    /// Write the new `area3_ppd.william_state` back.
    UpdateWilliamState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// Write the new `area3_ppd.imp_state` back (`ppd->imp_state = 6`,
    /// `forest.c:589`, the mantis turn-in nudging `imp_driver`'s own
    /// state chain past its "waiting for mantiss" gate).
    UpdateImpState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, quest)` (`forest.c:493`/`:514`).
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `tmp = questlog_done(co, 23); ... if (tmp == 1) { give_money(co,
    /// 2000, "Imp mantis quest"); }` (`forest.c:591-594`) - see the module
    /// doc comment for why the gold reward itself needs `ugaris-server`'s
    /// `World`/achievement-repository access.
    QuestDoneMantis { player_id: CharacterId },
}

impl World {
    /// C `william_driver`'s per-tick body (`forest.c:428-628`).
    pub fn process_forest_william_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ForestWilliamPlayerFacts>,
        area_id: u16,
    ) -> Vec<ForestWilliamOutcomeEvent> {
        let william_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FORESTWILLIAM
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for william_id in william_ids {
            self.process_forest_william_messages(william_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_forest_william_messages(
        &mut self,
        william_id: CharacterId,
        player_facts: &HashMap<CharacterId, ForestWilliamPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ForestWilliamOutcomeEvent>,
    ) {
        let Some(william_name) = self
            .characters
            .get(&william_id)
            .map(|william| william.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::ForestWilliam(mut data)) = self
            .characters
            .get(&william_id)
            .and_then(|william| william.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&william_id)
            .map(|william| std::mem::take(&mut william.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.forest_william_handle_char_message(
                    william_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.forest_william_handle_text_message(
                    william_id,
                    &william_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.forest_william_handle_give_message(
                    william_id,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(william) = self.characters.get_mut(&william_id) {
            william.driver_state = Some(CharacterDriverState::ForestWilliam(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`forest.c:611-613`).
        if let (Some(william), Some((tx, ty))) =
            (self.characters.get(&william_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(william.x), i32::from(william.y), tx, ty) {
                if let Some(william_mut) = self.characters.get_mut(&william_id) {
                    let _ = turn(william_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (hour >= 20 ||
        // hour < 6) { secure_move_driver(cn, 176, 120, DX_RIGHT, ...); }
        // else { secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy,
        // DX_RIGHT, ...); } }` (`forest.c:615-625`). The daytime post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3-family driver uses.
        let last_talk = match self
            .characters
            .get(&william_id)
            .and_then(|william| william.driver_state.as_ref())
        {
            Some(CharacterDriverState::ForestWilliam(data)) => data.last_talk,
            _ => return,
        };
        if last_talk + WILLIAM_RETURN_TO_POST_TICKS < self.tick.0 {
            let (target_x, target_y) = if self.date.hour >= 20 || self.date.hour < 6 {
                (WILLIAM_NIGHT_REST_X, WILLIAM_NIGHT_REST_Y)
            } else {
                let Some(william) = self.characters.get(&william_id) else {
                    return;
                };
                (william.rest_x, william.rest_y)
            };
            self.secure_move_driver(
                william_id,
                target_x,
                target_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `william_driver`'s `NT_CHAR` branch (`forest.c:444-541`).
    fn forest_william_handle_char_message(
        &mut self,
        william_id: CharacterId,
        data: &mut ForestWilliamDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestWilliamPlayerFacts>,
        events: &mut Vec<ForestWilliamOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(william) = self.characters.get(&william_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`forest.c:448-451`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`forest.c:453-456`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`forest.c:459-462`).
        if tick < data.last_talk + WILLIAM_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`forest.c:464-467`).
        if tick < data.last_talk + WILLIAM_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`forest.c:469-473`).
        if william_id == player_id
            || !char_see_char(&william, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`forest.c:475-479`).
        if char_dist(&william, &player) > WILLIAM_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->william_state) { ... }` (`forest.c:485-534`).
        match facts.william_state {
            0 => {
                // C's greeting always fires here, *before* the
                // `questlog_isdone` check (`forest.c:487-495`) - the
                // opposite order from `case 3` below.
                self.npc_quiet_say(
                    william_id,
                    &format!(
                        "Greetings, {}. So nice of thee to visit. I am called {}.",
                        player.name, william.name
                    ),
                );
                if facts.quest22_done {
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id,
                        new_state: 3,
                    });
                    // No `didsay` here (C never sets it in this branch).
                } else {
                    events.push(ForestWilliamOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 22,
                    });
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id,
                        new_state: 1,
                    });
                    didsay = true;
                }
            }
            1 => {
                self.npc_quiet_say(
                    william_id,
                    "The imp asked me to tell you to go east and then northeast and hunt some bears.",
                );
                events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            // `william_state == 2`: waiting for the imp to raise
            // `imp_state` past the bear-hunt gate.
            3 => {
                if facts.quest23_done {
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id,
                        new_state: 7,
                    });
                    // No say, no `didsay` (C's own early-out here has no
                    // `say()` call at all, unlike `case 0` above).
                } else {
                    self.npc_quiet_say(
                        william_id,
                        &format!(
                            "Ah, hello {}. The imp told me thou hast done him a favor. That's nice of thee.",
                            player.name
                        ),
                    );
                    events.push(ForestWilliamOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 23,
                    });
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id,
                        new_state: 4,
                    });
                    didsay = true;
                }
            }
            4 => {
                self.npc_quiet_say(
                    william_id,
                    "Now if I may be so bold as to make a request of my own? It might sound strange to thee, friend, but I can make a nice stew from praying mantisses. I'd pay thee handsomely if thou couldst hunt one of them down and bring it to me.",
                );
                events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                    player_id,
                    new_state: 5,
                });
                didsay = true;
            }
            5 => {
                self.npc_quiet_say(
                    william_id,
                    "They live in the northern corner of the forest, close to a large clearing. Thou needst go east, then north and then north-west to get there.",
                );
                events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                    player_id,
                    new_state: 6,
                });
                didsay = true;
            }
            // `william_state == 6` (waiting for the mantis) or `7`
            // (quest done): no-op, matching C's empty `case 6:`/`case 7:
            // break;`.
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`forest.c:536-540`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `william_driver`'s `NT_TEXT` branch (`forest.c:545-574`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area3::astro2`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn forest_william_handle_text_message(
        &mut self,
        william_id: CharacterId,
        william_name: &str,
        data: &mut ForestWilliamDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestWilliamPlayerFacts>,
        events: &mut Vec<ForestWilliamOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`forest.c:548-550`).
        let tick = self.tick.0;
        if tick > data.last_talk + WILLIAM_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`forest.c:552-555`).
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

        // C `analyse_text_driver`'s own guard clauses (`forest.c:112-124`):
        // ignore our own talk, non-players/player-likes, not-visible (no
        // active distance check - the `char_dist(cn,co)>16` guard is
        // commented out in C, `forest.c:125`).
        if william_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(william) = self.characters.get(&william_id).cloned() else {
            return;
        };
        if !char_see_char(&william, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let william_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.william_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, william_name, &speaker.name, FOREST_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(william_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`forest.c:558-568`): two mutually
            // exclusive state buckets, ported directly.
            TextAnalysisOutcome::Matched(2) => {
                if william_state <= 2 {
                    data.last_talk = 0;
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                } else if (3..=6).contains(&william_state) {
                    data.last_talk = 0;
                    events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                        player_id: speaker_id,
                        new_state: 3,
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
        // (`forest.c:570-573`) - note this does *not* touch `dat->
        // last_talk` (except the explicit resets inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `william_driver`'s `NT_GIVE` branch (`forest.c:577-603`): a
    /// mantis handed over while `william_state == 6` completes quest 23
    /// and nudges `imp_state` to `6`; anything else is handed straight
    /// back (falling back to destroying it if the player's inventory is
    /// full), matching C's plain `give_char_item` (not `give_char_item_
    /// smart`).
    fn forest_william_handle_give_message(
        &mut self,
        william_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestWilliamPlayerFacts>,
        events: &mut Vec<ForestWilliamOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&william_id)
            .and_then(|william| william.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let william_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.william_state)
            .unwrap_or(0);

        if template_id == IID_AREA16_MANTIS && william_state == 6 {
            // C `if (it[in].ID == IID_AREA16_MANTIS && ppd &&
            // ppd->william_state == 6) { destroy_item(in); ch[cn].citem =
            // 0; ppd->william_state = 7; ppd->imp_state = 6; say(...);
            // tmp = questlog_done(co, 23); destroy_item_byID(co,
            // IID_AREA16_MANTIS); if (tmp == 1) { give_money(...); } }`
            // (`forest.c:583-595`).
            self.destroy_item(item_id);
            events.push(ForestWilliamOutcomeEvent::UpdateWilliamState {
                player_id: giver_id,
                new_state: 7,
            });
            events.push(ForestWilliamOutcomeEvent::UpdateImpState {
                player_id: giver_id,
                new_state: 6,
            });
            self.npc_quiet_say(
                william_id,
                &format!(
                    "Ah. I thank thee, {}. This will make a nice stew.",
                    self.characters
                        .get(&giver_id)
                        .map(|giver| giver.name.clone())
                        .unwrap_or_default()
                ),
            );
            events.push(ForestWilliamOutcomeEvent::QuestDoneMantis {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA16_MANTIS);
        } else {
            // C `else { say("Thou hast better use..."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`forest.c:596-600`).
            self.npc_quiet_say(
                william_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}
