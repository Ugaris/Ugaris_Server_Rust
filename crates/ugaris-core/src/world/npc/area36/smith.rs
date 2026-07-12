//! Smith NPC (`CDR_CALIGARSMITH`), the dwarf blacksmith south west of the
//! Caligar library, who forges the three palace key parts into the
//! underground key for 5,000 gold and later sells a hand-written
//! translation dictionary for 10,000 gold (once Arkhata's monk has vouched
//! for his dwarven ancestry).
//!
//! Ports `src/area/36/caligar.c::smith_driver` (`:966-1192`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-212`, ported as
//! [`super::AREA36_QA`] in `world::npc::area36`).
//!
//! Like [`super::glori`]/[`super::arquin`], `smith_driver` keeps **no
//! NPC-local state at all** - `ch[cn].clan_serial` is reused as the same
//! raw 10-tick "pause facing the speaker" countdown, mutated directly on
//! `Character`; no outcome event needed for it.
//!
//! Smith's nine-state (`0`-`8`) chain has one fallthrough point (`case 0`
//! into `case 1`'s price-quote line, landing on state `2`) and one gate
//! (`case 2`, Arkhata's monk vouching for his ancestry, `:1033`) that falls
//! into `case 3`'s "This monk named Johnatan..." line, landing on state
//! `4` - both collapsed the same way as [`super::glori`]. Unlike every
//! other Caligar dialogue NPC in this file, the actual key-forging/
//! dictionary-selling transactions happen entirely in `NT_TEXT`'s "yes
//! okay"/"pay 10000g" answer codes (`3`/`5`, matching `smith_driver`'s own
//! price-quote lines in states `1`/`7`), gated on live `has_item`/`gold`
//! checks that are **independent of `smith_state`** - reproduced verbatim,
//! including the fact that a player who has never spoken to the smith at
//! all can still forge a key by walking up and saying "yes okay" while
//! holding the three parts and enough gold.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives `smith_last_talk`, not
//!   `ticker` - `now: i32` is threaded in the same way as
//!   `world::npc::area36::caligar_guard`'s own `now` parameter.
//! - Neither the forge nor the dictionary purchase speaks a success line
//!   at all in C (`caligar.c:1123-1147`/`1151-1171`) - only the failure
//!   branches ("cannot pay"/"bug #.../please report it"/"no space in
//!   inventory") speak. Reproduced verbatim: a successful purchase is
//!   silent except for the item/gold change itself.
//! - `NT_GIVE` is the plain "give back whatever we're still holding, or
//!   destroy it if the giver's inventory is full" boilerplate every
//!   dialogue-only NPC in this file repeats.
//! - C's per-message `remove_message(cn, msg)` calls have no equivalent
//!   here - the per-tick `driver_messages` drain already empties the
//!   queue exactly once per tick.
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not reachable in
//!   this driver, matching the established precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_CALIGARKEYP1, IID_CALIGARKEYP2, IID_CALIGARKEYP3};
use crate::world::*;

use super::AREA36_QA;

/// C `char_dist(cn, co) > 10` (`caligar.c:999`/`:1106`).
const CALIGAR_SMITH_DISTANCE: i32 = 10;
/// C `realtime - ppd->smith_last_talk < 4` (`caligar.c:1010`).
const CALIGAR_SMITH_TALK_COOLDOWN_SECONDS: i32 = 4;
/// C `ch[cn].clan_serial = 10` (`caligar.c:1079`).
const CALIGAR_SMITH_TALK_STALL_TICKS: u32 = 10;
/// C `arkhata_ppd::monk_state > 20` (`caligar.c:1033`/`:1153`).
const CALIGAR_SMITH_MONK_STATE_GATE: i32 = 20;
/// C `5000 * 100` (`caligar.c:1126`): the underground-key forging fee.
const CALIGAR_SMITH_KEY_FORGE_GOLD: u32 = 5000 * 100;
/// C `10000 * 100` (`caligar.c:1156`): the dictionary purchase price.
const CALIGAR_SMITH_DICTIONARY_GOLD: u32 = 10_000 * 100;

/// Per-player facts [`World::process_caligar_smith_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. `has_item`/
/// `gold` checks are resolved directly via `World` instead, since items
/// and gold live there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarSmithPlayerFacts {
    /// `PlayerRuntime::caligar_smith_state()`.
    pub smith_state: i32,
    /// `PlayerRuntime::caligar_smith_last_talk()`.
    pub smith_last_talk: i32,
    /// `PlayerRuntime::caligar_glori_state()` (`case 0`'s `ppd->
    /// glori_state == 16` gate, `caligar.c:1017`).
    pub glori_state: i32,
    /// `PlayerRuntime::arkhata_monk_state()` (`appd->monk_state`,
    /// `caligar.c:1033`/`:1153`).
    pub arkhata_monk_state: i32,
}

/// A side effect [`World::process_caligar_smith_actions`] could not apply
/// directly because it touches `PlayerRuntime`, or because it needs the
/// zone loader's item-template table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarSmithOutcomeEvent {
    /// C `ppd->smith_state++; ppd->smith_last_talk = realtime;`.
    AdvanceSmithTalk {
        player_id: CharacterId,
        new_state: i32,
        realtime_seconds: i32,
    },
    /// C `case 2:` (`analyse_text_driver` code `2`): resets back to the
    /// start of whichever mini-block is in progress (`caligar.c:1112-
    /// 1121`).
    ResetSmithMiniBlock { player_id: CharacterId },
    /// C `case 3:`'s successful path (`caligar.c:1123-1143`):
    /// `create_item("caligar_underground_key")` + `give_char_item`, then
    /// (only on success) destroy the three key parts and deduct 5,000
    /// gold. The `has_item`/gold pre-checks already passed in `World`
    /// before this was pushed.
    ForgeUndergroundKey {
        smith_id: CharacterId,
        player_id: CharacterId,
    },
    /// C `case 5:`'s successful path (`caligar.c:1159-1170`):
    /// `create_item("dictionary")` + `give_char_item`, then (only on
    /// success) deduct 10,000 gold. The monk-state/gold pre-checks already
    /// passed in `World` before this was pushed.
    PurchaseDictionary {
        smith_id: CharacterId,
        player_id: CharacterId,
    },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_CALIGARSMITH`
    /// characters (C `ch_driver`'s `CDR_CALIGARSMITH` case,
    /// `caligar.c:1869-1871`).
    pub fn process_caligar_smith_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarSmithPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CaligarSmithOutcomeEvent> {
        let smith_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARSMITH
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for smith_id in smith_ids {
            self.process_caligar_smith_messages(smith_id, player_facts, now, &mut events);
            self.caligar_smith_stall_or_move(smith_id, area_id);
        }
        events
    }

    fn process_caligar_smith_messages(
        &mut self,
        smith_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarSmithPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarSmithOutcomeEvent>,
    ) {
        let Some(smith_name) = self
            .characters
            .get(&smith_id)
            .map(|smith| smith.name.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&smith_id)
            .map(|smith| std::mem::take(&mut smith.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.caligar_smith_handle_char_message(
                    smith_id,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.caligar_smith_handle_text_message(
                    smith_id,
                    &smith_name,
                    message,
                    player_facts,
                    events,
                ),
                NT_GIVE => self.caligar_smith_handle_give_message(smith_id, message),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`caligar.c:1187-1189`).
        if let (Some(smith), Some((tx, ty))) =
            (self.characters.get(&smith_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(smith.x), i32::from(smith.y), tx, ty) {
                if let Some(smith_mut) = self.characters.get_mut(&smith_id) {
                    let _ = turn(smith_mut, direction as u8);
                }
            }
        }
    }

    /// C `if (ch[cn].clan_serial > 0) ch[cn].clan_serial--; else if
    /// (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
    /// lastact)) return;` (`caligar.c:1181-1185`).
    fn caligar_smith_stall_or_move(&mut self, smith_id: CharacterId, area_id: u16) {
        let Some(smith) = self.characters.get(&smith_id) else {
            return;
        };
        if smith.clan_serial > 0 {
            if let Some(smith_mut) = self.characters.get_mut(&smith_id) {
                smith_mut.clan_serial -= 1;
            }
            return;
        }
        let (post_x, post_y) = (smith.rest_x, smith.rest_y);
        self.secure_move_driver(
            smith_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `smith_driver`'s `NT_CHAR` branch (`caligar.c:977-1081`).
    #[allow(clippy::too_many_arguments)]
    fn caligar_smith_handle_char_message(
        &mut self,
        smith_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarSmithPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarSmithOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(smith) = self.characters.get(&smith_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if player.driver == CDR_LOSTCON {
            return;
        }
        if smith_id == player_id || !char_see_char(&smith, &player, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&smith, &player) > CALIGAR_SMITH_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now - facts.smith_last_talk < CALIGAR_SMITH_TALK_COOLDOWN_SECONDS {
            return;
        }

        let has_key_parts = self.character_has_item_template(player_id, IID_CALIGARKEYP1)
            && self.character_has_item_template(player_id, IID_CALIGARKEYP2)
            && self.character_has_item_template(player_id, IID_CALIGARKEYP3);

        // C `switch (ppd->smith_state) { ... }` (`caligar.c:1015-1076`).
        // The `case 0`/`case 2` fallthroughs are collapsed into their
        // landing state's line - see the module doc comment.
        let outcome: Option<(&str, i32)> = match facts.smith_state {
            0 if facts.glori_state == 16 && has_key_parts => Some((
                "Hello there. I hear you need a key made. Well, for a small fee of 5000 gold I would be more than willing to do it. Yes, Okay / No, not today",
                2,
            )),
            1 => Some((
                "Hello there. I hear you need a key made. Well, for a small fee of 5000 gold I would be more than willing to do it. Yes, Okay / No, not today",
                2,
            )),
            2 if facts.arkhata_monk_state > CALIGAR_SMITH_MONK_STATE_GATE => Some((
                "This monk named Johnatan is wiser than he should be. He is correct. I do descend from the dwarfs, more closely related to the frawds.",
                4,
            )),
            3 => Some((
                "This monk named Johnatan is wiser than he should be. He is correct. I do descend from the dwarfs, more closely related to the frawds.",
                4,
            )),
            4 => Some((
                "I don't know much of my family's history, but I know we lived up in the mountains for a long time.",
                5,
            )),
            5 => Some((
                "My father still lives up there somewhere. Some of us have later on built a life amongst you humans, and learned your ways and language.",
                6,
            )),
            6 => Some((
                "I even compiled a dictionary to help learning your common tounge, it should be most helpful to translate that book.",
                7,
            )),
            7 => Some((
                "Well I forged you a key for 5000g, a hand written dictionary like this must be worth at least the double. So pay 10000g must be a fair price don't you think?",
                8,
            )),
            _ => None,
        };

        let Some((line, new_state)) = outcome else {
            return;
        };

        self.npc_quiet_say(smith_id, line);
        events.push(CaligarSmithOutcomeEvent::AdvanceSmithTalk {
            player_id,
            new_state,
            realtime_seconds: now,
        });

        *face_target = Some((i32::from(player.x), i32::from(player.y)));
        if let Some(smith_mut) = self.characters.get_mut(&smith_id) {
            smith_mut.clan_serial = CALIGAR_SMITH_TALK_STALL_TICKS;
        }
    }

    /// C `smith_driver`'s `NT_TEXT` branch (`caligar.c:1095-1173`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn caligar_smith_handle_text_message(
        &mut self,
        smith_id: CharacterId,
        smith_name: &str,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarSmithPlayerFacts>,
        events: &mut Vec<CaligarSmithOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(smith) = self.characters.get(&smith_id).cloned() else {
            return;
        };
        if smith_id == speaker_id || !char_see_char(&smith, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&smith, &speaker) > CALIGAR_SMITH_DISTANCE {
            return;
        }

        match analyse_text_qa(text, smith_name, &speaker.name, AREA36_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(smith_id, &reply);
            }
            // "repeat"/"restart" (`caligar.c:1111-1122`).
            TextAnalysisOutcome::Matched(2) => {
                events.push(CaligarSmithOutcomeEvent::ResetSmithMiniBlock {
                    player_id: speaker_id,
                });
            }
            // "yes okay" (`caligar.c:1123-1147`): forge the underground
            // key from the three key parts for 5,000 gold.
            TextAnalysisOutcome::Matched(3) => {
                let has_key_parts = self.character_has_item_template(speaker_id, IID_CALIGARKEYP1)
                    && self.character_has_item_template(speaker_id, IID_CALIGARKEYP2)
                    && self.character_has_item_template(speaker_id, IID_CALIGARKEYP3);
                if !has_key_parts {
                    self.npc_quiet_say(
                        smith_id,
                        "You do not appear to have all the neccessary parts.",
                    );
                    return;
                }
                if speaker.gold < CALIGAR_SMITH_KEY_FORGE_GOLD {
                    self.npc_quiet_say(smith_id, "Sorry, it seems you cannot pay me.");
                    return;
                }
                events.push(CaligarSmithOutcomeEvent::ForgeUndergroundKey {
                    smith_id,
                    player_id: speaker_id,
                });
            }
            // "no not today" (`caligar.c:1148-1150`).
            TextAnalysisOutcome::Matched(4) => {
                self.npc_quiet_say(smith_id, "Okay, come back if you change your mind.");
            }
            // "pay 10000g" (`caligar.c:1151-1171`): sell the dictionary
            // once Arkhata's monk has vouched for the smith's ancestry.
            TextAnalysisOutcome::Matched(5) => {
                let Some(facts) = player_facts.get(&speaker_id) else {
                    return;
                };
                if facts.arkhata_monk_state < CALIGAR_SMITH_MONK_STATE_GATE {
                    return;
                }
                if speaker.gold < CALIGAR_SMITH_DICTIONARY_GOLD {
                    self.npc_quiet_say(smith_id, "Sorry, it seems you cannot pay me.");
                    return;
                }
                events.push(CaligarSmithOutcomeEvent::PurchaseDictionary {
                    smith_id,
                    player_id: speaker_id,
                });
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `smith_driver`'s `NT_GIVE` branch (`caligar.c:1083-1092`).
    fn caligar_smith_handle_give_message(
        &mut self,
        smith_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&smith_id)
            .and_then(|smith| smith.cursor_item.take())
        else {
            return;
        };
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARSMITH;
