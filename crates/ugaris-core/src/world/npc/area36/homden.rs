//! Homden NPC (`CDR_CALIGARHOMDEN`), the banished Carmin Clan brother
//! sheltering in the forest, who opens quest 59 ("find my stolen ring")
//! once the player shows him the assembled underground key, then narrates
//! the palace/Emperor backstory once the ring is returned.
//!
//! Ports `src/area/36/caligar.c::homden_driver` (`:1194-1395`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-212`, ported as
//! [`super::AREA36_QA`] in `world::npc::area36`).
//!
//! Like [`super::glori`]/[`super::arquin`]/[`super::smith`],
//! `homden_driver` keeps **no NPC-local state at all** - `ch[cn].
//! clan_serial` is reused as the same raw 10-tick "pause facing the
//! speaker" countdown, mutated directly on `Character`; no outcome event
//! needed for it.
//!
//! Homden's twelve-state (`0`-`11`) chain has one fallthrough point
//! (`case 0`, holding the assembled underground key, falls into `case 1`'s
//! "You come seeking my help?..." line plus `questlog_open(co, 59)`,
//! landing on state `2`) - collapsed the same way as [`super::glori`].
//! State `4` ("waiting for the ring") only advances via `NT_GIVE`, not
//! `NT_CHAR` - `homden_driver`'s own `switch` has an explicit `case 4:
//! break;` with no body at all.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives `homden_last_talk`, not
//!   `ticker` - `now: i32` is threaded in the same way as
//!   `world::npc::area36::caligar_guard`'s own `now` parameter.
//! - `NT_GIVE`'s ring check compares `it[in].ID == IID_CALIGARHOMDENRING`,
//!   which is numerically identical to `IID_CALIGARPALACEKEYPART`/
//!   [`crate::item_driver::IID_CALIGAR_PALACE_KEY_PART`] - a genuine C
//!   source duplicate define (see [`crate::item_driver::
//!   IID_CALIGARHOMDENRING`]'s own doc comment) reproduced verbatim: any
//!   item sharing that raw template ID completes the quest while
//!   `homden_state == 4`, not just the actual ring.
//! - Every other item given to Homden (including the ring itself, when
//!   `homden_state != 4`) falls through to the plain "give back whatever
//!   we're still holding, or destroy it if the giver's inventory is full"
//!   boilerplate every dialogue-only NPC in this file repeats.
//! - C's per-message `remove_message(cn, msg)` calls have no equivalent
//!   here - the per-tick `driver_messages` drain already empties the
//!   queue exactly once per tick.
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not reachable in
//!   this driver, matching the established precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_CALIGARDUNGEONKEY, IID_CALIGARHOMDENRING};
use crate::world::*;

use super::AREA36_QA;

/// C `char_dist(cn, co) > 10` (`caligar.c:1226`/`:1358`).
const CALIGAR_HOMDEN_DISTANCE: i32 = 10;
/// C `realtime - ppd->homden_last_talk < 4` (`caligar.c:1237`).
const CALIGAR_HOMDEN_TALK_COOLDOWN_SECONDS: i32 = 4;
/// C `ch[cn].clan_serial = 10` (`caligar.c:1322`).
const CALIGAR_HOMDEN_TALK_STALL_TICKS: u32 = 10;
/// C `ppd->homden_state == 4` (`caligar.c:1331`): the "waiting for the
/// ring" state `NT_GIVE`'s ring check gates on.
const CALIGAR_HOMDEN_WAITING_FOR_RING_STATE: i32 = 4;

/// Per-player facts [`World::process_caligar_homden_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. The dungeon
/// key `has_item` check is resolved directly via `World::
/// character_has_item_template` instead, since items live on `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarHomdenPlayerFacts {
    /// `PlayerRuntime::caligar_homden_state()`.
    pub homden_state: i32,
    /// `PlayerRuntime::caligar_homden_last_talk()`.
    pub homden_last_talk: i32,
}

/// A side effect [`World::process_caligar_homden_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarHomdenOutcomeEvent {
    /// C `ppd->homden_state++; ppd->homden_last_talk = realtime;`.
    AdvanceHomdenTalk {
        player_id: CharacterId,
        new_state: i32,
        realtime_seconds: i32,
    },
    /// C `questlog_open(co, 59)` (`caligar.c:1252`, `case 1`'s body).
    QuestOpen { player_id: CharacterId },
    /// C `case 2:` (`analyse_text_driver` code `2`): resets back to the
    /// start of whichever mini-block is in progress (`caligar.c:1363-
    /// 1374`).
    ResetHomdenMiniBlock { player_id: CharacterId },
    /// C `NT_GIVE`'s ring turn-in (`caligar.c:1331-1336`):
    /// `questlog_done(co, 59)`, `ppd->homden_state = 5`, and the ring is
    /// destroyed (not given back).
    CompleteRingQuest { player_id: CharacterId },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_CALIGARHOMDEN`
    /// characters (C `ch_driver`'s `CDR_CALIGARHOMDEN` case,
    /// `caligar.c:1872-1874`).
    pub fn process_caligar_homden_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarHomdenPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CaligarHomdenOutcomeEvent> {
        let homden_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARHOMDEN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for homden_id in homden_ids {
            self.process_caligar_homden_messages(homden_id, player_facts, now, &mut events);
            self.caligar_homden_stall_or_move(homden_id, area_id);
        }
        events
    }

    fn process_caligar_homden_messages(
        &mut self,
        homden_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarHomdenPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarHomdenOutcomeEvent>,
    ) {
        let Some(homden_name) = self
            .characters
            .get(&homden_id)
            .map(|homden| homden.name.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&homden_id)
            .map(|homden| std::mem::take(&mut homden.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.caligar_homden_handle_char_message(
                    homden_id,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.caligar_homden_handle_text_message(
                    homden_id,
                    &homden_name,
                    message,
                    events,
                ),
                NT_GIVE => self.caligar_homden_handle_give_message(
                    homden_id,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`caligar.c:1390-1392`).
        if let (Some(homden), Some((tx, ty))) =
            (self.characters.get(&homden_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(homden.x), i32::from(homden.y), tx, ty) {
                if let Some(homden_mut) = self.characters.get_mut(&homden_id) {
                    let _ = turn(homden_mut, direction as u8);
                }
            }
        }
    }

    /// C `if (ch[cn].clan_serial > 0) ch[cn].clan_serial--; else if
    /// (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
    /// lastact)) return;` (`caligar.c:1384-1388`).
    fn caligar_homden_stall_or_move(&mut self, homden_id: CharacterId, area_id: u16) {
        let Some(homden) = self.characters.get(&homden_id) else {
            return;
        };
        if homden.clan_serial > 0 {
            if let Some(homden_mut) = self.characters.get_mut(&homden_id) {
                homden_mut.clan_serial -= 1;
            }
            return;
        }
        let (post_x, post_y) = (homden.rest_x, homden.rest_y);
        self.secure_move_driver(
            homden_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `homden_driver`'s `NT_CHAR` branch (`caligar.c:1204-1324`).
    #[allow(clippy::too_many_arguments)]
    fn caligar_homden_handle_char_message(
        &mut self,
        homden_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarHomdenPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarHomdenOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(homden) = self.characters.get(&homden_id).cloned() else {
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
        if homden_id == player_id || !char_see_char(&homden, &player, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&homden, &player) > CALIGAR_HOMDEN_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now - facts.homden_last_talk < CALIGAR_HOMDEN_TALK_COOLDOWN_SECONDS {
            return;
        }

        let has_dungeon_key = self.character_has_item_template(player_id, IID_CALIGARDUNGEONKEY);

        // C `switch (ppd->homden_state) { ... }` (`caligar.c:1242-1319`).
        // The `case 0` fallthrough is collapsed into `case 1`'s line - see
        // the module doc comment. `case 4` has no `NT_CHAR` body at all.
        let outcome: Option<(&str, i32, bool)> = match facts.homden_state {
            0 if has_dungeon_key => Some((
                "You come seeking my help? I'd be glad to help if it means my brothers will be put to a stop. However, I need your help first.",
                2,
                true,
            )),
            1 => Some((
                "You come seeking my help? I'd be glad to help if it means my brothers will be put to a stop. However, I need your help first.",
                2,
                true,
            )),
            2 => Some((
                "A group of amazons keep invading my camp at night, and stealing my personal things. One item, a powerful ring, was taken and I am sure it was them.",
                3,
                false,
            )),
            3 => Some((
                "If you could please go and find it for me while I gather my thoughts on my brothers I would reward thee. There is a cave to the east, start your search there.",
                4,
                false,
            )),
            5 => Some(("Thank you, %s.", 6, false)),
            6 => Some((
                "Now, about my brothers. They are planning to resurrect the last Emporer. If they succeed, they hope to trick the citizens of Aston into thinking that the Emporer has returned and try to restore his royal status.",
                7,
                false,
            )),
            7 => Some((
                "Once that happens, they will slowly begin destroying the town, and have said their first target would be the Labyrinths that Ishtar made to strengthen his army.",
                8,
                false,
            )),
            8 => Some((
                "We can not let this happen. The fate of Astonia depends on those filthy brothers of mine being halted! The door that the key opens can be found down a hole to the north.",
                9,
                false,
            )),
            9 => Some((
                "It's a passage that leads to the palace. There will be three levels in the palace to test you. Not even I am sure how to navigate it. I do know the plaque you seek is locked in a chest on the last floor of the palace.",
                10,
                false,
            )),
            10 => Some((
                "If they do not have that plaque, they cannot raise the Emporer. But, I suggest you hurry. Once their army is complete, they will begin trying to raise the Emporer. Good luck adventurer!",
                11,
                false,
            )),
            _ => None,
        };

        let Some((line, new_state, opens_quest_59)) = outcome else {
            return;
        };

        if line == "Thank you, %s." {
            self.npc_quiet_say(homden_id, &format!("Thank you, {}.", player.name));
        } else {
            self.npc_quiet_say(homden_id, line);
        }
        if opens_quest_59 {
            events.push(CaligarHomdenOutcomeEvent::QuestOpen { player_id });
        }
        events.push(CaligarHomdenOutcomeEvent::AdvanceHomdenTalk {
            player_id,
            new_state,
            realtime_seconds: now,
        });

        *face_target = Some((i32::from(player.x), i32::from(player.y)));
        if let Some(homden_mut) = self.characters.get_mut(&homden_id) {
            homden_mut.clan_serial = CALIGAR_HOMDEN_TALK_STALL_TICKS;
        }
    }

    /// C `homden_driver`'s `NT_TEXT` branch (`caligar.c:1347-1376`), wired
    /// through the generic `analyse_text_qa` matcher.
    fn caligar_homden_handle_text_message(
        &mut self,
        homden_id: CharacterId,
        homden_name: &str,
        message: &CharacterDriverMessage,
        events: &mut Vec<CaligarHomdenOutcomeEvent>,
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
        let Some(homden) = self.characters.get(&homden_id).cloned() else {
            return;
        };
        if homden_id == speaker_id
            || !char_see_char(&homden, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&homden, &speaker) > CALIGAR_HOMDEN_DISTANCE {
            return;
        }

        if let TextAnalysisOutcome::Matched(2) =
            analyse_text_qa(text, homden_name, &speaker.name, AREA36_QA)
        {
            events.push(CaligarHomdenOutcomeEvent::ResetHomdenMiniBlock {
                player_id: speaker_id,
            });
        } else if let TextAnalysisOutcome::Said(reply) =
            analyse_text_qa(text, homden_name, &speaker.name, AREA36_QA)
        {
            self.npc_quiet_say(homden_id, &reply);
        }
    }

    /// C `homden_driver`'s `NT_GIVE` branch (`caligar.c:1327-1344`).
    fn caligar_homden_handle_give_message(
        &mut self,
        homden_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarHomdenPlayerFacts>,
        events: &mut Vec<CaligarHomdenOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&homden_id)
            .and_then(|homden| homden.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
            return;
        };

        let waiting_for_ring = player_facts
            .get(&giver_id)
            .is_some_and(|facts| facts.homden_state == CALIGAR_HOMDEN_WAITING_FOR_RING_STATE);

        if item.template_id == IID_CALIGARHOMDENRING && waiting_for_ring {
            events.push(CaligarHomdenOutcomeEvent::CompleteRingQuest {
                player_id: giver_id,
            });
            self.destroy_item(item_id);
            return;
        }

        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARHOMDEN;
