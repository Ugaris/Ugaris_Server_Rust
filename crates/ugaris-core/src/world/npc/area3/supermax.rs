//! Supermax NPC (`CDR_SUPERMAX`), Aston's past-maxes raiser.
//!
//! Ports `src/area/3/area3.c::supermax_driver` (`:2432-2594`) plus its
//! three helpers `supermax_list`/`supermax_raise`/`supermax_lower`
//! (`area3.c:2328-2430`) and the shared C helpers `skillmax`/
//! `supermax_canraise`/`supermax_cost` (`src/system/skill.c:103-172`),
//! ported as [`crate::item_driver::skillmax`]/[`crate::item_driver::
//! supermax_canraise`]/[`crate::item_driver::supermax_cost`] (crate-
//! visible siblings of the existing `raise_value`/`raise_cost` family in
//! `item_driver::scrolls`, reused here rather than duplicated).
//!
//! `ppd->supermax_state`/`ppd->supermax_gold` (C `struct misc_ppd`,
//! `src/common/misc_ppd.h:28-29`) live on `crate::player::PlayerRuntime`
//! (`supermax_state`/`set_supermax_state`/`supermax_gold`/
//! `add_supermax_gold`, `player/settings.rs`) - same split as every other
//! area-3 driver: the caller supplies a per-player fact snapshot
//! ([`SupermaxPlayerFacts`]) up front and applies the returned
//! [`SupermaxOutcomeEvent`]s afterwards. Unlike `ppd->kassim_state` &co
//! (which live in the area-3-specific `area3_ppd`), `supermax_state`/
//! `supermax_gold` are on the *global* `misc_ppd` struct in C, so any
//! other misc_ppd-consuming driver added later shares the same
//! `PlayerRuntime` accessors.
//!
//! Deviations/gaps (documented, not silent), all copied verbatim from C
//! per the porting rules (`AGENTS.md`: "copy ... stupid-looking edge
//! cases"):
//! - `supermax_raise` calls `update_char(co)` and sets `CF_ITEMS` after
//!   bumping the skill; `supermax_lower` does neither - preserved exactly
//!   (see `World::supermax_lower_skill`'s own doc comment).
//! - Every driver-initiated line (`supermax_list`'s "You can raise the
//!   following..."/"Oops..." header lines are the sole exception - see
//!   below) uses the *loud* `say(cn, ...)`, not `quiet_say`, unlike most
//!   other area-3 drivers (Kassim, Sir Jones, etc. mostly use
//!   `quiet_say`) - ported as [`World::npc_say`].
//! - `supermax_list`'s two `log_char(co, LOG_SYSTEM, ...)` calls (the
//!   header + the maxed-skill percentage rows, plus the "Oops, none is
//!   maxed" fallback) and the two `CF_NOEXP` guard messages in
//!   `supermax_list`/`supermax_raise`/`supermax_lower` are private,
//!   ported via [`World::queue_system_text`] - but `supermax_list`'s
//!   *own* "You cannot raise anything while you don't have any
//!   experience to spend." early-out (`area3.c:2339`) uses `say`, not
//!   `log_char`, an inconsistency preserved from C.
//! - `dlog(...)` debug-only calls are omitted (no player-visible effect).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{skillmax, supermax_canraise, supermax_cost};
use crate::world::values::full_skill_name;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:2479`).
const SUPERMAX_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const SUPERMAX_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:2464`).
const SUPERMAX_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 30` (`area3.c:2587`): idle "return to post" threshold.
const SUPERMAX_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `2000 * 100` (`area3.c:2393,2404`): gold fee per past-max raise.
const SUPERMAX_RAISE_FEE: u32 = 2000 * 100;
/// C `ch[co].value[1][skl] >= 250` (`area3.c:2385`): the hard ceiling
/// `supermax_raise` refuses to cross.
const SUPERMAX_VALUE_CEILING: i16 = 250;

/// Per-player facts [`World::process_supermax_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupermaxPlayerFacts {
    /// `PlayerRuntime::supermax_state()`.
    pub supermax_state: i32,
    /// `PlayerRuntime::supermax_gold()`.
    pub supermax_gold: u32,
}

/// A side effect [`World::process_supermax_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupermaxOutcomeEvent {
    /// Write the new `misc_ppd.supermax_state` back.
    UpdateSupermaxState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->supermax_gold += 2000 * 100;` (`area3.c:2405`).
    AddSupermaxGold { player_id: CharacterId, amount: u32 },
}

impl World {
    /// C `supermax_driver`'s per-tick body (`area3.c:2432-2594`).
    pub fn process_supermax_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, SupermaxPlayerFacts>,
        area_id: u16,
    ) -> Vec<SupermaxOutcomeEvent> {
        let supermax_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SUPERMAX
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for supermax_id in supermax_ids {
            self.process_supermax_messages(supermax_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_supermax_messages(
        &mut self,
        supermax_id: CharacterId,
        player_facts: &HashMap<CharacterId, SupermaxPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SupermaxOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Supermax(mut data)) = self
            .characters
            .get(&supermax_id)
            .and_then(|supermax| supermax.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&supermax_id)
            .map(|supermax| std::mem::take(&mut supermax.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.supermax_handle_char_message(
                    supermax_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.supermax_handle_text_message(
                    supermax_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
                    // (`area3.c:2566-2568`).
                    if let Some(item_id) = self
                        .characters
                        .get_mut(&supermax_id)
                        .and_then(|supermax| supermax.cursor_item.take())
                    {
                        self.destroy_item(item_id);
                    }
                }
                _ => {}
            }
        }

        if let Some(supermax) = self.characters.get_mut(&supermax_id) {
            supermax.driver_state = Some(CharacterDriverState::Supermax(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:2583-2585`).
        if let (Some(supermax), Some((tx, ty))) =
            (self.characters.get(&supermax_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(supermax.x), i32::from(supermax.y), tx, ty)
            {
                if let Some(supermax_mut) = self.characters.get_mut(&supermax_id) {
                    let _ = turn(supermax_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`area3.c:2587-2591`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3 driver already uses.
        let last_talk = match self.characters.get(&supermax_id) {
            Some(supermax) => match supermax.driver_state.as_ref() {
                Some(CharacterDriverState::Supermax(data)) => data.last_talk,
                _ => return,
            },
            None => return,
        };
        if last_talk + SUPERMAX_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(supermax) = self.characters.get(&supermax_id) else {
                return;
            };
            let (post_x, post_y) = (supermax.rest_x, supermax.rest_y);
            self.secure_move_driver(
                supermax_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `supermax_driver`'s `NT_CHAR` branch (`area3.c:2447-2521`).
    fn supermax_handle_char_message(
        &mut self,
        supermax_id: CharacterId,
        data: &mut SupermaxDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SupermaxPlayerFacts>,
        events: &mut Vec<SupermaxOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(supermax) = self.characters.get(&supermax_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:2452-2455`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:2458-2461`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) { remove_message;
        // continue; }` (`area3.c:2464-2467`).
        if tick < data.last_talk + SUPERMAX_TALK_MIN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`area3.c:2473-2476`).
        if supermax_id == player_id
            || !char_see_char(&supermax, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`area3.c:2479-2482`).
        if char_dist(&supermax, &player) > SUPERMAX_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        let smax = skillmax(&player);

        let mut didsay = false;
        match facts.supermax_state {
            // C `case 0:` (`area3.c:2492-2495`).
            0 => {
                self.npc_say(
                    supermax_id,
                    &format!(
                        "Hello, {}. I am {}, and I can turn your life upside down.",
                        player.name, supermax.name
                    ),
                );
                events.push(SupermaxOutcomeEvent::UpdateSupermaxState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            // C `case 1:` (`area3.c:2496-2501`).
            1 => {
                self.npc_say(
                    supermax_id,
                    &format!(
                        "I can raise any of your attributes, skills or spells past your normal maximum of {smax}."
                    ),
                );
                events.push(SupermaxOutcomeEvent::UpdateSupermaxState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            // C `case 2:` (`area3.c:2502-2506`).
            2 => {
                self.supermax_list(supermax_id, player_id, &player);
                events.push(SupermaxOutcomeEvent::UpdateSupermaxState {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            // C `case 3:` (`area3.c:2507-2513`).
            3 => {
                self.npc_say(
                    supermax_id,
                    "To raise a skill, say: \"raise SKILLNAME\". This costs 2000g, and a lot of experience. To lower a skill, say: \"lower SKILLNAME\". You'll get the experience back, but not the gold. To see the list again, say: \"list\".",
                );
                events.push(SupermaxOutcomeEvent::UpdateSupermaxState {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            // C's switch has no `case 4:` or beyond - the greeting
            // sequence plateaus silently (`area3.c:2514`).
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...; }`
        // (`area3.c:2515-2519`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `supermax_driver`'s `NT_TEXT` branch (`area3.c:2524-2562`).
    fn supermax_handle_text_message(
        &mut self,
        supermax_id: CharacterId,
        data: &mut SupermaxDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SupermaxPlayerFacts>,
        events: &mut Vec<SupermaxOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(supermax) = self.characters.get(&supermax_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`area3.c:223-238`):
        // ignore our own talk, non-players, distance > 12, not-visible.
        if supermax_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if char_dist(&supermax, &speaker) > SUPERMAX_QA_DISTANCE
            || !char_see_char(&supermax, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        let outcome = analyse_text_qa(text, &supermax.name, &speaker.name, AREA3_QA);
        match outcome {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(supermax_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) - resets the greeting sequence
            // (`area3.c:2532-2538`).
            TextAnalysisOutcome::Matched(2) => {
                let state = player_facts
                    .get(&speaker_id)
                    .map(|facts| facts.supermax_state)
                    .unwrap_or(0);
                if state > 0 {
                    events.push(SupermaxOutcomeEvent::UpdateSupermaxState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                    data.last_talk = 0;
                }
                didsay = true;
            }
            // C `case 5:` (`area3.c:2539-2541`).
            TextAnalysisOutcome::Matched(5) => {
                self.supermax_list(supermax_id, speaker_id, &speaker);
                didsay = true;
            }
            // C `case 6:` (`area3.c:2542-2548`).
            TextAnalysisOutcome::Matched(6) => {
                let gold_spent = player_facts
                    .get(&speaker_id)
                    .map(|facts| facts.supermax_gold)
                    .unwrap_or(0);
                if gold_spent > 0 {
                    self.npc_say(
                        supermax_id,
                        &format!(
                            "You spent {} gold already. The Astonian Wildlife Fund says: 'Thank you!'",
                            gold_spent / 100
                        ),
                    );
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(code) if (101..200).contains(&code) => {
                self.supermax_raise(supermax_id, speaker_id, (code - 100) as usize, events);
                didsay = true;
            }
            TextAnalysisOutcome::Matched(code) if (201..300).contains(&code) => {
                self.supermax_lower(supermax_id, speaker_id, (code - 200) as usize);
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...; }`
        // (`area3.c:2557-2561`).
        if didsay {
            data.last_talk = self.tick.0;
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
        }
    }

    /// C `supermax_list` (`area3.c:2328-2363`). Only reads `ch[co]`
    /// fields directly - unlike the greeting-sequence state machine, it
    /// never touches `ppd->supermax_state`/`supermax_gold`, so no
    /// `SupermaxPlayerFacts` lookup is needed here.
    fn supermax_list(
        &mut self,
        supermax_id: CharacterId,
        player_id: CharacterId,
        player: &Character,
    ) {
        // C `if (ch[co].flags & CF_NOEXP) { log_char(...); return; }`
        // (`area3.c:2332-2335`).
        if player.flags.contains(CharacterFlags::NOEXP) {
            self.queue_system_text(
                player_id,
                "You cannot raise your skills when /noexp is set.",
            );
            return;
        }
        let left = i64::from(player.exp) - i64::from(player.exp_used);
        // C `if (left < 1) { say(cn, ...); return; }` (`area3.c:2338-2341`).
        if left < 1 {
            self.npc_say(
                supermax_id,
                "You cannot raise anything while you don't have any experience to spend.",
            );
            return;
        }
        let smax = skillmax(player);

        self.queue_system_text(
            player_id,
            "You can raise the following skills. The percentage shows how much experience of the needed cost you already have:",
        );

        let mut count = 0;
        for index in 0..crate::entity::CHARACTER_VALUE_COUNT {
            if supermax_canraise(index) == 0 {
                continue;
            }
            let current = player
                .values
                .get(1)
                .and_then(|values| values.get(index))
                .copied()
                .unwrap_or(0);
            if current < smax {
                continue;
            }
            let Some(value) = crate::world::character_value_from_index(index) else {
                continue;
            };
            let cost = supermax_cost(player, index, current);
            let percent = 100.0 / cost as f64 * left as f64;
            self.queue_system_text(
                player_id,
                format!("{}\u{8}{percent:5.2}%", full_skill_name(value)),
            );
            count += 1;
        }
        if count == 0 {
            self.queue_system_text(
                player_id,
                "Oops. You cannot raise any skill. None is maxed.",
            );
        }
    }

    /// C `supermax_raise` (`area3.c:2365-2408`).
    fn supermax_raise(
        &mut self,
        supermax_id: CharacterId,
        player_id: CharacterId,
        skl: usize,
        events: &mut Vec<SupermaxOutcomeEvent>,
    ) {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        // C `if (ch[co].flags & CF_NOEXP) { log_char(...); return; }`
        // (`area3.c:2370-2373`).
        if player.flags.contains(CharacterFlags::NOEXP) {
            self.queue_system_text(
                player_id,
                "You cannot raise your skills when /noexp is set.",
            );
            return;
        }
        if skl >= crate::entity::CHARACTER_VALUE_COUNT {
            return;
        }
        let current = player
            .values
            .get(1)
            .and_then(|values| values.get(skl))
            .copied()
            .unwrap_or(0);
        let left = i64::from(player.exp) - i64::from(player.exp_used);
        let cost = supermax_cost(&player, skl, current);
        let smax = skillmax(&player);
        let Some(value) = crate::world::character_value_from_index(skl) else {
            return;
        };
        let skill_name = full_skill_name(value);

        // C `if (ch[co].value[1][skl] < smax) { say(...); return; }`
        // (`area3.c:2381-2384`).
        if current < smax {
            self.npc_say(
                supermax_id,
                &format!(
                    "You can only raise skills you have already maxed, {}.",
                    player.name
                ),
            );
            return;
        }
        // C `if (ch[co].value[1][skl] >= 250) { say(...); return; }`
        // (`area3.c:2385-2388`).
        if current >= SUPERMAX_VALUE_CEILING {
            self.npc_say(
                supermax_id,
                &format!("I cannot raise any skill beyond 250 yet, {}.", player.name),
            );
            return;
        }
        // C `if (cost > left) { say(...); return; }` (`area3.c:2389-2392`).
        if i64::from(cost) > left {
            self.npc_say(
                supermax_id,
                &format!(
                    "You do not have enough experience to raise {skill_name}, {}.",
                    player.name
                ),
            );
            return;
        }
        // C `if (ch[co].gold < 2000 * 100) { say(...); return; }`
        // (`area3.c:2393-2396`).
        if player.gold < SUPERMAX_RAISE_FEE {
            self.npc_say(
                supermax_id,
                &format!("You cannot pay the fee of 2000 gold, {}.", player.name),
            );
            return;
        }

        self.npc_say(
            supermax_id,
            &format!("Your {skill_name} has been raised, {}.", player.name),
        );

        // C `ch[co].value[1][skl]++; ch[co].exp_used += cost; ch[co].gold
        // -= 2000*100; ppd->supermax_gold += 2000*100; ch[co].flags |=
        // CF_ITEMS; update_char(co);` (`area3.c:2402-2407`).
        if let Some(player_mut) = self.characters.get_mut(&player_id) {
            player_mut.values[1][skl] = player_mut.values[1][skl].saturating_add(1);
            player_mut.exp_used = player_mut.exp_used.saturating_add(cost);
            player_mut.gold = player_mut.gold.saturating_sub(SUPERMAX_RAISE_FEE);
            player_mut.flags.insert(CharacterFlags::ITEMS);
        }
        events.push(SupermaxOutcomeEvent::AddSupermaxGold {
            player_id,
            amount: SUPERMAX_RAISE_FEE,
        });
        self.update_character(player_id);
    }

    /// C `supermax_lower` (`area3.c:2410-2430`). Unlike `supermax_raise`,
    /// C does *not* call `update_char` or set `CF_ITEMS` here - preserved
    /// exactly (see the module doc comment).
    fn supermax_lower(&mut self, supermax_id: CharacterId, player_id: CharacterId, skl: usize) {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        // C `if (ch[co].flags & CF_NOEXP) { log_char(...); return; }`
        // (`area3.c:2414-2417`).
        if player.flags.contains(CharacterFlags::NOEXP) {
            self.queue_system_text(
                player_id,
                "You cannot lower your skills when /lockexp is set.",
            );
            return;
        }
        if skl >= crate::entity::CHARACTER_VALUE_COUNT {
            return;
        }
        let smax = skillmax(&player);
        let current = player
            .values
            .get(1)
            .and_then(|values| values.get(skl))
            .copied()
            .unwrap_or(0);
        let Some(value) = crate::world::character_value_from_index(skl) else {
            return;
        };
        let skill_name = full_skill_name(value);

        // C `if (ch[co].value[1][skl] <= smax) { say(...); return; }`
        // (`area3.c:2420-2423`).
        if current <= smax {
            self.npc_say(
                supermax_id,
                &format!(
                    "You can only lower skills you have already raised past the max, {}.",
                    player.name
                ),
            );
            return;
        }

        let lowered = current.saturating_sub(1);

        self.npc_say(
            supermax_id,
            &format!("Your {skill_name} has been lowered, {}.", player.name),
        );

        // C `ch[co].value[1][skl]--; cost = supermax_cost(co, skl,
        // ch[co].value[1][skl]); ch[co].exp_used -= cost;`
        // (`area3.c:2425-2428`): `cost` is computed *after* decrementing,
        // using the already-lowered value.
        let cost = supermax_cost(&player, skl, lowered);
        if let Some(player_mut) = self.characters.get_mut(&player_id) {
            player_mut.values[1][skl] = lowered;
            player_mut.exp_used = player_mut.exp_used.saturating_sub(cost);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_SUPERMAX;

/// C `struct supermax_driver_data` (`src/area/3/area3.c:2323-2326`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupermaxDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
