//! Lab 2 graveyard chapel keeper (`CDR_LAB2HERALD`, "Herald"), the quest
//! giver who sends players after Arathas and, once his ring is turned in,
//! opens the gate out of the graveyard level.
//!
//! Ports `src/area/22/lab2.c::lab2_herald_driver` (`:78-348`). All of the
//! dialogue state (`ppd->herald_talkstep`) lives in the *player's*
//! `struct lab_ppd` (`crate::player::PlayerRuntime::lab_ppd`, a byte blob
//! `World` cannot see), so following the same split already established
//! for `world::gatekeeper`'s `GateWelcomePlayerFacts`/
//! `GateWelcomeOutcomeEvent`, the caller supplies a per-player fact
//! snapshot ([`Lab2HeraldPlayerFacts`]) up front and applies the returned
//! [`Lab2HeraldOutcomeEvent`]s afterwards. `dat->last_talk`/`dat->
//! next_talk` (the herald's own cooldown state, shared across every
//! player it talks to - C's `dat` is per-*herald*, not per-player) is the
//! NPC's own [`Lab2HeraldDriverData`].
//!
//! Deviations/gaps (documented, not silent):
//! - `standard_message_driver(cn, msg, 0, 0)` (`lab2.c:333`), called
//!   unconditionally after every message this driver already handled
//!   explicitly, is not reproduced: with `agressive=0, helper=0` its
//!   `NT_CHAR`/`NT_SEEHIT` cases are dead code and its `NT_GOTHIT` case
//!   only calls `fight_driver_note_hit(cn)` (a hit-timestamp this driver
//!   never reads, since the herald never fights) - never observably
//!   different, same precedent as every other non-fighting NPC's own
//!   module doc comment.
//! - The wandering "return to post" tail (`secure_move_driver(cn,
//!   ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN, ret, lastact)`) reuses
//!   `rest_x`/`rest_y` for `tmpx`/`tmpy`, the same substitution every
//!   other stationary NPC in this file uses.
use crate::drvlib::offset2dx;
use crate::item_driver::IID_LAB2_ARATHASRING;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;
use std::collections::HashMap;

/// C `char_dist(cn, co) > 10` (`lab2.c:150`): the `NT_CHAR` greeting range.
const LAB2_HERALD_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 30` (`lab2.c:341`): idle "return to post" threshold.
const LAB2_HERALD_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_lab2_herald_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab2HeraldPlayerFacts {
    /// `PlayerRuntime::legacy_lab2_herald_talkstep()`.
    pub herald_talkstep: u8,
}

/// A side effect [`World::process_lab2_herald_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab2HeraldOutcomeEvent {
    /// Write the new `ppd->herald_talkstep` back.
    UpdateTalkstep {
        player_id: CharacterId,
        new_value: u8,
    },
}

impl World {
    /// C `lab2_herald_driver`'s per-tick body (`lab2.c:78-348`).
    pub fn process_lab2_herald_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab2HeraldPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab2HeraldOutcomeEvent> {
        let herald_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB2HERALD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for herald_id in herald_ids {
            self.process_lab2_herald_messages(herald_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab2_herald_messages(
        &mut self,
        herald_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab2HeraldPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab2HeraldOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab2Herald(mut data)) = self
            .characters
            .get(&herald_id)
            .and_then(|herald| herald.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&herald_id)
            .map(|herald| std::mem::take(&mut herald.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_TEXT => self.lab2_herald_handle_text_message(
                    herald_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                NT_GIVE => {
                    self.lab2_herald_handle_give_message(herald_id, message, player_facts, events)
                }
                NT_CHAR => self.lab2_herald_handle_char_message(
                    herald_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(herald) = self.characters.get_mut(&herald_id) {
            herald.driver_state = Some(CharacterDriverState::Lab2Herald(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`lab2.c:337-339`).
        if let (Some(herald), Some((tx, ty))) =
            (self.characters.get(&herald_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(herald.x), i32::from(herald.y), tx, ty) {
                if let Some(herald_mut) = self.characters.get_mut(&herald_id) {
                    let _ = turn(herald_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; } do_idle(cn, TICKS);` (`lab2.c:341-347`).
        let last_talk = match self.characters.get(&herald_id) {
            Some(herald) => match herald.driver_state.as_ref() {
                Some(CharacterDriverState::Lab2Herald(data)) => data.last_talk,
                _ => return,
            },
            None => return,
        };
        if last_talk + LAB2_HERALD_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(herald) = self.characters.get(&herald_id) else {
                return;
            };
            let (post_x, post_y) = (herald.rest_x, herald.rest_y);
            self.secure_move_driver(
                herald_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `lab2_herald_driver`'s `NT_TEXT` branches (`lab2.c:98-101,287-
    /// 331`): both the generic `tabunga` god-mode debug echo and the
    /// keyword-driven talkstep jump fire for the same message.
    fn lab2_herald_handle_text_message(
        &mut self,
        herald_id: CharacterId,
        data: &mut Lab2HeraldDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab2HeraldPlayerFacts>,
        events: &mut Vec<Lab2HeraldOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `lab2.c:98-101`: `tabunga(cn, co, (char *)msg->dat2)`.
        self.apply_tabunga_text_notification(herald_id, speaker_id, text);

        // C `lab2.c:287-331`: the keyword-driven talkstep jump.
        if speaker_id == herald_id {
            return;
        }
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(herald) = self.characters.get(&herald_id).cloned() else {
            return;
        };
        if !char_see_char(&herald, &speaker, &self.map, self.date.daylight) {
            return;
        }
        if !player_facts.contains_key(&speaker_id) {
            return;
        }

        let upper = text.to_ascii_uppercase();
        if upper.contains("ARATHAS") {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 10,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND / 2;
        } else if upper.contains("ELIAS") {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 20,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND / 2;
        } else if upper.contains("FAMILY") && upper.contains("VAULT") {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 30,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND / 2;
        } else if upper.contains("ADMINISTRATIVE") && upper.contains("BUILDING") {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 40,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND / 2;
        } else if upper.contains("DIARY") {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 50,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND / 2;
        } else if upper.contains("REPEAT") {
            self.npc_say(herald_id, &format!("I will repeat, {}", speaker.name));
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: speaker_id,
                new_value: 0,
            });
            data.next_talk = self.tick.0 + TICKS_PER_SECOND;
        }
    }

    /// C `lab2_herald_driver`'s `NT_GIVE` branch (`lab2.c:103-120`).
    fn lab2_herald_handle_give_message(
        &mut self,
        herald_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab2HeraldPlayerFacts>,
        events: &mut Vec<Lab2HeraldOutcomeEvent>,
    ) {
        // C `if (!ch[cn].citem) { remove_message; continue; }`.
        let Some(item_id) = self
            .characters
            .get(&herald_id)
            .and_then(|herald| herald.cursor_item)
        else {
            return;
        };

        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let is_ring = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.template_id == IID_LAB2_ARATHASRING);
        let giver_is_player = self
            .characters
            .get(&giver_id)
            .is_some_and(|giver| giver.flags.contains(CharacterFlags::PLAYER));

        if giver_is_player && is_ring && player_facts.contains_key(&giver_id) {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id: giver_id,
                new_value: 60,
            });
        }

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;` - unconditional,
        // regardless of what was handed over.
        self.destroy_item(item_id);
    }

    /// C `lab2_herald_driver`'s `NT_CHAR` branch (`lab2.c:122-285`): the
    /// greeting/dialogue state machine, keyed off the seen player's own
    /// `ppd->herald_talkstep`.
    fn lab2_herald_handle_char_message(
        &mut self,
        herald_id: CharacterId,
        data: &mut Lab2HeraldDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab2HeraldPlayerFacts>,
        events: &mut Vec<Lab2HeraldOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(herald) = self.characters.get(&herald_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue; }`
        // (`lab2.c:126-129`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue; }`
        // (`lab2.c:132-135`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->next_talk) { remove_message; continue; }`
        // (`lab2.c:138-141`).
        if tick < data.next_talk {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`lab2.c:144-147`).
        if herald_id == player_id || !char_see_char(&herald, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`lab2.c:150-153`).
        if char_dist(&herald, &player) > LAB2_HERALD_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_talkstep = facts.herald_talkstep;
        let mut next_talk_delta = 0u64;

        match facts.herald_talkstep {
            0 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "Hello {}. I am Herald, the Keeper of this graveyard. I can't say how \
                         glad I am to see thee here. I assume thou wantst to pass this test. I \
                         need thine help urgently. Horrible things happen here, as thou hast \
                         probably noticed. I don't dare to leave the chapel since the dead are \
                         rising from their graves. {COL_STR_LIGHT_BLUE}Arathas{COL_STR_RESET} \
                         has caused this abomination, may his soul rest in peace.",
                        player.name
                    ),
                );
                didsay = true;
                new_talkstep = 1;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            1 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "If thou wishest to help me, kill {COL_STR_LIGHT_BLUE}Arathas\
                         {COL_STR_RESET}. Bring me his ring as proof, and I will open thee a \
                         gate leading out of this part of the Labyrinth."
                    ),
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            10 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "I don't know much about him. I just started to read what \
                         {COL_STR_LIGHT_BLUE}Elias{COL_STR_RESET}, his brother, wrote in his \
                         {COL_STR_LIGHT_BLUE}diary{COL_STR_RESET} when the skeletons attacked me \
                         in my study. I'm lucky I got out alive. Now I'm hiding here in the \
                         chapel. The undeads dare not enter it."
                    ),
                );
                didsay = true;
                new_talkstep = 11;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            11 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "But thou wished to hear about Arathas. He and his brother stem from a \
                         family of well renowed mages. They have their own \
                         {COL_STR_LIGHT_BLUE}family vault{COL_STR_RESET} on this graveyard. \
                         Arathas died during some kind of magical experiment, and he was buried \
                         in the family vault a long time ago."
                    ),
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            20 => {
                self.npc_say(
                    herald_id,
                    "Elias is Arathas' brother. The last times I saw him, he seemed very \
                     frightened. He spoke about strange happenings in the crypt. Now we know \
                     what he was talking about.",
                );
                didsay = true;
                new_talkstep = 21;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            21 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "One day he entered the family vault. But he did not return, and after \
                         a while, his relatives divided his belongings among them. By now, they \
                         are all dead, too, and rest in this graveyard. If things were \
                         different, I'd show thee their graves, but with the undeads about I do \
                         not dare. Thou couldst check the books yourself, for the locations of \
                         their tombs. They are in the {COL_STR_LIGHT_BLUE}administrative \
                         building{COL_STR_RESET}. But beware, lots of skeletons and undeads are \
                         there, too."
                    ),
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 10;
            }
            30 => {
                self.npc_say(
                    herald_id,
                    "The family vault is located northeast of the chapel.",
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 5;
            }
            40 => {
                self.npc_say(
                    herald_id,
                    "The administrative building is located northwest of the chapel. All \
                     administrative records about the graveyard are stored there.",
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 5;
            }
            50 => {
                self.npc_say_bytes(
                    herald_id,
                    &format!(
                        "I left the diary in my rooms, in the north-eastern part of the \
                         {COL_STR_LIGHT_BLUE}administrative building{COL_STR_RESET}. I left it \
                         there, in my study, when I fled from the undeads."
                    ),
                );
                didsay = true;
                new_talkstep = 255;
                next_talk_delta = TICKS_PER_SECOND * 5;
            }
            60 => {
                self.npc_say(
                    herald_id,
                    &format!(
                        "I thank thee, {}. This ring proves that thou hast killed Arathas. I \
                         hope peace will return now.",
                        player.name
                    ),
                );
                didsay = true;
                new_talkstep = 61;
                next_talk_delta = TICKS_PER_SECOND * 5;
            }
            61 => {
                self.npc_say(
                    herald_id,
                    &format!(
                        "And here, my friend, is the Gate, as I have promised thee. Thou hast \
                         been most resourceful, {}.",
                        player.name
                    ),
                );
                didsay = true;
                new_talkstep = 62;
                next_talk_delta = TICKS_PER_SECOND * 5;
            }
            62 => {
                self.npc_say(
                    herald_id,
                    &format!("Mayest thou pass the last gate, {}", player.name),
                );
                didsay = true;
                new_talkstep = 255;
                self.queue_lab_exit_spawn(player_id, 30);
            }
            _ => {}
        }

        if new_talkstep != facts.herald_talkstep {
            events.push(Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id,
                new_value: new_talkstep,
            });
        }
        if next_talk_delta > 0 {
            data.next_talk = tick + next_talk_delta;
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir =
        // offset2dx(...); }` (`lab2.c:281-284`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab2_herald_driver_data { int last_talk; int next_talk; }`
/// (`lab2.c:73-76`): the herald's own driver memory, shared across every
/// player it talks to (distinct from the per-player `herald_talkstep` in
/// `crate::player::PlayerRuntime::lab_ppd` - see the module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab2HeraldDriverData {
    #[serde(default)]
    pub last_talk: u64,
    #[serde(default)]
    pub next_talk: u64,
}

/// C never parses zone-file args into `struct lab2_herald_driver_data`
/// (`set_data` zero-initializes it, and C's own `NT_CREATE` handler body is
/// empty - `lab2.c:96`) - no args to read here, same precedent as
/// `CDR_CAMHERMIT`/`CDR_YOAKIN` (`crate::zone`'s own doc comments).
pub fn apply_lab2_herald_create_message(character: &mut Character) {
    character.driver_state = Some(CharacterDriverState::Lab2Herald(
        Lab2HeraldDriverData::default(),
    ));
}
