//! Pentagram QA tester bot (`CDR_TESTER`).
//!
//! Ports `src/area/4/pents.c::pentagram_tester_driver` (`:1789-1830`), the
//! last piece of Area 4's `pents.c` file (see `world::pents`'s module doc
//! comment and `PORTING_TODO.md`'s Area 4 entry). This is a QA-only test
//! bot - a "tester" character template exists in the zone data
//! (`ugaris_data/zones/4/pents.chr`, `driver=77`) but is never placed on
//! any map or spawned by any C code path (`grep CDR_TESTER src/**` only
//! ever matches the `ch_driver`/`ch_respawn_driver` dispatch cases), so it
//! is not player-facing; still ported to the same fidelity bar as every
//! other NPC in this codebase.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `scan_item_driver`/message-notify caching (`item_to_pickup`,
//!   `item_to_use`, the 10-slot `visible_pents[]` round-robin) is replaced
//!   by a direct per-tick nearest-visible-candidate scan of `World::items`,
//!   the same class of simplification already established by
//!   `world/npc/janitor.rs` (see that module's own doc comment) - the
//!   difference being C's candidate is whichever pentagram/potion was
//!   *sighted first* (bounded by a 10-slot cache), this port's is whichever
//!   is *nearest*. `tester_state->target_character` (`pents.c:187`, set
//!   from `NT_CHAR` sightings of `CDR_PENTER` demons) is genuinely a
//!   write-only dead field in C - it is never read anywhere else in the
//!   driver - so it is not ported at all.
//! - Self-defense uses the same single-victim `NT_GOTHIT`-only model
//!   already established for `CDR_SANOA`/`CDR_ROBBER` (see `sanoa.rs`'s
//!   module doc comment for the full justification) in place of C's
//!   generic 10-slot `fight_driver_data` enemy list; `standard_message_
//!   driver(cn, msg, 0, 0)`'s hardcoded `aggressive=0, helper=0` means
//!   C's own enemy list only ever gains entries from `NT_GOTHIT` too, so
//!   this is behaviorally equivalent for a single attacker.
//! - `handle_tester_item_management` is a `void` C function whose internal
//!   `return;` (after successfully queuing a take/use action) only exits
//!   that helper, not `pentagram_tester_driver` itself - C's caller does
//!   *not* check whether an item action was queued before going on to call
//!   `fight_driver_attack_visible`/`regenerate_driver`/etc., which (since
//!   none of the underlying `do_walk`/`do_use`/`do_take` raw actions guard
//!   against clobbering an already-pending action) can let a later helper
//!   silently overwrite the queued take/use action within the same tick.
//!   This looks like an unintentional C bug rather than deliberate
//!   behavior; this port instead follows the established precedent from
//!   every other NPC in this codebase (`sanoa.rs`/`robber.rs`/etc.):
//!   return as soon as any step successfully queues an action for the
//!   tick.
//! - The pentagram-activation/quest-reward pipeline the tester's `use`
//!   action triggers (`World::apply_pentagram_activate` via the generic
//!   item-use completion pipeline, same as a player's) already no-ops its
//!   per-player reward half for a non-player activator (`ugaris-server`'s
//!   `pents::apply_pentagram_activation` returns early when `runtime.
//!   player_for_character_mut` finds no session) - no special-casing
//!   needed here.
//! - `handle_tester_healing` (ported as `tester_handle_healing`) is
//!   genuinely unreachable in C: `pentagram_tester_driver` calls
//!   `regenerate_driver(cn)` (return-and-idle whenever `hp < max_hp`)
//!   *before* `handle_tester_healing` (return-and-drink whenever `hp <
//!   max_hp * heal_threshold`, `heal_threshold <= 1.0`), and every `hp`
//!   that satisfies the second, stricter condition also satisfies the
//!   first - so `regenerate_driver` always intercepts first and `handle_
//!   tester_healing`'s own body can never execute. Ported anyway (same
//!   "port it, note the oddity" policy as `target_character`), but not
//!   unit-tested on its own since the full per-tick dispatch order can
//!   never reach it (a full-tick test would just be asserting `regenerate_
//!   simple_baddy`'s already-covered idle behavior).

use crate::world::*;

/// C `#define SCANDIST 20` (`drvlib.c:560`): the box `scan_item_driver`
/// notifies items within - reused here as the direct-scan search radius
/// (see module doc comment).
const TESTER_SCAN_DIST: usize = 20;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_TESTER`
    /// characters (C `ch_driver`'s `CDR_TESTER` case, `pents.c:1848-1850`).
    pub fn process_tester_actions(&mut self, area_id: u16) -> usize {
        let tester_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TESTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for tester_id in tester_ids {
            if self.process_tester_tick(tester_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `pentagram_tester_driver`'s per-tick body (`pents.c:1789-1830`).
    fn process_tester_tick(&mut self, tester_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&tester_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Tester(data)) => data,
            _ => TesterDriverData::default(),
        };

        // C `process_tester_messages`, narrowed to the `NT_GOTHIT`
        // self-defense branch - see module doc comment.
        let messages = self
            .characters
            .get_mut(&tester_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        for message in &messages {
            if message.message_type != NT_GOTHIT || message.dat1 <= 0 {
                continue;
            }
            let attacker_id = CharacterId(message.dat1 as u32);
            if let Some((tester, attacker)) = self
                .characters
                .get(&tester_id)
                .cloned()
                .zip(self.characters.get(&attacker_id).cloned())
            {
                if tester.group != attacker.group && can_attack(&tester, &attacker, &self.map) {
                    data.victim = Some(attacker_id);
                }
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&tester_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((tester, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&tester, &victim, &self.map, self.date.daylight) {
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

        // C `handle_tester_item_management`'s cursor-item shuffle: not
        // tick-consuming (`say`/`swap`/`destroy_item` are all synchronous),
        // so it never causes an early return.
        self.tester_handle_cursor_item(tester_id);

        // C's `item_to_pickup`/`take_driver` pair.
        if data.item_to_pickup.is_none() {
            if let Some(tester) = self.characters.get(&tester_id).cloned() {
                data.item_to_pickup = self.find_nearest_tester_pickup_item(&tester);
            }
        }
        if let Some(item_id) = data.item_to_pickup {
            let still_there = self.items.contains_key(&item_id);
            if still_there && self.tester_take_item(tester_id, item_id, area_id) {
                if let Some(character) = self.characters.get_mut(&tester_id) {
                    character.driver_state = Some(CharacterDriverState::Tester(data));
                }
                return true;
            }
            data.item_to_pickup = None;
        }

        // C's `item_to_use`/`use_driver` pair (find a fresh candidate if
        // needed, then try to use it while it's still unsolved).
        if data.item_to_use.is_none() {
            if let Some(tester) = self.characters.get(&tester_id).cloned() {
                data.item_to_use = self.find_nearest_tester_pentagram(&tester);
            }
        }
        if let Some(item_id) = data.item_to_use {
            let still_unsolved = self
                .items
                .get(&item_id)
                .is_some_and(|item| item.driver_data.get(1).copied().unwrap_or(0) == 0);
            if still_unsolved && self.tester_use_item(tester_id, item_id, area_id) {
                if let Some(character) = self.characters.get_mut(&tester_id) {
                    character.driver_state = Some(CharacterDriverState::Tester(data));
                }
                return true;
            }
            data.item_to_use = None;
        }

        if let Some(character) = self.characters.get_mut(&tester_id) {
            character.driver_state = Some(CharacterDriverState::Tester(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(tester_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let (victim_last_x, victim_last_y) = (data.victim_last_x, data.victim_last_y);
            let arrived = self.characters.get(&tester_id).is_some_and(|tester| {
                tester.x.abs_diff(victim_last_x) < 2 && tester.y.abs_diff(victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Tester(state)) = self
                    .characters
                    .get_mut(&tester_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                tester_id,
                victim_last_x,
                victim_last_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return true;
            }
        }

        // C `if (regenerate_driver(cn)) return;` / `if
        // (spell_self_driver(cn)) return;`.
        if self.spell_self_simple_baddy(tester_id) {
            return true;
        }
        if self.regenerate_simple_baddy(tester_id) {
            return true;
        }

        // C `handle_tester_healing`: synchronous `use_item`, never
        // consumes the tick.
        self.tester_handle_healing(tester_id, area_id);

        // C `handle_tester_movement`.
        self.tester_handle_movement(tester_id, area_id)
    }

    /// C `handle_tester_item_management`'s cursor-item shuffle
    /// (`pents.c:1678-1702`): stash a held potion in the backpack, or junk
    /// whatever's left on the cursor.
    fn tester_handle_cursor_item(&mut self, tester_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get(&tester_id)
            .and_then(|character| character.cursor_item)
        else {
            return;
        };
        self.npc_say(tester_id, "got item");
        let is_potion = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver == IDR_POTION);
        if is_potion {
            let empty_slot = self.characters.get(&tester_id).and_then(|character| {
                (30..INVENTORY_SIZE).find(|&slot| character.inventory[slot].is_none())
            });
            if let Some(slot) = empty_slot {
                if let Some(character) = self.characters.get_mut(&tester_id) {
                    character.inventory[slot] = character.cursor_item.take();
                }
            }
        }

        // Junk whatever's still on the cursor (either a non-potion, or a
        // potion with no free backpack slot).
        if let Some(item_id) = self
            .characters
            .get(&tester_id)
            .and_then(|character| character.cursor_item)
        {
            self.npc_say(tester_id, "junked item");
            self.destroy_item(item_id);
        }
    }

    /// C's `item_to_pickup` sighting filter (`process_tester_messages`'s
    /// `NT_ITEM` branch, `pents.c:1642-1647`) replaced by a direct nearest-
    /// visible scan - see module doc comment.
    fn find_nearest_tester_pickup_item(&self, tester: &Character) -> Option<ItemId> {
        let mut best: Option<(usize, ItemId)> = None;
        for item in self.items.values() {
            if item.driver != IDR_POTION {
                continue;
            }
            let dist = manhattan_distance(
                usize::from(tester.x),
                usize::from(tester.y),
                usize::from(item.x),
                usize::from(item.y),
            );
            if dist > TESTER_SCAN_DIST {
                continue;
            }
            if !char_see_item(tester, item, &self.map, self.date.daylight) {
                continue;
            }
            if best.is_none_or(|(best_dist, best_id)| {
                dist < best_dist || (dist == best_dist && item.id.0 < best_id.0)
            }) {
                best = Some((dist, item.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// C's `item_to_use`/`visible_pents[]` sighting filter
    /// (`process_tester_messages`'s `NT_ITEM` branch, `pents.c:1648-1661`)
    /// replaced by a direct nearest-visible-and-unsolved scan - see module
    /// doc comment.
    fn find_nearest_tester_pentagram(&self, tester: &Character) -> Option<ItemId> {
        let mut best: Option<(usize, ItemId)> = None;
        for item in self.items.values() {
            if item.driver != IDR_PENT {
                continue;
            }
            if item.driver_data.get(1).copied().unwrap_or(0) != 0 {
                continue; // already activated
            }
            let dist = manhattan_distance(
                usize::from(tester.x),
                usize::from(tester.y),
                usize::from(item.x),
                usize::from(item.y),
            );
            if dist > TESTER_SCAN_DIST {
                continue;
            }
            if !char_see_item(tester, item, &self.map, self.date.daylight) {
                continue;
            }
            if best.is_none_or(|(best_dist, best_id)| {
                dist < best_dist || (dist == best_dist && item.id.0 < best_id.0)
            }) {
                best = Some((dist, item.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// C `take_driver(cn, dat->item_to_pickup)`: walk to the item if not
    /// already adjacent, then pick it up. Same shape as `World::janitor_
    /// take_item` (`world/npc/janitor.rs`).
    fn tester_take_item(&mut self, tester_id: CharacterId, item_id: ItemId, area_id: u16) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let Some(tester) = self.characters.get(&tester_id) else {
            return false;
        };
        let direction =
            adjacent_direction(tester.x, tester.y, usize::from(item.x), usize::from(item.y));
        if let Some(direction) = direction {
            let Some(tester) = self.characters.get_mut(&tester_id) else {
                return false;
            };
            do_take(
                tester,
                &self.map,
                &item,
                direction as u8,
                true,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward(
                tester_id,
                usize::from(item.x),
                usize::from(item.y),
                1,
                area_id,
                false,
            )
        }
    }

    /// C `use_driver(cn, dat->item_to_use, 0)`: walk to the pentagram if
    /// not already adjacent, then use it (the generic item-use completion
    /// pipeline applies `IDR_PENT`'s `PentagramActivate` outcome, same as
    /// a player's). Same shape as `World::janitor_use_light`
    /// (`world/npc/janitor.rs`).
    fn tester_use_item(&mut self, tester_id: CharacterId, item_id: ItemId, area_id: u16) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(tester) = self.characters.get(&tester_id) else {
            return false;
        };
        let direction = adjacent_use_direction(
            tester.x,
            tester.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        );
        if let Some(direction) = direction {
            let Some(tester) = self.characters.get_mut(&tester_id) else {
                return false;
            };
            do_use(
                tester,
                &self.map,
                &item,
                direction as u8,
                0,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward_use_item(
                tester_id,
                usize::from(item.x),
                usize::from(item.y),
                item.flags,
                area_id,
            )
        }
    }

    /// C `handle_tester_healing` (`pents.c:1739-1755`): drink the first
    /// inventory potion with a positive HP effect once below the
    /// configured heal threshold.
    fn tester_handle_healing(&mut self, tester_id: CharacterId, area_id: u16) {
        let Some(tester) = self.characters.get(&tester_id).cloned() else {
            return;
        };
        let max_hp = character_value(&tester, CharacterValue::Hp) * POWERSCALE;
        if f64::from(tester.hp) >= f64::from(max_hp) * self.settings.tester_heal_threshold {
            return;
        }
        self.npc_say(tester_id, "wanna use potion");

        let potion_slot = (30..INVENTORY_SIZE).find_map(|slot| {
            let item_id = tester.inventory.get(slot).copied().flatten()?;
            let item = self.items.get(&item_id)?;
            (item.driver == IDR_POTION && item.driver_data.get(1).copied().unwrap_or(0) != 0)
                .then_some(item_id)
        });
        if let Some(item_id) = potion_slot {
            self.execute_item_driver_request(
                ItemDriverRequest::Driver {
                    driver: IDR_POTION,
                    item_id,
                    character_id: tester_id,
                    spec: 0,
                },
                area_id,
            );
        }
    }

    /// C `handle_tester_movement` (`pents.c:1765-1777`): keep walking
    /// toward the current wander destination; once it's unreachable (or
    /// already reached), roll a fresh one nearby and idle for this tick.
    fn tester_handle_movement(&mut self, tester_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&tester_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Tester(data)) => data,
            _ => TesterDriverData::default(),
        };

        if let (Ok(target_x), Ok(target_y)) =
            (usize::try_from(data.dest_x), usize::try_from(data.dest_y))
        {
            if self.setup_walk_toward(tester_id, target_x, target_y, 2, area_id, false) {
                return true;
            }
        }

        let Some((tester_x, tester_y)) = self
            .characters
            .get(&tester_id)
            .map(|tester| (tester.x, tester.y))
        else {
            return false;
        };
        let range = self.settings.tester_movement_range.max(0);
        let half = range / 2;
        let roll_x =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, range as u32) as i32;
        let roll_y =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, range as u32) as i32;
        data.dest_x = i32::from(tester_x) + roll_x - half;
        data.dest_y = i32::from(tester_y) + roll_y - half;

        if let Some(character) = self.characters.get_mut(&tester_id) {
            character.driver_state = Some(CharacterDriverState::Tester(data));
        }

        // C `do_idle(cn, TICKS);` (`pents.c:1776`).
        self.idle_simple_baddy(tester_id)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_TESTER;

/// C `struct tester_data` (`pents.c:184-191`). `target_character` is not
/// ported (write-only dead field, see module doc comment); `visible_pents`
/// is replaced by direct per-tick scans (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TesterDriverData {
    pub item_to_use: Option<ItemId>,
    pub item_to_pickup: Option<ItemId>,
    pub dest_x: i32,
    pub dest_y: i32,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
