//! Count Brannington NPC (`CDR_COUNTBRAN`), who runs "The Jewels of
//! Brannington" (quest 40) and hands out mausoleum keys as each of the
//! three stolen jewels is returned.
//!
//! Ports `src/area/29/brannington.c::count_brannington_driver` (`:590-851`)
//! plus its own `countbran_give_keys` helper (`:546-583`) and the shared
//! `analyse_text_driver`/`qa[]` table (`:86-206`, ported as
//! [`super::AREA29_QA`] in `world::npc::area29`, the same table every other
//! `brannington.c` NPC driver shares). Follows the same `World`/
//! `PlayerRuntime` split established by `world::npc::area29::spiritbran`:
//! the caller supplies a per-player fact snapshot ([`CountBranPlayerFacts`])
//! up front and applies the returned [`CountBranOutcomeEvent`]s afterwards,
//! since `staffer_ppd.countbran_state`/`countbran_bits` and the `QLOG` 40
//! quest-log entry live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `count_brannington_driver`'s five-state (`0`-`4`) greeting chain opens
//! quest 40, then waits at state `4` for `NT_GIVE`: each of the three
//! jewels (`IID_STAFF_COUNTJEWEL`/`COUNTESSAJEWEL`/`DAUGHTERJEWEL`) grants
//! its own manually-scaled exp reward (via [`crate::quest::scale_exp`],
//! *not* the `complete_legacy`/questlog-table exp path - quest 40's own
//! nominal exp is `0`, "exp awarded in driver" per
//! `src/system/questlog.c:150-151`), a one-time gold reward on first
//! completion, sets one `countbran_bits` bit, and calls
//! `countbran_give_keys` to hand out any of the three mausoleum keys the
//! player has unlocked but doesn't already carry. Once all three bits are
//! set, quest 40 is marked done (`QuestLog::mark_done`, not
//! `complete_legacy`, since the nominal exp is `0` - matches C's own
//! `questlog_done` no-op-exp behavior for this quest).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::spiritbran`'s own `NT_TEXT` branch, this
//!   driver's own C body has no `dat->current_victim` staleness-reset
//!   preamble and no victim-mismatch early-out at all - reproduced
//!   verbatim: replies to *any* nearby player's matched small talk, not
//!   just its tracked victim.
//! - C `case 3:` (`:700-706`) speaks a visible `say(cn, "reset done")` line
//!   (not `quiet_say`) before wiping *all three* Brannington family
//!   quest-40 states at once (`countbran_bits`/`countbran_state`/
//!   `countessabran_state`/`daughterbran_state`), unlike
//!   `world::npc::area28::aristocrat`/`world::npc::area29::spiritbran`'s
//!   single-state god resets - ported as
//!   [`CountBranOutcomeEvent::ResetAllBranStates`], applied by
//!   `ugaris-server`'s `apply_countbran_events` since it touches fields
//!   `world::npc::area29::countessabran`/`daughterbran` also read.
//! - C's `IID_ARKHATA_LETTER3` `NT_GIVE` sub-branch (`:814-818`) is not
//!   ported: it belongs to area 37's Arkhata quest chain
//!   (`src/area/37/arkhata.c`), which is still entirely unported (see
//!   `PORTING_TODO.md`'s Area 37 entry) and `arkhata_ppd.letter_bits` has
//!   no accessor in `crate::player` yet. A player handing in that letter
//!   here falls through to the generic "no use for it" branch instead - a
//!   documented gap, not a silent one.
//! - Unlike every other `brannington.c` NPC's identical fallback line
//!   (which this port's sibling files also use `npc_quiet_say` for), C's
//!   own fallback here (`:820`) is `quiet_say`, matching this file; C's
//!   `countbran_give_keys` "here, take this key" line (`:581`) is the
//!   non-quiet `say`, ported via `World::npc_say`.
//! - No self-defense/regen/spell-self cascade exists in C's `count_
//!   brannington_driver` body at all (matching `world::npc::area29::
//!   spiritbran`'s identical observation for other "pure talker" NPCs) -
//!   this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:850`) is not
//!   ported, matching the established `world::thomas`/`world::npc::area29::
//!   spiritbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::quest::scale_exp;
use crate::world::exp::level_value;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:640`).
const COUNTBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const COUNTBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:623`).
const COUNTBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:628`).
const COUNTBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:844`): idle "return to post" threshold.
const COUNTBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C questlog index 40, "The Jewels of Brannington".
const QLOG_COUNTBRAN: usize = 40;
/// C `ppd->countbran_bits & 1` (`brannington.c:553`/`725`): the Count's own
/// jewel.
const COUNTBRAN_BIT_COUNT_JEWEL: i32 = 1;
/// C `ppd->countbran_bits & 2` (`brannington.c:562`/`755`): the Countessa's
/// jewel.
const COUNTBRAN_BIT_COUNTESSA_JEWEL: i32 = 2;
/// C `ppd->countbran_bits & 4` (`brannington.c:571`/`785`): the Daughter's
/// jewel.
const COUNTBRAN_BIT_DAUGHTER_JEWEL: i32 = 4;
/// C `ppd->countbran_bits & (1 | 2 | 4)` (`brannington.c:751`/`782`/`810`):
/// all three jewels returned.
const COUNTBRAN_BITS_ALL_JEWELS: i32 = 1 | 2 | 4;
/// C `give_money(co, 1000 * 100, ...)` (`brannington.c:747`).
const COUNTBRAN_COUNT_JEWEL_GOLD: u32 = 1000 * 100;
/// C `give_money(co, 500 * 100, ...)` (`brannington.c:777`/`807`).
const COUNTBRAN_SIDE_JEWEL_GOLD: u32 = 500 * 100;

/// Per-player facts [`World::process_countbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountBranPlayerFacts {
    /// `PlayerRuntime::staffer_countbran_state()`.
    pub countbran_state: i32,
    /// `PlayerRuntime::staffer_countbran_bits()`.
    pub countbran_bits: i32,
    /// `PlayerRuntime::quest_log.count(40)` (C `questlog_count(co, 40)`),
    /// used to scale each jewel's manual exp reward.
    pub quest40_count: u8,
    /// `PlayerRuntime::quest_log.is_done(40)` (C `questlog_isdone(co,
    /// 40)`).
    pub quest40_is_done: bool,
}

/// A side effect [`World::process_countbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CountBranOutcomeEvent {
    /// Write the new `staffer_ppd.countbran_state` back.
    UpdateCountBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `if (!questlog_isdone(co, 40)) { questlog_open(co, 40); }`
    /// (`brannington.c:652-654`).
    QuestOpen { player_id: CharacterId },
    /// C `ppd->countbran_bits |= 1/2/4;`.
    SetCountBranBit { player_id: CharacterId, bit: i32 },
    /// C `questlog_done(co, 40)` once `countbran_bits == (1|2|4)`
    /// (`brannington.c:751`/`782`/`810`) - quest 40's own nominal exp is
    /// `0` (`src/system/questlog.c:150-151`), so this only needs
    /// `QuestLog::mark_done`'s bookkeeping, not the full `complete_legacy`
    /// exp path every jewel reward already handled manually.
    MarkQuestDone { player_id: CharacterId },
    /// C `countbran_give_keys` (`brannington.c:546-583`): create and hand
    /// over each listed mausoleum key (`1` = key 1, `2` = key 2, `3` = key
    /// 3) that the player doesn't already carry.
    GiveMausoleumKeys {
        player_id: CharacterId,
        keys: Vec<u8>,
    },
    /// C `case 3:` (`brannington.c:700-706`): the god-only "reset me" wipe,
    /// clearing `countbran_bits`/`countbran_state`/`countessabran_state`/
    /// `daughterbran_state` all at once.
    ResetAllBranStates { player_id: CharacterId },
}

impl World {
    /// C `count_brannington_driver`'s per-tick body (`brannington.c:590-
    /// 851`).
    pub fn process_countbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CountBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<CountBranOutcomeEvent> {
        let countbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_COUNTBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for countbran_id in countbran_ids {
            self.process_countbran_messages(countbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_countbran_messages(
        &mut self,
        countbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, CountBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<CountBranOutcomeEvent>,
    ) {
        let Some(countbran_name) = self
            .characters
            .get(&countbran_id)
            .map(|countbran| countbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::CountBran(mut data)) = self
            .characters
            .get(&countbran_id)
            .and_then(|countbran| countbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&countbran_id)
            .map(|countbran| std::mem::take(&mut countbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.countbran_handle_char_message(
                    countbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.countbran_handle_text_message(
                    countbran_id,
                    &countbran_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.countbran_handle_give_message(countbran_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(countbran) = self.characters.get_mut(&countbran_id) {
            countbran.driver_state = Some(CharacterDriverState::CountBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:840-842`).
        if let (Some(countbran), Some((tx, ty))) =
            (self.characters.get(&countbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(countbran.x), i32::from(countbran.y), tx, ty)
            {
                if let Some(countbran_mut) = self.characters.get_mut(&countbran_id) {
                    let _ = turn(countbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`brannington.c:844-848`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let last_talk = if let Some(countbran) = self.characters.get(&countbran_id) {
            match countbran.driver_state.as_ref() {
                Some(CharacterDriverState::CountBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + COUNTBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(countbran) = self.characters.get(&countbran_id) else {
                return;
            };
            let (post_x, post_y) = (countbran.rest_x, countbran.rest_y);
            self.secure_move_driver(
                countbran_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `count_brannington_driver`'s `NT_CHAR` branch (`brannington.c:607-
    /// 684`).
    #[allow(clippy::too_many_arguments)]
    fn countbran_handle_char_message(
        &mut self,
        countbran_id: CharacterId,
        data: &mut CountBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CountBranPlayerFacts>,
        events: &mut Vec<CountBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(countbran) = self.characters.get(&countbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:610-614`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:616-620`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:622-626`).
        if tick < data.last_talk + COUNTBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:628-631`).
        if tick < data.last_talk + COUNTBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:633-637`).
        if countbran_id == player_id
            || !char_see_char(&countbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:639-
        // 643`).
        if char_dist(&countbran, &player) > COUNTBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.countbran_state;
        match facts.countbran_state {
            // C `case 0:` (`brannington.c:650-657`).
            0 => {
                self.npc_quiet_say(
                    countbran_id,
                    &format!("Greetings, {}, welcome to Brannington!", player.name),
                );
                if !facts.quest40_is_done {
                    events.push(CountBranOutcomeEvent::QuestOpen { player_id });
                }
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:658-663`).
            1 => {
                self.npc_quiet_say(
                    countbran_id,
                    "My guards told me that you were coming, and they said you might be able to help me with my problems.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington.c:664-669`).
            2 => {
                self.npc_quiet_say(
                    countbran_id,
                    "You see, my family was recently robbed by three thief mages of our most valuable jewelry, and we would like to ask for your aid.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington.c:670-674`).
            3 => {
                self.npc_quiet_say(
                    countbran_id,
                    "If you can bring back our jewelry, we would be very thankful indeed.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:675-676`): waiting for
            // jewelry.
            4 => {}
            _ => {}
        }

        if new_state != facts.countbran_state {
            events.push(CountBranOutcomeEvent::UpdateCountBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:678-682`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `count_brannington_driver`'s `NT_TEXT` branch (`brannington.c:687-
    /// 713`).
    #[allow(clippy::too_many_arguments)]
    fn countbran_handle_text_message(
        &mut self,
        countbran_id: CharacterId,
        countbran_name: &str,
        data: &mut CountBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CountBranPlayerFacts>,
        events: &mut Vec<CountBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:690`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if countbran_id == speaker_id {
            return;
        }
        let Some(countbran) = self.characters.get(&countbran_id).cloned() else {
            return;
        };
        if char_dist(&countbran, &speaker) > COUNTBRAN_QA_DISTANCE
            || !char_see_char(&countbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let countbran_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.countbran_state)
            .unwrap_or(0);
        let countbran_bits = player_facts
            .get(&speaker_id)
            .map(|facts| facts.countbran_bits)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, countbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(countbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:693-699`): reset back to the
            // greeting and re-issue any unclaimed mausoleum keys.
            TextAnalysisOutcome::Matched(2) => {
                if countbran_state <= 4 {
                    data.last_talk = 0;
                    events.push(CountBranOutcomeEvent::UpdateCountBranState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                    self.countbran_give_keys(countbran_id, speaker_id, countbran_bits, events);
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:700-706`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")`
            // line first (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(countbran_id, "reset done");
                    events.push(CountBranOutcomeEvent::ResetAllBranStates {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not yet
            // ported) is unhandled by count's own C `switch` but still
            // counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:708-711`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `count_brannington_driver`'s `NT_GIVE` branch (`brannington.c:716-
    /// 832`).
    fn countbran_handle_give_message(
        &mut self,
        countbran_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CountBranPlayerFacts>,
        events: &mut Vec<CountBranOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&countbran_id)
            .and_then(|countbran| countbran.cursor_item.take())
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

        // C `if (it[in].ID == IID_STAFF_COUNTJEWEL && ppd &&
        // !(ppd->countbran_bits & 1))` (`brannington.c:725`).
        if item.template_id == IID_STAFF_COUNTJEWEL
            && is_player
            && facts.is_some_and(|facts| facts.countbran_bits & COUNTBRAN_BIT_COUNT_JEWEL == 0)
        {
            self.npc_quiet_say(
                countbran_id,
                &format!(
                    "Thank you so much for bringing this back, {}. It has been in the family for generations. Here is your reward.",
                    giver.name
                ),
            );
            self.destroy_items_by_template_id(giver_id, IID_STAFF_COUNTJEWEL);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_THIEFKEY1);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY1);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY2);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY3);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY12);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY23);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY13);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_REDKEY123);

            let prior_completions = facts.map(|facts| facts.quest40_count).unwrap_or(0);
            let scaled = scale_exp(prior_completions, 60000);
            let cap = i64::from(level_value(giver.level)) / 4;
            self.give_exp(giver_id, scaled.min(cap), u32::from(self.area_id));
            if prior_completions == 0 {
                self.countbran_give_money(giver_id, COUNTBRAN_COUNT_JEWEL_GOLD);
            }

            let new_bits =
                facts.map(|facts| facts.countbran_bits).unwrap_or(0) | COUNTBRAN_BIT_COUNT_JEWEL;
            events.push(CountBranOutcomeEvent::SetCountBranBit {
                player_id: giver_id,
                bit: COUNTBRAN_BIT_COUNT_JEWEL,
            });
            if new_bits == COUNTBRAN_BITS_ALL_JEWELS {
                events.push(CountBranOutcomeEvent::MarkQuestDone {
                    player_id: giver_id,
                });
            }
            self.countbran_give_keys(countbran_id, giver_id, new_bits, events);
            self.destroy_item(item_id);
            return;
        }

        // C `else if (it[in].ID == IID_STAFF_COUNTESSAJEWEL && ppd &&
        // !(ppd->countbran_bits & 2))` (`brannington.c:755`).
        if item.template_id == IID_STAFF_COUNTESSAJEWEL
            && is_player
            && facts.is_some_and(|facts| facts.countbran_bits & COUNTBRAN_BIT_COUNTESSA_JEWEL == 0)
        {
            self.npc_quiet_say(
                countbran_id,
                &format!(
                    "Ah, my wife will be most pleased! Here is your reward, {}. If you go to my wife she will give you an additional reward.",
                    giver.name
                ),
            );
            self.destroy_items_by_template_id(giver_id, IID_STAFF_COUNTESSAJEWEL);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_THIEFKEY2);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY1);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY2);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY3);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY12);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY23);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY13);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_BLUEKEY123);

            let prior_completions = facts.map(|facts| facts.quest40_count).unwrap_or(0);
            let scaled = scale_exp(prior_completions, 30000);
            let cap = i64::from(level_value(giver.level)) / 4;
            self.give_exp(giver_id, scaled.min(cap), u32::from(self.area_id));
            if prior_completions == 0 {
                self.countbran_give_money(giver_id, COUNTBRAN_SIDE_JEWEL_GOLD);
            }

            let new_bits = facts.map(|facts| facts.countbran_bits).unwrap_or(0)
                | COUNTBRAN_BIT_COUNTESSA_JEWEL;
            events.push(CountBranOutcomeEvent::SetCountBranBit {
                player_id: giver_id,
                bit: COUNTBRAN_BIT_COUNTESSA_JEWEL,
            });
            self.countbran_give_keys(countbran_id, giver_id, new_bits, events);
            if new_bits == COUNTBRAN_BITS_ALL_JEWELS {
                events.push(CountBranOutcomeEvent::MarkQuestDone {
                    player_id: giver_id,
                });
            }
            self.destroy_item(item_id);
            return;
        }

        // C `else if (it[in].ID == IID_STAFF_DAUGHTERJEWEL && ppd &&
        // !(ppd->countbran_bits & 4))` (`brannington.c:785`).
        if item.template_id == IID_STAFF_DAUGHTERJEWEL
            && is_player
            && facts.is_some_and(|facts| facts.countbran_bits & COUNTBRAN_BIT_DAUGHTER_JEWEL == 0)
        {
            self.npc_quiet_say(
                countbran_id,
                &format!(
                    "Returning this will heal my daughter's heart. She has been so upset about losing it. Let me reward you now, {}, and if you go to my daughter, she will further reward you.",
                    giver.name
                ),
            );
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DAUGHTERJEWEL);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_THIEFKEY3);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY1);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY2);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY3);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY12);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY23);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY13);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_GREENKEY123);

            let prior_completions = facts.map(|facts| facts.quest40_count).unwrap_or(0);
            let scaled = scale_exp(prior_completions, 30000);
            let cap = i64::from(level_value(giver.level)) / 4;
            self.give_exp(giver_id, scaled.min(cap), u32::from(self.area_id));
            if prior_completions == 0 {
                self.countbran_give_money(giver_id, COUNTBRAN_SIDE_JEWEL_GOLD);
            }

            let new_bits =
                facts.map(|facts| facts.countbran_bits).unwrap_or(0) | COUNTBRAN_BIT_DAUGHTER_JEWEL;
            events.push(CountBranOutcomeEvent::SetCountBranBit {
                player_id: giver_id,
                bit: COUNTBRAN_BIT_DAUGHTER_JEWEL,
            });
            if new_bits == COUNTBRAN_BITS_ALL_JEWELS {
                events.push(CountBranOutcomeEvent::MarkQuestDone {
                    player_id: giver_id,
                });
            }
            self.countbran_give_keys(countbran_id, giver_id, new_bits, events);
            self.destroy_item(item_id);
            return;
        }

        // C's `IID_ARKHATA_LETTER3` branch (`brannington.c:814-818`) is not
        // ported - see the module doc comment's deviations list.

        // C's fallback `else` branch (`brannington.c:819-825`): hand the
        // item back to the giver.
        self.npc_quiet_say(
            countbran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `countbran_give_keys` (`brannington.c:546-583`): hands out each
    /// mausoleum key the player has unlocked (via `countbran_bits`) but
    /// doesn't already carry, speaking a `say` (not `quiet_say`) summary
    /// line only when at least one key is actually due.
    fn countbran_give_keys(
        &mut self,
        countbran_id: CharacterId,
        giver_id: CharacterId,
        bits: i32,
        events: &mut Vec<CountBranOutcomeEvent>,
    ) {
        let mut keys = Vec::new();
        if bits & COUNTBRAN_BIT_COUNT_JEWEL != 0
            && !self.character_has_item_template(giver_id, IID_STAFF_MAUSOLEUMKEY1)
        {
            keys.push(1u8);
        }
        if bits & COUNTBRAN_BIT_COUNTESSA_JEWEL != 0
            && !self.character_has_item_template(giver_id, IID_STAFF_MAUSOLEUMKEY2)
        {
            keys.push(2u8);
        }
        if bits & COUNTBRAN_BIT_DAUGHTER_JEWEL != 0
            && !self.character_has_item_template(giver_id, IID_STAFF_MAUSOLEUMKEY3)
        {
            keys.push(3u8);
        }
        if keys.is_empty() {
            return;
        }
        let Some(giver_name) = self.characters.get(&giver_id).map(|c| c.name.clone()) else {
            return;
        };
        let cnt = keys.len();
        self.npc_say(
            countbran_id,
            &format!(
                "Here, take {} key{}, {}.",
                if cnt > 1 { "these" } else { "this" },
                if cnt > 1 { "s" } else { "" },
                giver_name
            ),
        );
        events.push(CountBranOutcomeEvent::GiveMausoleumKeys {
            player_id: giver_id,
            keys,
        });
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`):
    /// adds straight to `Character::gold` (unlike the aristocrat/yoatin
    /// gold rewards, which use `create_money_item` + `give_char_item` and
    /// so need `ZoneLoader` - this reward path needs nothing but `World`).
    fn countbran_give_money(&mut self, giver_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&giver_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(giver_id, give_money_message(amount));
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_COUNTBRAN, CDR_LOSTCON};
use crate::item_driver::{
    IID_STAFF_BLUEKEY1, IID_STAFF_BLUEKEY12, IID_STAFF_BLUEKEY123, IID_STAFF_BLUEKEY13,
    IID_STAFF_BLUEKEY2, IID_STAFF_BLUEKEY23, IID_STAFF_BLUEKEY3, IID_STAFF_COUNTESSAJEWEL,
    IID_STAFF_COUNTJEWEL, IID_STAFF_DAUGHTERJEWEL, IID_STAFF_GREENKEY1, IID_STAFF_GREENKEY12,
    IID_STAFF_GREENKEY123, IID_STAFF_GREENKEY13, IID_STAFF_GREENKEY2, IID_STAFF_GREENKEY23,
    IID_STAFF_GREENKEY3, IID_STAFF_MAUSOLEUMKEY1, IID_STAFF_MAUSOLEUMKEY2, IID_STAFF_MAUSOLEUMKEY3,
    IID_STAFF_REDKEY1, IID_STAFF_REDKEY12, IID_STAFF_REDKEY123, IID_STAFF_REDKEY13,
    IID_STAFF_REDKEY2, IID_STAFF_REDKEY23, IID_STAFF_REDKEY3, IID_STAFF_THIEFKEY1,
    IID_STAFF_THIEFKEY2, IID_STAFF_THIEFKEY3,
};

/// C `struct count_brannington_data` (`src/area/29/brannington.c:585-588`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CountBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_COUNTBRAN`] to `ugaris-server`'s `apply_countbran_events`.
pub const fn qlog_countbran() -> usize {
    QLOG_COUNTBRAN
}
