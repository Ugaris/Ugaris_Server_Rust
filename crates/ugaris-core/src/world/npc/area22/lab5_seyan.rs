//! Lab 5 "Laros" (`CDR_LAB5SEYAN`), the Seyan'Du quest giver who sends
//! players after the three Master Demons' heads and opens the gate to
//! Aston once all three are handed in.
//!
//! Ports `src/area/22/lab5.c::lab5_seyan_driver` (`:273-514`). All of the
//! dialogue state (`ppd->seyanstate`/`seyangot`) lives in the player's
//! own `DRD_LAB5_PLAYER` PPD slot (`crate::player::PlayerRuntime::
//! lab5_seyan_state`/`lab5_seyan_got` - a distinct slot from
//! `DRD_LAB_PPD`/`lab_ppd`, same precedent as `world::npc::area22::
//! lab4_seyan`'s own `lab4_seyan_state`/`_got`), so following the same
//! split, the caller supplies a per-player fact snapshot
//! ([`Lab5SeyanPlayerFacts`]) up front and applies the returned
//! [`Lab5SeyanOutcomeEvent`]s afterwards. `dat->cv_co`/`cv_serial`/
//! `lasttalk` (C's `static struct lab5_talk_data datbuf` - a function-
//! local static, i.e. actually shared across every character that ever
//! calls this driver, not per-NPC `set_data` state) is stored as this
//! NPC's own [`Lab5SeyanDriverData`]; since exactly one Laros is ever
//! spawned (`zones/22/lab5.chr`), storing it per-character is observably
//! identical to C's single shared static.
//!
//! The three demon heads (`IID_LAB5_HEAD1`/`_HEAD2`/`_HEAD3`) are
//! ordinary `item="lab5_headN"` equipment carried by `lab5_one_master`/
//! `lab5_two_master`/`lab5_three_master` (`zones/22/lab5.chr`) - they
//! drop through the already-ported generic on-death item-drop mechanic,
//! no scripted reward call needed. `lab5_daemon_driver`'s own
//! immortal-toggle (`world::npc::area22::lab5_daemon`) is what actually
//! makes killing a master demon possible.
//!
//! Deviations/gaps (documented, not silent):
//! - `standard_message_driver(cn, msg, 0, 0)` (`lab5.c:499`), called
//!   unconditionally after every message this driver already handled
//!   explicitly, is not reproduced - same precedent (and same reasoning)
//!   as `lab4_seyan`'s own module doc comment.
//! - The wandering "return to post" tail (`secure_move_driver(cn,
//!   ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN, ret, lastact)`) reuses
//!   `rest_x`/`rest_y` for `tmpx`/`tmpy`, same substitution as every
//!   other stationary NPC in this file.
//! - `has_potion` (`lab5.c:245-259`, case 3's "please deposit your
//!   potions" gate) is pure `World` logic over the seen player's own
//!   `Character::inventory`/`cursor_item` - no `PlayerRuntime` needed,
//!   see [`World::lab5_seyan_has_potion`].
//! - `struct lab5_player_data`'s remaining fields (`magegot`/`magestate`/
//!   `ritualdaemon`/`ritualstate`, used by `lab5_mage_driver` and
//!   `IDR_LAB5_ITEM`'s nameplate/realnameplate/entrance branches) are not
//!   yet ported.

use crate::drvlib::offset2dx;
use crate::item_driver::{IDR_POTION, IID_LAB5_HEAD1, IID_LAB5_HEAD2, IID_LAB5_HEAD3};
use crate::player::lab5_seyan_state_from_got;
use crate::world::*;
use std::collections::HashMap;

/// C `char_dist(cn, co) > 10` (`lab5.c:351`): the `NT_CHAR` greeting
/// range.
const LAB5_SEYAN_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`lab5.c:343`): only talk once the previous line's
/// cooldown has passed.
const LAB5_SEYAN_TALK_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 30` (`lab5.c:507`): idle "return to post" threshold.
const LAB5_SEYAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `seyangot` bit flags (`lab5.c:82`).
const GOT_HEAD1: u8 = 1 << 0;
const GOT_HEAD2: u8 = 1 << 1;
const GOT_HEAD3: u8 = 1 << 2;

/// Per-player facts [`World::process_lab5_seyan_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab5SeyanPlayerFacts {
    /// `PlayerRuntime::lab5_seyan_state`.
    pub seyanstate: u8,
    /// `PlayerRuntime::lab5_seyan_got`.
    pub seyangot: u8,
}

/// A side effect [`World::process_lab5_seyan_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab5SeyanOutcomeEvent {
    /// Write the new `ppd->seyanstate`/`seyangot` back. Both fields are
    /// always sent together since C's own `set_seyan_state` always
    /// recomputes `seyanstate` from the current `seyangot` bits.
    SetPlayerData {
        player_id: CharacterId,
        seyanstate: u8,
        seyangot: u8,
    },
}

impl World {
    /// C `lab5_seyan_driver`'s per-tick body (`lab5.c:273-514`).
    pub fn process_lab5_seyan_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab5SeyanPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab5SeyanOutcomeEvent> {
        let seyan_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB5SEYAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for seyan_id in seyan_ids {
            self.process_lab5_seyan_messages(seyan_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab5_seyan_messages(
        &mut self,
        seyan_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab5SeyanPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab5SeyanOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab5Seyan(mut data)) = self
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
                NT_GIVE => self.lab5_seyan_handle_give_message(
                    seyan_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                NT_CHAR => self.lab5_seyan_handle_char_message(
                    seyan_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => {
                    self.lab5_seyan_handle_text_message(seyan_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(seyan) = self.characters.get_mut(&seyan_id) {
            seyan.driver_state = Some(CharacterDriverState::Lab5Seyan(data.clone()));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`lab5.c:503-505`).
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
        // ret, lastact)) return; } do_idle(cn, TICKS);` (`lab5.c:507-513`).
        if data.lasttalk + LAB5_SEYAN_RETURN_TO_POST_TICKS < self.tick.0 {
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

    /// C `lab5_seyan_driver`'s `NT_GIVE` branch (`lab5.c:286-329`).
    fn lab5_seyan_handle_give_message(
        &mut self,
        seyan_id: CharacterId,
        data: &mut Lab5SeyanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab5SeyanPlayerFacts>,
        events: &mut Vec<Lab5SeyanOutcomeEvent>,
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
                Some(id) if id == IID_LAB5_HEAD1 => Some(GOT_HEAD1),
                Some(id) if id == IID_LAB5_HEAD2 => Some(GOT_HEAD2),
                Some(id) if id == IID_LAB5_HEAD3 => Some(GOT_HEAD3),
                _ => None,
            };
            if let (Some(bit), Some(facts)) = (bit, player_facts.get(&giver_id)) {
                let new_got = facts.seyangot | bit;
                events.push(Lab5SeyanOutcomeEvent::SetPlayerData {
                    player_id: giver_id,
                    seyanstate: lab5_seyan_state_from_got(new_got),
                    seyangot: new_got,
                });
                // C `if (dat->cv_co && (dat->cv_co != co || ch[dat->cv_co]
                // .serial != dat->cv_serial)) say(cn, "%s, please be
                // patient while I'm talking to others.", ch[co].name);`
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
                                "{giver_name}, please be patient while I'm talking to others."
                            ),
                        );
                    }
                }
            }
        }

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;` - unconditional.
        self.destroy_item(item_id);
    }

    /// C `lab5_seyan_driver`'s `NT_CHAR` branch (`lab5.c:331-465`): the
    /// greeting/dialogue state machine, keyed off the seen player's own
    /// `ppd->seyanstate`.
    fn lab5_seyan_handle_char_message(
        &mut self,
        seyan_id: CharacterId,
        data: &mut Lab5SeyanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab5SeyanPlayerFacts>,
        events: &mut Vec<Lab5SeyanOutcomeEvent>,
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
        // }` (`lab5.c:335-338`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`lab5.c:339-342`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->lasttalk + 5 * TICKS) { remove_message;
        // continue; }` (`lab5.c:343-346`).
        if tick < data.lasttalk + LAB5_SEYAN_TALK_COOLDOWN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`lab5.c:347-350`).
        if seyan_id == player_id || !char_see_char(&seyan, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`lab5.c:351-354`).
        if char_dist(&seyan, &player) > LAB5_SEYAN_GREET_DISTANCE {
            return;
        }

        // C `lab5.c:356-362`: drop the current victim if it's no longer
        // valid.
        if let Some(cv_co) = data.cv_co {
            let still_valid = self.characters.get(&cv_co).is_some_and(|cv| {
                cv.serial == data.cv_serial
                    && char_dist(&seyan, cv) <= LAB5_SEYAN_GREET_DISTANCE
                    && char_see_char(&seyan, cv, &self.map, self.date.daylight)
            });
            if !still_valid {
                data.cv_co = None;
            }
        }

        // C `lab5.c:364-368`: only talk to the current victim.
        if let Some(cv_co) = data.cv_co {
            if cv_co != player_id {
                return;
            }
        }

        // C `lab5.c:370-374`: set new victim.
        if data.cv_co.is_none() {
            data.cv_co = Some(player_id);
            data.cv_serial = player.serial;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.seyanstate;
        let mut clear_cv = false;

        match facts.seyanstate {
            0 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Hello {}. I am here to introduce thee to the quest that has to be done \
                         here.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 1;
            }
            1 => {
                self.npc_say(
                    seyan_id,
                    "There are three Demons controlling this Labyrinth. Your mission is \
                     extremely simple: Destroy them. To prove their death, bring me their \
                     heads. Then thou art worthy to enter the next Gate.",
                );
                didsay = true;
                new_state = 2;
            }
            2 => {
                self.npc_say(
                    seyan_id,
                    "But I have to tell thee, that thou shouldst not carry any healing or mana \
                     potions, nor a combo potion with thee when entering here. If thou hast \
                     some, please deposit them in thine depot at the Gatekeeper's.",
                );
                didsay = true;
                new_state = 3;
            }
            3 => {
                if self.lab5_seyan_has_potion(&player) {
                    clear_cv = true;
                } else {
                    self.npc_say(
                        seyan_id,
                        &format!("Go ahead now, {}, and fulfil thine destiny.", player.name),
                    );
                    didsay = true;
                    new_state = 4;
                }
            }
            4 => {
                self.npc_say(
                    seyan_id,
                    &format!(
                        "Ah, and {}, thou mightst find a friend of mine here. Listen carefully \
                         to his advice.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 5;
            }
            5 => {
                clear_cv = true;
            }
            10 => {
                if matches!(facts.seyangot, 1 | 2 | 4) {
                    self.npc_say(seyan_id, &format!("Very well done, {}.", player.name));
                } else if matches!(facts.seyangot, 3 | 5 | 6) {
                    self.npc_say(seyan_id, &format!("I'm impressed, {}.", player.name));
                }
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
                        "{}, thou broughtst me the three Demon's heads and proved thine worth.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 21;
            }
            21 => {
                self.npc_say(
                    seyan_id,
                    "Now I will open a magic gate for thee. Use it, and thou wilt be able to \
                     travel to the next part of the Labyrinth.",
                );
                didsay = true;
                new_state = 22;
            }
            22 => {
                self.queue_lab_exit_spawn(player_id, 15);
                self.npc_say(
                    seyan_id,
                    &format!("Mayest thou pass the last gate, {}", player.name),
                );
                didsay = true;
                new_state = 23;
            }
            23 => {
                clear_cv = true;
            }
            _ => {}
        }

        if new_state != facts.seyanstate {
            events.push(Lab5SeyanOutcomeEvent::SetPlayerData {
                player_id,
                seyanstate: new_state,
                seyangot: facts.seyangot,
            });
        }
        if clear_cv {
            data.cv_co = None;
        }

        // C `if (didsay) { dat->lasttalk = ticker; talkdir =
        // offset2dx(...); }` (`lab5.c:461-464`).
        if didsay {
            data.lasttalk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `lab5_seyan_driver`'s `NT_TEXT` branch (`lab5.c:467-497`): both
    /// the generic `tabunga` god-mode debug echo and the "REPEAT" keyword
    /// recompute.
    fn lab5_seyan_handle_text_message(
        &mut self,
        seyan_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab5SeyanPlayerFacts>,
        events: &mut Vec<Lab5SeyanOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `lab5.c:471`: `tabunga(cn, co, (char *)msg->dat2)`.
        self.apply_tabunga_text_notification(seyan_id, speaker_id, text);

        // C `lab5.c:473-496`.
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
            let recomputed = lab5_seyan_state_from_got(facts.seyangot);
            events.push(Lab5SeyanOutcomeEvent::SetPlayerData {
                player_id: speaker_id,
                seyanstate: recomputed,
                seyangot: facts.seyangot,
            });
        }
    }

    /// C `has_potion` (`lab5.c:245-259`): scans the player's backpack
    /// (slots 30..INVENTORYSIZE) plus cursor item for any `IDR_POTION`
    /// item. Pure `World` logic - see module doc comment.
    fn lab5_seyan_has_potion(&self, player: &Character) -> bool {
        let carries_potion = |item_id: &ItemId| {
            self.items
                .get(item_id)
                .is_some_and(|item| item.driver == IDR_POTION)
        };
        player
            .inventory
            .iter()
            .skip(30)
            .flatten()
            .any(carries_potion)
            || player.cursor_item.as_ref().is_some_and(carries_potion)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab5_talk_data { int cv_co; int cv_serial; int lasttalk; }`
/// (`lab5.c:92-96`): C's function-local `static struct lab5_talk_data
/// datbuf`, shared across every call - see the module doc comment for
/// why storing it per-character is safe (exactly one Laros is ever
/// spawned).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab5SeyanDriverData {
    #[serde(default)]
    pub cv_co: Option<CharacterId>,
    #[serde(default)]
    pub cv_serial: u32,
    #[serde(default)]
    pub lasttalk: u64,
}

/// C never parses zone-file args for Laros (`zones/22/lab5.chr`'s
/// `lab5_seyan` template has no `arg=`, and `lab5_seyan_driver` has no
/// `NT_CREATE` handler at all) - no args to read here, same precedent as
/// `CDR_LAB4SEYAN`.
pub fn apply_lab5_seyan_create_message(character: &mut Character) {
    character.driver_state = Some(CharacterDriverState::Lab5Seyan(
        Lab5SeyanDriverData::default(),
    ));
}
