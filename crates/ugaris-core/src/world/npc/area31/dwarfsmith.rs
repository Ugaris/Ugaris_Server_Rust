//! Dwarf Smith NPC (`CDR_DWARFSMITH`), Grimroot's blacksmith, who forges a
//! `lizard_elite_keyN` from a `lizard_moldN` plus exactly 5,000 silver
//! units.
//!
//! Ports `src/area/31/warrmines.c::dwarfsmith_driver` (`:723-903`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:70-194`, ported as
//! [`super::AREA31_QA`] in `world::npc::area31`). Follows the same
//! `World`/`PlayerRuntime` split as `world::npc::area31::dwarfchief`: the
//! caller supplies a per-player fact snapshot ([`DwarfsmithPlayerFacts`])
//! up front and applies the returned [`DwarfsmithOutcomeEvent`]s
//! afterwards, since `staffer_ppd.dwarfsmith_state`/`dwarfsmith_type` live
//! on `crate::player::PlayerRuntime`, not `World`.
//!
//! `dwarfsmith_driver` has no multi-mini-quest chain (no `questlog_open`/
//! `questlog_done` calls at all) - its three states are just "not yet
//! greeted" (`0`), "waiting for a mold" (`1`), and "waiting for exactly
//! 5,000 silver" (`2`). `NT_GIVE` handles the actual item exchange: giving
//! a `IID_LIZARDMOLD` while `state <= 1` remembers its `drdata[0]` variant
//! byte as `dwarfsmith_type` and advances to `2`; giving an `IDR_ENHANCE`
//! silver stack with `drdata[0] == 1` and exactly `5000` units while
//! `state == 2` forges and hands out the matching `lizard_elite_keyN` and
//! resets to `1` (type cleared).
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `warrmines.c` driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim.
//! - C's `case 2:` (`repeat`/`restart`) branch of the `NT_TEXT` switch is
//!   entirely commented out in the source (`:808-809`) - `analyse_text_
//!   driver`'s matched-code-`2` result is intentionally a silent no-op for
//!   this driver (still counts as `didsay`), unlike every other
//!   `warrmines.c`/`brannington.c` NPC which resets a dialogue state on
//!   it.
//! - C `case 3:` (`:810-815`) speaks a visible `say(cn, "reset done")`
//!   line before wiping only `dwarfsmith_state` to `0` (not
//!   `dwarfsmith_type`) - only if the speaker is `CF_GOD`.
//! - The "There you go, one key..." branch never explicitly zeroes
//!   `ch[cn].citem` (`:840-862`), so the silver stack given in is always
//!   consumed by the trailing "let it vanish" catch-all - reproduced by
//!   always calling `World::destroy_item` on the exchanged item after a
//!   successful forge, matching every other branch's item fate.
//! - `it[in].drdata[0]` on `IID_LIZARDMOLD` (the variant byte selecting
//!   `lizard_elite_key1/2/3`) is read via `item.driver_data.first()`.
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `dwarfsmith_driver` body at all - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:902`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA31_QA;

/// C `char_dist(cn, co) > 10` (`warrmines.c:772`).
const DWARFSMITH_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`warrmines.c:115`, the shared
/// `analyse_text_driver` copy's own guard).
const DWARFSMITH_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`warrmines.c:755`).
const DWARFSMITH_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`warrmines.c:760`).
const DWARFSMITH_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`warrmines.c:896`): idle "return to post" threshold.
const DWARFSMITH_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `it[in].drdata[0] == 1` silver marker (`warrmines.c:838`, matching
/// `silver_*.itm`'s `arg="01..."` template byte, same constant as
/// `world::npc::area29::broklin::ENHANCE_KIND_SILVER`).
const ENHANCE_KIND_SILVER: u8 = 1;
/// C `*(unsigned int *)(it[in].drdata + 1) == 5000` (`warrmines.c:839`):
/// the exact silver amount required.
const DWARFSMITH_SILVER_COST: u32 = 5000;

/// Which `lizard_elite_keyN` template to instantiate, keyed on the mold's
/// remembered `dwarfsmith_type` (C `switch (ppd->dwarfsmith_type)`,
/// `warrmines.c:841-855`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfsmithEliteKey {
    Key1,
    Key2,
    Key3,
}

/// Per-player facts [`World::process_dwarfsmith_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwarfsmithPlayerFacts {
    /// `PlayerRuntime::staffer_dwarfsmith_state()`.
    pub dwarfsmith_state: i32,
    /// `PlayerRuntime::staffer_dwarfsmith_type()`.
    pub dwarfsmith_type: i32,
}

/// A side effect [`World::process_dwarfsmith_actions`] could not apply
/// directly because it touches `PlayerRuntime`, or because it needs the
/// zone loader's item-template table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfsmithOutcomeEvent {
    /// Write the new `staffer_ppd.dwarfsmith_state` back.
    UpdateDwarfsmithState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->dwarfsmith_type = it[in].drdata[0]` (`warrmines.c:833`).
    UpdateDwarfsmithType {
        player_id: CharacterId,
        new_type: i32,
    },
    /// C `create_item("lizard_elite_keyN")` + `give_char_item`
    /// (`warrmines.c:841-860`).
    GrantEliteKey {
        player_id: CharacterId,
        key: DwarfsmithEliteKey,
    },
    /// C `case 3:` (`warrmines.c:810-815`): the god-only "reset me" state
    /// wipe (`dwarfsmith_type` is left untouched, see the module doc
    /// comment).
    ResetDwarfsmith { player_id: CharacterId },
}

impl World {
    /// C `dwarfsmith_driver`'s per-tick body (`warrmines.c:723-903`).
    pub fn process_dwarfsmith_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, DwarfsmithPlayerFacts>,
        area_id: u16,
    ) -> Vec<DwarfsmithOutcomeEvent> {
        let dwarfsmith_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DWARFSMITH
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for dwarfsmith_id in dwarfsmith_ids {
            self.process_dwarfsmith_messages(dwarfsmith_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_dwarfsmith_messages(
        &mut self,
        dwarfsmith_id: CharacterId,
        player_facts: &HashMap<CharacterId, DwarfsmithPlayerFacts>,
        area_id: u16,
        events: &mut Vec<DwarfsmithOutcomeEvent>,
    ) {
        let Some(dwarfsmith_name) = self
            .characters
            .get(&dwarfsmith_id)
            .map(|dwarfsmith| dwarfsmith.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::DwarfSmith(mut data)) = self
            .characters
            .get(&dwarfsmith_id)
            .and_then(|dwarfsmith| dwarfsmith.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&dwarfsmith_id)
            .map(|dwarfsmith| std::mem::take(&mut dwarfsmith.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.dwarfsmith_handle_char_message(
                    dwarfsmith_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.dwarfsmith_handle_text_message(
                    dwarfsmith_id,
                    &dwarfsmith_name,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.dwarfsmith_handle_give_message(
                    dwarfsmith_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(dwarfsmith) = self.characters.get_mut(&dwarfsmith_id) {
            dwarfsmith.driver_state = Some(CharacterDriverState::DwarfSmith(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`warrmines.c:892-894`).
        if let (Some(dwarfsmith), Some((tx, ty))) =
            (self.characters.get(&dwarfsmith_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(dwarfsmith.x), i32::from(dwarfsmith.y), tx, ty)
            {
                if let Some(dwarfsmith_mut) = self.characters.get_mut(&dwarfsmith_id) {
                    let _ = turn(dwarfsmith_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`warrmines.c:896-900`).
        let last_talk = if let Some(dwarfsmith) = self.characters.get(&dwarfsmith_id) {
            match dwarfsmith.driver_state.as_ref() {
                Some(CharacterDriverState::DwarfSmith(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + DWARFSMITH_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(dwarfsmith) = self.characters.get(&dwarfsmith_id) else {
                return;
            };
            let (post_x, post_y) = (dwarfsmith.rest_x, dwarfsmith.rest_y);
            self.secure_move_driver(
                dwarfsmith_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `dwarfsmith_driver`'s `NT_CHAR` branch (`warrmines.c:738-799`).
    #[allow(clippy::too_many_arguments)]
    fn dwarfsmith_handle_char_message(
        &mut self,
        dwarfsmith_id: CharacterId,
        data: &mut DwarfSmithDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfsmithPlayerFacts>,
        events: &mut Vec<DwarfsmithOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(dwarfsmith) = self.characters.get(&dwarfsmith_id).cloned() else {
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
        let tick = self.tick.0;
        if tick < data.last_talk + DWARFSMITH_TALK_MIN_TICKS {
            return;
        }
        if tick < data.last_talk + DWARFSMITH_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        if dwarfsmith_id == player_id
            || !char_see_char(&dwarfsmith, &player, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&dwarfsmith, &player) > DWARFSMITH_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->dwarfsmith_state) { case 0: ...; case 1: break;
        // case 2: break; }` (`warrmines.c:781-792`).
        if facts.dwarfsmith_state == 0 {
            self.npc_quiet_say(
                dwarfsmith_id,
                "Welcome to my smithy! If you are in need of my services, come to me and I will see what I can do for you. For now though, I'm afraid I can't do a whole lot.",
            );
            events.push(DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
                player_id,
                new_state: 1,
            });
            didsay = true;
        }

        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `dwarfsmith_driver`'s `NT_TEXT` branch (`warrmines.c:802-822`),
    /// wired through the generic `analyse_text_qa` matcher.
    fn dwarfsmith_handle_text_message(
        &mut self,
        dwarfsmith_id: CharacterId,
        dwarfsmith_name: &str,
        data: &mut DwarfSmithDriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<DwarfsmithOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
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

        if dwarfsmith_id == speaker_id {
            return;
        }
        let Some(dwarfsmith) = self.characters.get(&dwarfsmith_id).cloned() else {
            return;
        };
        if char_dist(&dwarfsmith, &speaker) > DWARFSMITH_QA_DISTANCE
            || !char_see_char(&dwarfsmith, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, dwarfsmith_name, &speaker.name, AREA31_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(dwarfsmith_id, &reply);
                didsay = true;
            }
            // C's own `case 2:` handling is commented out (`warrmines.c:
            // 808-809`) - see the module doc comment.
            TextAnalysisOutcome::Matched(2) => {
                didsay = true;
            }
            // C `case 3:` (`warrmines.c:810-815`): the god-only "reset me"
            // wipe.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(dwarfsmith_id, "reset done");
                    events.push(DwarfsmithOutcomeEvent::ResetDwarfsmith {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `dwarfsmith_driver`'s `NT_GIVE` branch (`warrmines.c:825-884`).
    #[allow(clippy::too_many_arguments)]
    fn dwarfsmith_handle_give_message(
        &mut self,
        dwarfsmith_id: CharacterId,
        data: &mut DwarfSmithDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfsmithPlayerFacts>,
        events: &mut Vec<DwarfsmithOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&dwarfsmith_id)
            .and_then(|dwarfsmith| dwarfsmith.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        // C `if (it[in].ID == IID_LIZARDMOLD && ppd &&
        // ppd->dwarfsmith_state <= 1)` (`warrmines.c:831`).
        if item.template_id == IID_LIZARDMOLD
            && facts.is_some_and(|facts| facts.dwarfsmith_state <= 1)
        {
            let mold_type = i32::from(item.driver_data.first().copied().unwrap_or(0));
            events.push(DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
                player_id: giver_id,
                new_state: 2,
            });
            events.push(DwarfsmithOutcomeEvent::UpdateDwarfsmithType {
                player_id: giver_id,
                new_type: mold_type,
            });
            self.npc_quiet_say(
                dwarfsmith_id,
                "What's this? A mold from the lizards? I guess I can make a key out of this, but I will need 5,000 silver to make it. You can't expect me to sacrifice my own ore for your adventuring!",
            );
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (it[in].driver == IDR_ENHANCE && it[in].drdata[0] ==
        // 1 && *(unsigned int *)(it[in].drdata + 1) == 5000 && ppd &&
        // ppd->dwarfsmith_state == 2)` (`warrmines.c:838-839`).
        if item.driver == IDR_ENHANCE
            && item.driver_data.first().copied() == Some(ENHANCE_KIND_SILVER)
            && enhance_amount(&item) == DWARFSMITH_SILVER_COST
            && facts.is_some_and(|facts| facts.dwarfsmith_state == 2)
        {
            self.npc_quiet_say(dwarfsmith_id, "There you go, one key for the adventurer.");
            let key = match facts.map(|facts| facts.dwarfsmith_type) {
                Some(1) => Some(DwarfsmithEliteKey::Key1),
                Some(2) => Some(DwarfsmithEliteKey::Key2),
                Some(3) => Some(DwarfsmithEliteKey::Key3),
                _ => {
                    // C `default: in2 = 0; quiet_say(cn, "oops. bug #
                    // 3266/%d", ppd->dwarfsmith_type);` (`warrmines.c:851-
                    // 854`).
                    self.npc_quiet_say(
                        dwarfsmith_id,
                        &format!(
                            "oops. bug # 3266/{}",
                            facts.map(|facts| facts.dwarfsmith_type).unwrap_or(0)
                        ),
                    );
                    None
                }
            };
            if let Some(key) = key {
                events.push(DwarfsmithOutcomeEvent::GrantEliteKey {
                    player_id: giver_id,
                    key,
                });
            }
            events.push(DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
                player_id: giver_id,
                new_state: 1,
            });
            events.push(DwarfsmithOutcomeEvent::UpdateDwarfsmithType {
                player_id: giver_id,
                new_type: 0,
            });
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else if (give_char_item(co, in))` branch
        // (`warrmines.c:863-876`): unlike every other `warrmines.c`/
        // `brannington.c` driver's fallback (which always speaks, then
        // tries to give the item back), this one only speaks *after* a
        // successful `give_char_item` - if the giver's inventory is full,
        // the item silently vanishes via the trailing catch-all with no
        // message at all.
        if self.give_char_item(giver_id, item_id) {
            if item.driver == IDR_ENHANCE {
                if item.driver_data.first().copied() != Some(ENHANCE_KIND_SILVER) {
                    self.npc_quiet_say(dwarfsmith_id, "I'll need silver, not any other material.");
                } else if enhance_amount(&item) != DWARFSMITH_SILVER_COST {
                    self.npc_quiet_say(dwarfsmith_id, "I'll need exactly 5000 units of silver.");
                } else {
                    self.npc_quiet_say(dwarfsmith_id, "I'll need a mold first.");
                }
            } else {
                self.npc_quiet_say(
                    dwarfsmith_id,
                    "Thou hast better use for this than I do. Well, if there is use for it at all.",
                );
            }
        } else {
            self.destroy_item(item_id);
        }
    }
}

/// C `*(unsigned int *)(it[in].drdata + 1)` (`warrmines.c:839` et al.): the
/// little-endian unit count stored right after the kind byte (same layout
/// as `world::npc::area29::broklin::enhance_amount`).
fn enhance_amount(item: &Item) -> u32 {
    item.driver_data
        .get(1..5)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_DWARFSMITH, CDR_LOSTCON};
use crate::entity::Item;
use crate::item_driver::{IDR_ENHANCE, IID_LIZARDMOLD};

/// C `struct dwarfsmith_data` (`src/area/31/warrmines.c:718-721`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DwarfSmithDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
