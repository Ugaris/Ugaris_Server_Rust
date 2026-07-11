//! Countessa Brannington NPC (`CDR_COUNTESSABRAN`), the Count's wife who
//! hands out a secondary reward once her own jewel (`IID_STAFF_
//! COUNTESSAJEWEL`) has been returned to the Count.
//!
//! Ports `src/area/29/brannington.c::countessa_brannington_driver`
//! (`:1509-1664`) plus the shared `analyse_text_driver`/`qa[]` table
//! (`:86-206`, ported as [`super::AREA29_QA`] in `world::npc::area29`, the
//! same table `world::npc::area29::countbran`/`spiritbran` share). Follows
//! the same `World`/`PlayerRuntime` split established by those siblings:
//! the caller supplies a per-player fact snapshot
//! ([`CountessaBranPlayerFacts`]) up front and applies the returned
//! [`CountessaBranOutcomeEvent`]s afterwards, since `staffer_ppd.
//! countessabran_state` (this NPC's own dialogue state) and `staffer_ppd.
//! countbran_bits` (the *shared* jewel-return bitfield
//! `world::npc::area29::countbran` also writes) both live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `countessa_brannington_driver`'s `NT_CHAR` handler is a four-state
//! (`0`-`3`) switch with C `// fall through intended` cascades gated on
//! `countbran_bits`: state `0`/`1` speak "please return my jewelry" only
//! while bit `2` (jewel returned) *and* bit `8` (already rewarded) are both
//! unset, otherwise silently cascading straight through to state `2`'s
//! reward check in the *same* driver call; state `2` grants the reward
//! (exp + 500g, sets bit `8`) exactly once, then state `3` is a permanent
//! no-op. Ported as an explicit `loop` matching each `case`'s fallthrough
//! precisely (see [`World::process_countessabran_actions`]'s dispatch).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::countbran`/`spiritbran`'s own `NT_TEXT`
//!   branch, this driver's own C body has no `dat->current_victim`
//!   staleness-reset preamble and no victim-mismatch early-out at all -
//!   reproduced verbatim.
//! - Unlike `world::npc::area29::countbran`'s own `NT_TEXT` handler, this
//!   driver's C `switch` (`:1619-1626`) has **no** `case 3` ("reset me")
//!   arm at all - a matched code `3` (or any other unhandled code) simply
//!   falls through to no-op inside the `switch`, but the outer `if
//!   (didsay)` still fires (C's `switch (didsay)`'s dispatch value stays
//!   truthy even when no `case` matches) - reproduced by folding every
//!   unhandled matched code into a single "still counts as `didsay`" arm.
//! - No self-defense/regen/spell-self cascade exists in C's `countessa_
//!   brannington_driver` body at all (matching `world::npc::area29::
//!   countbran`'s identical observation for other "pure talker" NPCs) -
//!   this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1663`) is not
//!   ported, matching the established `world::thomas`/`world::npc::area29::
//!   countbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::exp::level_value;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:1558`).
const COUNTESSABRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const COUNTESSABRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:1541`).
const COUNTESSABRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:1546`).
const COUNTESSABRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:1657`): idle "return to post" threshold.
const COUNTESSABRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `ppd->countbran_bits & 2` (`brannington.c:1569`): the Countessa's own
/// jewel (set by `world::npc::area29::countbran`'s `NT_GIVE` handler).
const COUNTBRAN_BIT_COUNTESSA_JEWEL: i32 = 2;
/// C `ppd->countbran_bits & 8` (`brannington.c:1589`/`1597`): "the
/// Countessa has already paid out her own reward".
const COUNTBRAN_BIT_COUNTESSA_REWARDED: i32 = 8;
/// C `give_money(co, 500 * 100, "Count Bran Quest 2B")` (`brannington.c:
/// 1599`).
const COUNTESSABRAN_REWARD_GOLD: u32 = 500 * 100;

/// Per-player facts [`World::process_countessabran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountessaBranPlayerFacts {
    /// `PlayerRuntime::staffer_countessabran_state()`.
    pub countessabran_state: i32,
    /// `PlayerRuntime::staffer_countbran_bits()` - shared with
    /// `world::npc::area29::countbran`.
    pub countbran_bits: i32,
}

/// A side effect [`World::process_countessabran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountessaBranOutcomeEvent {
    /// Write the new `staffer_ppd.countessabran_state` back.
    UpdateCountessaBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->countbran_bits |= 8;` (`brannington.c:1597`) - written
    /// through the *same* `countbran_bits` field
    /// `world::npc::area29::countbran` owns.
    SetCountessaBranRewardedBit { player_id: CharacterId },
}

impl World {
    /// C `countessa_brannington_driver`'s per-tick body (`brannington.c:
    /// 1509-1664`).
    pub fn process_countessabran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CountessaBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<CountessaBranOutcomeEvent> {
        let countessabran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_COUNTESSABRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for countessabran_id in countessabran_ids {
            self.process_countessabran_messages(
                countessabran_id,
                player_facts,
                area_id,
                &mut events,
            );
        }
        events
    }

    fn process_countessabran_messages(
        &mut self,
        countessabran_id: CharacterId,
        player_facts: &HashMap<CharacterId, CountessaBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<CountessaBranOutcomeEvent>,
    ) {
        let Some(countessabran_name) = self
            .characters
            .get(&countessabran_id)
            .map(|countessabran| countessabran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::CountessaBran(mut data)) = self
            .characters
            .get(&countessabran_id)
            .and_then(|countessabran| countessabran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&countessabran_id)
            .map(|countessabran| std::mem::take(&mut countessabran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.countessabran_handle_char_message(
                    countessabran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.countessabran_handle_text_message(
                    countessabran_id,
                    &countessabran_name,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.countessabran_handle_give_message(countessabran_id, message),
                _ => {}
            }
        }

        if let Some(countessabran) = self.characters.get_mut(&countessabran_id) {
            countessabran.driver_state = Some(CharacterDriverState::CountessaBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:1653-1655`).
        if let (Some(countessabran), Some((tx, ty))) =
            (self.characters.get(&countessabran_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(
                i32::from(countessabran.x),
                i32::from(countessabran.y),
                tx,
                ty,
            ) {
                if let Some(countessabran_mut) = self.characters.get_mut(&countessabran_id) {
                    let _ = turn(countessabran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`brannington.c:1657-1661`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::countbran` already uses.
        let last_talk = if let Some(countessabran) = self.characters.get(&countessabran_id) {
            match countessabran.driver_state.as_ref() {
                Some(CharacterDriverState::CountessaBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + COUNTESSABRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(countessabran) = self.characters.get(&countessabran_id) else {
                return;
            };
            let (post_x, post_y) = (countessabran.rest_x, countessabran.rest_y);
            self.secure_move_driver(
                countessabran_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `countessa_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 1525-1611`), including its `case 0`-`2` fallthrough cascade
    /// (`:1567-1604`) - ported as an explicit `loop` so a single driver
    /// call can walk straight from a fresh `0` through to the state `2`
    /// reward check exactly like C's `switch` fallthrough, without waiting
    /// for another tick.
    #[allow(clippy::too_many_arguments)]
    fn countessabran_handle_char_message(
        &mut self,
        countessabran_id: CharacterId,
        data: &mut CountessaBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CountessaBranPlayerFacts>,
        events: &mut Vec<CountessaBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(countessabran) = self.characters.get(&countessabran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:1528-1532`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:1534-1538`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:1540-1544`).
        if tick < data.last_talk + COUNTESSABRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:1546-1549`).
        if tick < data.last_talk + COUNTESSABRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:1551-1555`).
        if countessabran_id == player_id
            || !char_see_char(&countessabran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:1557-
        // 1561`).
        if char_dist(&countessabran, &player) > COUNTESSABRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.countessabran_state;
        let mut reward_granted = false;
        loop {
            match new_state {
                // C `case 0:` (`brannington.c:1568-1580`).
                0 => {
                    if facts.countbran_bits
                        & (COUNTBRAN_BIT_COUNTESSA_JEWEL | COUNTBRAN_BIT_COUNTESSA_REWARDED)
                        != 0
                    {
                        new_state = 1;
                        continue;
                    }
                    self.npc_quiet_say(
                        countessabran_id,
                        &format!(
                            "Have you come here to return to us the jewelry that has been handed down from generation to generation? We would be so thankful if you did kind {}!",
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
                // C `case 1:` (`brannington.c:1581-1587`).
                1 => {
                    if facts.countbran_bits
                        & (COUNTBRAN_BIT_COUNTESSA_JEWEL | COUNTBRAN_BIT_COUNTESSA_REWARDED)
                        != 0
                    {
                        new_state = 2;
                        continue;
                    }
                    break;
                }
                // C `case 2:` (`brannington.c:1588-1601`).
                2 => {
                    if facts.countbran_bits & COUNTBRAN_BIT_COUNTESSA_REWARDED != 0 {
                        new_state = 3;
                        continue;
                    }
                    self.npc_quiet_say(
                        countessabran_id,
                        &format!(
                            "Thank you for returning my jewelry! Let me reward you for your kindness, {}!",
                            player.name
                        ),
                    );
                    new_state = 3;
                    didsay = true;
                    reward_granted = true;
                    break;
                }
                // C `case 3: break;` (`brannington.c:1602-1603`): all done.
                _ => break,
            }
        }

        if reward_granted {
            events.push(CountessaBranOutcomeEvent::SetCountessaBranRewardedBit { player_id });
            // C `give_exp(co, min(get_bran_exp_base() * 2, level_value(ch
            // [co].level) / 4));` (`brannington.c:1598`).
            let cap = i64::from(level_value(player.level)) / 4;
            let base = i64::from(self.settings.bran_exp_base) * 2;
            self.give_exp(player_id, base.min(cap), u32::from(self.area_id));
            // C `give_money(co, 500 * 100, "Count Bran Quest 2B")`
            // (`brannington.c:1599`).
            self.countessabran_give_money(player_id, COUNTESSABRAN_REWARD_GOLD);
        }

        if new_state != facts.countessabran_state {
            events.push(CountessaBranOutcomeEvent::UpdateCountessaBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1605-1609`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `countessa_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1614-1632`).
    fn countessabran_handle_text_message(
        &mut self,
        countessabran_id: CharacterId,
        countessabran_name: &str,
        data: &mut CountessaBranDriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<CountessaBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1617`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if countessabran_id == speaker_id {
            return;
        }
        let Some(countessabran) = self.characters.get(&countessabran_id).cloned() else {
            return;
        };
        if char_dist(&countessabran, &speaker) > COUNTESSABRAN_QA_DISTANCE
            || !char_see_char(&countessabran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, countessabran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(countessabran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1620-1625`): reset back to the
            // greeting unconditionally (no `<=` state guard, unlike
            // `world::npc::area29::countbran`'s own `case 2`).
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                events.push(CountessaBranOutcomeEvent::UpdateCountessaBranState {
                    player_id: speaker_id,
                    new_state: 0,
                });
                didsay = true;
            }
            // Every other matched code is unhandled by countessa's own C
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
        // (`brannington.c:1627-1630`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `countessa_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1635-1645`): always hands the item straight back, no turn-in item
    /// is ever accepted by this NPC.
    fn countessabran_handle_give_message(
        &mut self,
        countessabran_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&countessabran_id)
            .and_then(|countessabran| countessabran.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            countessabran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`).
    fn countessabran_give_money(&mut self, giver_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&giver_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(giver_id, give_money_message(amount));
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_COUNTESSABRAN, CDR_LOSTCON};

/// C `struct countessa_brannington_data` (`src/area/29/brannington.c:1504-
/// 1507`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CountessaBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
