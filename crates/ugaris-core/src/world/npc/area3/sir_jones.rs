//! Sir Jones NPC (`CDR_SIRJONES`), the crypt-quest giver standing just
//! inside the door `world::thomas` guards.
//!
//! Ports `src/area/3/area3.c::sir_jones_driver` (`:1825-2065`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:100-311`, ported as
//! [`AREA3_QA`] in `world::npc::area3`, same table `world::thomas`
//! shares). Follows the same `World`/`PlayerRuntime` split established by
//! `world::yoakin`/`world::thomas`: the caller supplies a per-player fact
//! snapshot ([`SirJonesPlayerFacts`]) up front and applies the returned
//! [`SirJonesOutcomeEvent`]s afterwards, since `area3_ppd.crypt_state`/
//! `crypt_bonus` and the `QLOG` 18/19 quest-log entries live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `sir_jones_driver`'s sixteen-state (`0`-`15`) dialogue chain is the
//! crypt quest's main body: greeting -> "Aye"/"Nay" choice (optionally
//! sweetened to a 25-gold offer) -> "go slay the toughest creature" ->
//! (external: a player kills `CDR_VAMPIRE`, `world_events::death_hooks::
//! apply_vampire_death_from_hurt_event` completes quest 18 and sets
//! `crypt_state = 10`) -> "well done" + optional gold reward -> "an even
//! tougher creature" -> (external: a player kills `CDR_VAMPIRE2`,
//! `apply_vampire2_death_from_hurt_event` completes quest 19 and sets
//! `crypt_state = 15`).
//!
//! Deviations/gaps (documented, not silent):
//! - C's `case 10` falls through into `case 11`'s body with no
//!   intervening `break` (`area3.c:1952-1954`, `// fall through
//!   intended`) - a single `NT_CHAR` visit at `crypt_state == 10`
//!   double-increments to `12` (via `11`) and speaks only the `case 10`
//!   line; `case 11` itself has no text. Reproduced verbatim by jumping
//!   straight to `new_state = 12` from the `crypt_state == 10` branch.
//! - C's `case 12`/`case 13` bodies call `say()` but never set `didsay =
//!   1` (unlike every other talking case in this driver) - so those two
//!   lines are spoken without refreshing `dat->last_talk`/
//!   `dat->current_victim`/`talkdir`. Reproduced verbatim: the `12`
//!   (quest-19-not-yet-done sub-branch) and `13` transitions push their
//!   dialogue/event side effects but leave `face_target`/`data.
//!   current_victim`/`data.last_talk` untouched, exactly like every other
//!   "no-op" `crypt_state` case.
//! - No self-defense/regen/spell-self cascade exists in C's `sir_jones_
//!   driver` body at all (matching `world::astro1`'s identical
//!   observation for area 3's other "pure talker" NPCs) - this port
//!   omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:2064`) is
//!   not ported, matching the established `world::yoakin`/`world::
//!   thomas` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::quest::quest_exp::MONEY_AREA3_VAMPIRE1;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:1879`).
const SIRJONES_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const SIRJONES_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:1862`).
const SIRJONES_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:1867`, `:1986`).
const SIRJONES_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:2058`): idle "return to post" threshold.
const SIRJONES_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `questlog_open(co, 18)` (`area3.c:1896`).
const QLOG_SIRJONES_VAMPIRE1: usize = 18;
/// C `questlog_open(co, 19)` (`area3.c:1961`).
const QLOG_SIRJONES_VAMPIRE2: usize = 19;

/// Per-player facts [`World::process_sir_jones_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SirJonesPlayerFacts {
    /// `PlayerRuntime::area3_crypt_state()`.
    pub crypt_state: i32,
    /// `PlayerRuntime::area3_crypt_bonus()`.
    pub crypt_bonus: i32,
    /// `PlayerRuntime::quest_log.count(QLOG_SIRJONES_VAMPIRE1)` (C
    /// `questlog_count(co, 18)`).
    pub quest18_count: u8,
    /// `PlayerRuntime::quest_log.is_done(QLOG_SIRJONES_VAMPIRE2)` (C
    /// `questlog_isdone(co, 19)`).
    pub quest19_done: bool,
}

/// A side effect [`World::process_sir_jones_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SirJonesOutcomeEvent {
    /// Write the new `area3_ppd.crypt_state` back.
    UpdateCryptState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->crypt_bonus = 1;` (`area3.c:2012`).
    SetCryptBonus { player_id: CharacterId },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `create_money_item(MONEY_AREA3_VAMPIRE1)` + plain
    /// `give_char_item(co, in)` (`area3.c:1946-1951`) - unlike `world::
    /// yoakin`'s bear-tooth reward (`give_char_item_smart`'s auto-gold-
    /// convert `IF_MONEY` branch), C calls the *plain* `give_char_item`
    /// here, so the reward stays a literal carried money item instead of
    /// an instant gold credit; the caller needs `ZoneLoader::
    /// instantiate_item_template`, which `World` has no access to (same
    /// precedent as `LogainOutcomeEvent::QuestDone`'s mad-knight-quest
    /// reward in `ugaris-server`'s `area1.rs`).
    GoldEarned { player_id: CharacterId, amount: u32 },
}

impl World {
    /// C `sir_jones_driver`'s per-tick body (`area3.c:1830-2065`).
    pub fn process_sir_jones_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, SirJonesPlayerFacts>,
        area_id: u16,
    ) -> Vec<SirJonesOutcomeEvent> {
        let sir_jones_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SIRJONES
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for sir_jones_id in sir_jones_ids {
            self.process_sir_jones_messages(sir_jones_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_sir_jones_messages(
        &mut self,
        sir_jones_id: CharacterId,
        player_facts: &HashMap<CharacterId, SirJonesPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SirJonesOutcomeEvent>,
    ) {
        let Some(sir_jones_name) = self
            .characters
            .get(&sir_jones_id)
            .map(|sir_jones| sir_jones.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::SirJones(mut data)) = self
            .characters
            .get(&sir_jones_id)
            .and_then(|sir_jones| sir_jones.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&sir_jones_id)
            .map(|sir_jones| std::mem::take(&mut sir_jones.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.sir_jones_handle_char_message(
                    sir_jones_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.sir_jones_handle_text_message(
                    sir_jones_id,
                    &sir_jones_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.sir_jones_handle_give_message(sir_jones_id, message),
                _ => {}
            }
        }

        if let Some(sir_jones) = self.characters.get_mut(&sir_jones_id) {
            sir_jones.driver_state = Some(CharacterDriverState::SirJones(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:2054-2056`).
        if let (Some(sir_jones), Some((tx, ty))) =
            (self.characters.get(&sir_jones_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(sir_jones.x), i32::from(sir_jones.y), tx, ty)
            {
                if let Some(sir_jones_mut) = self.characters.get_mut(&sir_jones_id) {
                    let _ = turn(sir_jones_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`area3.c:2058-2062`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::yoakin`/`world::thomas` already use.
        let last_talk = if let Some(sir_jones) = self.characters.get(&sir_jones_id) {
            match sir_jones.driver_state.as_ref() {
                Some(CharacterDriverState::SirJones(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + SIRJONES_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(sir_jones) = self.characters.get(&sir_jones_id) else {
                return;
            };
            let (post_x, post_y) = (sir_jones.rest_x, sir_jones.rest_y);
            self.secure_move_driver(
                sir_jones_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `sir_jones_driver`'s `NT_CHAR` branch (`area3.c:1846-1980`).
    fn sir_jones_handle_char_message(
        &mut self,
        sir_jones_id: CharacterId,
        data: &mut SirJonesDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SirJonesPlayerFacts>,
        events: &mut Vec<SirJonesOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(sir_jones) = self.characters.get(&sir_jones_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:1849-1853`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:1855-1859`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:1861-1865`).
        if tick < data.last_talk + SIRJONES_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:1867-1870`).
        if tick < data.last_talk + SIRJONES_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:1872-1876`).
        if sir_jones_id == player_id
            || !char_see_char(&sir_jones, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:1878-1882`).
        if char_dist(&sir_jones, &player) > SIRJONES_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.crypt_state;
        match facts.crypt_state {
            // C `case 0: break;` (`area3.c:1892-1893`).
            0 => {}
            // C `case 1:` (`area3.c:1894-1899`).
            1 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    &format!("Welcome to my humble home, {}.", player.name),
                );
                events.push(SirJonesOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_SIRJONES_VAMPIRE1,
                });
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`area3.c:1900-1907`).
            2 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    &format!(
                        "Thou lookst like a tough {}. I guess thou wouldst be interested to hear about a fabulous opportunity.",
                        if player.flags.contains(CharacterFlags::WARRIOR) {
                            "warrior"
                        } else {
                            "mage"
                        }
                    ),
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`area3.c:1908-1913`).
            3 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    "One of my clerks found hints about a huge crypt located below the Aston graveyard. Being the coward he is, he rejected to explore it himself, though.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`area3.c:1914-1919`).
            4 => {
                self.npc_quiet_say_bytes(
                    sir_jones_id,
                    &format!(
                        "Would thou be willing to go? Say {COL_STR_LIGHT_BLUE}Aye{COL_STR_RESET} or {COL_STR_LIGHT_BLUE}Nay{COL_STR_RESET}!"
                    ),
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break; // waiting for answer` (`area3.c:1920-1921`).
            5 => {}
            // C `case 6:` (`area3.c:1922-1929`).
            6 => {
                self.npc_quiet_say_bytes(
                    sir_jones_id,
                    &format!(
                        "And if I offered thee 25 gold pieces as reward? {COL_STR_LIGHT_BLUE}Aye{COL_STR_RESET} or {COL_STR_LIGHT_BLUE}Nay{COL_STR_RESET}, {}!",
                        player.name
                    ),
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7: break; // waiting for answer` (`area3.c:1930-1931`).
            7 => {}
            // C `case 8:` (`area3.c:1932-1939`).
            8 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    &format!(
                        "Jolly good. I expect thee to slay the toughest creature thou canst find down there. Have a nice day, {}.",
                        player.name
                    ),
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9: break; // waiting for player to solve quest`
            // (`area3.c:1940-1941`).
            9 => {}
            // C `case 10:` falls through into `case 11`'s bare increment
            // with no intervening `break` (`area3.c:1942-1954`) - see the
            // module doc comment's deviation note.
            10 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    &format!(
                        "It seems thou foundst quite a challenge down there. Well done, {}.",
                        player.name
                    ),
                );
                if facts.crypt_bonus != 0 && facts.quest18_count == 1 {
                    events.push(SirJonesOutcomeEvent::GoldEarned {
                        player_id,
                        amount: MONEY_AREA3_VAMPIRE1.max(0) as u32,
                    });
                }
                new_state = 12;
                didsay = true;
            }
            // C `case 12:` (`area3.c:1955-1963`) - the `say()` here never
            // sets `didsay = 1` in C; reproduced verbatim (see the module
            // doc comment's deviation note).
            12 => {
                if facts.quest19_done {
                    new_state = 14;
                } else {
                    self.npc_quiet_say(
                        sir_jones_id,
                        &format!(
                            "I have heard rumors that there is an even tougher creature down there, {}.",
                            player.name
                        ),
                    );
                    events.push(SirJonesOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_SIRJONES_VAMPIRE2,
                    });
                    new_state = 13;
                }
            }
            // C `case 13:` (`area3.c:1964-1968`) - same "`say()` without
            // `didsay = 1`" quirk as `case 12`'s non-done sub-branch.
            13 => {
                self.npc_quiet_say(
                    sir_jones_id,
                    "I don't believe in these rumors, but it is said that thou canst gain entry to its lair by walking through the wall in the western corner of the Vampire Lords room.",
                );
                new_state = 14;
            }
            // C `case 14: break; // waiting for quest to be done`
            // (`area3.c:1969-1970`).
            14 => {}
            // C `case 15: break; // quest is done` (`area3.c:1971-1972`).
            15 => {}
            _ => {}
        }

        if new_state != facts.crypt_state {
            events.push(SirJonesOutcomeEvent::UpdateCryptState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:1974-1978`). Not touched
        // for `case 12`'s non-done sub-branch or `case 13` - see the
        // module doc comment.
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `sir_jones_driver`'s `NT_TEXT` branch (`area3.c:1983-2027`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::thomas`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn sir_jones_handle_text_message(
        &mut self,
        sir_jones_id: CharacterId,
        sir_jones_name: &str,
        data: &mut SirJonesDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SirJonesPlayerFacts>,
        events: &mut Vec<SirJonesOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:1986-1988`).
        let tick = self.tick.0;
        if tick > data.last_talk + SIRJONES_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:1990-1993`).
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

        // C `analyse_text_driver`'s own guard clauses (`area3.c:223-238`):
        // ignore our own talk, non-players, distance > 12, not-visible.
        if sir_jones_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(sir_jones) = self.characters.get(&sir_jones_id).cloned() else {
            return;
        };
        if char_dist(&sir_jones, &speaker) > SIRJONES_QA_DISTANCE
            || !char_see_char(&sir_jones, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let crypt_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.crypt_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, sir_jones_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(sir_jones_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) (`area3.c:1995-2004`).
            TextAnalysisOutcome::Matched(2) => {
                let new_state = if (1..=5).contains(&crypt_state) {
                    Some(1)
                } else if (12..=14).contains(&crypt_state) {
                    Some(12)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    events.push(SirJonesOutcomeEvent::UpdateCryptState {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (aye) (`area3.c:2005-2014`).
            TextAnalysisOutcome::Matched(3) => {
                if (1..=5).contains(&crypt_state) {
                    events.push(SirJonesOutcomeEvent::UpdateCryptState {
                        player_id: speaker_id,
                        new_state: 8,
                    });
                } else if (6..=7).contains(&crypt_state) {
                    events.push(SirJonesOutcomeEvent::UpdateCryptState {
                        player_id: speaker_id,
                        new_state: 8,
                    });
                    events.push(SirJonesOutcomeEvent::SetCryptBonus {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // C `case 4:` (nay) (`area3.c:2015-2020`).
            TextAnalysisOutcome::Matched(4) => {
                if (1..=5).contains(&crypt_state) {
                    events.push(SirJonesOutcomeEvent::UpdateCryptState {
                        player_id: speaker_id,
                        new_state: 6,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by sir_jones's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:2022-2026`) - note this does *not* touch
        // `dat->last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `sir_jones_driver`'s `NT_GIVE` branch (`area3.c:2029-2040`): Sir
    /// Jones never keeps anything handed to him.
    fn sir_jones_handle_give_message(
        &mut self,
        sir_jones_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&sir_jones_id)
            .and_then(|sir_jones| sir_jones.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            sir_jones_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_SIRJONES;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};

/// C `struct sir_jones_driver_data` (`src/area/3/area3.c:1825-1828`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SirJonesDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
