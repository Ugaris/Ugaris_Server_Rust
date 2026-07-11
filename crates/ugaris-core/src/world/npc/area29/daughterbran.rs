//! Daughter Brannington NPC (`CDR_DAUGHTERBRAN`), the Count's daughter who
//! hands out a secondary reward once her own jewel (`IID_STAFF_
//! DAUGHTERJEWEL`) has been returned to the Count.
//!
//! Ports `src/area/29/brannington.c::daughter_brannington_driver`
//! (`:1671-1827`) plus the shared `analyse_text_driver`/`qa[]` table
//! (`:86-206`, ported as [`super::AREA29_QA`] in `world::npc::area29`, the
//! same table `world::npc::area29::countbran`/`countessabran`/`spiritbran`
//! share). Follows the exact same `World`/`PlayerRuntime` split and `case
//! 0`-`2` fallthrough-cascade shape as
//! `world::npc::area29::countessabran` (see that module's doc comment for
//! the general mechanism) - only the gating bits (`4`/`16` instead of
//! `2`/`8`), dialogue text, and reward (exp + a `lollipop` item instead of
//! exp + 500g) differ.
//!
//! Deviations/gaps (documented, not silent):
//! - Same "no `dat->current_victim` staleness-reset preamble", "no `case
//!   3`" (text handler), and "no self-defense/regen/spell-self cascade"
//!   observations as `world::npc::area29::countessabran` - see that
//!   module's doc comment.
//! - The `lollipop` reward item needs `ZoneLoader::instantiate_item_
//!   template`, which `World` cannot reach - ported as
//!   [`DaughterBranOutcomeEvent::GiveLollipop`], applied by
//!   `ugaris-server`'s `apply_daughterbran_events`, same precedent as
//!   `world::npc::area28::yoatin`'s `WS_Hunter_Belt` reward. C's own `if
//!   (in) give_char_item(co, in);` (`:1760-1762`) has no "else" branch (a
//!   failed `create_item` call is silently ignored, no fallback) -
//!   reproduced by simply not pushing the event when the template lookup
//!   fails server-side, matching C's silent no-op.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1826`) is not
//!   ported, matching the established `world::thomas`/`world::npc::area29::
//!   countbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::exp::level_value;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:1720`).
const DAUGHTERBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const DAUGHTERBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:1703`).
const DAUGHTERBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:1708`).
const DAUGHTERBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:1820`): idle "return to post" threshold.
const DAUGHTERBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `ppd->countbran_bits & 4` (`brannington.c:1731`): the Daughter's own
/// jewel (set by `world::npc::area29::countbran`'s `NT_GIVE` handler).
const COUNTBRAN_BIT_DAUGHTER_JEWEL: i32 = 4;
/// C `ppd->countbran_bits & 16` (`brannington.c:1749`/`1757`): "the
/// Daughter has already paid out her own reward".
const COUNTBRAN_BIT_DAUGHTER_REWARDED: i32 = 16;

/// Per-player facts [`World::process_daughterbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaughterBranPlayerFacts {
    /// `PlayerRuntime::staffer_daughterbran_state()`.
    pub daughterbran_state: i32,
    /// `PlayerRuntime::staffer_countbran_bits()` - shared with
    /// `world::npc::area29::countbran`.
    pub countbran_bits: i32,
}

/// A side effect [`World::process_daughterbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaughterBranOutcomeEvent {
    /// Write the new `staffer_ppd.daughterbran_state` back.
    UpdateDaughterBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->countbran_bits |= 16;` (`brannington.c:1757`) - written
    /// through the *same* `countbran_bits` field
    /// `world::npc::area29::countbran` owns.
    SetDaughterBranRewardedBit { player_id: CharacterId },
    /// C `in = create_item("lollipop"); if (in) give_char_item(co, in);`
    /// (`brannington.c:1759-1762`).
    GiveLollipop { player_id: CharacterId },
}

impl World {
    /// C `daughter_brannington_driver`'s per-tick body (`brannington.c:
    /// 1671-1827`).
    pub fn process_daughterbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, DaughterBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<DaughterBranOutcomeEvent> {
        let daughterbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DAUGHTERBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for daughterbran_id in daughterbran_ids {
            self.process_daughterbran_messages(daughterbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_daughterbran_messages(
        &mut self,
        daughterbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, DaughterBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<DaughterBranOutcomeEvent>,
    ) {
        let Some(daughterbran_name) = self
            .characters
            .get(&daughterbran_id)
            .map(|daughterbran| daughterbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::DaughterBran(mut data)) = self
            .characters
            .get(&daughterbran_id)
            .and_then(|daughterbran| daughterbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&daughterbran_id)
            .map(|daughterbran| std::mem::take(&mut daughterbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.daughterbran_handle_char_message(
                    daughterbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.daughterbran_handle_text_message(
                    daughterbran_id,
                    &daughterbran_name,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.daughterbran_handle_give_message(daughterbran_id, message),
                _ => {}
            }
        }

        if let Some(daughterbran) = self.characters.get_mut(&daughterbran_id) {
            daughterbran.driver_state = Some(CharacterDriverState::DaughterBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:1816-1818`).
        if let (Some(daughterbran), Some((tx, ty))) =
            (self.characters.get(&daughterbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(daughterbran.x), i32::from(daughterbran.y), tx, ty)
            {
                if let Some(daughterbran_mut) = self.characters.get_mut(&daughterbran_id) {
                    let _ = turn(daughterbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`brannington.c:1820-1824`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::countbran` already uses.
        let last_talk = if let Some(daughterbran) = self.characters.get(&daughterbran_id) {
            match daughterbran.driver_state.as_ref() {
                Some(CharacterDriverState::DaughterBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + DAUGHTERBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(daughterbran) = self.characters.get(&daughterbran_id) else {
                return;
            };
            let (post_x, post_y) = (daughterbran.rest_x, daughterbran.rest_y);
            self.secure_move_driver(
                daughterbran_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `daughter_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 1687-1773`), including its `case 0`-`2` fallthrough cascade
    /// (`:1729-1767`) - ported as an explicit `loop`, same mechanism as
    /// `world::npc::area29::countessabran`'s own `NT_CHAR` handler.
    #[allow(clippy::too_many_arguments)]
    fn daughterbran_handle_char_message(
        &mut self,
        daughterbran_id: CharacterId,
        data: &mut DaughterBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DaughterBranPlayerFacts>,
        events: &mut Vec<DaughterBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(daughterbran) = self.characters.get(&daughterbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:1690-1694`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:1696-1700`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:1702-1706`).
        if tick < data.last_talk + DAUGHTERBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:1708-1711`).
        if tick < data.last_talk + DAUGHTERBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:1713-1717`).
        if daughterbran_id == player_id
            || !char_see_char(&daughterbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:1719-
        // 1723`).
        if char_dist(&daughterbran, &player) > DAUGHTERBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.daughterbran_state;
        let mut reward_granted = false;
        loop {
            match new_state {
                // C `case 0:` (`brannington.c:1730-1740`).
                0 => {
                    if facts.countbran_bits
                        & (COUNTBRAN_BIT_DAUGHTER_JEWEL | COUNTBRAN_BIT_DAUGHTER_REWARDED)
                        != 0
                    {
                        new_state = 1;
                        continue;
                    }
                    self.npc_quiet_say(
                        daughterbran_id,
                        &format!(
                            "My jewel! My jewel! {}, please, bring me back my grandmother's jewel!",
                            if player.flags.contains(CharacterFlags::MALE) {
                                "Sir"
                            } else {
                                "Lady"
                            }
                        ),
                    );
                    new_state = 1;
                    didsay = true;
                    break;
                }
                // C `case 1:` (`brannington.c:1741-1747`).
                1 => {
                    if facts.countbran_bits
                        & (COUNTBRAN_BIT_DAUGHTER_JEWEL | COUNTBRAN_BIT_DAUGHTER_REWARDED)
                        != 0
                    {
                        new_state = 2;
                        continue;
                    }
                    break;
                }
                // C `case 2:` (`brannington.c:1748-1764`).
                2 => {
                    if facts.countbran_bits & COUNTBRAN_BIT_DAUGHTER_REWARDED != 0 {
                        new_state = 3;
                        continue;
                    }
                    self.npc_quiet_say(
                        daughterbran_id,
                        &format!(
                            "Oh thank you great {}, you are my hero! Let me reward you for such heroism, {}!",
                            if player.flags.contains(CharacterFlags::MALE) {
                                "Sir"
                            } else {
                                "Lady"
                            },
                            player.name
                        ),
                    );
                    new_state = 3;
                    didsay = true;
                    reward_granted = true;
                    break;
                }
                // C `case 3: break;` (`brannington.c:1765-1766`): all done.
                _ => break,
            }
        }

        if reward_granted {
            events.push(DaughterBranOutcomeEvent::SetDaughterBranRewardedBit { player_id });
            // C `give_exp(co, min(get_bran_exp_base() * 2, level_value(ch
            // [co].level) / 4));` (`brannington.c:1758`).
            let cap = i64::from(level_value(player.level)) / 4;
            let base = i64::from(self.settings.bran_exp_base) * 2;
            self.give_exp(player_id, base.min(cap), u32::from(self.area_id));
            events.push(DaughterBranOutcomeEvent::GiveLollipop { player_id });
        }

        if new_state != facts.daughterbran_state {
            events.push(DaughterBranOutcomeEvent::UpdateDaughterBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1768-1772`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `daughter_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1777-1795`).
    fn daughterbran_handle_text_message(
        &mut self,
        daughterbran_id: CharacterId,
        daughterbran_name: &str,
        data: &mut DaughterBranDriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<DaughterBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1780`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if daughterbran_id == speaker_id {
            return;
        }
        let Some(daughterbran) = self.characters.get(&daughterbran_id).cloned() else {
            return;
        };
        if char_dist(&daughterbran, &speaker) > DAUGHTERBRAN_QA_DISTANCE
            || !char_see_char(&daughterbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, daughterbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(daughterbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1783-1788`): reset back to the
            // greeting unconditionally (no `<=` state guard, unlike
            // `world::npc::area29::countbran`'s own `case 2`).
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                events.push(DaughterBranOutcomeEvent::UpdateDaughterBranState {
                    player_id: speaker_id,
                    new_state: 0,
                });
                didsay = true;
            }
            // Every other matched code is unhandled by daughter's own C
            // `switch` (which has no `case 3`, unlike `world::npc::area29::
            // countbran`'s own text handler) but still counts as `didsay`
            // (C's `switch (didsay)` dispatch value stays truthy even when
            // no `case` matches).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:1790-1793`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `daughter_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1798-1808`): always hands the item straight back, no turn-in item
    /// is ever accepted by this NPC.
    fn daughterbran_handle_give_message(
        &mut self,
        daughterbran_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&daughterbran_id)
            .and_then(|daughterbran| daughterbran.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            daughterbran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_DAUGHTERBRAN, CDR_LOSTCON};

/// C `struct daughter_brannington_data` (`src/area/29/brannington.c:1666-
/// 1669`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DaughterBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
