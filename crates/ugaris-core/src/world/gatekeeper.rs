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
//!   `5`-`8`) now run the full precondition check
//!   (`crate::character_driver::gate_enter_test_precheck`, ported from
//!   `enter_test`'s validation half, `gatekeeper.c:316-390`) and reply
//!   exactly like C on every *failure* path: the paid/lab/noexp/carried-
//!   item checks send a private `queue_system_text` (C's `log_char(cn,
//!   LOG_SYSTEM, ...)`, addressed to the player only), and an invalid
//!   class choice makes the gatekeeper itself say "That is not a possible
//!   choice." (C's `say(cn, ...)` in the caller). The *success* path
//!   (`GateEnterTestOutcome::Ready`) now emits a
//!   [`GateWelcomeOutcomeEvent::EnterTestReady`] event: the room search's
//!   own state check (`gate_room_is_clear`) and the player-side of
//!   `enter_room`'s success tail (`gate_finish_enter_room`: teleport,
//!   spell-slot stripping, HP/mana/endurance/`regen_ticker` reset) live
//!   here, but the opponent's `create_char`/`drop_char` (needs
//!   `ZoneLoader::instantiate_character_template`, which `World` cannot
//!   call) is handled by `ugaris-server`'s
//!   `spawns::gate_enter_test_spawn_room`, invoked from
//!   `apply_gate_welcome_events` the same way other loader-dependent spawns
//!   are (see `spawns.rs`'s `spawn_swampspawn_character` precedent). Two
//!   gaps remain, both already called out in `PORTING_TODO.md`: (1)
//!   `destroy_chareffects(cn)` is a no-op - `Character` has no active-spell-
//!   effect list yet; (2) the opponent's `tmpx`/`tmpy` "return to post"
//!   coordinates (consumed once `gate_fight_driver` is ported) reuse
//!   `rest_x`/`rest_y`, since `Character` has no dedicated `tmpx`/`tmpy`
//!   field (same substitution `respawn_npc_character` already uses for
//!   other NPCs).
//! - The `NT_GIVE` handler's `give_driver` retry-until-adjacent semantics
//!   (`src/system/drvlib.c::give_driver`, pathfinding toward the giver)
//!   are simplified to a direct give-or-destroy, matching the
//!   `world::trader` precedent (`trader_give_char_item`/
//!   `trader_return_or_destroy_cursor_item`) - the giver is already known
//!   to be adjacent/visible since the give action itself required it.
//! - The idle "return to post" `secure_move_driver` safety net
//!   (`gatekeeper.c:627-631`) reuses `rest_x`/`rest_y` as the NPC's post
//!   position (C's `tmpx`/`tmpy`), the same substitution already used for
//!   the opponent's post position and other stationary NPCs (`world::
//!   bank`, `respawn_npc_character`). `ret`/`lastact` are always passed as
//!   `0`, matching the simplification already accepted for this class of
//!   driver (see `process_gate_welcome_actions`'s caller).

use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    analyse_text_qa, gate_enter_test_precheck, gate_welcome_dialogue_step,
    gate_welcome_state_after_repeat, GateEnterTestOutcome, GateEnterTestPrecheck,
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
/// C `TICKS * 30` (`gatekeeper.c:627`): idle "return to post" threshold.
const GATE_WELCOME_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

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
    /// `gate_enter_test_precheck` returned `Ready`
    /// (`gatekeeper.c:571-578`'s call into `enter_test`'s `take_money`/
    /// `enter_room` tail, `gatekeeper.c:392-407`). The caller must attempt
    /// the full opponent-spawn side effect
    /// (`ugaris-server::spawns::gate_enter_test_spawn_room`) since it needs
    /// `ZoneLoader` access `World` doesn't have.
    EnterTestReady { player_id: CharacterId, class: i32 },
}

/// C `enter_test`'s carried-item count (`gatekeeper.c:368-375`): inventory
/// slots `30..INVENTORYSIZE` plus `ch[cn].citem`.
fn gate_carried_item_count(character: &Character) -> u32 {
    let inventory_count = character
        .inventory
        .iter()
        .skip(INVENTORY_START_INVENTORY)
        .filter(|slot| slot.is_some())
        .count() as u32;
    inventory_count + u32::from(character.cursor_item.is_some())
}

impl World {
    /// C `gate_welcome_driver`'s per-tick body (`gatekeeper.c:417-634`).
    pub fn process_gate_welcome_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
        area_id: u16,
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
            self.process_gate_welcome_messages(gate_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_gate_welcome_messages(
        &mut self,
        gate_id: CharacterId,
        player_facts: &HashMap<CharacterId, GateWelcomePlayerFacts>,
        area_id: u16,
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

        // C `if (dat->last_talk + TICKS*30 < ticker)
        // { if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_UP, ret,
        // lastact)) return; }` (`gatekeeper.c:627-631`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `respawn_npc_character`/`world::bank` already use
        // for other stationary NPCs' spawn tiles. `ret`/`lastact` are
        // always `0` here: like `world::trader`/`world::bank`, this driver
        // doesn't thread the C driver dispatcher's own last-action/return
        // code through (those only matter to avoid re-attempting a move
        // immediately after a same-tick door-use), a simplification
        // already accepted for this class of stationary NPC driver.
        let last_talk = if let Some(gate) = self.characters.get(&gate_id) {
            match gate.driver_state.as_ref() {
                Some(CharacterDriverState::GateWelcome(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + GATE_WELCOME_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(gate) = self.characters.get(&gate_id) else {
                return;
            };
            let (post_x, post_y) = (gate.rest_x, gate.rest_y);
            self.secure_move_driver(gate_id, post_x, post_y, Direction::Up as u8, 0, 0, area_id);
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
            // C `case 5: case 6: case 7: case 8: if (!enter_test(co,
            // didsay)) say(cn, "That is not a possible choice.");`
            // (`gatekeeper.c:571-578`), with `enter_test`'s own
            // preconditions (`gatekeeper.c:316-390`) ported as
            // `gate_enter_test_precheck`. See the module doc comment for
            // why `Ready` (the success path) is still a no-op.
            TextAnalysisOutcome::Matched(class @ 5..=8) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let precheck = gate_enter_test_precheck(GateEnterTestPrecheck {
                        is_paid: speaker.flags.contains(CharacterFlags::PAID),
                        needs_lab: facts.needs_lab,
                        is_god: speaker.flags.contains(CharacterFlags::GOD),
                        is_noexp: speaker.flags.contains(CharacterFlags::NOEXP),
                        flags: speaker.flags,
                        carried_item_count: gate_carried_item_count(&speaker),
                        class,
                    });
                    match precheck {
                        GateEnterTestOutcome::NotPaid => self.queue_system_text(
                            speaker_id,
                            "Sorry, only paying players may take the test.",
                        ),
                        GateEnterTestOutcome::LabNotSolved => self.queue_system_text(
                            speaker_id,
                            "Sorry, you may not enter before you have solved the labyrinth.",
                        ),
                        GateEnterTestOutcome::NoExpMode => self.queue_system_text(
                            speaker_id,
                            "Sorry, you may not enter if you have the /noexp mode turned on.",
                        ),
                        GateEnterTestOutcome::CarryingItems { count } => self.queue_system_text(
                            speaker_id,
                            format!(
                                "Sorry, you may not enter while you are carrying items. You currently have {count} items."
                            ),
                        ),
                        GateEnterTestOutcome::CarryingTooManyItems { count } => self
                            .queue_system_text(
                                speaker_id,
                                format!(
                                    "Sorry, you may not enter while you are carrying more than three items. You currently have {count} items."
                                ),
                            ),
                        GateEnterTestOutcome::InvalidClass => {
                            self.npc_say(gate_id, "That is not a possible choice.");
                        }
                        // C's `enter_room` success path (`take_money` +
                        // opponent spawn): deferred to the caller since it
                        // needs `ZoneLoader` access - see the module doc
                        // comment.
                        GateEnterTestOutcome::Ready => {
                            events.push(GateWelcomeOutcomeEvent::EnterTestReady {
                                player_id: speaker_id,
                                class,
                            });
                        }
                    }
                }
                didsay = true;
            }
            // "aye"/"nay" (codes `3`/`4`) are unhandled by C's `switch`
            // but still count as `didsay`.
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

    /// C `enter_room`'s room-clear scan (`gatekeeper.c:233-240`): every
    /// tile in the 9x17 room must have no character, and any item present
    /// must not be `IF_TAKE` (i.e. not pick-up-able - fixed furniture is
    /// fine).
    pub fn gate_room_is_clear(&self, xs: u16, ys: u16) -> bool {
        for x in xs..xs + 9 {
            for y in ys..ys + 17 {
                let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) else {
                    return false;
                };
                if tile.character != 0 {
                    return false;
                }
                if tile.item != 0
                    && self
                        .items
                        .get(&ItemId(tile.item))
                        .is_some_and(|item| item.flags.contains(ItemFlags::TAKE))
                {
                    return false;
                }
            }
        }
        true
    }

    /// C `take_money(cn, val)` (`src/system/tool.c:3820-3826`).
    pub fn gate_take_money(&mut self, player_id: CharacterId, amount: u32) -> bool {
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

    /// C `give_money_silent(cn, val, reason)` (`src/system/tool.c:
    /// 1441-1449`), minus the `dlog`/Macro-Daemon activity tracking, which
    /// have no Rust equivalent yet (same omission as every other
    /// `give_money_silent` call site in this codebase).
    pub fn gate_give_money_silent(&mut self, player_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
    }

    /// The player-side tail of `enter_room`'s success path
    /// (`gatekeeper.c:277-303`), once the opponent is already spawned at
    /// `(xs + 4, ys + 13)`: `teleport_char_driver(cn, xs + 4, ys + 4)`
    /// (including its "already close enough" failure check),
    /// `destroy_chareffects` (a documented no-op - see the module doc
    /// comment), stripping spell slots `12..=29`, the two `log_char`
    /// notices, and resetting HP/mana/endurance to `POWERSCALE * 1` plus
    /// `regen_ticker = ticker`. Returns `false` (matching C's
    /// `teleport_char_driver` failure) when the player was already within
    /// Manhattan distance `1` of the target tile; the caller must then
    /// destroy the already-spawned opponent and try the next room.
    pub fn gate_finish_enter_room(&mut self, player_id: CharacterId, xs: u16, ys: u16) -> bool {
        let Some(player) = self.characters.get(&player_id) else {
            return false;
        };
        let target_x = xs + 4;
        let target_y = ys + 4;
        let dx = i32::from(player.x) - i32::from(target_x);
        let dy = i32::from(player.y) - i32::from(target_y);
        if dx.abs() + dy.abs() < 2 {
            return false;
        }
        if !self.teleport_character(player_id, target_x, target_y, false) {
            return false;
        }

        // C `destroy_chareffects(cn)` (`gatekeeper.c:281`): no active-spell-
        // effect list is modeled on `Character` yet, so this is a
        // documented no-op (see the module doc comment).

        // C `for (n = 12; n < 30; n++) if ((in = ch[cn].item[n]))
        // { destroy_item(in); ch[cn].item[n] = 0; }` (`gatekeeper.c:
        // 282-286`).
        let Some(player) = self.characters.get_mut(&player_id) else {
            return false;
        };
        let stripped_items: Vec<ItemId> = (INVENTORY_START_SPELLS..=INVENTORY_LAST_SPELLS)
            .filter_map(|slot| player.inventory[slot].take())
            .collect();
        for item_id in stripped_items {
            self.destroy_item(item_id);
        }

        self.queue_system_text(player_id, "All your spells have been removed.");
        self.queue_system_text(
            player_id,
            "Once you are ready for the test, use the door to the south-west to enter the room containing your opponent. You have ten minutes from now on.",
        );

        let tick = self.tick.0.min(u64::from(u32::MAX)) as u32;
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.hp = POWERSCALE;
            if player.mana != 0 {
                player.mana = POWERSCALE;
            }
            player.endurance = POWERSCALE;
            player.regen_ticker = tick;
        }
        true
    }
}
