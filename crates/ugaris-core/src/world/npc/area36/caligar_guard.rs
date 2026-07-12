//! Caligar entrance guard NPC duo (`CDR_CALIGARGUARD`), Eulc and Margana,
//! whose alternating five-line banter ("Human entry is not permitted!" /
//! "He let the bed in!" / "Quiet you fool!" / "Backwards is the key to
//! entry!" / "Ugh, I said quiet! ...") teaches the player the "walk
//! backwards through the gate" riddle.
//!
//! Ports `src/area/36/caligar.c::guard_driver` (`:234-393`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-212`, ported as
//! [`super::AREA36_QA`] in `world::npc::area36`). Both guards are
//! `CF_NOATTACK`/`CF_IMMORTAL` (`zones/36/Caligar_Guards.chr`), so unlike
//! most Caligar NPCs, this driver never needs the `CDR_SIMPLEBADDY`
//! combat-AI gate-widening.
//!
//! Unlike every other Caligar/warrmines/brannington-style dialogue driver
//! in this codebase, this driver keeps **no NPC-local state at all** - C's
//! `guard_driver` has no `set_data(cn, ...)` call for its own struct, only
//! `set_data(co, DRD_CALIGAR_PPD, ...)` for the *player* it is talking to
//! (`ppd->guard_state`/`guard_last_talk`, shared by both guards). Which
//! guard is allowed to speak at each state is decided purely by `me`
//! (`!strcmp(ch[cn].name, "Eulc")`, recomputed fresh every tick from the
//! live character name) and the player's own `guard_state` parity - so
//! Eulc and Margana alternate turns automatically as the shared counter
//! advances, without either guard needing to remember whose turn it is.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives every timer in this
//!   driver, not `ticker` (game ticks) - `now: i32` is threaded in from
//!   `ugaris-server`'s `current_unix_time()`, same precedent as
//!   `world::npc::area17::guard`'s own `now` parameter.
//! - The `ch[co].y > 106` "don't talk to people on our side of the fence"
//!   guard (`caligar.c:278-281`) is reproduced verbatim even though its
//!   real-world effect (which side of a specific map line the target
//!   stands on) is purely geometric, same as every other raw coordinate
//!   check ported elsewhere in this codebase.
//! - C's per-message `remove_message(cn, msg)` calls have no equivalent
//!   here - the per-tick `driver_messages` drain (`std::mem::take`)
//!   already empties the queue exactly once per tick, the same
//!   "implicit removal" precedent used by every other ported NPC driver.
//! - The `NT_GIVE` handler is the same "give back whatever we're still
//!   holding, or destroy it if the giver's inventory is full" boilerplate
//!   every dialogue-only NPC in this codebase repeats (`caligar.c:344-
//!   352`) - the guards never intentionally keep anything.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:392`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent
//!   (`secure_move_driver`'s own fallback already covers "hold position").

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::world::*;

use super::AREA36_QA;

/// C `char_dist(cn, co) > 10` (`caligar.c:272`/`:366`).
const CALIGAR_GUARD_DISTANCE: i32 = 10;
/// C `realtime - ppd->guard_last_talk < 3` (`caligar.c:289`).
const CALIGAR_GUARD_TALK_COOLDOWN_SECONDS: i32 = 3;
/// C `realtime - ppd->guard_last_talk > 600` (`caligar.c:336`): the
/// state-5 "give up and reset the riddle" timeout.
const CALIGAR_GUARD_RESET_TIMEOUT_SECONDS: i32 = 600;
/// C `ch[co].y > 106` (`caligar.c:278`).
const CALIGAR_GUARD_FENCE_Y: u16 = 106;

/// Per-player facts [`World::process_caligar_guard_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarGuardPlayerFacts {
    /// `PlayerRuntime::caligar_guard_state()`.
    pub guard_state: i32,
    /// `PlayerRuntime::caligar_guard_last_talk()`.
    pub guard_last_talk: i32,
}

/// A side effect [`World::process_caligar_guard_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarGuardOutcomeEvent {
    /// C `ppd->guard_state++; ppd->guard_last_talk = realtime;`
    /// (`caligar.c:294-334`, every successful line).
    AdvanceGuardTalk {
        player_id: CharacterId,
        new_state: i32,
        realtime_seconds: i32,
    },
    /// C `case 5: if (realtime - ppd->guard_last_talk > 600)
    /// ppd->guard_state = 0;` (`caligar.c:336-338`) - `guard_last_talk`
    /// is deliberately left untouched, matching C.
    ResetGuardStateTimeout { player_id: CharacterId },
    /// C `case 2:` (`analyse_text_driver` code `2`, "repeat"/"restart"):
    /// `ppd->guard_state == 3` resets back to `0` (`caligar.c:372-378`).
    ResetGuardStateIfThree { player_id: CharacterId },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_CALIGARGUARD`
    /// characters (C `ch_driver`'s `CDR_CALIGARGUARD` case,
    /// `caligar.c:1857-1859`).
    pub fn process_caligar_guard_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarGuardPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CaligarGuardOutcomeEvent> {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guard_id in guard_ids {
            self.process_caligar_guard_messages(guard_id, player_facts, now, &mut events);
            let (post_x, post_y) = self
                .characters
                .get(&guard_id)
                .map(|character| (character.rest_x, character.rest_y))
                .unwrap_or_default();
            // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy,
            // DX_UP, ret, lastact)) return; do_idle(cn, TICKS);`
            // (`caligar.c:388-392`).
            self.secure_move_driver(guard_id, post_x, post_y, Direction::Up as u8, 0, 0, area_id);
        }
        events
    }

    fn process_caligar_guard_messages(
        &mut self,
        guard_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarGuardOutcomeEvent>,
    ) {
        let Some(guard_name) = self
            .characters
            .get(&guard_id)
            .map(|guard| guard.name.clone())
        else {
            return;
        };
        // C `if (!strcmp(ch[cn].name, "Eulc")) me = 0; else me = 1;`
        // (`caligar.c:239-243`).
        let me_zero = guard_name == "Eulc";

        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|guard| std::mem::take(&mut guard.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.caligar_guard_handle_char_message(
                    guard_id,
                    me_zero,
                    message,
                    player_facts,
                    now,
                    events,
                ),
                NT_TEXT => {
                    self.caligar_guard_handle_text_message(guard_id, &guard_name, message, events)
                }
                NT_GIVE => self.caligar_guard_handle_give_message(guard_id, message),
                _ => {}
            }
        }
    }

    /// C `guard_driver`'s `NT_CHAR` branch (`caligar.c:249-341`).
    #[allow(clippy::too_many_arguments)]
    fn caligar_guard_handle_char_message(
        &mut self,
        guard_id: CharacterId,
        me_zero: bool,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaligarGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarGuardOutcomeEvent>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
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
        if guard_id == player_id || !char_see_char(&guard, &player, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&guard, &player) > CALIGAR_GUARD_DISTANCE {
            return;
        }
        if player.y > CALIGAR_GUARD_FENCE_Y {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now - facts.guard_last_talk < CALIGAR_GUARD_TALK_COOLDOWN_SECONDS {
            return;
        }

        // C `switch (ppd->guard_state) { case 0..4: if (me != N) break;
        // quiet_say(...); ppd->guard_last_talk = realtime;
        // ppd->guard_state++; break; case 5: ... }` (`caligar.c:294-339`).
        let me = i32::from(!me_zero);
        let line = match facts.guard_state {
            0 if me == 0 => Some("Human entry is not permitted! Leave at once!"),
            1 if me == 1 => Some("He let the bed in!"),
            2 if me == 0 => Some("Quiet you fool!"),
            3 if me == 1 => Some("Backwards is the key to entry!"),
            4 if me == 0 => Some("Ugh, I said quiet! We aren't supposed to let humans in!"),
            _ => None,
        };

        if let Some(line) = line {
            self.npc_quiet_say(guard_id, line);
            events.push(CaligarGuardOutcomeEvent::AdvanceGuardTalk {
                player_id,
                new_state: facts.guard_state + 1,
                realtime_seconds: now,
            });
            return;
        }

        if facts.guard_state == 5
            && now - facts.guard_last_talk > CALIGAR_GUARD_RESET_TIMEOUT_SECONDS
        {
            events.push(CaligarGuardOutcomeEvent::ResetGuardStateTimeout { player_id });
        }
    }

    /// C `guard_driver`'s `NT_TEXT` branch (`caligar.c:355-380`), wired
    /// through the generic `analyse_text_qa` matcher.
    fn caligar_guard_handle_text_message(
        &mut self,
        guard_id: CharacterId,
        guard_name: &str,
        message: &CharacterDriverMessage,
        events: &mut Vec<CaligarGuardOutcomeEvent>,
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
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        if guard_id == speaker_id || !char_see_char(&guard, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&guard, &speaker) > CALIGAR_GUARD_DISTANCE {
            return;
        }

        match analyse_text_qa(text, guard_name, &speaker.name, AREA36_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(guard_id, &reply);
            }
            TextAnalysisOutcome::Matched(2) => {
                events.push(CaligarGuardOutcomeEvent::ResetGuardStateIfThree {
                    player_id: speaker_id,
                });
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `guard_driver`'s `NT_GIVE` branch (`caligar.c:344-352`).
    fn caligar_guard_handle_give_message(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&guard_id)
            .and_then(|guard| guard.cursor_item.take())
        else {
            return;
        };
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARGUARD;
