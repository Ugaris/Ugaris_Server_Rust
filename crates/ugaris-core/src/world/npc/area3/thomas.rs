//! Thomas NPC (`CDR_THOMAS`), the crypt entrance guard who unlocks
//! `sir_jones_driver`'s quest chain once a player is high enough level.
//!
//! Ports `src/area/3/area3.c::thomas_driver` (`:1677-1823`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:100-311`, ported as
//! [`AREA3_QA`] in `world::npc::area3`). Follows the same `World`/
//! `PlayerRuntime` split established by `world::yoakin`: the caller
//! supplies a per-player fact snapshot ([`ThomasPlayerFacts`]) up front
//! and applies the returned [`ThomasOutcomeEvent`]s afterwards, since
//! `area3_ppd.crypt_state` lives on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! `thomas_driver` only ever reads/writes `crypt_state` `0`/`1` - the
//! rest of the crypt quest chain (states `1`-`15`) belongs to
//! `world::sir_jones`, the NPC standing just inside the door Thomas
//! guards.
//!
//! Deviations/gaps (documented, not silent):
//! - No self-defense/regen/spell-self cascade exists in C's `thomas_
//!   driver` body at all (matching `world::astro1`'s identical
//!   observation for area 3's other "pure talker" NPCs) - this port
//!   omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:1822`) is
//!   not ported, matching the established `world::yoakin`/`world::
//!   camhermit` precedent for stationary dialogue NPCs: it only throttles
//!   how often the C driver itself re-runs, which has no player-visible
//!   effect in this codebase's message-driven tick architecture.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:1731`).
const THOMAS_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const THOMAS_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:1714`).
const THOMAS_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:1719`, `:1763`).
const THOMAS_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:1816`): idle "return to post" threshold.
const THOMAS_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level > 18` (`area3.c:1742`): the minimum level for Thomas to
/// wave a player inside.
const THOMAS_MIN_LEVEL: u32 = 18;

/// Per-player facts [`World::process_thomas_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThomasPlayerFacts {
    /// `PlayerRuntime::area3_crypt_state()`.
    pub crypt_state: i32,
    /// `ch[co].level`, needed for the `> 18` greeting gate.
    pub level: u32,
}

/// A side effect [`World::process_thomas_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThomasOutcomeEvent {
    /// Write the new `area3_ppd.crypt_state` back.
    UpdateCryptState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `thomas_driver`'s per-tick body (`area3.c:1682-1823`).
    pub fn process_thomas_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ThomasPlayerFacts>,
        area_id: u16,
    ) -> Vec<ThomasOutcomeEvent> {
        let thomas_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_THOMAS
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for thomas_id in thomas_ids {
            self.process_thomas_messages(thomas_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_thomas_messages(
        &mut self,
        thomas_id: CharacterId,
        player_facts: &HashMap<CharacterId, ThomasPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ThomasOutcomeEvent>,
    ) {
        let Some(thomas_name) = self
            .characters
            .get(&thomas_id)
            .map(|thomas| thomas.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Thomas(mut data)) = self
            .characters
            .get(&thomas_id)
            .and_then(|thomas| thomas.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&thomas_id)
            .map(|thomas| std::mem::take(&mut thomas.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.thomas_handle_char_message(
                    thomas_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.thomas_handle_text_message(
                    thomas_id,
                    &thomas_name,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.thomas_handle_give_message(thomas_id, message),
                _ => {}
            }
        }

        if let Some(thomas) = self.characters.get_mut(&thomas_id) {
            thomas.driver_state = Some(CharacterDriverState::Thomas(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:1812-1814`).
        if let (Some(thomas), Some((tx, ty))) =
            (self.characters.get(&thomas_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(thomas.x), i32::from(thomas.y), tx, ty) {
                if let Some(thomas_mut) = self.characters.get_mut(&thomas_id) {
                    let _ = turn(thomas_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`area3.c:1816-1820`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::yoakin` already uses.
        let last_talk = if let Some(thomas) = self.characters.get(&thomas_id) {
            match thomas.driver_state.as_ref() {
                Some(CharacterDriverState::Thomas(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + THOMAS_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(thomas) = self.characters.get(&thomas_id) else {
                return;
            };
            let (post_x, post_y) = (thomas.rest_x, thomas.rest_y);
            self.secure_move_driver(
                thomas_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `thomas_driver`'s `NT_CHAR` branch (`area3.c:1698-1757`).
    fn thomas_handle_char_message(
        &mut self,
        thomas_id: CharacterId,
        data: &mut ThomasDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ThomasPlayerFacts>,
        events: &mut Vec<ThomasOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(thomas) = self.characters.get(&thomas_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:1701-1705`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:1707-1711`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:1713-1717`).
        if tick < data.last_talk + THOMAS_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:1719-1722`).
        if tick < data.last_talk + THOMAS_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:1724-1728`).
        if thomas_id == player_id || !char_see_char(&thomas, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:1730-1734`).
        if char_dist(&thomas, &player) > THOMAS_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->crypt_state) { case 0: if (ch[co].level > 18) {
        // ... ppd->crypt_state++; didsay = 1; } break; case 1: break; }`
        // (`area3.c:1740-1750`).
        if facts.crypt_state == 0 && facts.level > THOMAS_MIN_LEVEL {
            self.npc_quiet_say(
                thomas_id,
                &format!(
                    "Be greeted, {}. Please go inside, my master wishes to talk to thee.",
                    player.name
                ),
            );
            events.push(ThomasOutcomeEvent::UpdateCryptState {
                player_id,
                new_state: 1,
            });
            didsay = true;
        }
        // `crypt_state == 1` and any other value: no-op, matching C's
        // empty `case 1: break;`.

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:1751-1755`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `thomas_driver`'s `NT_TEXT` branch (`area3.c:1760-1785`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::yoakin`'s text handler).
    fn thomas_handle_text_message(
        &mut self,
        thomas_id: CharacterId,
        thomas_name: &str,
        data: &mut ThomasDriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<ThomasOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:1763-1765`).
        let tick = self.tick.0;
        if tick > data.last_talk + THOMAS_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:1767-1770`).
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
        if thomas_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(thomas) = self.characters.get(&thomas_id).cloned() else {
            return;
        };
        if char_dist(&thomas, &speaker) > THOMAS_QA_DISTANCE
            || !char_see_char(&thomas, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, thomas_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(thomas_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`area3.c:1773-1778`).
            TextAnalysisOutcome::Matched(2) => {
                events.push(ThomasOutcomeEvent::UpdateCryptState {
                    player_id: speaker_id,
                    new_state: 0,
                });
                didsay = true;
            }
            // Every other matched code is unhandled by thomas's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:1780-1784`) - note this does *not* touch
        // `dat->last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `thomas_driver`'s `NT_GIVE` branch (`area3.c:1788-1798`): Thomas
    /// never keeps anything handed to him.
    fn thomas_handle_give_message(
        &mut self,
        thomas_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&thomas_id)
            .and_then(|thomas| thomas.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            thomas_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_THOMAS;

/// C `struct thomas_driver_data` (`src/area/3/area3.c:1677-1680`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ThomasDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
