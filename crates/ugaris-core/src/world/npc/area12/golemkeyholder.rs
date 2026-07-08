//! Mine-vault keyholder golem (`CDR_GOLEMKEYHOLDER`).
//!
//! Ports `src/area/12/mine.c::keyhold_fight_driver` - the boss golem
//! `keyholder_door`/`IDR_MINEKEYDOOR` (`World::apply_item_driver_outcome`'s
//! `ItemDriverOutcome::MineKeyDoor` handling, `world/item_outcomes.rs`)
//! spawns into whichever of the 9 fixed vault rooms is currently empty
//! (`ugaris-server::mine::spawn_keyholder_golem`) once a player pays 2000
//! gold to open the door. `mine.c`'s own `ch_died_driver` case for this
//! driver is `return 1;` with no body - a "claimed, no reward" declaration
//! - so no death hook exists for this NPC, unlike `CDR_GATE_FIGHT`'s
//! `gate_fight_dead`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `keyhold_fight_driver` is *byte-for-byte* identical to
//!   `src/system/gatekeeper.c::gate_fight_driver` (`world::npc::gate_fight`,
//!   already ported) except for one self-destruct timeout constant
//!   (`TICKS*60*5` here vs. `TICKS*60*10` there) and the absence of the
//!   `NT_NPC`/`NTID_GATEKEEPER` victim-assignment message (`keyholder_door`
//!   never sends one). Both drivers share the same underlying mechanism for
//!   picking their one opponent in practice: `standard_message_driver(cn,
//!   msg, 1, 0)`'s `NT_CHAR` branch adds any visible valid enemy as soon as
//!   one is seen (`drvlib.c:2470-2476`), and each private room only ever
//!   contains the single summoning player, so the two mechanisms produce
//!   the same observable "attacks the player who let it in" behavior. This
//!   port takes advantage of already knowing that player's id at spawn time
//!   (`spawn_keyholder_golem` sets `victim` directly) instead of modeling
//!   the generic `NT_CHAR`-triggered enemy-list machinery, the same
//!   single-victim simplification precedent already established by
//!   `world::npc::gate_fight`/`area1::asturin`/`area1::robber`/
//!   `area1::sanoa`.
//! - "attack visible" reuses `World::attack_driver_direct` and "follow
//!   invisible" reuses `secure_move_driver` toward the last known position,
//!   same substitutions `gate_fight` already made for C's generic
//!   `fight_driver_attack_visible`/`fight_driver_follow_invisible`.

use crate::world::*;

/// C `TICKS * 60 * 5` (`mine.c:1242`): the keyholder golem self-destructs
/// after 5 minutes if nobody kills it (half of `gate_fight_driver`'s
/// 10-minute timeout - see module doc comment).
const GOLEMKEYHOLD_SELF_DESTRUCT_TICKS: u64 = TICKS_PER_SECOND * 60 * 5;

impl World {
    /// C `keyhold_fight_driver`'s per-tick body (`mine.c:1216-1270`).
    pub fn process_golemkeyhold_actions(&mut self, area_id: u16) -> usize {
        let golem_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GOLEMKEYHOLDER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for golem_id in golem_ids {
            if self.process_golemkeyhold_tick(golem_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    fn process_golemkeyhold_tick(&mut self, golem_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&golem_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::GolemKeyhold(data)) => data,
            _ => GolemKeyholdDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&golem_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            // C `if (msg->type == NT_CREATE) dat->creation_time = ticker;`
            // (`mine.c:1229-1232`).
            if message.message_type == NT_CREATE {
                data.creation_time = self.tick.0;
            }
        }

        // C `if (ticker - dat->creation_time > TICKS*60*5) { say(cn,
        // "Thats all folks!"); remove_destroy_char(cn); return; }`
        // (`mine.c:1242-1246`).
        if self.tick.0.saturating_sub(data.creation_time) > GOLEMKEYHOLD_SELF_DESTRUCT_TICKS {
            self.npc_say(golem_id, "Thats all folks!");
            self.remove_character(golem_id);
            return true;
        }

        // C `fight_driver_update(cn)` (`mine.c:1248`), narrowed to the
        // single tracked `victim` (see module doc comment).
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&golem_id)
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

        if let Some(character) = self.characters.get_mut(&golem_id) {
            character.driver_state = Some(CharacterDriverState::GolemKeyhold(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`
        // (`mine.c:1250-1252`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(golem_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`
            // (`mine.c:1253-1255`): walk toward the last known position;
            // give up once close enough without finding him there.
            let arrived = self.characters.get(&golem_id).is_some_and(|fighter| {
                fighter.x.abs_diff(data.victim_last_x) < 2
                    && fighter.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::GolemKeyhold(state)) = self
                    .characters
                    .get_mut(&golem_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                golem_id,
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
        // ret, lastact)) return;` (`mine.c:1257-1259`): return to the spawn
        // position. `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, same
        // substitution `gate_fight`'s spawn already established.
        let (post_x, post_y) = self
            .characters
            .get(&golem_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            golem_id,
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
        // return; do_idle(cn, TICKS);` (`mine.c:1261-1269`).
        if self.regenerate_simple_baddy(golem_id) {
            return true;
        }
        if self.spell_self_simple_baddy(golem_id) {
            return true;
        }
        self.idle_simple_baddy(golem_id)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct gate_fight_driver_data` reused verbatim by `keyhold_fight_
/// driver` (`mine.c:1211-1214`) - identical field shape to
/// [`crate::world::npc::gate_fight::GateFightDriverData`], kept as its own
/// type per this codebase's one-file-per-NPC convention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GolemKeyholdDriverData {
    pub creation_time: u64,
    pub victim: Option<CharacterId>,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
    pub victim_visible: bool,
}
