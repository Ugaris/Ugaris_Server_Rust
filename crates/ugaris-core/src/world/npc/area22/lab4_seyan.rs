//! Lab 4 "Observer" (`CDR_LAB4SEYAN`), the Seyan'Du quest giver who sends
//! players after the Gnalb King's Crown and the Gnalb Mage's Szepter and
//! opens the gate to Lab 5 once both are handed in.
//!
//! Ports `src/area/22/lab4.c::lab4_seyan_driver` (`:106-337`). All of the
//! dialogue state (`ppd->seyan4state`/`seyan4got`) lives in the player's
//! own `DRD_LAB4_PLAYER` PPD slot (`crate::player::PlayerRuntime::
//! lab4_seyan_state`/`lab4_seyan_got` - a distinct slot from
//! `DRD_LAB_PPD`/`lab_ppd`, same precedent as `world::npc::area22::
//! lab2_deamon`'s own `lab2_deamon_checked`), so following the same split
//! already established for `world::gatekeeper`/`world::npc::area22::
//! lab2_herald`, the caller supplies a per-player fact snapshot
//! ([`Lab4SeyanPlayerFacts`]) up front and applies the returned
//! [`Lab4SeyanOutcomeEvent`]s afterwards. `dat->cv_co`/`cv_serial`/
//! `lasttalk` (C's `static struct lab4_seyan_data datbuf` - a function-
//! local static, i.e. actually shared across every character that ever
//! calls this driver, not per-NPC `set_data` state) is stored as this
//! NPC's own [`Lab4SeyanDriverData`]; since exactly one Seyan is ever
//! spawned (`zones/22/lab4.chr`), storing it per-character is observably
//! identical to C's single shared static.
//!
//! Deviations/gaps (documented, not silent):
//! - `standard_message_driver(cn, msg, 0, 0)` (`lab4.c:322`), called
//!   unconditionally after every message this driver already handled
//!   explicitly, is not reproduced: with `agressive=0, helper=0` its
//!   `NT_CHAR`/`NT_SEEHIT` cases are dead code and its `NT_GOTHIT` case
//!   only calls `fight_driver_note_hit(cn)` (a hit-timestamp this driver
//!   never reads, since the seyan never fights) - never observably
//!   different, same precedent as `lab2_herald`'s own module doc comment.
//! - The wandering "return to post" tail (`secure_move_driver(cn,
//!   ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN, ret, lastact)`) reuses
//!   `rest_x`/`rest_y` for `tmpx`/`tmpy`, same substitution as every
//!   other stationary NPC in this file.

use crate::drvlib::offset2dx;
use crate::item_driver::{IID_LAB4_CROWN, IID_LAB4_SZEPTER};
use crate::player::lab4_seyan_state_from_got;
use crate::world::*;
use std::collections::HashMap;

/// C `char_dist(cn, co) > 10` (`lab4.c:177`): the `NT_CHAR` greeting
/// range.
const LAB4_SEYAN_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`lab4.c:169`): only talk once the previous line's
/// cooldown has passed.
const LAB4_SEYAN_TALK_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 30` (`lab4.c:330`): idle "return to post" threshold.
const LAB4_SEYAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `seyan4got` bit flags (`lab4.c:85`).
const GOT_CROWN: u8 = 1 << 0;
const GOT_SZEPTER: u8 = 1 << 1;

/// Per-player facts [`World::process_lab4_seyan_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab4SeyanPlayerFacts {
    /// `PlayerRuntime::lab4_seyan_state`.
    pub seyan4state: u8,
    /// `PlayerRuntime::lab4_seyan_got`.
    pub seyan4got: u8,
}

/// A side effect [`World::process_lab4_seyan_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab4SeyanOutcomeEvent {
    /// Write the new `ppd->seyan4state`/`seyan4got` back. Both fields are
    /// always sent together since C's own `set_seyan_state` always
    /// recomputes `seyan4state` from the current `seyan4got` bits.
    SetPlayerData {
        player_id: CharacterId,
        seyan4state: u8,
        seyan4got: u8,
    },
}

impl World {
    /// C `lab4_seyan_driver`'s per-tick body (`lab4.c:106-337`).
    pub fn process_lab4_seyan_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab4SeyanPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab4SeyanOutcomeEvent> {
        let seyan_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB4SEYAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for seyan_id in seyan_ids {
            self.process_lab4_seyan_messages(seyan_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab4_seyan_messages(
        &mut self,
        seyan_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab4SeyanPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab4SeyanOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab4Seyan(mut data)) = self
            .characters
            .get(&seyan_id)
            .and_then(|seyan| seyan.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&seyan_id)
            .map(|seyan| std::mem::take(&mut seyan.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_GIVE => self.lab4_seyan_handle_give_message(
                    seyan_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                NT_CHAR => self.lab4_seyan_handle_char_message(
                    seyan_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => {
                    self.lab4_seyan_handle_text_message(seyan_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(seyan) = self.characters.get_mut(&seyan_id) {
            seyan.driver_state = Some(CharacterDriverState::Lab4Seyan(data.clone()));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`lab4.c:326-328`).
        if let (Some(seyan), Some((tx, ty))) =
            (self.characters.get(&seyan_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(seyan.x), i32::from(seyan.y), tx, ty) {
                if let Some(seyan_mut) = self.characters.get_mut(&seyan_id) {
                    let _ = turn(seyan_mut, direction as u8);
                }
            }
        }

        // C `if (dat->lasttalk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; } do_idle(cn, TICKS);` (`lab4.c:330-336`).
        if data.lasttalk + LAB4_SEYAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(seyan) = self.characters.get(&seyan_id) else {
                return;
            };
            let (post_x, post_y) = (seyan.rest_x, seyan.rest_y);
            self.secure_move_driver(
                seyan_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `lab4_seyan_driver`'s `NT_GIVE` branch (`lab4.c:119-155`).
    fn lab4_seyan_handle_give_message(
        &mut self,
        seyan_id: CharacterId,
        data: &mut Lab4SeyanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab4SeyanPlayerFacts>,
        events: &mut Vec<Lab4SeyanOutcomeEvent>,
    ) {
        // C `if (!ch[cn].citem) { remove_message; continue; }`.
        let Some(item_id) = self
            .characters
            .get(&seyan_id)
            .and_then(|seyan| seyan.cursor_item)
        else {
            return;
        };

        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let giver_is_player = self
            .characters
            .get(&giver_id)
            .is_some_and(|giver| giver.flags.contains(CharacterFlags::PLAYER));

        if giver_is_player {
            let template_id = self.items.get(&item_id).map(|item| item.template_id);
            let bit = match template_id {
                Some(id) if id == IID_LAB4_CROWN => Some(GOT_CROWN),
                Some(id) if id == IID_LAB4_SZEPTER => Some(GOT_SZEPTER),
                _ => None,
            };
            if let (Some(bit), Some(facts)) = (bit, player_facts.get(&giver_id)) {
                let new_got = facts.seyan4got | bit;
                events.push(Lab4SeyanOutcomeEvent::SetPlayerData {
                    player_id: giver_id,
                    seyan4state: lab4_seyan_state_from_got(new_got),
                    seyan4got: new_got,
                });
                // C `if (dat->cv_co && (dat->cv_co != co || ch[dat->cv_co]
                // .serial != dat->cv_serial)) say(cn, "%s, please be
                // patient while i'm talking to others.", ch[co].name);`
                if let Some(cv_co) = data.cv_co {
                    let cv_still_matches = cv_co == giver_id
                        && self
                            .characters
                            .get(&cv_co)
                            .is_some_and(|cv| cv.serial == data.cv_serial);
                    if !cv_still_matches {
                        let giver_name = self
                            .characters
                            .get(&giver_id)
                            .map(|giver| giver.name.clone())
                            .unwrap_or_default();
                        self.npc_say(
                            seyan_id,
                            &format!(
                                "{giver_name}, please be patient while i'm talking to others."
                            ),
                        );
                    }
                }
            }
        }

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;` - unconditional.
        self.destroy_item(item_id);
    }

    /// C `lab4_seyan_driver`'s `NT_CHAR` branch (`lab4.c:157-288`): the
    /// greeting/dialogue state machine, keyed off the seen player's own
    /// `ppd->seyan4state`.
    fn lab4_seyan_handle_char_message(
        &mut self,
        seyan_id: CharacterId,
        data: &mut Lab4SeyanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab4SeyanPlayerFacts>,
        events: &mut Vec<Lab4SeyanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(seyan) = self.characters.get(&seyan_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`lab4.c:161-164`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`lab4.c:165-168`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->lasttalk + 5 * TICKS) { remove_message;
        // continue; }` (`lab4.c:169-172`).
        if tick < data.lasttalk + LAB4_SEYAN_TALK_COOLDOWN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`lab4.c:173-176`).
        if seyan_id == player_id || !char_see_char(&seyan, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`lab4.c:177-180`).
        if char_dist(&seyan, &player) > LAB4_SEYAN_GREET_DISTANCE {
            return;
        }

        // C `lab4.c:182-188`: drop the current victim if it's no longer
        // valid.
        if let Some(cv_co) = data.cv_co {
            let still_valid = self.characters.get(&cv_co).is_some_and(|cv| {
                cv.serial == data.cv_serial
                    && char_dist(&seyan, cv) <= LAB4_SEYAN_GREET_DISTANCE
                    && char_see_char(&seyan, cv, &self.map, self.date.daylight)
            });
            if !still_valid {
                data.cv_co = None;
            }
        }

        // C `lab4.c:190-194`: only talk to the current victim.
        if let Some(cv_co) = data.cv_co {
            if cv_co != player_id {
                return;
            }
        }

        // C `lab4.c:196-200`: set new victim.
        if data.cv_co.is_none() {
            data.cv_co = Some(player_id);
            data.cv_serial = player.serial;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.seyan4state;
        let mut clear_cv = false;

        match facts.seyan4state {
            0 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Hello {}. This is thy first mission in the Labyrinth.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 1;
            }
            1 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Listen, {}. To the east is an entrance to the Gnalbs winter residence.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 2;
            }
            2 => {
                self.npc_say(
                    seyan_id,
                    "The Gnalbs are peaceful creatures, but their King and their Mage and the \
                     Guards are not.",
                );
                didsay = true;
                new_state = 3;
            }
            3 => {
                self.npc_say(
                    seyan_id,
                    "Bring me the King's Crown, and the Mage's Szepter to prove thou art worthy \
                     to enter the next Gate.",
                );
                didsay = true;
                new_state = 4;
            }
            4 => {
                self.npc_say(
                    seyan_id,
                    &format!("Go ahead now, {}, and fulfil thine destiny.", player.name),
                );
                didsay = true;
                new_state = 5;
            }
            5 => {
                clear_cv = true;
            }
            10 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Thou broughtst me the Kings Crown. Now, {}, seek for the Mage's \
                         Szepter.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 11;
            }
            11 => {
                clear_cv = true;
            }
            20 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Thou broughtst me the Mages Szepter. Now, {}, seek for the King's \
                         Crown.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 21;
            }
            21 => {
                clear_cv = true;
            }
            30 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "{}, thou broughtst me the King's Crown and the Mage's Szepter.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 31;
            }
            31 => {
                self.npc_say(
                    seyan_id,
                    "Now I will open a magic gate for thee. Use it, and thou wilt be able to \
                     travel to the next part of the Labyrinth.",
                );
                didsay = true;
                new_state = 32;
            }
            32 => {
                self.queue_lab_exit_spawn(player_id, 10);
                self.npc_say(
                    seyan_id,
                    &format!("Mayest Thou Past The Last Gate, {}", player.name),
                );
                didsay = true;
                new_state = 33;
            }
            33 => {
                clear_cv = true;
            }
            _ => {}
        }

        if new_state != facts.seyan4state {
            events.push(Lab4SeyanOutcomeEvent::SetPlayerData {
                player_id,
                seyan4state: new_state,
                seyan4got: facts.seyan4got,
            });
        }
        if clear_cv {
            data.cv_co = None;
        }

        // C `if (didsay) { dat->lasttalk = ticker; talkdir =
        // offset2dx(...); }` (`lab4.c:284-287`).
        if didsay {
            data.lasttalk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `lab4_seyan_driver`'s `NT_TEXT` branch (`lab4.c:290-320`): both
    /// the generic `tabunga` god-mode debug echo and the "REPEAT" keyword
    /// recompute.
    fn lab4_seyan_handle_text_message(
        &mut self,
        seyan_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab4SeyanPlayerFacts>,
        events: &mut Vec<Lab4SeyanOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `lab4.c:294`: `tabunga(cn, co, (char *)msg->dat2)`.
        self.apply_tabunga_text_notification(seyan_id, speaker_id, text);

        // C `lab4.c:296-319`.
        if speaker_id == seyan_id {
            return;
        }
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(seyan) = self.characters.get(&seyan_id).cloned() else {
            return;
        };
        if !char_see_char(&seyan, &speaker, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&speaker_id) else {
            return;
        };

        if text.to_ascii_uppercase().contains("REPEAT") {
            self.npc_say(seyan_id, &format!("I will repeat, {}", speaker.name));
            let recomputed = lab4_seyan_state_from_got(facts.seyan4got);
            events.push(Lab4SeyanOutcomeEvent::SetPlayerData {
                player_id: speaker_id,
                seyan4state: recomputed,
                seyan4got: facts.seyan4got,
            });
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab4_seyan_data { int cv_co; int cv_serial; int lasttalk;
/// }` (`lab4.c:88-92`): C's function-local `static struct
/// lab4_seyan_data datbuf`, shared across every call - see the module doc
/// comment for why storing it per-character is safe (exactly one Seyan
/// is ever spawned).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab4SeyanDriverData {
    #[serde(default)]
    pub cv_co: Option<CharacterId>,
    #[serde(default)]
    pub cv_serial: u32,
    #[serde(default)]
    pub lasttalk: u64,
}

/// C never parses zone-file args for the seyan (`zones/22/lab4.chr`'s
/// `lab4_seyan` template has no `arg=`, and `lab4_seyan_driver` has no
/// `NT_CREATE` handler at all) - no args to read here, same precedent as
/// `CDR_LAB2HERALD`.
pub fn apply_lab4_seyan_create_message(character: &mut Character) {
    character.driver_state = Some(CharacterDriverState::Lab4Seyan(
        Lab4SeyanDriverData::default(),
    ));
}
