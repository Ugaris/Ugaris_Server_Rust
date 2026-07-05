//! Self-healing data-integrity scan behind `/checksanity` (C
//! `src/system/consistency.c`'s four `consistency_check_*` functions,
//! dispatched from `command.c:7443-7457`'s `CF_GOD`-gated `/checksanity`,
//! and also run unconditionally every 20 seconds by C's own
//! `validate_data_20s_task`, `server.c:227-234` - that periodic side is
//! *not* wired here yet, only the on-demand command; see the task note in
//! `PORTING_TODO.md` for the remaining gap).
//!
//! C walks its fixed-size global `it[]`/`ch[]`/`con[]`/`map[]` arrays
//! looking for desynced back-references - an item that thinks it's
//! carried/on-the-ground/contained but the character/map-tile/container
//! it points at doesn't agree, or vice versa - and repairs every one it
//! finds by clearing the dangling side, incrementing a per-category error
//! counter. This `World` is a straight-line port of the same *duplicated*
//! reference data model (`Item::carried_by`/`x`+`y`/`contained_in` vs.
//! `Character::inventory`/`cursor_item`, `MapTile::item`), so the exact
//! same class of bug remains possible here and this check is a
//! meaningful, not merely cosmetic, port.
//!
//! Deviations from the C original:
//! - No per-anomaly console log line (C's `elog`) is emitted; `ugaris-
//!   core` has no established logging convention (unlike `ugaris-server`,
//!   which uses `tracing`), and the command's own player-facing report is
//!   only ever the four aggregate counts anyway - the same "untracked
//!   console-only C side effect" convention already applied elsewhere in
//!   this codebase (e.g. `dlog`/`write_scrollback`).
//! - C's player-facing "You encountered a bug (consist1)..." message sent
//!   to the *owning* player when their own carried item is found dangling
//!   (`consistency_check_chars`) is not sent; only the aggregate count
//!   reaches the admin who ran `/checksanity`.
//! - There is no `MAXITEM`/`MAXCHARS`/`MAXMAP` fixed-array bound to
//!   violate in a `HashMap`-keyed world, so C's numeric
//!   out-of-bounds/negative-index branches have no Rust analogue; only
//!   the "does the referenced id actually exist, and does it agree" class
//!   of check applies.
//! - This Rust port has no separate `con[]` array (see `containers.rs`'s
//!   doc comment in `ugaris-server`): "is this item a container" is just
//!   `Item::content_id != 0`, and its contents are derived on the fly as
//!   every other item whose `contained_in` points back at it. So
//!   `consistency_check_containers` here iterates every container item
//!   and checks every item whose `contained_in` points at it - the
//!   reverse direction from C's forward `con[ct].item[]` walk, but
//!   functionally equivalent since there is no forward array to desync
//!   against in the first place.

use std::collections::HashMap;

use super::*;

/// Aggregate result of [`World::consistency_check`]: the four counters
/// `/checksanity` reports (`command.c:7443-7457`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsistencyReport {
    pub item_errors: u32,
    pub map_errors: u32,
    pub char_errors: u32,
    pub container_errors: u32,
}

enum ItemFix {
    None,
    ClearCarried,
    ClearPosition,
    ClearContained,
    RemoveItem,
}

impl World {
    /// C `/checksanity`'s full sweep (`command.c:7443-7457`): runs all
    /// four checks in the same order C's dispatcher does, sharing one
    /// "how many places reference this item" counter across the
    /// map/character/container passes (C's file-static `item_used` array,
    /// reset by `consistency_check_map` on every call).
    pub fn consistency_check(&mut self) -> ConsistencyReport {
        let item_errors = self.consistency_check_items();
        let mut item_used: HashMap<ItemId, u32> = HashMap::new();
        let map_errors = self.consistency_check_map(&mut item_used);
        let char_errors = self.consistency_check_chars(&mut item_used);
        let container_errors = self.consistency_check_containers(&mut item_used);
        ConsistencyReport {
            item_errors,
            map_errors,
            char_errors,
            container_errors,
        }
    }

    /// C `consistency_check_items` (`consistency.c:30-146`): every item
    /// must be linked from exactly one of a carrying character, a map
    /// tile, or a container, matching C's `if/else if/else if/else`
    /// priority chain (carried wins over on-the-ground wins over
    /// contained wins over "not linked at all", which C frees outright).
    fn consistency_check_items(&mut self) -> u32 {
        let mut err = 0u32;
        let ids: Vec<ItemId> = self.items.keys().copied().collect();
        for id in ids {
            let Some(item) = self.items.get(&id) else {
                continue;
            };
            if item.flags.contains(ItemFlags::VOID) {
                continue;
            }

            let fix = if let Some(carrier_id) = item.carried_by {
                let linked_back = self.characters.get(&carrier_id).is_some_and(|c| {
                    c.cursor_item == Some(id) || c.inventory.iter().any(|slot| *slot == Some(id))
                });
                if linked_back {
                    ItemFix::None
                } else {
                    ItemFix::ClearCarried
                }
            } else if item.x != 0 {
                let linked_back = self
                    .map
                    .tile(item.x as usize, item.y as usize)
                    .is_some_and(|tile| tile.item == id.0);
                if linked_back {
                    ItemFix::None
                } else {
                    ItemFix::ClearPosition
                }
            } else if let Some(container_id) = item.contained_in {
                let valid_container = self
                    .items
                    .get(&container_id)
                    .is_some_and(|container| container.content_id != 0);
                if valid_container {
                    ItemFix::None
                } else {
                    ItemFix::ClearContained
                }
            } else {
                ItemFix::RemoveItem
            };

            match fix {
                ItemFix::None => {}
                ItemFix::ClearCarried => {
                    if let Some(item) = self.items.get_mut(&id) {
                        item.carried_by = None;
                    }
                    err += 1;
                }
                ItemFix::ClearPosition => {
                    if let Some(item) = self.items.get_mut(&id) {
                        item.x = 0;
                        item.y = 0;
                    }
                    err += 1;
                }
                ItemFix::ClearContained => {
                    if let Some(item) = self.items.get_mut(&id) {
                        item.contained_in = None;
                    }
                    err += 1;
                }
                ItemFix::RemoveItem => {
                    self.items.remove(&id);
                    err += 1;
                }
            }
        }
        err
    }

    /// C `consistency_check_map` (`consistency.c:161-236`): every item id
    /// referenced by a map tile must exist, be un-carried and
    /// un-contained, and agree on its own `x`/`y` with the tile it's
    /// found at; also rejects the same item id being referenced from more
    /// than one tile (via the shared `item_used` counter).
    fn consistency_check_map(&mut self, item_used: &mut HashMap<ItemId, u32>) -> u32 {
        item_used.clear();
        let mut err = 0u32;
        let width = self.map.width();
        let height = self.map.height();
        for y in 0..height {
            for x in 0..width {
                let tile_item = match self.map.tile(x, y) {
                    Some(tile) if tile.item != 0 => tile.item,
                    _ => continue,
                };
                let id = ItemId(tile_item);
                let bad = match self.items.get(&id) {
                    None => true,
                    Some(item) => {
                        item.flags.contains(ItemFlags::VOID)
                            || item.carried_by.is_some()
                            || item.contained_in.is_some()
                            || item.x as usize != x
                            || item.y as usize != y
                    }
                };
                if bad {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        tile.item = 0;
                    }
                    err += 1;
                    continue;
                }
                let count = item_used.entry(id).or_insert(0);
                *count += 1;
                if *count > 1 {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        tile.item = 0;
                    }
                    err += 1;
                }
            }
        }
        err
    }

    /// C `consistency_check_chars` (`consistency.c:239-378`): every item
    /// id referenced by a character's inventory slots or its cursor
    /// (`citem`) must exist, be un-void, actually claim to be carried by
    /// that same character, and have no stray `x`/`contained_in` of its
    /// own; also rejects the same item id appearing more than once across
    /// every character (via the shared `item_used` counter). Each slot is
    /// checked independently, matching C's `continue`-per-slot control
    /// flow (only the first matching problem for a given slot is fixed).
    fn consistency_check_chars(&mut self, item_used: &mut HashMap<ItemId, u32>) -> u32 {
        let mut err = 0u32;
        let char_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        for cn in char_ids {
            let Some(inventory_size) = self.characters.get(&cn).map(|c| c.inventory.len()) else {
                continue;
            };
            // Slots `0..inventory_size` are the real inventory; slot
            // `inventory_size` is a virtual slot standing in for C's
            // separate `citem` (cursor item) field.
            for slot in 0..=inventory_size {
                let Some(character) = self.characters.get(&cn) else {
                    break;
                };
                let Some(id) = (if slot == inventory_size {
                    character.cursor_item
                } else {
                    character.inventory[slot]
                }) else {
                    continue;
                };

                enum CharFix {
                    None,
                    ClearSlot,
                    ClearPosition,
                    ClearContained,
                }

                let fix = match self.items.get(&id) {
                    None => CharFix::ClearSlot,
                    Some(item) => {
                        if item.flags.contains(ItemFlags::VOID) {
                            CharFix::ClearSlot
                        } else if item.carried_by != Some(cn) {
                            CharFix::ClearSlot
                        } else if item.x != 0 {
                            CharFix::ClearPosition
                        } else if item.contained_in.is_some() {
                            CharFix::ClearContained
                        } else {
                            CharFix::None
                        }
                    }
                };

                match fix {
                    CharFix::None => {
                        let count = item_used.entry(id).or_insert(0);
                        *count += 1;
                        if *count > 1 {
                            clear_character_slot(self, cn, slot, inventory_size);
                            err += 1;
                        }
                    }
                    CharFix::ClearSlot => {
                        clear_character_slot(self, cn, slot, inventory_size);
                        err += 1;
                    }
                    CharFix::ClearPosition => {
                        if let Some(item) = self.items.get_mut(&id) {
                            item.x = 0;
                            item.y = 0;
                        }
                        err += 1;
                    }
                    CharFix::ClearContained => {
                        if let Some(item) = self.items.get_mut(&id) {
                            item.contained_in = None;
                        }
                        err += 1;
                    }
                }
            }
        }
        err
    }

    /// C `consistency_check_containers` (`consistency.c:381-452`): every
    /// item claiming to be `contained_in` a container item must have no
    /// stray `x`/`carried_by` of its own, and must not be double-counted
    /// against the shared `item_used` counter (which by this point also
    /// carries every map/inventory reference).
    fn consistency_check_containers(&mut self, item_used: &mut HashMap<ItemId, u32>) -> u32 {
        let mut err = 0u32;
        let container_ids: Vec<ItemId> = self
            .items
            .values()
            .filter(|item| item.content_id != 0)
            .map(|item| item.id)
            .collect();
        for container_id in container_ids {
            let contained_ids: Vec<ItemId> = self
                .items
                .values()
                .filter(|item| item.contained_in == Some(container_id))
                .map(|item| item.id)
                .collect();
            for id in contained_ids {
                enum ContainerFix {
                    None,
                    ClearPosition,
                    ClearCarried,
                }

                let Some(item) = self.items.get(&id) else {
                    continue;
                };
                let fix = if item.x != 0 {
                    ContainerFix::ClearPosition
                } else if item.carried_by.is_some() {
                    ContainerFix::ClearCarried
                } else {
                    ContainerFix::None
                };

                match fix {
                    ContainerFix::None => {
                        let count = item_used.entry(id).or_insert(0);
                        *count += 1;
                        if *count > 1 {
                            if let Some(item) = self.items.get_mut(&id) {
                                item.contained_in = None;
                            }
                            err += 1;
                        }
                    }
                    ContainerFix::ClearPosition => {
                        if let Some(item) = self.items.get_mut(&id) {
                            item.x = 0;
                            item.y = 0;
                        }
                        err += 1;
                    }
                    ContainerFix::ClearCarried => {
                        if let Some(item) = self.items.get_mut(&id) {
                            item.carried_by = None;
                        }
                        err += 1;
                    }
                }
            }
        }
        err
    }
}

fn clear_character_slot(world: &mut World, cn: CharacterId, slot: usize, inventory_size: usize) {
    if let Some(character) = world.characters.get_mut(&cn) {
        if slot == inventory_size {
            character.cursor_item = None;
        } else {
            character.inventory[slot] = None;
        }
    }
}
