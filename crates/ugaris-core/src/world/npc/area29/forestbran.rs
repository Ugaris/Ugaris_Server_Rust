//! Forester Brannington NPC (`CDR_FORESTBRAN`), the Brannington Forest hint
//! giver who decodes thief-mage treasure maps into dig locations. Has no
//! quest of its own (no `questlog_open`/`questlog_done` call anywhere in
//! the C body).
//!
//! Ports `src/area/29/brannington.c::forest_brannington_driver` (`:1315-
//! 1502`) plus the shared `analyse_text_driver`/`qa[]` table (`:86-206`,
//! ported as [`super::AREA29_QA`]). Follows the same `World`/
//! `PlayerRuntime` split established by `world::npc::area29::spiritbran`:
//! the caller supplies a per-player fact snapshot ([`ForestBranPlayerFacts`])
//! up front and applies the returned [`ForestBranOutcomeEvent`]s afterwards,
//! since `staffer_ppd.forestbran_state`/`forestbran_done` live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `forest_brannington_driver`'s five-state (`0`-`4`) greeting chain:
//! "Welcome ..., how are you today?" -> "I've heard of your ventures..." ->
//! "thief mages talking about maps... hidden on monsters" -> "give it to
//! me, and I'll tell you where to dig" -> (waiting: state `4`). Separately,
//! handing over `IID_STAFF_FORESTMAP` reads the *read-only*
//! `forestbran_done` counter (already advanced elsewhere by the ported
//! `IDR_FORESTSPADE` treasure-dig path, `crates/ugaris-server/src/
//! area_apply.rs`) to pick one of five location hints (or a "found them
//! all" line at `5`), then unconditionally destroys the map - this driver
//! never itself increments `forestbran_done`.
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::spiritbran`'s own `NT_TEXT` branch, this
//!   driver's C body has no `dat->current_victim` staleness-reset preamble
//!   and no victim-mismatch early-out - reproduced verbatim.
//! - `case 3:` (`:1421-1426`, "reset me", god-only) speaks a visible
//!   `say(cn, "reset done")` line (not `quiet_say`), same as `spiritbran`'s
//!   own reset branch, and resets *both* `forestbran_state` *and*
//!   `forestbran_done` to `0` (`ppd->forestbran_state = ppd->forestbran_done
//!   = 0;`) - unlike every other Brannington-family reset, which only wipes
//!   dialogue-progress state.
//! - No self-defense/regen/spell-self cascade exists in C's driver body at
//!   all, matching the established "pure talker" precedent.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1501`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:1364`).
const FORESTBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const FORESTBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:1347`).
const FORESTBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:1352`).
const FORESTBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:1495`): idle "return to post" threshold.
const FORESTBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_forestbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestBranPlayerFacts {
    /// `PlayerRuntime::staffer_forestbran_state()`.
    pub forestbran_state: i32,
    /// `PlayerRuntime::forestbran_done()`, read-only here (advanced
    /// elsewhere by `IDR_FORESTSPADE`'s treasure-dig path).
    pub forestbran_done: u8,
}

/// A side effect [`World::process_forestbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestBranOutcomeEvent {
    /// Write the new `staffer_ppd.forestbran_state` back.
    UpdateForestBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`brannington.c:1421-1426`): the god-only "reset me"
    /// wipe, clearing *both* `forestbran_state` and `forestbran_done`.
    ResetForestBran { player_id: CharacterId },
}

impl World {
    /// C `forest_brannington_driver`'s per-tick body (`brannington.c:1315-
    /// 1502`).
    pub fn process_forestbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ForestBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<ForestBranOutcomeEvent> {
        let forestbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FORESTBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for forestbran_id in forestbran_ids {
            self.process_forestbran_messages(forestbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_forestbran_messages(
        &mut self,
        forestbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, ForestBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ForestBranOutcomeEvent>,
    ) {
        let Some(forestbran_name) = self
            .characters
            .get(&forestbran_id)
            .map(|forestbran| forestbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::ForestBran(mut data)) = self
            .characters
            .get(&forestbran_id)
            .and_then(|forestbran| forestbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&forestbran_id)
            .map(|forestbran| std::mem::take(&mut forestbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.forestbran_handle_char_message(
                    forestbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.forestbran_handle_text_message(
                    forestbran_id,
                    &forestbran_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.forestbran_handle_give_message(forestbran_id, message, player_facts)
                }
                _ => {}
            }
        }

        if let Some(forestbran) = self.characters.get_mut(&forestbran_id) {
            forestbran.driver_state = Some(CharacterDriverState::ForestBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:1491-1493`).
        if let (Some(forestbran), Some((tx, ty))) =
            (self.characters.get(&forestbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(forestbran.x), i32::from(forestbran.y), tx, ty)
            {
                if let Some(forestbran_mut) = self.characters.get_mut(&forestbran_id) {
                    let _ = turn(forestbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`brannington.c:1495-1499`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let last_talk = if let Some(forestbran) = self.characters.get(&forestbran_id) {
            match forestbran.driver_state.as_ref() {
                Some(CharacterDriverState::ForestBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + FORESTBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(forestbran) = self.characters.get(&forestbran_id) else {
                return;
            };
            let (post_x, post_y) = (forestbran.rest_x, forestbran.rest_y);
            self.secure_move_driver(
                forestbran_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `forest_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 1331-1406`).
    #[allow(clippy::too_many_arguments)]
    fn forestbran_handle_char_message(
        &mut self,
        forestbran_id: CharacterId,
        data: &mut ForestBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestBranPlayerFacts>,
        events: &mut Vec<ForestBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(forestbran) = self.characters.get(&forestbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:1334-1338`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:1340-1344`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:1346-1350`).
        if tick < data.last_talk + FORESTBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:1352-1355`).
        if tick < data.last_talk + FORESTBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:1357-1361`).
        if forestbran_id == player_id
            || !char_see_char(&forestbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:1363-
        // 1367`).
        if char_dist(&forestbran, &player) > FORESTBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.forestbran_state;
        match facts.forestbran_state {
            // C `case 0:` (`brannington.c:1374-1378`).
            0 => {
                self.npc_quiet_say(
                    forestbran_id,
                    &format!("Welcome {}, how are you today?", player.name),
                );
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:1379-1384`).
            1 => {
                self.npc_quiet_say(
                    forestbran_id,
                    "I've heard of your ventures, and thought I might tell you something that might interest you.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1385-1390`).
            2 => {
                self.npc_quiet_say(
                    forestbran_id,
                    "While I was in the forest, I once overheard the thief mages talking about maps they have hidden on monsters.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington.c:1391-1396`).
            3 => {
                self.npc_quiet_say(
                    forestbran_id,
                    "These maps supposedly lead to treasures they have hidden in the forest. If you have one, you can give it to me, and I'll tell you where to dig to find the treasure.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:1397-1398`): waiting for
            // maps.
            4 => {}
            _ => {}
        }

        if new_state != facts.forestbran_state {
            events.push(ForestBranOutcomeEvent::UpdateForestBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1400-1404`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `forest_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1409-1433`), wired through the generic `analyse_text_qa` matcher
    /// (same pattern as `world::npc::area29::spiritbran`'s text handler).
    /// This branch has no victim-staleness-reset preamble and no victim-
    /// mismatch early-out (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn forestbran_handle_text_message(
        &mut self,
        forestbran_id: CharacterId,
        forestbran_name: &str,
        data: &mut ForestBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestBranPlayerFacts>,
        events: &mut Vec<ForestBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1412`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if forestbran_id == speaker_id {
            return;
        }
        let Some(forestbran) = self.characters.get(&forestbran_id).cloned() else {
            return;
        };
        if char_dist(&forestbran, &speaker) > FORESTBRAN_QA_DISTANCE
            || !char_see_char(&forestbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let forestbran_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.forestbran_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, forestbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(forestbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1415-1420`): reset back to the
            // greeting if not yet past it.
            TextAnalysisOutcome::Matched(2) => {
                if forestbran_state <= 4 {
                    data.last_talk = 0;
                    events.push(ForestBranOutcomeEvent::UpdateForestBranState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:1421-1426`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")` line
            // first (see the module doc comment) and clears *both*
            // `forestbran_state` and `forestbran_done`.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(forestbran_id, "reset done");
                    events.push(ForestBranOutcomeEvent::ResetForestBran {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not yet
            // ported) is unhandled by forestbran's own C `switch` but still
            // counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:1428-1431`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `forest_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1436-1483`).
    fn forestbran_handle_give_message(
        &mut self,
        forestbran_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestBranPlayerFacts>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&forestbran_id)
            .and_then(|forestbran| forestbran.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };

        // C `ppd = set_data(co, DRD_STAFFER_PPD, ...); if (it[in].ID ==
        // IID_STAFF_FORESTMAP && ppd)` (`brannington.c:1441-1443`). Only a
        // player's `PlayerRuntime` has a `staffer_ppd`, so a non-player
        // giver always falls to the generic hand-back branch below.
        let forestbran_done = if item.template_id == IID_STAFF_FORESTMAP {
            player_facts
                .get(&giver_id)
                .map(|facts| facts.forestbran_done)
        } else {
            None
        };

        if let Some(forestbran_done) = forestbran_done {
            // C's `forestbran_done` hint `switch` (`:1444-1468`).
            let hint = match forestbran_done {
                0 => "Ah, I see you have brought me a map. Let me see where this one is hidden. hhhmmm... It is beneath a dead tree...",
                1 => "Ah, I see you have brought me a map. Let me see where this one is hidden. hhhmmm... It is under the heat of a fire...",
                2 => "Ah, I see you have brought me a map. Let me see where this one is hidden. hhhmmm... It is next to an empty bucket...",
                3 => "Ah, I see you have brought me a map. Let me see where this one is hidden. hhhmmm... It is inside a circle of stones...",
                4 => "Ah, I see you have brought me a map. Let me see where this one is hidden. hhhmmm... It is next to a pair of bags...",
                _ => "This is the first map again, I'm afraid. I think you've found all the treasures.",
            };
            self.npc_say(forestbran_id, hint);
            // C's outer "let it vanish, then" (`brannington.c:1477-1481`):
            // the `IID_STAFF_FORESTMAP` branch never zeroes `ch[cn].citem`
            // itself, so the map is always destroyed after the hint,
            // regardless of `forestbran_done` (even at the "found them
            // all" case `5`).
            self.destroy_item(item_id);
        } else {
            // C's fallback `else` branch (`brannington.c:1469-1475`): hand
            // the item back to the giver, destroying it if that fails.
            // Either way `ch[cn].citem` ends up cleared, matching the
            // outer "let it vanish, then" no-op for this branch.
            self.npc_quiet_say(
                forestbran_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_FORESTBRAN, CDR_LOSTCON};
use crate::item_driver::IID_STAFF_FORESTMAP;

/// C `struct forest_brannington_data` (`src/area/29/brannington.c:1315-
/// 1321` inline local declaration mirrored on `world::npc::area29::
/// spiritbran`'s `struct spirit_brannington_data` shape).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForestBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
