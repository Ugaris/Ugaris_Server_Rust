//! Vampire Lord 2 boss NPC (`CDR_VAMPIRE2`, the `vampire_lord2` zone-file
//! template, `zones/2/below2.chr:2656-2720`), the "strong_vampire"
//! crypt boss.
//!
//! Ports `src/area/2/area2.c::vampire2_driver` (`:655-725`): an otherwise
//! ordinary fight NPC with one special rule - whoever hits it while
//! wielding the "right" ceremonial dagger (`IID_AREA2_DAGGERRIGHT`) kills
//! it outright (crediting them as the killer); whoever hits it with the
//! visually-identical "wrong" dagger (`IID_AREA2_DAGGERWRONG`) merely
//! shatters that dagger. Its death (via the right dagger) completes the
//! "Toughestest Monster" quest - see
//! `ugaris-server::world_events::apply_vampire2_death_from_hurt_event`.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other area-2
//!   NPC (see `world::superior`'s module doc comment).
//! - `fight_driver_set_dist(cn, 30, 0, 60)` (`area2.c:664`, on
//!   `NT_CREATE`) is not ported, same precedent as every sibling
//!   single-victim NPC.
//! - `kill_char(cn, co)` (`area2.c:671`, the correct-dagger instant kill)
//!   has no direct primitive in this codebase - C's `kill_char` bypasses
//!   `hurt`'s armor/shield/HP math entirely and jumps straight to the
//!   death driver/respawn/kill-score bookkeeping. This port instead
//!   routes through `World::apply_legacy_hurt` with `armor_percent=0`
//!   (`reduce_hurt_by_armor` then applies zero armor reduction) and a
//!   damage value far beyond any realistic HP pool, which reaches the
//!   exact same death bookkeeping (`kill_character_followup`: respawn
//!   registration, kill-score exp, achievements, first-kill, military
//!   mission checks) through the normal death path instead of a bespoke
//!   one - the only observable difference is a debug-only "hurt by ..."
//!   log line gated on `show_attack_debug`, which this NPC's boss-tier HP
//!   pool would never realistically show anyway.
//! - `dlog(co, in, "dropped because vampire was killed/not killed with
//!   it")` (`area2.c:674`, `:688`) is a server-side audit log with no
//!   observable client effect and is not ported, same precedent as every
//!   other `dlog` call in this codebase (see e.g. `world::james`'s module
//!   doc comment).

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_VAMPIRE2`
    /// characters (C `ch_driver`'s `CDR_VAMPIRE2` case, `area2.c:997-999`).
    pub fn process_vampire2_actions(&mut self, area_id: u16) -> usize {
        let vampire2_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_VAMPIRE2
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for vampire2_id in vampire2_ids {
            if self.process_vampire2_tick(vampire2_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `vampire2_driver`'s per-tick body (`area2.c:655-725`).
    fn process_vampire2_tick(&mut self, vampire2_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&vampire2_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Vampire2(data)) => data,
            _ => Vampire2DriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&vampire2_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    // C `area2.c:667-694`: the dagger check takes priority
                    // over the generic defend branch and, when it fires,
                    // returns immediately - skipping every remaining
                    // message and the rest of this tick's body.
                    if self
                        .vampire2_check_dagger_hit(vampire2_id, attacker_id)
                        .is_some()
                    {
                        return true;
                    }
                    if self.vampire2_is_valid_enemy(vampire2_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                }
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    if self.vampire2_is_valid_enemy(vampire2_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                _ => {}
            }
        }

        // C `fight_driver_update(cn)`.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&vampire2_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((vampire2, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&vampire2, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&vampire2_id) {
            character.driver_state = Some(CharacterDriverState::Vampire2(data));
        }

        // C `area2.c:706-711`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(vampire2_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            let arrived = self.characters.get(&vampire2_id).is_some_and(|vampire2| {
                vampire2.x.abs_diff(data.victim_last_x) < 2
                    && vampire2.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Vampire2(state)) = self
                    .characters
                    .get_mut(&vampire2_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                vampire2_id,
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

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)` (`area2.c:713-716`): return to post (`rest_x`/
        // `rest_y` substitution, same as every sibling NPC).
        let (post_x, post_y) = self
            .characters
            .get(&vampire2_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            vampire2_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `area2.c:718-721`.
        if self.regenerate_simple_baddy(vampire2_id) {
            return true;
        }
        if self.spell_self_simple_baddy(vampire2_id) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`area2.c:724`).
        self.idle_simple_baddy(vampire2_id)
    }

    /// C `area2.c:667-694`: check whether the attacker's right-hand
    /// weapon is one of the two ceremonial daggers, and apply the correct
    /// consequence. Returns `None` when neither dagger was wielded (so
    /// the caller should fall through to the generic defend branch).
    fn vampire2_check_dagger_hit(
        &mut self,
        vampire2_id: CharacterId,
        attacker_id: CharacterId,
    ) -> Option<VampireDaggerOutcome> {
        let attacker = self.characters.get(&attacker_id)?;
        if attacker.flags.is_empty() {
            return None;
        }
        let weapon_id = attacker
            .inventory
            .get(worn_slot::RIGHT_HAND)
            .copied()
            .flatten()?;
        let weapon_template = self.items.get(&weapon_id)?.template_id;

        if weapon_template == IID_AREA2_DAGGERRIGHT {
            self.npc_say(vampire2_id, "Arrrgh!");
            // C `kill_char(cn, co)` - see module doc comment.
            self.apply_legacy_hurt(vampire2_id, Some(attacker_id), i32::MAX / 2, 1, 0, 0);
            self.destroy_item(weapon_id);
            self.queue_system_text(attacker_id, "Your dagger was destroyed in the blow.");
            self.destroy_items_by_template_id(attacker_id, IID_AREA2_DAGGERRIGHT);
            self.destroy_items_by_template_id(attacker_id, IID_AREA2_DAGGERWRONG);
            return Some(VampireDaggerOutcome::Killed);
        }
        if weapon_template == IID_AREA2_DAGGERWRONG {
            self.npc_say(vampire2_id, "Hahahaha!");
            self.destroy_item(weapon_id);
            self.queue_system_text(attacker_id, "Your dagger was destroyed in the blow.");
            return Some(VampireDaggerOutcome::WrongDagger);
        }
        None
    }

    /// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`).
    fn vampire2_is_valid_enemy(&self, character_id: CharacterId, target_id: CharacterId) -> bool {
        if character_id == target_id {
            return false;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        character.group != target.group
            && can_attack(character, target, &self.map)
            && char_see_char(character, target, &self.map, self.date.daylight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VampireDaggerOutcome {
    Killed,
    WrongDagger,
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_VAMPIRE2;
use crate::item_driver::{IID_AREA2_DAGGERRIGHT, IID_AREA2_DAGGERWRONG};

/// C `vampire2_driver` has no `struct ...driver_data` of its own (it never
/// calls `set_data`) - this port's own single-victim self-defense
/// tracking (see module doc comment) is the only state it needs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Vampire2DriverData {
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
