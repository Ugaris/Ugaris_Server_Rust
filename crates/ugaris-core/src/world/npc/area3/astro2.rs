//! Astro2 NPC (`CDR_ASTRO2`), the astronomer whose lost notes quest
//! (`QLOG` 16, "The Lost Astronomer's Notes") is the payoff for area 2's
//! `IID_AREA2_ASTRONOTE` item.
//!
//! Ports `src/area/3/area3.c::astro2_driver` (`:1493-1675`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:100-311`, ported as
//! [`AREA3_QA`] in `world::npc::area3`, the same table `world::thomas`/
//! `world::sir_jones` share). Follows the same `World`/`PlayerRuntime`
//! split established by `world::thomas`: the caller supplies a per-player
//! fact snapshot ([`Astro2PlayerFacts`]) up front and applies the returned
//! [`Astro2OutcomeEvent`]s afterwards, since `area3_ppd.astro2_state`
//! (and the `QLOG_ASTRO2` quest-log entry) live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `Astro2OutcomeEvent::QuestDone`'s money reward
//!   (`create_money_item(MONEY_AREA3_MOONIES)` + plain `give_char_item`,
//!   `area3.c:1636-1641`) is entirely deferred to `ugaris-server`'s
//!   `apply_astro2_events`, gated on `times_done == 1` (C's `if (tmp ==
//!   1)`) - same `ZoneLoader`-needs-`instantiate_item_template` precedent
//!   as `world::sir_jones`'s `GoldEarned`/`world::logain`'s `QuestDone`.
//! - No self-defense/regen/spell-self cascade exists in C's `astro2_
//!   driver` body at all (matching `world::astro1`/`world::thomas`'s
//!   identical observation for area 3's other "pure talker" NPCs) - this
//!   port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:1674`) is
//!   not ported, matching the established `world::thomas`/`world::
//!   sir_jones` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA2_ASTRONOTE;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:1542`).
const ASTRO2_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const ASTRO2_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:1525`).
const ASTRO2_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:1530`, `:1592`).
const ASTRO2_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:1668`): idle "return to post" threshold.
const ASTRO2_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_astro2_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Astro2PlayerFacts {
    /// `PlayerRuntime::area3_astro2_state()`.
    pub astro2_state: i32,
}

/// A side effect [`World::process_astro2_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Astro2OutcomeEvent {
    /// Write the new `area3_ppd.astro2_state` back.
    UpdateAstro2State {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 16)`.
    QuestOpen { player_id: CharacterId },
    /// C `tmp = questlog_done(co, 16); ... if (tmp == 1) { create_money_
    /// item(MONEY_AREA3_MOONIES) + give_char_item }` (`area3.c:1633-
    /// 1641`) - see the module doc comment for why the money reward
    /// itself needs `ugaris-server`'s `ZoneLoader`.
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `astro2_driver`'s per-tick body (`area3.c:1493-1675`).
    pub fn process_astro2_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Astro2PlayerFacts>,
        area_id: u16,
    ) -> Vec<Astro2OutcomeEvent> {
        let astro2_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ASTRO2
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for astro2_id in astro2_ids {
            self.process_astro2_messages(astro2_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_astro2_messages(
        &mut self,
        astro2_id: CharacterId,
        player_facts: &HashMap<CharacterId, Astro2PlayerFacts>,
        area_id: u16,
        events: &mut Vec<Astro2OutcomeEvent>,
    ) {
        let Some(astro2_name) = self
            .characters
            .get(&astro2_id)
            .map(|astro2| astro2.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Astro2(mut data)) = self
            .characters
            .get(&astro2_id)
            .and_then(|astro2| astro2.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&astro2_id)
            .map(|astro2| std::mem::take(&mut astro2.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.astro2_handle_char_message(
                    astro2_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.astro2_handle_text_message(
                    astro2_id,
                    &astro2_name,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.astro2_handle_give_message(astro2_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(astro2) = self.characters.get_mut(&astro2_id) {
            astro2.driver_state = Some(CharacterDriverState::Astro2(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:1664-1666`).
        if let (Some(astro2), Some((tx, ty))) =
            (self.characters.get(&astro2_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(astro2.x), i32::from(astro2.y), tx, ty) {
                if let Some(astro2_mut) = self.characters.get_mut(&astro2_id) {
                    let _ = turn(astro2_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`area3.c:1668-1672`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::thomas` already uses.
        let last_talk = if let Some(astro2) = self.characters.get(&astro2_id) {
            match astro2.driver_state.as_ref() {
                Some(CharacterDriverState::Astro2(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + ASTRO2_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(astro2) = self.characters.get(&astro2_id) else {
                return;
            };
            let (post_x, post_y) = (astro2.rest_x, astro2.rest_y);
            self.secure_move_driver(
                astro2_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `astro2_driver`'s `NT_CHAR` branch (`area3.c:1508-1586`).
    fn astro2_handle_char_message(
        &mut self,
        astro2_id: CharacterId,
        data: &mut Astro2DriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Astro2PlayerFacts>,
        events: &mut Vec<Astro2OutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(astro2) = self.characters.get(&astro2_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:1512-1516`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:1518-1522`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:1524-1528`).
        if tick < data.last_talk + ASTRO2_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:1530-1533`).
        if tick < data.last_talk + ASTRO2_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:1535-1539`).
        if astro2_id == player_id || !char_see_char(&astro2, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:1541-1545`).
        if char_dist(&astro2, &player) > ASTRO2_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->astro2_state) { case 0: ... case 1: ... case 2:
        // ... case 3: ... case 4: break; case 5: break; }`
        // (`area3.c:1551-1579`).
        match facts.astro2_state {
            0 => {
                self.npc_quiet_say(
                    astro2_id,
                    &format!(
                        "Ah. Hello {}. I am {}, the astronomer.",
                        player.name, astro2.name
                    ),
                );
                events.push(Astro2OutcomeEvent::QuestOpen { player_id });
                events.push(Astro2OutcomeEvent::UpdateAstro2State {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_quiet_say(
                    astro2_id,
                    "Me and my colleagues, we've been watching the moon from our big telescope in the garden south-east of here.",
                );
                events.push(Astro2OutcomeEvent::UpdateAstro2State {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    astro2_id,
                    "But a few days ago, some strange creatures invaded the garden, and drove us away. We had to leave our notes behind.",
                );
                events.push(Astro2OutcomeEvent::UpdateAstro2State {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    astro2_id,
                    "Could thou try to get those notes back? I'd... well, I'd pay thee handsomely!",
                );
                events.push(Astro2OutcomeEvent::UpdateAstro2State {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            // `astro2_state == 4` or `5` (or any other value): no-op,
            // matching C's empty `case 4:`/`case 5: break;`.
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:1580-1584`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `astro2_driver`'s `NT_TEXT` branch (`area3.c:1589-1614`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::thomas`'s text handler).
    fn astro2_handle_text_message(
        &mut self,
        astro2_id: CharacterId,
        astro2_name: &str,
        data: &mut Astro2DriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<Astro2OutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:1592-1594`).
        let tick = self.tick.0;
        if tick > data.last_talk + ASTRO2_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:1596-1599`).
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
        if astro2_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(astro2) = self.characters.get(&astro2_id).cloned() else {
            return;
        };
        if char_dist(&astro2, &speaker) > ASTRO2_QA_DISTANCE
            || !char_see_char(&astro2, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, astro2_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(astro2_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`area3.c:1602-1607`).
            TextAnalysisOutcome::Matched(2) => {
                events.push(Astro2OutcomeEvent::UpdateAstro2State {
                    player_id: speaker_id,
                    new_state: 0,
                });
                didsay = true;
            }
            // Every other matched code is unhandled by astro2's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:1609-1613`) - note this does *not* touch
        // `dat->last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `astro2_driver`'s `NT_GIVE` branch (`area3.c:1617-1650`).
    fn astro2_handle_give_message(
        &mut self,
        astro2_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Astro2PlayerFacts>,
        events: &mut Vec<Astro2OutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&astro2_id)
            .and_then(|astro2| astro2.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        if template_id == IID_AREA2_ASTRONOTE && facts.is_some_and(|facts| facts.astro2_state <= 4)
        {
            // C `if (it[in].ID == IID_AREA2_ASTRONOTE && ppd->astro2_state
            // <= 4) { say("Oh, jolly good! ..."); ppd->astro2_state = 5;
            // destroy_item(ch[cn].citem); ch[cn].citem = 0; tmp =
            // questlog_done(co, 16); destroy_item_byID(co, IID_AREA2_
            // ASTRONOTE); if (tmp == 1) { ... } }` (`area3.c:1624-1641`).
            self.npc_quiet_say(astro2_id, "Oh, jolly good! Thou gotst them back.");
            events.push(Astro2OutcomeEvent::UpdateAstro2State {
                player_id: giver_id,
                new_state: 5,
            });
            self.destroy_item(item_id);
            events.push(Astro2OutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA2_ASTRONOTE);
        } else {
            // C `else { say("Thou hast better use..."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`area3.c:1642-1648`) - the plain `give_char_
            // item`, not `give_char_item_smart`, same documented asymmetry
            // as `world::thomas`'s own `NT_GIVE` handler.
            self.npc_quiet_say(
                astro2_id,
                "Thou hast better use for this than I do. Well, if there is a use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_ASTRO2;

/// C `struct astro2_driver_data` (`src/area/3/area3.c:1488-1491`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Astro2DriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
