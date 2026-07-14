//! Forest ranger NPC (`CDR_FOREST_RANGER`), area 1's bear-attack warning
//! sentry near the forest stone circle.
//!
//! Ports `src/area/1/gwendylon.c::forest_ranger_driver` (`:2284-2473`)
//! plus its shared file-local `analyse_text_driver`/`qa` table (`:98-224`,
//! already ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`). Follows the same
//! `World`/`PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`ForestRangerPlayerFacts`]) up front and
//! applies the returned [`ForestRangerOutcomeEvent`]s afterwards, since
//! `forest_ranger_state`/`forest_ranger_seen_timer` (both `area1_ppd`
//! fields) live on `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - The `FOREST_RANGER_STATE_ENTRY` branch gates on `ch[cn].level`
//!   (`gwendylon.c:2344`) - the *ranger's own* character level, not the
//!   greeted player's - a genuine (if unusual) asymmetry preserved here
//!   exactly as written in C, unlike every sibling NPC in this file whose
//!   entry branch gates on the player's own quest/level facts.
//! - The `NT_CHAR` branch's two consecutive throttle checks
//!   (`gwendylon.c:2311-2320`) are both `ticker < dat->last_talk + TICKS *
//!   10`; the second one additionally requires `dat->current_victim &&
//!   dat->current_victim != co`, but since the first (unconditional)
//!   check already `continue`s whenever that same inequality holds, the
//!   second condition can never be reached with a passing first check -
//!   dead code in the C source itself (apparently a copy/paste of
//!   `camhermit_driver`'s two-window pattern that was never adjusted to
//!   use a shorter first threshold). Only the single always-reachable
//!   `TICKS * 10` gate is ported; see [`FOREST_RANGER_TALK_MIN_TICKS`].
//! - The idle body's `WN_LHAND` torch upkeep (`gwendylon.c:2438-2451`:
//!   equip a fresh `torch` item if the ranger has none, or relight an
//!   unlit one via `use_item`) is not ported. This is a cosmetic
//!   light-radius detail around a single stationary `CF_IMMORTAL` NPC,
//!   orthogonal to the dialogue/state-machine behavior this slice covers;
//!   porting it would require threading the full `execute_item_driver_
//!   request`/`apply_item_driver_outcome` item-driver pipeline (built for
//!   player-initiated `use` requests) through an NPC-idle call site that
//!   doesn't otherwise touch it anywhere else in this codebase yet.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, CDR_FOREST_RANGER, GWENDYLON_QA,
};
use crate::drvlib::offset2dx;
use crate::world::*;

/// C `char_dist(cn, co) > 15` (`gwendylon.c:2331`): the `NT_CHAR` greeting
/// range - wider than `terion`/`yoakin`/`camhermit`'s shared `> 10`.
const FOREST_RANGER_GREET_DISTANCE: i32 = 15;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const FOREST_RANGER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:2312`, `:2317`, `:2405`): the `NT_CHAR`
/// greeting throttle and the `NT_TEXT` `current_victim` reset window. See
/// the module doc comment for why only one `TICKS * 10` threshold - not
/// two, unlike `camhermit`/`yoakin` - actually governs `NT_CHAR`.
const FOREST_RANGER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:2468`): idle "return to post" threshold.
const FOREST_RANGER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `#define FOREST_RANGER_EXTEND_WAIT_TIME 60` (`gwendylon.c:2282`): the
/// `FOREST_RANGER_STATE_GREET` repeat-greeting window.
const FOREST_RANGER_EXTEND_WAIT_TIME: i32 = 60;
/// C `ch[cn].level > 30` (`gwendylon.c:2344`): the ranger's own level
/// gating the entry branch's hint-vs-warning fork.
const FOREST_RANGER_HINT_LEVEL_THRESHOLD: u32 = 30;

/// C's bare `int` state values for `ppd->forest_ranger_state`
/// (`src/common/npc_states.h:100-104`).
const FOREST_RANGER_STATE_ENTRY: i32 = 0;
const FOREST_RANGER_STATE_WARNING_1: i32 = 1;
const FOREST_RANGER_STATE_WARNING_2: i32 = 2;
const FOREST_RANGER_STATE_HINT_1: i32 = 3;
const FOREST_RANGER_STATE_GREET: i32 = 4;

/// Per-player facts [`World::process_forest_ranger_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestRangerPlayerFacts {
    /// `PlayerRuntime::area1_forest_ranger_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_forest_ranger_seen_timer()` (C `realtime`
    /// wall-clock seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
}

/// A side effect [`World::process_forest_ranger_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestRangerOutcomeEvent {
    /// Write the new `area1_ppd.forest_ranger_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->forest_ranger_seen_timer = realtime;` after
    /// every processed `NT_CHAR` message (`gwendylon.c:2389`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
}

impl World {
    /// C `forest_ranger_driver`'s per-tick body (`gwendylon.c:2284-2473`).
    /// `now` is C's wall-clock `realtime` (seconds).
    pub fn process_forest_ranger_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ForestRangerPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<ForestRangerOutcomeEvent> {
        let ranger_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FOREST_RANGER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for ranger_id in ranger_ids {
            self.process_forest_ranger_messages(ranger_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_forest_ranger_messages(
        &mut self,
        ranger_id: CharacterId,
        player_facts: &HashMap<CharacterId, ForestRangerPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<ForestRangerOutcomeEvent>,
    ) {
        let Some(ranger_name) = self
            .characters
            .get(&ranger_id)
            .map(|ranger| ranger.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::ForestRanger(mut data)) = self
            .characters
            .get(&ranger_id)
            .and_then(|ranger| ranger.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&ranger_id)
            .map(|ranger| std::mem::take(&mut ranger.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.forest_ranger_handle_char_message(
                    ranger_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.forest_ranger_handle_text_message(
                    ranger_id,
                    &ranger_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.forest_ranger_handle_give_message(ranger_id, message),
                _ => {}
            }
        }

        if let Some(ranger) = self.characters.get_mut(&ranger_id) {
            ranger.driver_state = Some(CharacterDriverState::ForestRanger(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:2462-2464`).
        if let (Some(ranger), Some((tx, ty))) =
            (self.characters.get(&ranger_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(ranger.x), i32::from(ranger.y), tx, ty) {
                if let Some(ranger_mut) = self.characters.get_mut(&ranger_id) {
                    let _ = turn(ranger_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:2466-
        // 2472`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::camhermit`/
        // `world::yoakin`/`world::terion` already use for other stationary
        // NPCs' spawn tiles.
        let last_talk = if let Some(ranger) = self.characters.get(&ranger_id) {
            match ranger.driver_state.as_ref() {
                Some(CharacterDriverState::ForestRanger(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + FOREST_RANGER_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(ranger) = self.characters.get(&ranger_id) else {
                return;
            };
            let (post_x, post_y) = (ranger.rest_x, ranger.rest_y);
            self.secure_move_driver(
                ranger_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `forest_ranger_driver`'s `NT_CHAR` branch (`gwendylon.c:2296-
    /// 2414`).
    #[allow(clippy::too_many_arguments)]
    fn forest_ranger_handle_char_message(
        &mut self,
        ranger_id: CharacterId,
        data: &mut ForestRangerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestRangerPlayerFacts>,
        now: i32,
        events: &mut Vec<ForestRangerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(ranger) = self.characters.get(&ranger_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:2300-2303`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:2305-2309`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`gwendylon.c:2311-2314`) - see the module doc comment for why
        // the C source's second, `current_victim`-gated check
        // (`gwendylon.c:2317-2320`) is unreachable dead code and not
        // ported.
        if tick < data.last_talk + FOREST_RANGER_TALK_MIN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:2322-2325`).
        if ranger_id == player_id || !char_see_char(&ranger, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 15) continue;` (`gwendylon.c:2327-
        // 2330`).
        if char_dist(&ranger, &player) > FOREST_RANGER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        match facts.state {
            FOREST_RANGER_STATE_ENTRY => {
                // C `case FOREST_RANGER_STATE_ENTRY:` (`gwendylon.c:2343-
                // 2348`): gates on the *ranger's own* level, not the
                // player's - silent transition either way, no dialogue.
                new_state = if ranger.level > FOREST_RANGER_HINT_LEVEL_THRESHOLD {
                    FOREST_RANGER_STATE_HINT_1
                } else {
                    FOREST_RANGER_STATE_WARNING_1
                };
            }
            FOREST_RANGER_STATE_WARNING_1 => {
                self.npc_quiet_say(
                    ranger_id,
                    "Hail thee, adventurer! Take heed of my warning, the beasts of the forest have grown vile and dangerous. To the South of here is a monster far more dangerous than the rest, thou shouldst take care to keep out of its reach.",
                );
                new_state = FOREST_RANGER_STATE_WARNING_2;
                didsay = true;
            }
            FOREST_RANGER_STATE_WARNING_2 => {
                self.npc_quiet_say(
                    ranger_id,
                    "It lingers in the dead wood and thou art too weak to face it.",
                );
                new_state = FOREST_RANGER_STATE_GREET;
                didsay = true;
            }
            FOREST_RANGER_STATE_HINT_1 => {
                self.npc_quiet_say(
                    ranger_id,
                    "There is some strange magic at work here traveler. Yesterday I bested a savage bear by those large stones to the North, and my sword seemed to glow in company with the sun.",
                );
                new_state = FOREST_RANGER_STATE_GREET;
                didsay = true;
            }
            FOREST_RANGER_STATE_GREET
                // C `case FOREST_RANGER_STATE_GREET:` (`gwendylon.c:2373-
                // 2377`).
                if now.saturating_sub(facts.seen_timer) > FOREST_RANGER_EXTEND_WAIT_TIME => {
                    self.npc_quiet_say(ranger_id, "Hail thee, adventurer.");
                    didsay = true;
                }
            // Every other value: no-op, matching C's `switch` with no
            // matching `case`.
            _ => {}
        }

        if new_state != facts.state {
            events.push(ForestRangerOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->forest_ranger_seen_timer = realtime;`
        // (`gwendylon.c:2389`): unconditional, regardless of `didsay`.
        events.push(ForestRangerOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:2391-2395`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `forest_ranger_driver`'s `NT_TEXT` branch (`gwendylon.c:2399-
    /// 2422`), wired through the generic `analyse_text_qa` matcher (same
    /// pattern as `world::camhermit`/`world::yoakin`/`world::terion`'s
    /// text handlers).
    #[allow(clippy::too_many_arguments)]
    fn forest_ranger_handle_text_message(
        &mut self,
        ranger_id: CharacterId,
        ranger_name: &str,
        data: &mut ForestRangerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestRangerPlayerFacts>,
        events: &mut Vec<ForestRangerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:2405-2407`).
        let tick = self.tick.0;
        if tick > data.last_talk + FOREST_RANGER_TALK_MIN_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:2409-2412`).
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

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if ranger_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(ranger) = self.characters.get(&ranger_id).cloned() else {
            return;
        };
        if char_dist(&ranger, &speaker) > FOREST_RANGER_QA_DISTANCE
            || !char_see_char(&ranger, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, ranger_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(ranger_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:2404-2411`): reset to entry only
            // if currently at the `GREET` steady state.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state == FOREST_RANGER_STATE_GREET {
                        events.push(ForestRangerOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: FOREST_RANGER_STATE_ENTRY,
                        });
                        data.last_talk = 0;
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by forest_ranger's own
            // C `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:2417-2420`).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `forest_ranger_driver`'s `NT_GIVE` branch (`gwendylon.c:2426-
    /// 2436`).
    fn forest_ranger_handle_give_message(
        &mut self,
        ranger_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&ranger_id)
            .and_then(|ranger| ranger.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            ranger_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        self.give_char_item_smart(giver_id, item_id, true);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct forest_ranger_driver_data` (`src/area/1/gwendylon.c:2275-
/// 2278`): the bear-attack-warning sentry NPC's own driver memory
/// (`CDR_FOREST_RANGER`, distinct from the per-player
/// `forest_ranger_state`/`forest_ranger_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see
/// `world::forest_ranger`'s module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForestRangerDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
