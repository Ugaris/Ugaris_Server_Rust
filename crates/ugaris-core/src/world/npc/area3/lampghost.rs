//! Lamp-extinguisher ghost NPC (`CDR_LAMPGHOST`), area 3's palace-light
//! puzzle janitor.
//!
//! Ports `src/area/3/area3.c::lampghost_driver` (`:2631-2727`) plus its
//! two driver-table siblings, `lampghost_respawn` (`:2729-2739`) and
//! `lampghost_dead` (`:2741-2752`): a self-defense/aggressive-sighting
//! grunt (same `standard_message_driver(cn, msg, 1, 0)` shape as
//! `CDR_SUPERIOR`) whose idle job, once no enemy is being fought, is to
//! walk to and extinguish the nearest currently-lit palace lamp
//! (`IDR_ONOFFLIGHT` items registered via `add_lamp`/`onofflight_driver`'s
//! automatic-call path - see `world::light`'s `schedule_registered_area3_
//! lamp_extinguish`) - competing with every other lampghost for whichever
//! lamp is cheapest to reach.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for `CDR_SUPERIOR`/`CDR_ROBBER`/etc: C's generic 10-slot `struct
//!   fight_driver_data` is narrowed to a single tracked `victim`, using
//!   the exact `is_valid_enemy(cn, co, -1)` predicate on both `NT_CHAR`
//!   sightings (`aggressive=1`) and `NT_GOTHIT` self-defense.
//! - C's `lamp[MAXLAMP]` global array (`in`/`cn`/`cost` per slot, filled
//!   by `add_lamp`) is replaced by two pieces of existing/new state
//!   instead of a parallel registry: "is this item a registered lamp" is
//!   already `it[in].drdata[6] != 0` (ported as `Item::driver_data[6]` -
//!   see `world::light`'s `schedule_registered_area3_lamp_extinguish`,
//!   which scans this exact flag), and "who currently claims this lamp,
//!   at what cost" is the new [`World::area3_lamp_claims`] map, keyed
//!   directly by `ItemId` instead of a `lamp[]` slot index.
//! - C's target-selection loop breaks cost ties by keeping whichever
//!   candidate was visited *first* in `lamp[]` registration order (an
//!   implementation detail with no in-game observable meaning - any tied
//!   lamp is an equally valid target). This port breaks ties by lowest
//!   `ItemId` instead, the same substitution already used by
//!   `world::janitor`'s nearest-light/nearest-take-item scans.
//! - `fight_driver_set_dist(cn, 80, 0, 80)` (`area3.c:2646`, on
//!   `NT_CREATE`) is not ported, same precedent as every other
//!   single-victim NPC's own module doc comment.
//! - `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
//!   lastact)` substitutes C's plain `move_driver(cn, ch[cn].tmpx,
//!   ch[cn].tmpy, 0)` return-to-post call for the final idle-walk step,
//!   the same substitution already used by `world::astro1`/`world::
//!   robber`/`world::superior` (a strict superset: falls back to
//!   `secure_move_driver`'s teleport/turn-to-face only when the simpler
//!   `move_driver` would already return `0`, so never observably
//!   different from C in practice).
//! - `lampghost_respawn`'s `map[m].light > 4` gate (block respawn while
//!   the palace is still lit) is ported in `ugaris-server`'s
//!   `spawns::respawn_npc_character`, the one place that already has
//!   direct `ZoneLoader`/tile access for the generic NPC respawn path -
//!   see that function's own `CDR_LAMPGHOST` branch.
//! - `lampghost_dead`'s claim release is ported as
//!   `World::release_lampghost_lamp_claim`, called from
//!   `ugaris-server`'s `apply_lampghost_death_from_hurt_event` death
//!   hook (`world_events/death_hooks.rs`) instead of `immortal_dead`
//!   (unlike every other area-3 quest NPC's shared no-op death hook,
//!   `CDR_LAMPGHOST` has its own).

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_LAMPGHOST`
    /// characters (C `ch_driver`'s `CDR_LAMPGHOST` case, `area3.c:2884-
    /// 2886`).
    pub fn process_lampghost_actions(&mut self, area_id: u16) -> usize {
        let lampghost_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAMPGHOST
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for lampghost_id in lampghost_ids {
            if self.process_lampghost_tick(lampghost_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `lampghost_driver`'s per-tick body (`area3.c:2631-2727`).
    fn process_lampghost_tick(&mut self, lampghost_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&lampghost_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Lampghost(data)) => data,
            _ => LampghostDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&lampghost_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `standard_message_driver`'s `NT_CHAR` branch
                // (`drvlib.c:2470-2476`, `aggressive=1`): any newly-seen
                // valid enemy becomes the tracked victim.
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    if self.lampghost_is_valid_enemy(lampghost_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`): defend against whoever hit us.
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    if self.lampghost_is_valid_enemy(lampghost_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `ch[cn].speed_mode = SM_NORMAL;` (`area3.c:2654`).
        if let Some(character) = self.characters.get_mut(&lampghost_id) {
            character.speed_mode = SpeedMode::Normal;
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&lampghost_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((lampghost, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&lampghost, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&lampghost_id) {
            character.driver_state = Some(CharacterDriverState::Lampghost(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(lampghost_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let arrived = self.characters.get(&lampghost_id).is_some_and(|lampghost| {
                lampghost.x.abs_diff(data.victim_last_x) < 2
                    && lampghost.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Lampghost(state)) = self
                    .characters
                    .get_mut(&lampghost_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                lampghost_id,
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

        // C `ch[cn].speed_mode = SM_STEALTH;` (`area3.c:2665`).
        if let Some(character) = self.characters.get_mut(&lampghost_id) {
            character.speed_mode = SpeedMode::Stealth;
        }

        // C `if (spell_self_driver(cn)) return;` (`area3.c:2667-2669`).
        if self.spell_self_simple_baddy(lampghost_id) {
            return true;
        }
        // C `if (regenerate_driver(cn)) return;` (`area3.c:2670-2672`).
        if self.regenerate_simple_baddy(lampghost_id) {
            return true;
        }

        // C `area3.c:2674-2720`: hold on to (or find, then claim) the
        // nearest currently-lit registered lamp, and walk over to
        // extinguish it.
        let mut state = match self
            .characters
            .get(&lampghost_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Lampghost(data)) => data,
            _ => LampghostDriverData::default(),
        };

        // C `area3.c:2674-2683`: drop the current job if the lamp already
        // went dark, or if somebody else claimed it out from under us.
        if let Some(claimed_item_id) = state.claimed_lamp {
            let still_lit = self
                .items
                .get(&claimed_item_id)
                .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) != 0);
            if !still_lit {
                self.area3_lamp_claims.remove(&claimed_item_id);
                state.claimed_lamp = None;
            } else if !self
                .area3_lamp_claims
                .get(&claimed_item_id)
                .is_some_and(|&(claimant, _)| claimant == lampghost_id)
            {
                state.claimed_lamp = None;
            }
        }

        // C `area3.c:2685-2713`.
        if state.claimed_lamp.is_none() {
            state.claimed_lamp = self.lampghost_find_lamp(lampghost_id);
        }

        if let Some(character) = self.characters.get_mut(&lampghost_id) {
            character.driver_state = Some(CharacterDriverState::Lampghost(state));
        }

        // C `area3.c:2715-2720`: `if (use_driver(cn, in, 0)) return;`.
        if let Some(lamp_id) = state.claimed_lamp {
            if self.lampghost_use_lamp(lampghost_id, lamp_id, area_id) {
                return true;
            }
        }

        // C `move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, 0)` (`area3.c:
        // 2722-2724`): return to post - `secure_move_driver` substitution,
        // see module doc comment.
        let (post_x, post_y) = self
            .characters
            .get(&lampghost_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            lampghost_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`area3.c:2726`).
        self.idle_simple_baddy(lampghost_id)
    }

    /// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`).
    fn lampghost_is_valid_enemy(&self, character_id: CharacterId, target_id: CharacterId) -> bool {
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

    /// C `area3.c:2685-2713`: scan every registered (`it[in].drdata[6] !=
    /// 0`), currently-lit (`it[in].drdata[0] != 0`) lamp for the closest
    /// one not already claimed by a strictly-cheaper competitor, then
    /// claim it. Returns `None` (C's `bestn == 0`) when no lamp qualifies.
    fn lampghost_find_lamp(&mut self, lampghost_id: CharacterId) -> Option<ItemId> {
        let (lx, ly) = self
            .characters
            .get(&lampghost_id)
            .map(|lampghost| (lampghost.x, lampghost.y))?;

        let mut candidates: Vec<(ItemId, u16, u16)> = self
            .items
            .values()
            .filter(|item| {
                item.driver == IDR_ONOFFLIGHT
                    && item.driver_data.get(6).copied().unwrap_or(0) != 0
                    && item.driver_data.first().copied().unwrap_or(0) != 0
            })
            .map(|item| (item.id, item.x, item.y))
            .collect();
        // Deterministic tie-break substitution for C's registration-order
        // tie-break - see module doc comment.
        candidates.sort_by_key(|(item_id, _, _)| item_id.0);

        let mut best: Option<(i32, ItemId)> = None;
        for (item_id, ix, iy) in candidates {
            let cost = map_dist(lx, ly, ix, iy);
            if let Some((best_cost, _)) = best {
                if cost >= best_cost {
                    continue;
                }
            }
            if let Some(&(claimant, claimed_cost)) = self.area3_lamp_claims.get(&item_id) {
                if claimant != lampghost_id && cost >= claimed_cost {
                    continue;
                }
            }
            best = Some((cost, item_id));
        }

        let (cost, item_id) = best?;
        self.area3_lamp_claims.insert(item_id, (lampghost_id, cost));
        Some(item_id)
    }

    /// C `use_driver(cn, in, 0)` for the claimed lamp: walk to it if not
    /// already adjacent, then use it (toggling it off via `IDR_ONOFFLIGHT`'s
    /// item driver on action completion). Identical shape to `World::
    /// janitor_use_light`/`World::robber_use_waypoint_item`.
    fn lampghost_use_lamp(
        &mut self,
        lampghost_id: CharacterId,
        lamp_id: ItemId,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&lamp_id).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(lampghost) = self.characters.get(&lampghost_id) else {
            return false;
        };
        let direction = adjacent_use_direction(
            lampghost.x,
            lampghost.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        );
        if let Some(direction) = direction {
            let Some(lampghost) = self.characters.get_mut(&lampghost_id) else {
                return false;
            };
            do_use(
                lampghost,
                &self.map,
                &item,
                direction as u8,
                0,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward_use_item(
                lampghost_id,
                usize::from(item.x),
                usize::from(item.y),
                item.flags,
                area_id,
            )
        }
    }

    /// C `lampghost_dead` (`area3.c:2741-2752`): release this lampghost's
    /// lamp claim, if any, so another lampghost (or the same one on
    /// respawn) can pick it up.
    pub fn release_lampghost_lamp_claim(&mut self, lampghost_id: CharacterId) {
        self.area3_lamp_claims
            .retain(|_, &mut (claimant, _)| claimant != lampghost_id);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_LAMPGHOST;

/// C `struct lampghost_driver_data` (`area3.c:2627-2629`, just `int ln`),
/// plus this port's own single-victim self-defense tracking (see module
/// doc comment). `ln` (the C `lamp[]` slot index) becomes `claimed_lamp`
/// (a direct `ItemId`, since this port has no parallel slot array).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LampghostDriverData {
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
    pub claimed_lamp: Option<ItemId>,
}
