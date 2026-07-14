//! Carlos NPC (`CDR_CARLOS`), the Imperial Army's Chief Investigator of
//! the Occult, standing next to the Imperial Vault.
//!
//! Ports `src/area/3/area3.c::carlos_driver` (`:2067-2321`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:100-311`, ported as
//! [`AREA3_QA`] in `world::npc::area3`). Follows the same `World`/
//! `PlayerRuntime` split established by `world::sir_jones`/`world::kelly`:
//! the caller supplies a per-player fact snapshot ([`CarlosPlayerFacts`])
//! up front and applies the returned [`CarlosOutcomeEvent`]s afterwards,
//! since `staffer_ppd.carlos_state`/`carlos2_state` and the `QLOG` 20/61
//! quest-log entries live on `crate::player::PlayerRuntime`, not `World`.
//!
//! Carlos actually runs *two* independent quest chains gated by
//! `questlog_count(co, 61) < 1` (`area3.c:2130`): while quest 61 (the
//! Imperial Vault ritual) has never been completed, every visit drives
//! the `carlos2_state` (`0`-`5`) ritual chain; once quest 61 has been
//! completed at least once, every subsequent visit instead drives the
//! `carlos_state` (`0`-`6`) dragon-staff chain (quest 20, `QLF_REPEATABLE`
//! - the achievement/turn-in fires every time, not just on first
//!   completion, matching C's unconditional `achievement_award` call at
//!   `area3.c:2271`).
//!
//! Deviations/gaps (documented, not silent):
//! - C's `NT_TEXT` "repeat"/"restart" handler (`case 2`, `area3.c:2264-
//!   2270`) has a real quirk: it checks `ppd->carlos2_state >= 0 &&
//!   ppd->carlos_state <= 4` (not `carlos2_state <= 4` as the variable
//!   name would suggest) before resetting `carlos2_state` to `0`, falling
//!   back to resetting `carlos_state` to `0` only when `carlos_state <=
//!   5`. Since `carlos2_state` only ever increments from `0`, `>= 0` is
//!   always true, so the first branch is effectively gated on
//!   `carlos_state <= 4` alone. Reproduced verbatim (not "fixed"): see
//!   [`World::carlos_handle_text_message`].
//! - C's `case 4` (`area3.c:2196-2210`) speaks its main dialogue line
//!   unconditionally, then separately - only if `give_char_item` for the
//!   freshly created `carlos_key` succeeds - speaks a second "thou wilt
//!   need this key" line. Since item creation needs `ZoneLoader`
//!   (unavailable to `World`), that second conditional line is spoken by
//!   `ugaris-server`'s `apply_carlos_events` after a successful grant, not
//!   here (see [`CarlosOutcomeEvent::GrantCarlosKey`]).
//! - No self-defense/regen/spell-self cascade exists in C's `carlos_
//!   driver` body at all (matching every other area-3 "pure talker" NPC's
//!   identical observation) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:2320`) is
//!   not ported, matching the established sibling-driver precedent for
//!   stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, mem_add_driver, mem_check_driver, TextAnalysisOutcome,
};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_CARLOS_DOOR, IID_MAX_CHRONICLES, IID_MAX_RITUAL, IID_STAFF_DRAGONKEY1,
    IID_STAFF_DRAGONKEY2, IID_STAFF_DRAGONKEY3, IID_STAFF_DRAGONKEY4, IID_STAFF_DRAGONSTAFF,
};
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:2119`).
const CARLOS_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const CARLOS_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:2102`).
const CARLOS_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:2107`, `:2226`).
const CARLOS_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:2317`): idle "return to post" threshold.
const CARLOS_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level < 26` (`area3.c:2136`): the ritual-quest greeting gate.
const CARLOS_RITUAL_MIN_LEVEL: u32 = 26;
/// C `questlog_open(co, 61)` (`area3.c:2144`).
const QLOG_CARLOS_RITUAL: usize = 61;
/// C `questlog_open(co, 20)` (`area3.c:2172`).
const QLOG_CARLOS_STAFF: usize = 20;

/// Per-player facts [`World::process_carlos_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarlosPlayerFacts {
    /// `PlayerRuntime::staffer_carlos_state()`.
    pub carlos_state: i32,
    /// `PlayerRuntime::staffer_carlos2_state()`.
    pub carlos2_state: i32,
    /// `ch[co].level` (`area3.c:2136`).
    pub level: u32,
    /// `PlayerRuntime::quest_log.count(QLOG_CARLOS_RITUAL)` (C
    /// `questlog_count(co, 61)`).
    pub quest61_count: u8,
}

/// A side effect [`World::process_carlos_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarlosOutcomeEvent {
    /// Write the new `staffer_ppd.carlos_state` back.
    UpdateCarlosState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// Write the new `staffer_ppd.carlos2_state` back.
    UpdateCarlos2State {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `case 4`'s conditional key grant (`area3.c:2205-2210`):
    /// `create_item("carlos_key")` + `give_char_item`, speaking "Thou
    /// wilt need this key to unlock the door in front of the stairs
    /// down." only on success. The `!has_item` gate is already checked
    /// directly in `World`.
    GrantCarlosKey {
        player_id: CharacterId,
        carlos_id: CharacterId,
    },
    /// C `questlog_done(co, 20);` + unconditional
    /// `achievement_award(co, ACHIEVEMENT_DRAGONSBANE, 1)`
    /// (`area3.c:2266-2267`, `NT_GIVE`).
    DragonStaffQuestDone { player_id: CharacterId },
    /// C `questlog_done(co, 61);` (`area3.c:2280`, `NT_GIVE`) - the exp/
    /// resend half; no achievement or extra reward attached.
    RitualQuestDone { player_id: CharacterId },
}

impl World {
    /// C `carlos_driver`'s per-tick body (`area3.c:2072-2321`).
    pub fn process_carlos_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CarlosPlayerFacts>,
        area_id: u16,
    ) -> Vec<CarlosOutcomeEvent> {
        let carlos_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CARLOS
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for carlos_id in carlos_ids {
            self.process_carlos_messages(carlos_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_carlos_messages(
        &mut self,
        carlos_id: CharacterId,
        player_facts: &HashMap<CharacterId, CarlosPlayerFacts>,
        area_id: u16,
        events: &mut Vec<CarlosOutcomeEvent>,
    ) {
        let Some(carlos_name) = self
            .characters
            .get(&carlos_id)
            .map(|carlos| carlos.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Carlos(mut data)) = self
            .characters
            .get(&carlos_id)
            .and_then(|carlos| carlos.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&carlos_id)
            .map(|carlos| std::mem::take(&mut carlos.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.carlos_handle_char_message(
                    carlos_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.carlos_handle_text_message(
                    carlos_id,
                    &carlos_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.carlos_handle_give_message(carlos_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(carlos) = self.characters.get_mut(&carlos_id) {
            carlos.driver_state = Some(CharacterDriverState::Carlos(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:2313-2315`).
        if let (Some(carlos), Some((tx, ty))) =
            (self.characters.get(&carlos_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(carlos.x), i32::from(carlos.y), tx, ty) {
                if let Some(carlos_mut) = self.characters.get_mut(&carlos_id) {
                    let _ = turn(carlos_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_UP, ret,
        // lastact)) return; }` (`area3.c:2317-2319`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::sir_jones`/`world::kelly` already use.
        let last_talk = if let Some(carlos) = self.characters.get(&carlos_id) {
            match carlos.driver_state.as_ref() {
                Some(CharacterDriverState::Carlos(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + CARLOS_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(carlos) = self.characters.get(&carlos_id) else {
                return;
            };
            let (post_x, post_y) = (carlos.rest_x, carlos.rest_y);
            self.secure_move_driver(
                carlos_id,
                post_x,
                post_y,
                Direction::Up as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `carlos_driver`'s `NT_CHAR` branch (`area3.c:2086-2213`).
    fn carlos_handle_char_message(
        &mut self,
        carlos_id: CharacterId,
        data: &mut CarlosDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CarlosPlayerFacts>,
        events: &mut Vec<CarlosOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(carlos) = self.characters.get(&carlos_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:2089-2093`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:2095-2099`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:2101-2105`).
        if tick < data.last_talk + CARLOS_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:2107-2110`).
        if tick < data.last_talk + CARLOS_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:2112-2116`).
        if carlos_id == player_id || !char_see_char(&carlos, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:2118-2122`).
        if char_dist(&carlos, &player) > CARLOS_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;

        // C `if (questlog_count(co, 61) < 1) { switch
        // (ppd->carlos2_state) ... } else { switch (ppd->carlos_state)
        // ... }` (`area3.c:2130-2213`).
        if facts.quest61_count < 1 {
            let mut new_state = facts.carlos2_state;
            match facts.carlos2_state {
                // C `case 0:` (`area3.c:2135-2151`).
                0 => {
                    if facts.level < CARLOS_RITUAL_MIN_LEVEL {
                        if !mem_check_driver(&carlos.driver_memory, 0, player_id.0) {
                            self.npc_quiet_say(
                                carlos_id,
                                &format!(
                                    "Hello {}. I might have a quest for thee at a later date. Check back when thou hast grown more powerful.",
                                    player.name
                                ),
                            );
                            if let Some(carlos_mut) = self.characters.get_mut(&carlos_id) {
                                mem_add_driver(&mut carlos_mut.driver_memory, 0, player_id.0);
                            }
                        }
                    } else {
                        self.npc_quiet_say(
                            carlos_id,
                            &format!(
                                "Greetings, {}. I am Carlos, Chief Investigator of the Occult in the Imperial Army.",
                                player.name
                            ),
                        );
                        new_state = 1;
                        events.push(CarlosOutcomeEvent::QuestOpen {
                            player_id,
                            quest: QLOG_CARLOS_RITUAL,
                        });
                    }
                    didsay = true;
                }
                // C `case 1:` (`area3.c:2152-2156`).
                1 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "I need your help in aquiring a magical ritual that may aid in preventing any more attacks on the city.",
                    );
                    new_state = 2;
                    didsay = true;
                }
                // C `case 2:` (`area3.c:2157-2161`).
                2 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "The only known copy of this ritual was on a scroll stored in the Imperial Vault. I need you to find and bring the scroll to me.",
                    );
                    new_state = 3;
                    didsay = true;
                }
                // C `case 3:` (`area3.c:2162-2166`).
                3 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "Please report to Rouven at the vault next door.",
                    );
                    new_state = 4;
                    didsay = true;
                }
                // C `case 4: break; // waiting for ritual` (`area3.c:2167-
                // 2168`).
                4 => {}
                // C `case 5: break; // all done` (`area3.c:2169-2170`).
                5 => {}
                _ => {}
            }
            if new_state != facts.carlos2_state {
                events.push(CarlosOutcomeEvent::UpdateCarlos2State {
                    player_id,
                    new_state,
                });
            }
        } else {
            let mut new_state = facts.carlos_state;
            match facts.carlos_state {
                // C `case 0:` (`area3.c:2176-2181`).
                0 => {
                    self.npc_quiet_say(
                        carlos_id,
                        &format!(
                            "Hello again, {}. I have another mission for thee.",
                            player.name
                        ),
                    );
                    new_state = 1;
                    events.push(CarlosOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_CARLOS_STAFF,
                    });
                    didsay = true;
                }
                // C `case 1:` (`area3.c:2182-2187`).
                1 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "I need thy help to investigate a strange noise coming from below the crypt. 200 years ago some strange creatures were found.",
                    );
                    new_state = 2;
                    didsay = true;
                }
                // C `case 2:` (`area3.c:2188-2193`).
                2 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "I sent my top army down to kill theses creatures and take away a special Staff. The staff had magical properties that allowed these creatures to live.",
                    );
                    new_state = 3;
                    didsay = true;
                }
                // C `case 3:` (`area3.c:2194-2199`).
                3 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "My army captured the head creature and brought his staff to me. It has been brought to my attention that the staff has gone missing.",
                    );
                    new_state = 4;
                    didsay = true;
                }
                // C `case 4:` (`area3.c:2200-2211`).
                4 => {
                    self.npc_quiet_say(
                        carlos_id,
                        "Please go below the crypt and find out what the strange noise is and if thou findst the staff bring it back to me.",
                    );
                    new_state = 5;
                    didsay = true;
                    if !self.character_has_item_template(player_id, IID_CARLOS_DOOR) {
                        events.push(CarlosOutcomeEvent::GrantCarlosKey {
                            player_id,
                            carlos_id,
                        });
                    }
                }
                // C `case 5: break; // waiting for staff` (`area3.c:2212-
                // 2213`).
                5 => {}
                // C `case 6: break; // got staff... waiting forever here.`
                // (`area3.c:2214-2215`).
                6 => {}
                _ => {}
            }
            if new_state != facts.carlos_state {
                events.push(CarlosOutcomeEvent::UpdateCarlosState {
                    player_id,
                    new_state,
                });
            }
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:2216-2220`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `carlos_driver`'s `NT_TEXT` branch (`area3.c:2223-2244`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::sir_jones`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn carlos_handle_text_message(
        &mut self,
        carlos_id: CharacterId,
        carlos_name: &str,
        data: &mut CarlosDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CarlosPlayerFacts>,
        events: &mut Vec<CarlosOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:2226-2228`).
        let tick = self.tick.0;
        if tick > data.last_talk + CARLOS_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:2230-2233`).
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
        if carlos_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(carlos) = self.characters.get(&carlos_id).cloned() else {
            return;
        };
        if char_dist(&carlos, &speaker) > CARLOS_QA_DISTANCE
            || !char_see_char(&carlos, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let facts = player_facts
            .get(&speaker_id)
            .copied()
            .unwrap_or(CarlosPlayerFacts {
                carlos_state: 0,
                carlos2_state: 0,
                level: 0,
                quest61_count: 0,
            });

        let mut didsay = false;
        match analyse_text_qa(text, carlos_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(carlos_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) (`area3.c:2246-2255`). The C
            // condition is `ppd->carlos2_state >= 0 && ppd->carlos_state
            // <= 4` - since `carlos2_state` only ever increments from `0`,
            // `>= 0` is always true, so this is effectively gated on
            // `carlos_state <= 4` alone (see the module doc comment's
            // deviation note; reproduced verbatim, not "fixed").
            TextAnalysisOutcome::Matched(2) => {
                if facts.carlos_state <= 4 {
                    events.push(CarlosOutcomeEvent::UpdateCarlos2State {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                } else if facts.carlos_state <= 5 {
                    events.push(CarlosOutcomeEvent::UpdateCarlosState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by carlos's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:2258-2262`) - note this does *not* touch `dat->
        // last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `carlos_driver`'s `NT_GIVE` branch (`area3.c:2277-2308`).
    fn carlos_handle_give_message(
        &mut self,
        carlos_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CarlosPlayerFacts>,
        events: &mut Vec<CarlosOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&carlos_id)
            .and_then(|carlos| carlos.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let Some(giver_name) = self.characters.get(&giver_id).map(|c| c.name.clone()) else {
            self.destroy_item(item_id);
            return;
        };
        // C never checks `ppd` for null here (`set_data` always returns a
        // usable pointer); the defaults below (`<= 5`/`<= 4` both true at
        // `0`) match C's zero-initialized `staffer_ppd` for a never-seen
        // player, same precedent as `world::kelly`'s `NT_GIVE` handler.
        let carlos_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.carlos_state)
            .unwrap_or(0);
        let carlos2_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.carlos2_state)
            .unwrap_or(0);

        if template_id == IID_STAFF_DRAGONSTAFF && carlos_state <= 5 {
            // C `if (ppd && ppd->carlos_state <= 5 && it[in].ID ==
            // IID_STAFF_DRAGONSTAFF) { ppd->carlos_state = 6; quiet_say(
            // cn, "Well done, %s, that is the staff I wanted."); tmp =
            // questlog_done(co, 20); achievement_award(co,
            // ACHIEVEMENT_DRAGONSBANE, 1); destroy_item_byID(co,
            // IID_STAFF_DRAGONSTAFF); destroy_item_byID(co,
            // IID_STAFF_DRAGONKEY1..4); }` (`area3.c:2261-2270`).
            events.push(CarlosOutcomeEvent::UpdateCarlosState {
                player_id: giver_id,
                new_state: 6,
            });
            self.npc_quiet_say(
                carlos_id,
                &format!("Well done, {giver_name}, that is the staff I wanted."),
            );
            events.push(CarlosOutcomeEvent::DragonStaffQuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DRAGONSTAFF);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DRAGONKEY1);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DRAGONKEY2);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DRAGONKEY3);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_DRAGONKEY4);
            self.destroy_item(item_id);
        } else if template_id == IID_MAX_RITUAL && carlos2_state <= 4 {
            // C `} else if (ppd && ppd->carlos2_state <= 4 && it[in].ID ==
            // IID_MAX_RITUAL) { ppd->carlos2_state = 5; quiet_say(cn,
            // "Well done, %s, that is the ritual I wanted."); questlog_
            // done(co, 61); destroy_item_byID(co, IID_MAX_CHRONICLES); }`
            // (`area3.c:2271-2276`).
            events.push(CarlosOutcomeEvent::UpdateCarlos2State {
                player_id: giver_id,
                new_state: 5,
            });
            self.npc_quiet_say(
                carlos_id,
                &format!("Well done, {giver_name}, that is the ritual I wanted."),
            );
            events.push(CarlosOutcomeEvent::RitualQuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_MAX_CHRONICLES);
            self.destroy_item(item_id);
        } else {
            // C `else { say(cn, "Thou hast better use for this than I do.
            // Well, if there is a use for it at all."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`area3.c:2298-2304`).
            self.npc_quiet_say(
                carlos_id,
                "Thou hast better use for this than I do. Well, if there is a use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_CARLOS;

/// C `struct carlos_driver_data` (`src/area/3/area3.c:2067-2070`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CarlosDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
