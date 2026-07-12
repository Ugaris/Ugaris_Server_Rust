//! Arquin NPC (`CDR_CALIGARARQUIN`), stationed just outside the Caligar
//! library, who explains what the training-facility obelisks unlock once
//! Glori is ready for the next step, and points the player at Homden once
//! they have forged the underground key.
//!
//! Ports `src/area/36/caligar.c::arquin_driver` (`:805-964`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-212`, ported as
//! [`super::AREA36_QA`] in `world::npc::area36`).
//!
//! Like [`super::glori`], `arquin_driver` keeps **no NPC-local state at
//! all** - `ch[cn].clan_serial` is reused as the same raw 10-tick "pause
//! facing the speaker" countdown, mutated directly on `Character` since it
//! lives on `World`; no outcome event needed. `arquin_driver` never opens
//! or completes a quest (no `questlog_open`/`questlog_done` calls anywhere
//! in its body), unlike `glori_driver`/`homden_driver`.
//!
//! Arquin's seven-state (`0`-`6`) chain has two fallthrough points where a
//! gate-checked state advances twice in the same tick, only speaking the
//! second (landing) state's line - same collapsing precedent as
//! [`super::glori`]/`world::npc::area31::dwarfchief`:
//! - `case 0` (`ppd->glori_state == 12`, an *exact* equality check, not a
//!   `>=`) falls into `case 1`'s "Obelisks? Well, they may be used..."
//!   line, landing on state `2`.
//! - `case 3` (holding the assembled dungeon key) falls into `case 4`'s
//!   "Aha, I see you have gotten a hold of the key..." line, landing on
//!   state `5`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives `arquin_last_talk`, not
//!   `ticker` - `now: i32` is threaded in the same way as
//!   `world::npc::area36::caligar_guard`'s own `now` parameter.
//! - `NT_GIVE` is the plain "give back whatever we're still holding, or
//!   destroy it if the giver's inventory is full" boilerplate every
//!   dialogue-only NPC in this file repeats - unlike `glori_driver`,
//!   `arquin_driver` has no item-specific hint branch at all.
//! - C's per-message `remove_message(cn, msg)` calls have no equivalent
//!   here - the per-tick `driver_messages` drain (`std::mem::take`)
//!   already empties the queue exactly once per tick.
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not reachable in
//!   this driver, matching the established precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_CALIGARDUNGEONKEY;
use crate::world::*;

use super::AREA36_QA;

/// C `char_dist(cn, co) > 10` (`caligar.c:837`/`:927`).
const CALIGAR_ARQUIN_DISTANCE: i32 = 10;
/// C `realtime - ppd->arquin_last_talk < 4` (`caligar.c:848`).
const CALIGAR_ARQUIN_TALK_COOLDOWN_SECONDS: i32 = 4;
/// C `ch[cn].clan_serial = 10` (`caligar.c:900`).
const CALIGAR_ARQUIN_TALK_STALL_TICKS: u32 = 10;

/// Per-player facts [`World::process_caligar_arquin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. The dungeon
/// key `has_item` check is resolved directly via `World::
/// character_has_item_template` instead, since items live on `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarArquinPlayerFacts {
    /// `PlayerRuntime::caligar_arquin_state()`.
    pub arquin_state: i32,
    /// `PlayerRuntime::caligar_arquin_last_talk()`.
    pub arquin_last_talk: i32,
    /// `PlayerRuntime::caligar_glori_state()` (`case 0`'s `ppd->
    /// glori_state == 12` gate, `caligar.c:855`).
    pub glori_state: i32,
}

/// A side effect [`World::process_caligar_arquin_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarArquinOutcomeEvent {
    /// C `ppd->arquin_state++; ppd->arquin_last_talk = realtime;`.
    AdvanceArquinTalk {
        player_id: CharacterId,
        new_state: i32,
        realtime_seconds: i32,
    },
    /// C `case 2:` (`analyse_text_driver` code `2`): resets back to the
    /// start of whichever mini-block is in progress (`caligar.c:932-943`).
    ResetArquinMiniBlock { player_id: CharacterId },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_CALIGARARQUIN`
    /// characters (C `ch_driver`'s `CDR_CALIGARARQUIN` case,
    /// `caligar.c:1866-1868`).
    pub fn process_caligar_arquin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarArquinPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CaligarArquinOutcomeEvent> {
        let arquin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARARQUIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for arquin_id in arquin_ids {
            self.process_caligar_arquin_messages(arquin_id, player_facts, now, &mut events);
            self.caligar_arquin_stall_or_move(arquin_id, area_id);
        }
        events
    }

    fn process_caligar_arquin_messages(
        &mut self,
        arquin_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarArquinPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarArquinOutcomeEvent>,
    ) {
        let Some(arquin_name) = self
            .characters
            .get(&arquin_id)
            .map(|arquin| arquin.name.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&arquin_id)
            .map(|arquin| std::mem::take(&mut arquin.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.caligar_arquin_handle_char_message(
                    arquin_id,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.caligar_arquin_handle_text_message(
                    arquin_id,
                    &arquin_name,
                    message,
                    events,
                ),
                NT_GIVE => self.caligar_arquin_handle_give_message(arquin_id, message),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`caligar.c:959-961`).
        if let (Some(arquin), Some((tx, ty))) =
            (self.characters.get(&arquin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(arquin.x), i32::from(arquin.y), tx, ty) {
                if let Some(arquin_mut) = self.characters.get_mut(&arquin_id) {
                    let _ = turn(arquin_mut, direction as u8);
                }
            }
        }
    }

    /// C `if (ch[cn].clan_serial > 0) ch[cn].clan_serial--; else if
    /// (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
    /// lastact)) return;` (`caligar.c:953-957`).
    fn caligar_arquin_stall_or_move(&mut self, arquin_id: CharacterId, area_id: u16) {
        let Some(arquin) = self.characters.get(&arquin_id) else {
            return;
        };
        if arquin.clan_serial > 0 {
            if let Some(arquin_mut) = self.characters.get_mut(&arquin_id) {
                arquin_mut.clan_serial -= 1;
            }
            return;
        }
        let (post_x, post_y) = (arquin.rest_x, arquin.rest_y);
        self.secure_move_driver(
            arquin_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `arquin_driver`'s `NT_CHAR` branch (`caligar.c:815-902`).
    #[allow(clippy::too_many_arguments)]
    fn caligar_arquin_handle_char_message(
        &mut self,
        arquin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarArquinPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarArquinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(arquin) = self.characters.get(&arquin_id).cloned() else {
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
        if arquin_id == player_id || !char_see_char(&arquin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&arquin, &player) > CALIGAR_ARQUIN_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now - facts.arquin_last_talk < CALIGAR_ARQUIN_TALK_COOLDOWN_SECONDS {
            return;
        }

        let has_dungeon_key = self.character_has_item_template(player_id, IID_CALIGARDUNGEONKEY);

        // C `switch (ppd->arquin_state) { ... }` (`caligar.c:853-897`). The
        // `case 0`/`case 3` fallthroughs are collapsed into their landing
        // state's line - see the module doc comment.
        let outcome: Option<(&str, i32)> = match facts.arquin_state {
            0 if facts.glori_state == 12 => Some((
                "Obelisks? Well, they may be used as some type of key. Judging by the names of each one I would assume that they open the locked gates inside each training area.",
                2,
            )),
            1 => Some((
                "Obelisks? Well, they may be used as some type of key. Judging by the names of each one I would assume that they open the locked gates inside each training area.",
                2,
            )),
            2 => Some((
                "If you can get into them you can kill the minions being trained there. Bring anything you might find to Glori.",
                3,
            )),
            3 if has_dungeon_key => Some((
                "Aha, I see you have gotten a hold of the key. I know of someone who may be able to tell you what it unlocks. He is a brother of the Carmin Clan, named Homden.",
                5,
            )),
            4 => Some((
                "Aha, I see you have gotten a hold of the key. I know of someone who may be able to tell you what it unlocks. He is a brother of the Carmin Clan, named Homden.",
                5,
            )),
            5 => Some((
                "He was banished from this city by his brothers for not helping them with their plans of destruction, and being that this area is barricaded, he had no choice but to establish shelter in the forest.",
                6,
            )),
            _ => None,
        };

        let Some((line, new_state)) = outcome else {
            return;
        };

        self.npc_quiet_say(arquin_id, line);
        events.push(CaligarArquinOutcomeEvent::AdvanceArquinTalk {
            player_id,
            new_state,
            realtime_seconds: now,
        });

        *face_target = Some((i32::from(player.x), i32::from(player.y)));
        if let Some(arquin_mut) = self.characters.get_mut(&arquin_id) {
            arquin_mut.clan_serial = CALIGAR_ARQUIN_TALK_STALL_TICKS;
        }
    }

    /// C `arquin_driver`'s `NT_TEXT` branch (`caligar.c:916-946`), wired
    /// through the generic `analyse_text_qa` matcher.
    fn caligar_arquin_handle_text_message(
        &mut self,
        arquin_id: CharacterId,
        arquin_name: &str,
        message: &CharacterDriverMessage,
        events: &mut Vec<CaligarArquinOutcomeEvent>,
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
        let Some(arquin) = self.characters.get(&arquin_id).cloned() else {
            return;
        };
        if arquin_id == speaker_id
            || !char_see_char(&arquin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&arquin, &speaker) > CALIGAR_ARQUIN_DISTANCE {
            return;
        }

        match analyse_text_qa(text, arquin_name, &speaker.name, AREA36_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(arquin_id, &reply);
            }
            TextAnalysisOutcome::Matched(2) => {
                events.push(CaligarArquinOutcomeEvent::ResetArquinMiniBlock {
                    player_id: speaker_id,
                });
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `arquin_driver`'s `NT_GIVE` branch (`caligar.c:904-913`).
    fn caligar_arquin_handle_give_message(
        &mut self,
        arquin_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&arquin_id)
            .and_then(|arquin| arquin.cursor_item.take())
        else {
            return;
        };
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARARQUIN;
