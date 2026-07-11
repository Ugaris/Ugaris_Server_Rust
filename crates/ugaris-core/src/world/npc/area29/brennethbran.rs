//! Brenneth Brannington NPC (`CDR_BRENNETHBRAN`), the memory-loss assassin
//! who runs "A Grolm's Spoils"/"A Thief's Loot"/"A Necromancer's Notes"
//! (quests 41-43).
//!
//! Ports `src/area/29/brannington.c::brenneth_brannington_driver` (`:858-
//! 1121`) plus the shared `analyse_text_driver`/`qa[]` table (`:86-206`,
//! ported as [`super::AREA29_QA`] in `world::npc::area29`, the same table
//! every other `brannington.c` NPC driver shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area29::
//! spiritbran`/`countbran`: the caller supplies a per-player fact snapshot
//! ([`BrennethBranPlayerFacts`]) up front and applies the returned
//! [`BrennethBranOutcomeEvent`]s afterwards, since `staffer_ppd.
//! brennethbran_state` and the `QLOG` 41/42/43 quest-log entries live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `brenneth_brannington_driver`'s sixteen-state (`0`-`15`) dialogue chain
//! is three back-to-back mini quests sharing one state counter: greeting
//! (opens quest 41) -> "lost my memory" -> "attacked by something" ->
//! "perhaps a grolm has something" (waiting: state `4`) -> (`NT_GIVE`:
//! hand in `IID_STAFF_BRENNETHDAGGER`, quest 41 done, state jumps to `5`)
//! -> "has my name on it" (opens quest 42) -> "maybe I was foolish" ->
//! "keep looking, maybe hidden" (waiting: state `8`) -> (`NT_GIVE`: hand in
//! `IID_STAFF_BRENNETHPOTION`, quest 42 done, state jumps to `9`) ->
//! "similar on my sword, a lethal poison" (opens quest 43) -> "can't
//! imagine wanting to poison" (waiting: state `11`) -> (`NT_GIVE`: hand in
//! `IID_STAFF_BRENNETHJOURNAL`, quest 43 done, state jumps to `12`) -> "I
//! was to kill the thief mages" -> "giving that life up now" -> "thank you"
//! (with an `emote`) -> done (state `15`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::spiritbran`/`countbran`'s own `NT_TEXT`
//!   branch, this driver's own C body has no `dat->current_victim`
//!   staleness-reset preamble and no victim-mismatch early-out at all -
//!   reproduced verbatim: replies to *any* nearby player's matched small
//!   talk, not just its tracked victim.
//! - C `case 2:` (`:1021-1039`) resets to whichever of the three mini
//!   quests' greeting state the player is currently mid-way through (four
//!   separate range checks: `<=4` -> `0`, `5..=8` -> `5`, `9..=11` -> `9`,
//!   `12..=15` -> `12`), ported as [`BrennethBranOutcomeEvent::
//!   ResetToMiniQuestStart`] with the resolved target state computed in
//!   `World` itself (pure `i32` math, no need to round-trip through
//!   `ugaris-server`).
//! - C `case 3:` (`:1040-1045`) speaks a visible `say(cn, "reset done")`
//!   line (not `quiet_say`) before wiping the state to `0` - only if the
//!   speaker is `CF_GOD`, matching `world::npc::area29::spiritbran`'s own
//!   `case 3` precedent exactly.
//! - No self-defense/regen/spell-self cascade exists in C's `brenneth_
//!   brannington_driver` body at all (matching every other `brannington.c`
//!   "pure talker" NPC) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1120`) is not
//!   ported, matching the established `world::npc::area29::spiritbran`/
//!   `countbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:907`).
const BRENNETHBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const BRENNETHBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:890`).
const BRENNETHBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:895`).
const BRENNETHBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:1114`): idle "return to post" threshold.
const BRENNETHBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_brennethbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrennethBranPlayerFacts {
    /// `PlayerRuntime::staffer_brennethbran_state()`.
    pub brennethbran_state: i32,
    /// `PlayerRuntime::quest_log.is_done(42)` (C `questlog_isdone(co,
    /// 42)`, `brannington.c:945`): `case 5`'s fast-forward guard.
    pub quest42_is_done: bool,
    /// `PlayerRuntime::quest_log.is_done(43)` (C `questlog_isdone(co,
    /// 43)`, `brannington.c:969`): `case 9`'s fast-forward guard.
    pub quest43_is_done: bool,
}

/// A side effect [`World::process_brennethbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrennethBranOutcomeEvent {
    /// Write the new `staffer_ppd.brennethbran_state` back.
    UpdateBrennethBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 41/42/43)`.
    QuestOpen { player_id: CharacterId, quest: u32 },
    /// C `questlog_done(co, 41/42/43)`.
    QuestDone { player_id: CharacterId, quest: u32 },
    /// C `case 2:` (`brannington.c:1021-1039`): reset back to the start of
    /// whichever of the three mini quests the player is currently mid-way
    /// through. `new_state` is already resolved to the target state.
    ResetToMiniQuestStart {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`brannington.c:1040-1045`): the god-only "reset me"
    /// full state wipe.
    ResetBrennethBran { player_id: CharacterId },
}

impl World {
    /// C `brenneth_brannington_driver`'s per-tick body (`brannington.c:858-
    /// 1121`).
    pub fn process_brennethbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, BrennethBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<BrennethBranOutcomeEvent> {
        let brennethbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BRENNETHBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for brennethbran_id in brennethbran_ids {
            self.process_brennethbran_messages(brennethbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_brennethbran_messages(
        &mut self,
        brennethbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, BrennethBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<BrennethBranOutcomeEvent>,
    ) {
        let Some(brennethbran_name) = self
            .characters
            .get(&brennethbran_id)
            .map(|brennethbran| brennethbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::BrennethBran(mut data)) = self
            .characters
            .get(&brennethbran_id)
            .and_then(|brennethbran| brennethbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&brennethbran_id)
            .map(|brennethbran| std::mem::take(&mut brennethbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.brennethbran_handle_char_message(
                    brennethbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.brennethbran_handle_text_message(
                    brennethbran_id,
                    &brennethbran_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.brennethbran_handle_give_message(
                    brennethbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(brennethbran) = self.characters.get_mut(&brennethbran_id) {
            brennethbran.driver_state = Some(CharacterDriverState::BrennethBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:1110-1112`).
        if let (Some(brennethbran), Some((tx, ty))) =
            (self.characters.get(&brennethbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(brennethbran.x), i32::from(brennethbran.y), tx, ty)
            {
                if let Some(brennethbran_mut) = self.characters.get_mut(&brennethbran_id) {
                    let _ = turn(brennethbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`brannington.c:1114-1118`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let last_talk = if let Some(brennethbran) = self.characters.get(&brennethbran_id) {
            match brennethbran.driver_state.as_ref() {
                Some(CharacterDriverState::BrennethBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + BRENNETHBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(brennethbran) = self.characters.get(&brennethbran_id) else {
                return;
            };
            let (post_x, post_y) = (brennethbran.rest_x, brennethbran.rest_y);
            self.secure_move_driver(
                brennethbran_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `brenneth_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 874-1013`).
    #[allow(clippy::too_many_arguments)]
    fn brennethbran_handle_char_message(
        &mut self,
        brennethbran_id: CharacterId,
        data: &mut BrennethBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BrennethBranPlayerFacts>,
        events: &mut Vec<BrennethBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(brennethbran) = self.characters.get(&brennethbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:877-881`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:883-887`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:889-893`).
        if tick < data.last_talk + BRENNETHBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:895-898`).
        if tick < data.last_talk + BRENNETHBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:900-904`).
        if brennethbran_id == player_id
            || !char_see_char(&brennethbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:906-
        // 910`).
        if char_dist(&brennethbran, &player) > BRENNETHBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.brennethbran_state;
        match facts.brennethbran_state {
            // C `case 0:` (`brannington.c:917-923`).
            0 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "Greetings stranger... I'm afraid I can't be of much help to you, as I can't recall much, except my name...",
                );
                events.push(BrennethBranOutcomeEvent::QuestOpen {
                    player_id,
                    quest: 41,
                });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:924-929`).
            1 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "Perhaps you can help me, though I do not wish to burden you... You see, it appears that I have lost my memory due to being attacked by something...",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington.c:930-935`).
            2 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "I overheard my captor say something about a grolm before I got rescued by strangers who dropped me off here and went on their way...",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington.c:936-941`).
            3 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "Perhaps one of these... grolms... has taken something that may help me recall why I'm here... If you find anything, please bring it back to me...",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:942-943`): waiting for
            // dagger.
            4 => {}
            // C `case 5:` (`brannington.c:944-953`).
            5 => {
                if facts.quest42_is_done {
                    new_state = 9;
                } else {
                    self.npc_quiet_say(
                        brennethbran_id,
                        "It does have my name on it, but I don't recall anything of being a fighter...",
                    );
                    events.push(BrennethBranOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 42,
                    });
                    new_state = 6;
                    didsay = true;
                }
            }
            // C `case 6:` (`brannington.c:954-959`).
            6 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "Maybe I was foolish enough to think I could defeat grolms, and that got the best of me...",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`brannington.c:960-965`).
            7 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "If you will, please continue looking for more items... Maybe my captor managed to hide something...",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8: break;` (`brannington.c:966-967`): waiting for
            // potion.
            8 => {}
            // C `case 9:` (`brannington.c:968-978`).
            9 => {
                if facts.quest43_is_done {
                    new_state = 12;
                } else {
                    self.npc_quiet_say(
                        brennethbran_id,
                        "I found something similar on my sword, and when the bartender looked at it, he said it was a lethal poison... This potion must be just that... ",
                    );
                    events.push(BrennethBranOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 43,
                    });
                    new_state = 10;
                    didsay = true;
                }
            }
            // C `case 10:` (`brannington.c:979-983`).
            10 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "I just can't imagine why I would want to poison... anyone or anything...",
                );
                new_state = 11;
                didsay = true;
            }
            // C `case 11: break;` (`brannington.c:984-985`): waiting for
            // journal.
            11 => {}
            // C `case 12:` (`brannington.c:986-991`).
            12 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "I was to kill these thief mages were they to get out of hand... And probably even you and who knows who else...",
                );
                new_state = 13;
                didsay = true;
            }
            // C `case 13:` (`brannington.c:992-997`).
            13 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "But I'm going to give that life up now... This loss of memory is perhaps more of a blessing than it is a curse...",
                );
                new_state = 14;
                didsay = true;
            }
            // C `case 14:` (`brannington.c:998-1003`).
            14 => {
                self.npc_quiet_say(
                    brennethbran_id,
                    "Thank you for helping me, I will not forget this, I hope.",
                );
                self.npc_emote(brennethbran_id, "smiles");
                new_state = 15;
                didsay = true;
            }
            // C `case 15: break;` (`brannington.c:1004-1005`): all done.
            15 => {}
            _ => {}
        }

        if new_state != facts.brennethbran_state {
            events.push(BrennethBranOutcomeEvent::UpdateBrennethBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1007-1011`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `brenneth_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1016-1052`), wired through the generic `analyse_text_qa` matcher
    /// (same pattern as `world::npc::area29::spiritbran`/`countbran`'s text
    /// handler). This branch has no victim-staleness-reset preamble and no
    /// victim-mismatch early-out (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn brennethbran_handle_text_message(
        &mut self,
        brennethbran_id: CharacterId,
        brennethbran_name: &str,
        data: &mut BrennethBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BrennethBranPlayerFacts>,
        events: &mut Vec<BrennethBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1019`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if brennethbran_id == speaker_id {
            return;
        }
        let Some(brennethbran) = self.characters.get(&brennethbran_id).cloned() else {
            return;
        };
        if char_dist(&brennethbran, &speaker) > BRENNETHBRAN_QA_DISTANCE
            || !char_see_char(&brennethbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let brennethbran_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.brennethbran_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, brennethbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(brennethbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1022-1039`): reset back to the
            // start of whichever of the three mini quests is in progress.
            TextAnalysisOutcome::Matched(2) => {
                let new_state = if brennethbran_state <= 4 {
                    Some(0)
                } else if (5..=8).contains(&brennethbran_state) {
                    Some(5)
                } else if (9..=11).contains(&brennethbran_state) {
                    Some(9)
                } else if (12..=15).contains(&brennethbran_state) {
                    Some(12)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    data.last_talk = 0;
                    events.push(BrennethBranOutcomeEvent::ResetToMiniQuestStart {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:1040-1045`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")` line
            // first.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(brennethbran_id, "reset done");
                    events.push(BrennethBranOutcomeEvent::ResetBrennethBran {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not yet
            // ported) is unhandled by brenneth's own C `switch` but still
            // counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:1047-1050`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `brenneth_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1055-1102`).
    #[allow(clippy::too_many_arguments)]
    fn brennethbran_handle_give_message(
        &mut self,
        brennethbran_id: CharacterId,
        data: &mut BrennethBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BrennethBranPlayerFacts>,
        events: &mut Vec<BrennethBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&brennethbran_id)
            .and_then(|brennethbran| brennethbran.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let tick = self.tick.0;

        // C `if (it[in].ID == IID_STAFF_BRENNETHDAGGER && ppd &&
        // ppd->brennethbran_state <= 4)` (`brannington.c:1062`).
        if item.template_id == IID_STAFF_BRENNETHDAGGER
            && facts.is_some_and(|facts| facts.brennethbran_state <= 4)
        {
            self.npc_quiet_say(brennethbran_id, "I see... so this was my dagger?");
            events.push(BrennethBranOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 41,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BRENNETHDAGGER);
            events.push(BrennethBranOutcomeEvent::UpdateBrennethBranState {
                player_id: giver_id,
                new_state: 5,
            });
            data.last_talk = tick;
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (it[in].ID == IID_STAFF_BRENNETHPOTION && ppd &&
        // ppd->brennethbran_state >= 5 && ppd->brennethbran_state <= 8)`
        // (`brannington.c:1070-1071`).
        if item.template_id == IID_STAFF_BRENNETHPOTION
            && facts.is_some_and(|facts| (5..=8).contains(&facts.brennethbran_state))
        {
            self.npc_quiet_say(
                brennethbran_id,
                "A potion? And the bottle has the same symbol on it as my sword?",
            );
            events.push(BrennethBranOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 42,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BRENNETHPOTION);
            events.push(BrennethBranOutcomeEvent::UpdateBrennethBranState {
                player_id: giver_id,
                new_state: 9,
            });
            data.last_talk = tick;
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (it[in].ID == IID_STAFF_BRENNETHJOURNAL && ppd &&
        // ppd->brennethbran_state >= 9 && ppd->brennethbran_state <= 11)`
        // (`brannington.c:1079-1080`).
        if item.template_id == IID_STAFF_BRENNETHJOURNAL
            && facts.is_some_and(|facts| (9..=11).contains(&facts.brennethbran_state))
        {
            self.npc_quiet_say(brennethbran_id, "Now it all makes sense...\t");
            events.push(BrennethBranOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 43,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BRENNETHJOURNAL);
            events.push(BrennethBranOutcomeEvent::UpdateBrennethBranState {
                player_id: giver_id,
                new_state: 12,
            });
            data.last_talk = tick;
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`brannington.c:1088-1094`): hand the
        // item back to the giver. C explicitly zeroes `ch[cn].citem` inside
        // this branch itself (`:1093`), so the trailing "let it vanish"
        // catch-all (`:1096-1100`) is a no-op for this path - unlike the
        // three quest-item branches above, which leave `citem` set and rely
        // on that catch-all.
        self.npc_quiet_say(
            brennethbran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_BRENNETHBRAN, CDR_LOSTCON};
use crate::item_driver::{
    IID_STAFF_BRENNETHDAGGER, IID_STAFF_BRENNETHJOURNAL, IID_STAFF_BRENNETHPOTION,
};

/// C `struct brenneth_brannington_data` (`src/area/29/brannington.c:858-
/// ~863`, inline local declaration mirrored on `world::npc::area29::
/// spiritbran`'s `struct spirit_brannington_data` shape).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrennethBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
