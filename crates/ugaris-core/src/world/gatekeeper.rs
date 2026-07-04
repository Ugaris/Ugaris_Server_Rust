//! Gatekeeper-welcome NPC (`CDR_GATE_WELCOME`).
//!
//! Ports the message-loop slice of
//! `src/system/gatekeeper.c::gate_welcome_driver` (the stationary NPC in
//! front of the Ishtar labyrinth). The pure dialogue state machine and
//! `enter_test` preconditions were already ported in
//! `crate::character_driver` (`gate_welcome_dialogue_step`,
//! `gate_enter_test_precheck`, `GATEKEEPER_QA`) - this module wires them
//! into `World`'s message loop the same way `world/trader.rs` wires
//! `TRADER_QA`.
//!
//! Two pieces of per-player state the dialogue needs -
//! `PlayerRuntime::gate_welcome_state` (`gate_ppd.welcome_state`) and
//! `teleport_next_lab`'s truthiness (from `PlayerRuntime::lab_solved_bits`,
//! via `crate::character_driver::needs_next_lab`) - live in
//! `crate::player::PlayerRuntime`, owned by `ugaris-server`
//! (`ServerRuntime::players`), not `World`. Following the same split
//! already established for `world::bank`'s `BankEvent`, the caller
//! (`ugaris-server`'s tick loop) supplies a per-player fact snapshot
//! ([`GateWelcomePlayerFacts`]) up front and applies the returned
//! [`GateWelcomeOutcomeEvent`]s afterwards.
//!
//! Deviations/gaps (documented, not silent):
//! - `enter_test`'s class-choice codes (`analyse_text_driver` answer codes
//!   `5`-`8`) are not wired yet: `enter_room`'s private-room opponent
//!   spawn (`create_char`/`drop_char`/inventory-stripping) has no `World`
//!   counterpart yet. A matched class-choice message is bookkept (counts
//!   as `didsay`, updates `current_victim`/`last_talk`) but produces no
//!   reply, unlike C's "That is not a possible choice." (shown only when
//!   `enter_test`'s class-validation fails) or a successful test start.
//!   See `PORTING_TODO.md`'s "Gatekeeper NPC" task.
//! - The `NT_GIVE` handler's `give_driver` retry-until-adjacent semantics
//!   (`src/system/drvlib.c::give_driver`, pathfinding toward the giver)
//!   are simplified to a direct give-or-destroy, matching the
//!   `world::trader` precedent (`trader_give_char_item`/
//!   `trader_return_or_destroy_cursor_item`) - the giver is already known
//!   to be adjacent/visible since the give action itself required it.
//! - The idle "return to post" `secure_move_driver` safety net
//!   (`gatekeeper.c:627-631`) is not ported: this NPC's spawn/post
//!   position (`ch[cn].tmpx`/`tmpy`) is not modeled on `Character` yet.

use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    analyse_text_qa, gate_welcome_dialogue_step, gate_welcome_state_after_repeat,
    GateWelcomeContext, GateWelcomeDriverData, TextAnalysisOutcome, GATEKEEPER_QA,
};
use crate::drvlib::offset2dx;

/// C `char_dist(cn, co) > 10` (`gatekeeper.c:466`): the `NT_CHAR` greeting
/// range.
const GATE_WELCOME_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gatekeeper.c:140`): `analyse_text_driver`'s
/// own, slightly wider, small-talk range.
const GATE_WELCOME_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gatekeeper.c:449`).
const GATE_WELCOME_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gatekeeper.c:454,555`).
const GATE_WELCOME_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `dat->amgivingback < 20` (`gatekeeper.c:605`).
const GATE_WELCOME_GIVEBACK_LIMIT: i32 = 20;

/// Per-player facts [`World::process_gate_welcome_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateWelcomePlayerFacts {
    /// `PlayerRuntime::gate_welcome_state` (`gate_ppd.welcome_state`).
    pub welcome_state: i32,
    /// `crate::character_driver::needs_next_lab(player.lab_solved_bits)`.
    pub needs_lab: bool,
}

/// A side effect [`World::process_gate_welcome_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateWelcomeOutcomeEvent {
    /// Write the new `gate_ppd.welcome_state` back (either the dialogue
    /// advancing, or a "repeat"/"restart" reset).
    UpdateWelcomeState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 9: if (ch[co].flags & CF_GOD) del_data(co, DRD_LAB_PPD);`
    /// (`gatekeeper.c:579-583`).
    ResetLabPpd { player_id: CharacterId },
}

impl World {
    /// C `gate_welcome_driver`'s per-tick body (`gatekeeper.c:417-634`),
    /// minus the idle "return to post" safety net (see the module doc
    /// comment).
    pub fn process_gate_welcome_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
    ) -> Vec<GateWelcomeOutcomeEvent> {
        let gate_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GATE_WELCOME
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for gate_id in gate_ids {
            self.process_gate_welcome_messages(gate_id, player_facts, &mut events);
        }
        events
    }

    fn process_gate_welcome_messages(
        &mut self,
        gate_id: CharacterId,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
        events: &mut Vec<GateWelcomeOutcomeEvent>,
    ) {
        let Some(gate_name) = self.characters.get(&gate_id).map(|gate| gate.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::GateWelcome(mut data)) = self
            .characters
            .get(&gate_id)
            .and_then(|gate| gate.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&gate_id)
            .map(|gate| std::mem::take(&mut gate.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.gate_welcome_handle_char_message(
                    gate_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.gate_welcome_handle_text_message(
                    gate_id,
                    &gate_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.gate_welcome_handle_give_message(gate_id, &mut data, message),
                _ => {}
            }
        }

        // C `dat->amgivingback = 0;` (`gatekeeper.c:621`): unconditionally
        // reset every tick, regardless of whether an `NT_GIVE` fired.
        data.amgivingback = 0;

        if let Some(gate) = self.characters.get_mut(&gate_id) {
            gate.driver_state = Some(CharacterDriverState::GateWelcome(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gatekeeper.c:623-625`).
        if let (Some(gate), Some((tx, ty))) = (self.characters.get(&gate_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(gate.x), i32::from(gate.y), tx, ty) {
                if let Some(gate_mut) = self.characters.get_mut(&gate_id) {
                    let _ = turn(gate_mut, direction as u8);
                }
            }
        }
    }

    /// C `gate_welcome_driver`'s `NT_CHAR` branch (`gatekeeper.c:432-548`).
    fn gate_welcome_handle_char_message(
        &mut self,
        gate_id: CharacterId,
        data: &mut GateWelcomeDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
        events: &mut Vec<GateWelcomeOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(gate) = self.characters.get(&gate_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue; }`
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue; }`
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        if tick < data.last_talk + GATE_WELCOME_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;`
        if tick < data.last_talk + GATE_WELCOME_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        if gate_id == player_id || !char_see_char(&gate, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;`
        if char_dist(&gate, &player) > GATE_WELCOME_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let outcome = gate_welcome_dialogue_step(GateWelcomeContext {
            player_name: &player.name,
            welcome_state: facts.welcome_state,
            needs_lab: facts.needs_lab,
            flags: player.flags,
        });

        if outcome.welcome_state != facts.welcome_state {
            events.push(GateWelcomeOutcomeEvent::UpdateWelcomeState {
                player_id,
                new_state: outcome.welcome_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gatekeeper.c:543-547`) - `didsay`
        // is only set by branches that call `say()`, i.e. `outcome.text`
        // is `Some`.
        if let Some(text) = outcome.text {
            self.npc_say(gate_id, &text);
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `gate_welcome_driver`'s `NT_TEXT` branch (`gatekeeper.c:552-590`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world/trader.rs::trader_qa_reply`).
    fn gate_welcome_handle_text_message(
        &mut self,
        gate_id: CharacterId,
        gate_name: &str,
        data: &mut GateWelcomeDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
        events: &mut Vec<GateWelcomeOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let tick = self.tick.0;

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gatekeeper.c:555-557`).
        if tick > data.last_talk + GATE_WELCOME_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)
        // { remove_message; continue; }` (`gatekeeper.c:559-562`).
        if data
            .current_victim
            .is_some_and(|victim| victim != speaker_id)
        {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gatekeeper.c:
        // 126-146`): ignore our own talk, non-players, distance > 12,
        // not-visible (the log-type/`LOG_SYSTEM`/`LOG_INFO` guard doesn't
        // apply - Rust `push_driver_text_message` only ever emits plain
        // speech).
        if gate_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(gate) = self.characters.get(&gate_id).cloned() else {
            return;
        };
        if char_dist(&gate, &speaker) > GATE_WELCOME_QA_DISTANCE
            || !char_see_char(&gate, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, gate_name, &speaker.name, GATEKEEPER_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(gate_id, &reply);
                didsay = true;
            }
            // C `case 2: ppd = ...; if (ppd && ppd->welcome_state <= 6)
            // ppd->welcome_state = 0;` (`gatekeeper.c:565-570`).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    events.push(GateWelcomeOutcomeEvent::UpdateWelcomeState {
                        player_id: speaker_id,
                        new_state: gate_welcome_state_after_repeat(facts.welcome_state),
                    });
                }
                didsay = true;
            }
            // C `case 9: if (ch[co].flags & CF_GOD) del_data(co,
            // DRD_LAB_PPD);` (`gatekeeper.c:579-583`).
            TextAnalysisOutcome::Matched(9) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    events.push(GateWelcomeOutcomeEvent::ResetLabPpd {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Codes `3`/`4` ("aye"/"nay", unhandled by the switch) and
            // `5`-`8` (class choice - `enter_test` not wired yet, see the
            // module doc comment) still count as `didsay` in C.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gatekeeper.c:585-589`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `gate_welcome_driver`'s `NT_GIVE` branch (`gatekeeper.c:592-613`).
    /// See the module doc comment for the `give_driver` simplification.
    fn gate_welcome_handle_give_message(
        &mut self,
        gate_id: CharacterId,
        data: &mut GateWelcomeDriverData,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&gate_id)
            .and_then(|gate| gate.cursor_item.take())
        else {
            return;
        };

        if data.amgivingback == 0 {
            self.npc_say(
                gate_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            data.amgivingback = 1;
        } else {
            data.amgivingback += 1;
        }

        let given = data.amgivingback < GATE_WELCOME_GIVEBACK_LIMIT
            && self.gate_welcome_give_char_item(giver_id, item_id);
        if !given {
            self.destroy_item(item_id);
        }
    }

    /// C `give_char_item(cn, in)` (`src/system/tool.c:3371-3394`), reused
    /// the same way `world::trader`'s `trader_give_char_item` does.
    fn gate_welcome_give_char_item(&mut self, target_id: CharacterId, item_id: ItemId) -> bool {
        let Some(target) = self.characters.get_mut(&target_id) else {
            return false;
        };
        if target.cursor_item.is_none() {
            target.cursor_item = Some(item_id);
        } else {
            let Some(slot) = target
                .inventory
                .iter_mut()
                .skip(INVENTORY_START_INVENTORY)
                .find(|slot| slot.is_none())
            else {
                return false;
            };
            *slot = Some(item_id);
        }
        target.flags.insert(CharacterFlags::ITEMS);
        if let Some(item) = self.items.get_mut(&item_id) {
            item.carried_by = Some(target_id);
        }
        true
    }
}
