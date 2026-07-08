//! Two-City tavern barkeeper (`CDR_TWOBARKEEPER`) - the Two Towns' guest-
//! pass broker into Exkordon.
//!
//! Ports `src/area/17/two.c::barkeeper` (`:776-958`); C's `ch_died_driver`/
//! `ch_respawn_driver` dispatch for `CDR_TWOBARKEEPER` are plain `return
//! 1;` no-ops, so no death/respawn hook exists for this NPC.
//!
//! Unlike `two_skelly`/`alchemist`/`sanwyn`, this driver reads/writes
//! several `twocity_ppd` fields shared with the still-unported
//! `guard_driver` (`legal_status`/`legal_fine`/`citizen_status`, see
//! `world::npc::area17::{LS_CLEAN, LS_FINE, LS_DEAD, CS_ENEMY, CS_GUEST,
//! CS_CITIZEN, CS_HONOR}`) in addition to its own `barkeeper_state`/
//! `barkeeper_last`. All of these live on `crate::player::PlayerRuntime`,
//! not `World`, so the caller supplies a per-player fact snapshot
//! ([`TwoBarkeeperPlayerFacts`]) up front and applies the returned
//! [`TwoBarkeeperOutcomeEvent`]s afterwards - same split as every other
//! Two-City NPC in this module.
//!
//! The guest-pass purchase (`buy pass`, QA answer_code 13) needs both a
//! `PlayerRuntime`-owned cost lookup (`legal_status`/`legal_fine`) *and* a
//! `Character::gold` deduction (`take_money`) - since `gold` lives on
//! `Character`, which `World` *can* see, the whole purchase (cost
//! computation, `take_money`, and the `say`) is resolved directly inside
//! `World` using the facts snapshot, and only the resulting
//! `citizen_status`/`legal_status`/`legal_fine` writeback is deferred via
//! [`TwoBarkeeperOutcomeEvent::BuyPass`] - unlike `alchemist`'s reward,
//! which needs `ZoneLoader`/`QuestLog` and must be deferred whole.
//!
//! A real C quirk reproduced exactly, not "fixed": `barkeeper_state == 2`'s
//! `NT_CHAR` branch (`two.c:861-865`) is dead code - `if (realtime -
//! ppd->barkeeper_last > 60*10) ppd->barkeeper_state = 2;` reassigns the
//! field to the value it already holds, so nothing observable ever
//! happens once the guest-pass offer has been made; this port has no
//! `barkeeper_state == 2` arm at all (the implicit `_ => {}` covers it),
//! with this doc comment as the record of why. Another: `NT_GIVE`
//! (`two.c:931-939`) destroys *any* item handed over unconditionally -
//! unlike every sibling driver in this file, there is no give-back
//! fallback at all.
use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOBARKEEPER};
use crate::drvlib::offset2dx;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

use super::{CS_GUEST, LS_DEAD, LS_FINE, TWOCITY_QA};

/// C `char_dist(cn, co) > 10` (`two.c:825`).
const TWO_BARKEEPER_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:808`).
const TWO_BARKEEPER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:813`, `:879`).
const TWO_BARKEEPER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:951`): idle "return to post" threshold.
const TWO_BARKEEPER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `cost = 15000 + ppd->legal_fine` (`two.c:900`).
const TWO_BARKEEPER_PASS_COST_FINE_BASE: u32 = 15000;
/// C `cost = 250000` (`two.c:902`).
const TWO_BARKEEPER_PASS_COST_DEAD: u32 = 250000;
/// C `cost = 15000` (`two.c:904`).
const TWO_BARKEEPER_PASS_COST_CLEAN: u32 = 15000;

/// C `struct barkeeper_data` (`two.c:770-773`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoBarkeeperDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_two_barkeeper_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoBarkeeperPlayerFacts {
    /// `PlayerRuntime::twocity_barkeeper_state()`.
    pub barkeeper_state: i32,
    /// `PlayerRuntime::twocity_citizen_status()`.
    pub citizen_status: i32,
    /// `PlayerRuntime::twocity_legal_status()`.
    pub legal_status: i32,
    /// `PlayerRuntime::twocity_legal_fine()`.
    pub legal_fine: i32,
}

/// A side effect [`World::process_two_barkeeper_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoBarkeeperOutcomeEvent {
    /// Write the new `twocity_ppd.barkeeper_state` back.
    UpdateBarkeeperState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->barkeeper_last = realtime;` (`two.c:858`).
    UpdateBarkeeperLast {
        player_id: CharacterId,
        realtime: i32,
    },
    /// C `ppd->citizen_status = CS_GUEST; ppd->legal_status = LS_CLEAN;
    /// ppd->legal_fine = 0;` (`two.c:912-914`), the successful guest-pass
    /// purchase's `PlayerRuntime` writeback.
    BuyPass { player_id: CharacterId },
}

impl World {
    /// C `barkeeper`'s per-tick body (`two.c:776-958`). `now` is C's
    /// wall-clock `realtime` (seconds).
    pub fn process_two_barkeeper_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoBarkeeperPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<TwoBarkeeperOutcomeEvent> {
        let barkeeper_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOBARKEEPER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for barkeeper_id in barkeeper_ids {
            self.process_two_barkeeper_tick(barkeeper_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_two_barkeeper_tick(
        &mut self,
        barkeeper_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoBarkeeperPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<TwoBarkeeperOutcomeEvent>,
    ) {
        let Some(barkeeper_name) = self.characters.get(&barkeeper_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::TwoBarkeeper(mut data)) = self
            .characters
            .get(&barkeeper_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&barkeeper_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_barkeeper_handle_char_message(
                    barkeeper_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_barkeeper_handle_text_message(
                    barkeeper_id,
                    &barkeeper_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_barkeeper_handle_give_message(barkeeper_id),
                _ => {}
            }
        }

        if let Some(barkeeper) = self.characters.get_mut(&barkeeper_id) {
            barkeeper.driver_state = Some(CharacterDriverState::TwoBarkeeper(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:947-949`).
        if let (Some(barkeeper), Some((tx, ty))) =
            (self.characters.get(&barkeeper_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(barkeeper.x), i32::from(barkeeper.y), tx, ty)
            {
                if let Some(barkeeper_mut) = self.characters.get_mut(&barkeeper_id) {
                    let _ = turn(barkeeper_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&barkeeper_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoBarkeeper(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact))
        // return; } do_idle(cn, TICKS);` (`two.c:951-957`). `tmpx`/`tmpy`
        // reuse `rest_x`/`rest_y`, the same substitution every other
        // stationary NPC in this codebase makes.
        if data.last_talk_tick + TWO_BARKEEPER_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&barkeeper_id)
                .map(|barkeeper| (barkeeper.rest_x, barkeeper.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                barkeeper_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:957`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `barkeeper`'s `NT_CHAR` branch (`two.c:792-872`).
    #[allow(clippy::too_many_arguments)]
    fn two_barkeeper_handle_char_message(
        &mut self,
        barkeeper_id: CharacterId,
        data: &mut TwoBarkeeperDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoBarkeeperPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoBarkeeperOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(barkeeper) = self.characters.get(&barkeeper_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:796-799`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message;
        // continue; }` (`two.c:802-805`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:808-811`).
        if tick < data.last_talk_tick + TWO_BARKEEPER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->
        // current_victim != co) continue;` (`two.c:813-816`).
        if tick < data.last_talk_tick + TWO_BARKEEPER_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:819-822`).
        if barkeeper_id == player_id
            || !char_see_char(&barkeeper, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:825-828`).
        if char_dist(&barkeeper, &player) > TWO_BARKEEPER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->barkeeper_state) { ... }` (`two.c:834-866`).
        match facts.barkeeper_state {
            0 => {
                self.npc_say(
                    barkeeper_id,
                    &format!(
                        "Hello, {}. Welcome to the tavern of the Two Towns.",
                        player.name
                    ),
                );
                events.push(TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                // C `if (ppd->citizen_status < CS_GUEST || ppd->
                // legal_status == LS_DEAD) { ... }` (`two.c:841-859`).
                if facts.citizen_status < CS_GUEST || facts.legal_status == LS_DEAD {
                    if facts.legal_status == LS_FINE {
                        self.npc_say_bytes(
                            barkeeper_id,
                            &format!(
                                "If thou needst go into Exkordon, I can help thee. Wouldst thou like to buy a guest pass? ({COL_STR_LIGHT_BLUE}buy pass{COL_STR_RESET} for 150G and pay {}G fines, for a total of {}G)",
                                facts.legal_fine / 100,
                                facts.legal_fine / 100 + 150
                            ),
                        );
                    } else if facts.legal_status == LS_DEAD {
                        self.npc_say_bytes(
                            barkeeper_id,
                            &format!(
                                "If thou needst go into Exkordon, I can help thee. But since thou hast killed the governor's double, it will be expensive. Wouldst thou like to buy a guest pass and the guard's forgiveness? ({COL_STR_LIGHT_BLUE}buy pass{COL_STR_RESET} for 2500G)"
                            ),
                        );
                    } else {
                        self.npc_say_bytes(
                            barkeeper_id,
                            &format!(
                                "If thou needst go into Exkordon, I can help thee. Wouldst thou like to buy a guest pass? ({COL_STR_LIGHT_BLUE}buy pass{COL_STR_RESET} for 150G)"
                            ),
                        );
                    }
                    events.push(TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
                        player_id,
                        new_state: 2,
                    });
                    events.push(TwoBarkeeperOutcomeEvent::UpdateBarkeeperLast {
                        player_id,
                        realtime: now,
                    });
                    didsay = true;
                }
            }
            // `barkeeper_state == 2`: C's own dead-code no-op
            // reassignment (`ppd->barkeeper_state = 2;`, `two.c:862-864`)
            // - no observable effect, see this module's doc comment.
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`two.c:867-871`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `barkeeper`'s `NT_TEXT` branch (`two.c:876-928`), wired through
    /// the generic `analyse_text_qa` matcher (same pattern as `world::
    /// npc::area17::two_skelly`/`alchemist`/`sanwyn`'s text handlers).
    #[allow(clippy::too_many_arguments)]
    fn two_barkeeper_handle_text_message(
        &mut self,
        barkeeper_id: CharacterId,
        barkeeper_name: &str,
        data: &mut TwoBarkeeperDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoBarkeeperPlayerFacts>,
        events: &mut Vec<TwoBarkeeperOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->
        // current_victim) dat->current_victim = 0;` (`two.c:879-881`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_BARKEEPER_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:883-886`).
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

        // C `analyse_text_driver`'s own guard clauses (`two.c:126-144`):
        // ignore our own talk, non-players/player-likes, not-visible.
        if barkeeper_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(barkeeper) = self.characters.get(&barkeeper_id).cloned() else {
            return;
        };
        if !char_see_char(&barkeeper, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let facts = player_facts
            .get(&speaker_id)
            .copied()
            .unwrap_or(TwoBarkeeperPlayerFacts {
                barkeeper_state: 0,
                citizen_status: 0,
                legal_status: 0,
                legal_fine: 0,
            });

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as `two_skelly`/`alchemist`/
        // `sanwyn`.
        match analyse_text_qa(text, barkeeper_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(barkeeper_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:889-895`): resets both
            // `last_talk` and `barkeeper_state` to `0` while
            // `barkeeper_state <= 2` (always true - `barkeeper_state`
            // never exceeds `2`).
            TextAnalysisOutcome::Matched(2) => {
                if facts.barkeeper_state <= 2 {
                    data.last_talk_tick = 0;
                    events.push(TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 13:` (buy pass) (`two.c:896-921`).
            TextAnalysisOutcome::Matched(13) => {
                if facts.citizen_status < CS_GUEST || facts.legal_status == LS_DEAD {
                    let cost = if facts.legal_status == LS_FINE {
                        TWO_BARKEEPER_PASS_COST_FINE_BASE
                            .saturating_add(facts.legal_fine.max(0) as u32)
                    } else if facts.legal_status == LS_DEAD {
                        TWO_BARKEEPER_PASS_COST_DEAD
                    } else {
                        TWO_BARKEEPER_PASS_COST_CLEAN
                    };
                    if self.two_barkeeper_take_money(speaker_id, cost) {
                        self.npc_say(
                            barkeeper_id,
                            &format!(
                                "Thou canst now enter Exkordon, {}. But do be careful there, they are most strict with their laws.",
                                speaker.name
                            ),
                        );
                        events.push(TwoBarkeeperOutcomeEvent::BuyPass {
                            player_id: speaker_id,
                        });
                    } else {
                        self.npc_say(barkeeper_id, "Thou dost not have enough money.");
                    }
                } else {
                    self.npc_say(barkeeper_id, "But thou hast a pass already.");
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:924-927`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `barkeeper`'s `NT_GIVE` branch (`two.c:931-939`): unlike every
    /// sibling driver in this file, *any* item handed over is destroyed
    /// unconditionally - there is no give-back fallback.
    fn two_barkeeper_handle_give_message(&mut self, barkeeper_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get(&barkeeper_id)
            .and_then(|barkeeper| barkeeper.cursor_item)
        else {
            return;
        };
        if let Some(barkeeper) = self.characters.get_mut(&barkeeper_id) {
            barkeeper.cursor_item = None;
        }
        self.destroy_item(item_id);
    }

    /// C `take_money(cn, val)` (`src/system/tool.c:3820-3826`), a private
    /// copy matching every other NPC's own inline `take_money` copy (see
    /// `world::gatekeeper::gate_take_money`'s doc comment for the same
    /// precedent).
    fn two_barkeeper_take_money(&mut self, player_id: CharacterId, amount: u32) -> bool {
        let Some(player) = self.characters.get_mut(&player_id) else {
            return false;
        };
        if player.gold < amount {
            return false;
        }
        player.gold -= amount;
        player.flags.insert(CharacterFlags::ITEMS);
        true
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
