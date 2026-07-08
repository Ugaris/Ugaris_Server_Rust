//! Vampire Lord boss NPC (`CDR_VAMPIRE`, the `vampire_lord` zone-file
//! template, `zones/2/below2.chr:2296-2367`).
//!
//! Ports `src/area/2/area2.c::vampire_driver` (`:574-653`): a fight NPC
//! that is normally unkillable (`CF_NODEATH`) except in the few seconds
//! after it has seen a player wearing the fully-assembled sun amulet
//! (`IID_AREA2_SUN123`), and otherwise alternates between returning to its
//! spawn point (right after a "death") and roaming toward a fixed crypt
//! tile once enough time has passed. Its death (once genuinely killable)
//! completes the "Toughest Monster" quest - see
//! `ugaris-server::world_events::apply_vampire_death_from_hurt_event`.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other area-2
//!   NPC (see `world::superior`'s module doc comment) - `standard_message_
//!   driver(cn, msg, 1, 0)` (`aggressive=1`) tracks one victim from both
//!   `NT_CHAR` sightings and `NT_GOTHIT` defense via `is_valid_enemy`.
//! - `fight_driver_set_dist(cn, 30, 0, 60)` (`area2.c:590`, on
//!   `NT_CREATE`) is not ported, same precedent as every sibling
//!   single-victim NPC.
//! - `fight_driver_set_home(cn, ch[cn].x, ch[cn].y)` (`area2.c:637`,
//!   `:642`) re-homes C's generic distance-gated engine to the vampire's
//!   current tile after every return-to-post/roam attempt; since this
//!   port never reads a "home" position for gating (no `fight_driver_set_
//!   dist` equivalent - see above), this call has no observable effect on
//!   the ported behavior and is not ported, matching precedent.
//! - `killed` (`struct vampire_driver_data`) is initialized to the
//!   current tick the first time this NPC is ever processed (this port's
//!   only observation point for "just spawned", since `Zone Loader`
//!   doesn't have `World`'s tick counter available at spawn time),
//!   exactly matching C's own `NT_CREATE` handler (`dat->killed =
//!   ticker;`, `area2.c:589`) firing on the character's first tick.

use crate::world::*;

/// C `IID_AREA2_SUN123` neck-check window: `ticker - dat->amulet <
/// TICKS*5` (`area2.c:610`).
const VAMPIRE_AMULET_VULNERABLE_TICKS: u64 = 5;
/// C `ticker - dat->amulet > TICKS*10` (`area2.c:596`): re-announcement
/// cooldown for "Oh no!".
const VAMPIRE_AMULET_ANNOUNCE_COOLDOWN_TICKS: u64 = 10;
/// C `ticker - dat->killed < TICKS*120` (`area2.c:633`): how long after a
/// "death" the vampire keeps returning to its own spawn tile before
/// roaming to the fixed crypt tile instead.
const VAMPIRE_RETURN_HOME_TICKS: u64 = 120;
/// C `POWERSCALE/2` (`area2.c:616`): the near-death HP threshold that
/// triggers the fake-death mist-teleport escape while vulnerable.
const VAMPIRE_NEAR_DEATH_HP: i32 = crate::entity::POWERSCALE / 2;
/// C `secure_move_driver(cn, 232, 123, ...)` (`area2.c:639`): the fixed
/// crypt tile the vampire roams to once past the return-home window.
const VAMPIRE_ROAM_TILE: (u16, u16) = (232, 123);

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_VAMPIRE`
    /// characters (C `ch_driver`'s `CDR_VAMPIRE` case, `area2.c:994-996`).
    pub fn process_vampire_actions(&mut self, area_id: u16) -> usize {
        let vampire_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_VAMPIRE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for vampire_id in vampire_ids {
            if self.process_vampire_tick(vampire_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `vampire_driver`'s per-tick body (`area2.c:574-653`).
    fn process_vampire_tick(&mut self, vampire_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&vampire_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Vampire(data)) => data,
            // C `NT_CREATE`: `dat->killed = ticker;` - see module doc
            // comment for why this port seeds it here instead.
            _ => VampireDriverData {
                killed: self.tick.0,
                ..Default::default()
            },
        };

        let messages = self
            .characters
            .get_mut(&vampire_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    // C `area2.c:592-601`: does the seen character wear
                    // the assembled sun amulet?
                    let wears_amulet = self
                        .characters
                        .get(&seen_id)
                        .and_then(|seen| seen.inventory.get(worn_slot::NECK).copied().flatten())
                        .and_then(|item_id| self.items.get(&item_id))
                        .is_some_and(|item| item.template_id == IID_AREA2_SUN123);
                    if wears_amulet {
                        if self.tick.0.saturating_sub(data.amulet)
                            > VAMPIRE_AMULET_ANNOUNCE_COOLDOWN_TICKS * TICKS_PER_SECOND
                        {
                            self.npc_say(vampire_id, "Oh no!");
                        }
                        data.amulet = self.tick.0;
                    }
                    if self.vampire_is_valid_enemy(vampire_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    if self.vampire_is_valid_enemy(vampire_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `area2.c:610-614`.
        let vulnerable = self.tick.0.saturating_sub(data.amulet)
            < VAMPIRE_AMULET_VULNERABLE_TICKS * TICKS_PER_SECOND;
        if let Some(character) = self.characters.get_mut(&vampire_id) {
            if vulnerable {
                character.flags.remove(CharacterFlags::NODEATH);
            } else {
                character.flags.insert(CharacterFlags::NODEATH);
            }
        }

        // C `area2.c:616-622`: near-death while vulnerable -> fake-death
        // mist escape back to the spawn tile instead of really dying.
        let near_death = self.characters.get(&vampire_id).is_some_and(|vampire| {
            vampire.hp < VAMPIRE_NEAR_DEATH_HP && vampire.flags.contains(CharacterFlags::NODEATH)
        });
        if near_death {
            if let Some(character) = self.characters.get_mut(&vampire_id) {
                character.driver_state = Some(CharacterDriverState::Vampire(data));
            }
            let (spawn_x, spawn_y) = self
                .characters
                .get(&vampire_id)
                .map(|vampire| (vampire.x, vampire.y))
                .unwrap_or_default();
            self.create_mist_effect(i32::from(spawn_x), i32::from(spawn_y));
            let (post_x, post_y) = self
                .characters
                .get(&vampire_id)
                .map(|vampire| (vampire.rest_x, vampire.rest_y))
                .unwrap_or_default();
            self.teleport_character(vampire_id, post_x, post_y, false);
            if let Some(vampire) = self.characters.get_mut(&vampire_id) {
                vampire.hp = crate::entity::POWERSCALE;
            }
            if let Some(CharacterDriverState::Vampire(state)) = self
                .characters
                .get_mut(&vampire_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                state.killed = self.tick.0;
            }
            return true;
        }

        // C `fight_driver_update(cn)`.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&vampire_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((vampire, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&vampire, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&vampire_id) {
            character.driver_state = Some(CharacterDriverState::Vampire(data));
        }

        // C `area2.c:624-631`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(vampire_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            let arrived = self.characters.get(&vampire_id).is_some_and(|vampire| {
                vampire.x.abs_diff(data.victim_last_x) < 2
                    && vampire.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Vampire(state)) = self
                    .characters
                    .get_mut(&vampire_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                vampire_id,
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

        // C `area2.c:633-643`.
        let (target_x, target_y) = if self.tick.0.saturating_sub(data.killed)
            < VAMPIRE_RETURN_HOME_TICKS * TICKS_PER_SECOND
        {
            self.characters
                .get(&vampire_id)
                .map(|character| (character.rest_x, character.rest_y))
                .unwrap_or_default()
        } else {
            VAMPIRE_ROAM_TILE
        };
        if self.secure_move_driver(
            vampire_id,
            target_x,
            target_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `area2.c:645-648`.
        if self.regenerate_simple_baddy(vampire_id) {
            return true;
        }
        if self.spell_self_simple_baddy(vampire_id) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`area2.c:652`).
        self.idle_simple_baddy(vampire_id)
    }

    /// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`).
    fn vampire_is_valid_enemy(&self, character_id: CharacterId, target_id: CharacterId) -> bool {
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

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_VAMPIRE;
use crate::item_driver::IID_AREA2_SUN123;

/// C `struct vampire_driver_data` (`area2.c:569-572`), plus this port's
/// own single-victim self-defense tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VampireDriverData {
    pub killed: u64,
    pub amulet: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
