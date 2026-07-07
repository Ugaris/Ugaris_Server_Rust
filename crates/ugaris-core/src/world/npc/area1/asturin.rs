//! Private-quarters guard NPC (`CDR_ASTURIN`), area 1's city guard who
//! keeps players out of Asturin's own rooms (and, past a certain point in
//! the map, out of the city gate itself).
//!
//! Ports `src/area/1/gwendylon.c::asturin_driver` (`:4421-4533`) plus its
//! `asturin_dead` death hook (`:4535-4542`, ported as
//! `crate::world::hurt::apply_asturin_death_from_hurt_event` in
//! `ugaris-server`'s `world_events::death_hooks`, mirroring the
//! `CDR_GATE_FIGHT`/`CDR_CALIGARSKELLY` death-hook shape - the killer's
//! `PlayerRuntime::area1_asturin_state`/`area1_asturin_seen_timer` (both
//! `area1_ppd` fields) live outside `World`). Unlike every other area-1
//! NPC ported so far, Asturin combines two behaviors C keeps in the same
//! function: a positional-greeting/warning state machine gated on the
//! *player's* x coordinate (not a `qa` dialogue table beyond the shared
//! "repeat" reset), and a full self-defense/regen/spell-self/return-to-
//! post cascade every tick (`fight_driver_update`/`fight_driver_
//! attack_visible`/`fight_driver_follow_invisible`/`spell_self_driver`/
//! `regenerate_driver`/`secure_move_driver`), which every other area-1
//! NPC ported so far omits entirely (they are peaceful dialogue-only
//! NPCs) or replaces with a walking route (`world::robber`/`world::sanoa`).
//!
//! Deviations/gaps (documented, not silent):
//! - C's self-defense cascade (`fight_driver_update`/`fight_driver_
//!   attack_visible`/`fight_driver_follow_invisible`) is backed by the
//!   fully generic 10-slot `struct fight_driver_data`. This port tracks
//!   only the single most-recent attacker as `victim` (set from the
//!   unconditional `NT_GOTHIT` message every hurt character already
//!   receives - see `World::apply_legacy_hurt`), the same single-enemy
//!   simplification already established for `CDR_ROBBER`/`CDR_SANOA`/
//!   `CDR_GATE_FIGHT`. "Attack visible" reuses `World::attack_driver_
//!   direct`; "follow invisible" reuses `secure_move_driver` toward the
//!   last known position.
//! - `fight_driver_set_dist(cn, 20, 0, 40)` (`gwendylon.c:4441`, on
//!   `NT_CREATE`) is not ported - same precedent as `world::robber`/
//!   `world::sanoa`'s own module doc comments (their single-victim model
//!   has no equivalent gate; never observably different in practice
//!   since C's `home` is reset to Asturin's own current position every
//!   tick anyway).
//! - `struct asturin_driver_data`'s `state` field (`gwendylon.c:4422`) is
//!   never read or written anywhere in `asturin_driver`'s body - dead
//!   even in C, same precedent as `world::terion`/`world::reskin`'s own
//!   dead `last_walk`/`pos` fields - so it is not ported; this port's own
//!   `AsturinDriverData` carries only the single-victim self-defense
//!   tracking fields.
//! - `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
//!   lastact)` (`gwendylon.c:4528`): the NPC's post position (C's
//!   `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same substitution
//!   `world::camhermit`/`world::gatekeeper`/`world::bank` already use for
//!   other stationary NPCs' spawn tiles. Unlike those NPCs, C calls this
//!   unconditionally every tick (no `last_talk + TICKS*30` throttle
//!   exists in `asturin_driver`'s body at all), ported the same way here.
//! - `do_idle(cn, TICKS / 2)` (`gwendylon.c:4532`): half the standard
//!   one-second idle duration every other simple grunt NPC in this file
//!   uses (`World::idle_simple_baddy`, which is hardcoded to a full
//!   `TICKS_PER_SECOND`), so this port calls `do_idle` directly with
//!   `TICKS_PER_SECOND / 2` instead of reusing that helper.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON, GWENDYLON_QA, NTID_ASTURIN,
};
use crate::world::*;

/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const ASTURIN_QA_DISTANCE: i32 = 12;
/// C `realtime - ppd->asturin_seen_timer > 10` (`gwendylon.c:4449`).
const ASTURIN_STATE_RESET_SECONDS: i32 = 10;
/// C `realtime - ppd->asturin_seen_timer > 30` (`gwendylon.c:4452`).
const ASTURIN_WELCOME_RESET_SECONDS: i32 = 30;
/// C `ch[co].x < 115` (`gwendylon.c:4456`): the outer "GUARDS!" boundary.
const ASTURIN_GUARDS_BOUNDARY_X: u16 = 115;
/// C `ch[co].x < 118` (`gwendylon.c:4462`): the "go back" warning zone.
const ASTURIN_WARNING_BOUNDARY_X: u16 = 118;

/// Per-player facts [`World::process_asturin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsturinPlayerFacts {
    /// `PlayerRuntime::area1_asturin_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_asturin_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
}

/// A side effect [`World::process_asturin_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsturinOutcomeEvent {
    /// Write the new `area1_ppd.asturin_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->asturin_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:4487`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_ASTURIN`
    /// characters (C `ch_driver`'s `CDR_ASTURIN` case, `gwendylon.c:6105-
    /// 6107`).
    pub fn process_asturin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, AsturinPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<AsturinOutcomeEvent> {
        let asturin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ASTURIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for asturin_id in asturin_ids {
            self.process_asturin_tick(asturin_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    /// C `asturin_driver`'s per-tick body (`gwendylon.c:4425-4533`).
    fn process_asturin_tick(
        &mut self,
        asturin_id: CharacterId,
        player_facts: &HashMap<CharacterId, AsturinPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<AsturinOutcomeEvent>,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&asturin_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Asturin(data)) => data,
            _ => AsturinDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&asturin_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR => {
                    self.asturin_handle_char_message(asturin_id, message, player_facts, now, events)
                }
                NT_TEXT => self.asturin_handle_text_message(asturin_id, message, events),
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    if let Some((asturin, attacker)) = self
                        .characters
                        .get(&asturin_id)
                        .cloned()
                        .zip(self.characters.get(&attacker_id).cloned())
                    {
                        // C `if (ch[cn].group == ch[co].group) break; if
                        // (!can_attack(cn,co)) break; fight_driver_add_enemy
                        // (cn, co, 1, 1);`.
                        if asturin.group != attacker.group
                            && can_attack(&asturin, &attacker, &self.map)
                        {
                            data.victim = Some(attacker_id);
                        }
                    }
                }
                _ => {}
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&asturin_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((asturin, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&asturin, &victim, &self.map, self.date.daylight) {
                        data.victim_visible = true;
                        data.victim_last_x = victim.x;
                        data.victim_last_y = victim.y;
                    } else {
                        data.victim_visible = false;
                    }
                }
                _ => {
                    data.victim = None;
                    data.victim_visible = false;
                }
            }
        }

        if let Some(character) = self.characters.get_mut(&asturin_id) {
            character.driver_state = Some(CharacterDriverState::Asturin(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(asturin_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let arrived = self.characters.get(&asturin_id).is_some_and(|asturin| {
                asturin.x.abs_diff(data.victim_last_x) < 2
                    && asturin.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Asturin(state)) = self
                    .characters
                    .get_mut(&asturin_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                asturin_id,
                data.victim_last_x,
                data.victim_last_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return true;
            }
        }

        // C `if (spell_self_driver(cn)) return;`.
        if self.spell_self_simple_baddy(asturin_id) {
            return true;
        }
        // C `if (regenerate_driver(cn)) return;`.
        if self.regenerate_simple_baddy(asturin_id) {
            return true;
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT,
        // ret, lastact)) return;` - unconditional every tick, unlike the
        // `last_talk`-gated return-to-post used by the dialogue-only area-1
        // NPCs (see module doc comment).
        let Some((post_x, post_y)) = self
            .characters
            .get(&asturin_id)
            .map(|asturin| (asturin.rest_x, asturin.rest_y))
        else {
            return false;
        };
        if self.secure_move_driver(
            asturin_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS / 2);` (`gwendylon.c:4532`).
        self.characters
            .get_mut(&asturin_id)
            .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok())
    }

    /// C `asturin_driver`'s `NT_CHAR` branch (`gwendylon.c:4444-4488`).
    fn asturin_handle_char_message(
        &mut self,
        asturin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, AsturinPlayerFacts>,
        now: i32,
        events: &mut Vec<AsturinOutcomeEvent>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(asturin) = self.characters.get(&asturin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if ((ch[co].flags & CF_PLAYER) && ch[co].driver != CDR_LOSTCON
        // && char_dist(cn, co) < 16 && char_see_char(cn, co) && (ppd = ...))`
        // (`gwendylon.c:4447-4448`).
        if !player.flags.contains(CharacterFlags::PLAYER) || player.driver == CDR_LOSTCON {
            return;
        }
        if char_dist(&asturin, &player) >= 16 {
            return;
        }
        if !char_see_char(&asturin, &player, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        let mut state = facts.state;

        // C `if (realtime - ppd->asturin_seen_timer > 10 && ppd->
        // asturin_state >= 1 && ppd->asturin_state <= 3) { ppd->
        // asturin_state = 0; }` (`gwendylon.c:4449-4451`).
        if now.saturating_sub(facts.seen_timer) > ASTURIN_STATE_RESET_SECONDS
            && (1..=3).contains(&state)
        {
            state = 0;
        }
        // C `if (realtime - ppd->asturin_seen_timer > 30 && ppd->
        // asturin_state >= 7 && ppd->asturin_state <= 8) { ppd->
        // asturin_state = 8; }` (`gwendylon.c:4452-4454`).
        if now.saturating_sub(facts.seen_timer) > ASTURIN_WELCOME_RESET_SECONDS
            && (7..=8).contains(&state)
        {
            state = 8;
        }

        if player.x < ASTURIN_GUARDS_BOUNDARY_X {
            // C `gwendylon.c:4456-4461`.
            if state < 3 {
                self.npc_shout(asturin_id, "GUARDS!");
                self.notify_area(
                    asturin.x,
                    asturin.y,
                    NT_NPC,
                    NTID_ASTURIN,
                    asturin_id.0 as i32,
                    player_id.0 as i32,
                );
                state = 3;
            }
        } else if player.x < ASTURIN_WARNING_BOUNDARY_X {
            // C `gwendylon.c:4462-4470`.
            if state < 2 {
                self.npc_quiet_say(
                    asturin_id,
                    &format!("Go back {}, you have no business here!", player.name),
                );
                state = 2;
            }
            if (4..=5).contains(&state) {
                self.npc_quiet_say(
                    asturin_id,
                    &format!(
                        "Alright, alright, {}, go ahead, just don't hit me again!",
                        player.name
                    ),
                );
                state = 6;
            }
        } else {
            // C `gwendylon.c:4471-4486`.
            match state {
                0 => {
                    self.npc_quiet_say(
                        asturin_id,
                        &format!(
                            "Hello {}. These rooms are private. Please go back.",
                            player.name
                        ),
                    );
                    state += 1;
                }
                4 => {
                    self.npc_quiet_say(
                        asturin_id,
                        &format!(
                            "Hello {}. These rooms are private. Please go back.",
                            player.name
                        ),
                    );
                    state += 1;
                }
                7 => {
                    self.npc_quiet_say(
                        asturin_id,
                        &format!("Be greeted, {}. Welcome.", player.name),
                    );
                    state += 1;
                }
                _ => {}
            }
        }

        if state != facts.state {
            events.push(AsturinOutcomeEvent::UpdateState {
                player_id,
                new_state: state,
            });
        }
        // C `ppd->asturin_seen_timer = realtime;` (`gwendylon.c:4487`):
        // unconditional.
        events.push(AsturinOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });
    }

    /// C `asturin_driver`'s `NT_TEXT` branch (`gwendylon.c:4491-4502`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`/`world::reskin`).
    fn asturin_handle_text_message(
        &mut self,
        asturin_id: CharacterId,
        message: &CharacterDriverMessage,
        events: &mut Vec<AsturinOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        let Some(asturin_name) = self
            .characters
            .get(&asturin_id)
            .map(|asturin| asturin.name.clone())
        else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if asturin_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(asturin) = self.characters.get(&asturin_id).cloned() else {
            return;
        };
        if char_dist(&asturin, &speaker) > ASTURIN_QA_DISTANCE
            || !char_see_char(&asturin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        match analyse_text_qa(text, &asturin_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(asturin_id, &reply);
            }
            // C `case 2:` (`gwendylon.c:4495-4500`): reset `asturin_state`
            // back to 0.
            TextAnalysisOutcome::Matched(2) => {
                events.push(AsturinOutcomeEvent::UpdateState {
                    player_id: speaker_id,
                    new_state: 0,
                });
            }
            // Every other matched code is unhandled by asturin's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one).
            TextAnalysisOutcome::Matched(_) => {}
            TextAnalysisOutcome::NoMatch => {}
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_ASTURIN;

/// C `struct asturin_driver_data` (`src/area/1/gwendylon.c:4421-4423`): the
/// dead `state` field is not ported (see module doc comment); this port's
/// own single-victim self-defense tracking follows the same shape as
/// `world::robber`/`world::sanoa`'s own driver data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AsturinDriverData {
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
