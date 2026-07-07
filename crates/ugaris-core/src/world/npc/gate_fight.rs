//! Gatekeeper-fight NPC (`CDR_GATE_FIGHT`).
//!
//! Ports `src/system/gatekeeper.c::gate_fight_driver` (the opponent spawned
//! into a private test room by `ugaris-server::spawns::
//! gate_enter_test_spawn_room`, itself triggered from `world::gatekeeper`'s
//! `GateWelcomeOutcomeEvent::EnterTestReady`) and `gate_fight_dead` (the
//! reward-and-teleport tail run when the player kills it).
//!
//! Deviations/gaps (documented, not silent):
//! - C's `gate_fight_driver` is backed by the fully generic
//!   `struct fight_driver_data`/`DRD_FIGHTDRIVER` (a 10-slot enemy list with
//!   visibility tracking, attack scoring, and pathfinding-to-last-seen-
//!   position - `drvlib.c:2170-2345`), shared by many NPC types. This port
//!   only tracks the single `victim` this NPC ever fights: C's
//!   `gate_fight_driver` never calls `fight_driver_add_enemy` itself, only
//!   setting `dat->victim` once from the `NT_NPC`/`NTID_GATEKEEPER` message
//!   (`gatekeeper.c:659-661`) that `gate_enter_test_spawn_room` sends right
//!   after creating this NPC; `standard_message_driver(cn, msg, 1, 0)`
//!   (`gatekeeper.c:663`) exists only to catch incidental `NT_CHAR`/
//!   `NT_GOTHIT` reactions from someone else attacking it, which cannot
//!   happen in this private, single-opponent duel room. "attack visible"
//!   reuses the already-generic `World::attack_driver_direct`
//!   (adjacent-attack-or-pathfind-toward, see `world/npc_fight.rs`); "follow
//!   invisible" reuses `secure_move_driver` toward the victim's last known
//!   position instead of C's dedicated `pathfinder`-based
//!   `fight_driver_follow_invisible`.
//! - `gate_fight_dead`'s case `8` (plain Seyan'Du) now calls `turn_seyan`
//!   (`src/system/tool.c:4278-4389`, ported at `World::apply_turn_seyan`,
//!   `world/turn_seyan.rs`) when the caller can supply the `"seyan_m"`
//!   template's base values; see that module's doc comment for its own
//!   documented gaps (`destroy_chareffects` no-op, `DRD_DEPOT_PPD` quest-
//!   flag stripping not ported). Falls back to an honest placeholder
//!   message (instead of a possibly-untrue "You are a Seyan'Du now.") if
//!   the template lookup or reroll fails, while still performing C's
//!   unconditional post-`switch` `teleport_char_driver(co, 181, 198)`.

use crate::world::*;

/// C `TICKS * 60 * 10` (`gatekeeper.c:668`): self-destruct after 10 minutes.
const GATE_FIGHT_SELF_DESTRUCT_TICKS: u64 = TICKS_PER_SECOND * 60 * 10;

impl World {
    /// C `gate_fight_driver`'s per-tick body (`gatekeeper.c:641-696`).
    pub fn process_gate_fight_actions(&mut self, area_id: u16) -> usize {
        let fight_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GATE_FIGHT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for fight_id in fight_ids {
            if self.process_gate_fight_tick(fight_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    fn process_gate_fight_tick(&mut self, fight_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&fight_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::GateFight(data)) => data,
            _ => GateFightDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&fight_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            // C `if (msg->type == NT_CREATE) dat->creation_time = ticker;`
            // (`gatekeeper.c:655-657`).
            if message.message_type == NT_CREATE {
                data.creation_time = self.tick.0;
            }
            // C `if (msg->type == NT_NPC && msg->dat1 == NTID_GATEKEEPER)
            // dat->victim = msg->dat2;` (`gatekeeper.c:659-661`).
            if message.message_type == NT_NPC && message.dat1 == NTID_GATEKEEPER {
                data.victim = Some(CharacterId(message.dat2.max(0) as u32));
            }
        }

        // C `if (ticker - dat->creation_time > TICKS*60*10) { say(cn,
        // "Thats all folks!"); remove_destroy_char(cn); return; }`
        // (`gatekeeper.c:668-672`).
        if self.tick.0.saturating_sub(data.creation_time) > GATE_FIGHT_SELF_DESTRUCT_TICKS {
            self.npc_say(fight_id, "Thats all folks!");
            self.remove_character(fight_id);
            return true;
        }

        // C `fight_driver_update(cn)` (`gatekeeper.c:674`), narrowed to the
        // single tracked `victim` (see module doc comment).
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&fight_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((fighter, victim)) => {
                    if char_see_char(&fighter, &victim, &self.map, self.date.daylight) {
                        data.victim_visible = true;
                        data.victim_last_x = victim.x;
                        data.victim_last_y = victim.y;
                    } else {
                        data.victim_visible = false;
                    }
                }
                None => {
                    // Victim gone (dead/removed/logged out): give up, same
                    // observable end state as C's `fight_driver_update`
                    // trashing a stale/deleted enemy slot.
                    data.victim = None;
                    data.victim_visible = false;
                }
            }
        }

        if let Some(character) = self.characters.get_mut(&fight_id) {
            character.driver_state = Some(CharacterDriverState::GateFight(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`
        // (`gatekeeper.c:676-678`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(fight_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`
            // (`gatekeeper.c:679-681`): walk toward the last known position;
            // give up once close enough without finding him there.
            let arrived = self.characters.get(&fight_id).is_some_and(|fighter| {
                fighter.x.abs_diff(data.victim_last_x) < 2
                    && fighter.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::GateFight(state)) = self
                    .characters
                    .get_mut(&fight_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                fight_id,
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

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN,
        // ret, lastact)) return;` (`gatekeeper.c:683-685`): return to the
        // post position. `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, the same
        // substitution `gate_enter_test_spawn_room` already made when
        // spawning this opponent (see `spawns.rs`).
        let (post_x, post_y) = self
            .characters
            .get(&fight_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            fight_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return; do_idle(cn, TICKS);` (`gatekeeper.c:687-693`).
        if self.regenerate_simple_baddy(fight_id) {
            return true;
        }
        if self.spell_self_simple_baddy(fight_id) {
            return true;
        }
        self.idle_simple_baddy(fight_id)
    }

    /// C `gate_fight_dead`'s tail (`gatekeeper.c:705-763`), given the
    /// killer's `gate_ppd.target_class` (`crate::player::PlayerRuntime::
    /// gate_target_class`, which `World` cannot read directly - the caller,
    /// `ugaris-server::world_events::apply_gate_fight_death_from_hurt_event`,
    /// supplies it). Always sends the "Well done." log (C's unconditional
    /// `log_char(co, LOG_SYSTEM, 0, "Well done.")` before the `switch`);
    /// applies the class-specific Arch flag/grats broadcast for classes
    /// 5-7 unless the guard fails (matching C's early `return` that also
    /// skips the final teleport). Class 8 needs `"seyan_m"`'s template
    /// base values (`World::apply_turn_seyan`, `world/turn_seyan.rs`) to
    /// do the reroll - `seyan_base_values` is `Some` only when the caller
    /// (which owns the `ZoneLoader`) could look that template up; falls
    /// back to an honest placeholder message otherwise, same as when
    /// `apply_turn_seyan` itself fails (e.g. `killer_id` vanished).
    pub fn apply_gate_fight_reward(
        &mut self,
        killer_id: CharacterId,
        target_class: i32,
        seyan_base_values: Option<&[i16]>,
    ) -> bool {
        let Some(killer) = self.characters.get(&killer_id).cloned() else {
            return false;
        };

        self.queue_system_text(killer_id, "Well done.");

        let mut skip_teleport = false;
        match target_class {
            // C `case 5: // arch warrior` (`gatekeeper.c:713-721`).
            5 => {
                if killer
                    .flags
                    .intersects(CharacterFlags::MAGE | CharacterFlags::ARCH)
                {
                    skip_teleport = true;
                } else {
                    if let Some(character) = self.characters.get_mut(&killer_id) {
                        character.flags.insert(CharacterFlags::ARCH);
                        character.values[1][CharacterValue::Rage as usize] = 1;
                    }
                    self.queue_system_text(killer_id, "You are an Arch-Warrior now.");
                    self.queue_gate_fight_grats(&killer.name, "an Arch-Warrior");
                }
            }
            // C `case 6: // arch mage` (`gatekeeper.c:722-730`).
            6 => {
                if killer
                    .flags
                    .intersects(CharacterFlags::WARRIOR | CharacterFlags::ARCH)
                {
                    skip_teleport = true;
                } else {
                    if let Some(character) = self.characters.get_mut(&killer_id) {
                        character.flags.insert(CharacterFlags::ARCH);
                        character.values[1][CharacterValue::Duration as usize] = 1;
                    }
                    self.queue_system_text(killer_id, "You are an Arch-Mage now.");
                    self.queue_gate_fight_grats(&killer.name, "an Arch-Mage");
                }
            }
            // C `case 7: // arch seyan'dr` (`gatekeeper.c:731-741`).
            7 => {
                if killer.flags.contains(CharacterFlags::ARCH)
                    || !killer.flags.contains(CharacterFlags::WARRIOR)
                    || !killer.flags.contains(CharacterFlags::MAGE)
                {
                    skip_teleport = true;
                } else {
                    if let Some(character) = self.characters.get_mut(&killer_id) {
                        character.flags.insert(CharacterFlags::ARCH);
                    }
                    self.queue_system_text(killer_id, "You are an Arch-Seyan'Du now.");
                    self.queue_gate_fight_grats(&killer.name, "an Arch-Seyan'Du");
                }
            }
            // C `case 8: // seyan'du` (`gatekeeper.c:742-748`): note both
            // guard checks are commented out in C, so this always runs
            // unconditionally (no `skip_teleport` case for 8).
            8 => {
                let turned = seyan_base_values
                    .is_some_and(|base_values| self.apply_turn_seyan(killer_id, base_values));
                if turned {
                    self.queue_system_text(killer_id, "You are a Seyan'Du now.");
                    self.queue_gate_fight_grats(&killer.name, "a Seyan'Du");
                } else {
                    self.queue_system_text(
                        killer_id,
                        "Turning into a Seyan'Du is not supported on this server build yet; you have been returned safely.",
                    );
                }
            }
            _ => {}
        }

        if !skip_teleport {
            // C `teleport_char_driver(co, 181, 198);` (`gatekeeper.c:762`,
            // unconditional after the `switch` unless a case above
            // `return`ed early).
            self.teleport_character(killer_id, 181, 198, false);
        }
        true
    }

    /// C `sprintf(buf, "0000000000" COL_MAUVE "Grats: %s is %s now!",
    /// ch[co].name, ...); server_chat(6, buf);` (`gatekeeper.c:717-718,
    /// 726-727, 736-737`) - only the article+title text differs per class.
    fn queue_gate_fight_grats(&mut self, name: &str, article_title: &str) {
        let mut message = b"0000000000".to_vec();
        message.extend_from_slice(crate::text::COL_MAUVE);
        message.extend_from_slice(format!("Grats: {name} is {article_title} now!").as_bytes());
        self.queue_channel_broadcast(6, message);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct gate_fight_driver_data` (`src/system/gatekeeper.c:636-639`):
/// the private-room opponent's own driver memory (`CDR_GATE_FIGHT`). Unlike
/// C's generic `struct fight_driver_data`/`DRD_FIGHTDRIVER` (a 10-slot enemy
/// list this driver never actually populates, since it only ever fights the
/// single `victim` set via the `NT_NPC`/`NTID_GATEKEEPER` message - see
/// `world::gate_fight`'s module doc comment), this only tracks that one
/// opponent plus its last-known position/visibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateFightDriverData {
    pub creation_time: u64,
    pub victim: Option<CharacterId>,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
    pub victim_visible: bool,
}
