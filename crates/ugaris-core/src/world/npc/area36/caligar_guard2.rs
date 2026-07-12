//! Caligar interior guard NPC (`CDR_CALIGARGUARD2`), a combat-capable
//! sentry who taunts an approaching player before falling through to
//! plain `CDR_SIMPLEBADDY` self-defense/idle AI.
//!
//! Ports `src/area/36/caligar.c::guard2_driver` (`:395-442`): an `NT_CHAR`
//! taunt ("Halt! You will die where you stand!") gated by a per-player 15
//! second cooldown, followed by an unconditional every-tick tail call to
//! `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`. The
//! SimpleBaddy AI half is ported by widening the shared combat/idle gates
//! in `world::npc_fight`/`world::npc_idle` (same precedent as
//! `CDR_TEUFELRAT`/`CDR_TWOROBBER`/`CDR_PENTER`) plus a
//! `template.driver == CDR_CALIGARGUARD2` `NT_CREATE` seed in `zone.rs`.
//!
//! Like `world::npc::area36::caligar_guard`, this driver keeps no NPC-local state
//! of its own - `ppd->guard2_last_talk` lives entirely on the *player*
//! being taunted (`crate::player::PlayerRuntime::caligar_guard2_last_talk`),
//! same "player-keyed PPD, no per-NPC struct" shape.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `realtime` (wall-clock seconds) drives the cooldown, not
//!   `ticker` - `now: i32` is threaded in the same way as
//!   `world::npc::area36::caligar_guard`'s own `now` parameter.
//! - C's message loop never calls `remove_message` at all in this driver
//!   - irrelevant here since the per-tick `driver_messages` drain
//!     (`std::mem::take`) already empties the queue exactly once per
//!     tick.
//! - `say` (not `quiet_say`) is used for the taunt, matching C exactly -
//!   ported via [`World::npc_say`].

use std::collections::HashMap;

use crate::world::*;

/// C `char_dist(cn, co) > 10` (`caligar.c:424`).
const CALIGAR_GUARD2_DISTANCE: i32 = 10;
/// C `realtime - ppd->guard2_last_talk < 15` (`caligar.c:433`).
const CALIGAR_GUARD2_TALK_COOLDOWN_SECONDS: i32 = 15;

/// Per-player facts [`World::process_caligar_guard2_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaligarGuard2PlayerFacts {
    /// `PlayerRuntime::caligar_guard2_last_talk()`.
    pub guard2_last_talk: i32,
}

/// A side effect [`World::process_caligar_guard2_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarGuard2OutcomeEvent {
    /// C `ppd->guard2_last_talk = realtime;` (`caligar.c:438`).
    UpdateGuard2LastTalk {
        player_id: CharacterId,
        realtime_seconds: i32,
    },
}

impl World {
    /// Ports the `NT_CHAR` taunt half of `CDR_CALIGARGUARD2`'s per-tick
    /// dispatch (C `ch_driver`'s `CDR_CALIGARGUARD2` case,
    /// `caligar.c:1860-1862`); the `CDR_SIMPLEBADDY` combat/idle tail is
    /// ported separately via the widened AI gates - see the module doc
    /// comment.
    pub fn process_caligar_guard2_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaligarGuard2PlayerFacts>,
        now: i32,
    ) -> Vec<CaligarGuard2OutcomeEvent> {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CALIGARGUARD2
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guard_id in guard_ids {
            self.process_caligar_guard2_messages(guard_id, player_facts, now, &mut events);
        }
        events
    }

    fn process_caligar_guard2_messages(
        &mut self,
        guard_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaligarGuard2PlayerFacts>,
        now: i32,
        events: &mut Vec<CaligarGuard2OutcomeEvent>,
    ) {
        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|guard| std::mem::take(&mut guard.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            if message.message_type != NT_CHAR {
                continue;
            }
            let player_id = CharacterId(message.dat1.max(0) as u32);
            let Some(guard) = self.characters.get(&guard_id).cloned() else {
                continue;
            };
            let Some(player) = self.characters.get(&player_id).cloned() else {
                continue;
            };

            if !player.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if player.driver == CDR_LOSTCON {
                continue;
            }
            if guard_id == player_id
                || !char_see_char(&guard, &player, &self.map, self.date.daylight)
            {
                continue;
            }
            if char_dist(&guard, &player) > CALIGAR_GUARD2_DISTANCE {
                continue;
            }

            let Some(facts) = player_facts.get(&player_id) else {
                continue;
            };
            if now - facts.guard2_last_talk < CALIGAR_GUARD2_TALK_COOLDOWN_SECONDS {
                continue;
            }

            self.npc_say(guard_id, "Halt! You will die where you stand!");
            events.push(CaligarGuard2OutcomeEvent::UpdateGuard2LastTalk {
                player_id,
                realtime_seconds: now,
            });
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CALIGARGUARD2;
