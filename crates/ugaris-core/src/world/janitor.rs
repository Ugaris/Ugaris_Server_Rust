//! `CDR_JANITOR` lamp-lighting/item-tidying NPC.
//!
//! Ports `src/module/base.c`'s `janitor_driver`: toggling nearby toylights
//! (`IDR_TOYLIGHT`) to match the current day/night state, picking up loose
//! `IF_TAKE` junk within its half of town, stashing it in the deep
//! inventory slots (`30..INVENTORY_SIZE`, matching C's comment `"30-
//! (INVENTORYSIZE-1) inventory"`), and dropping it off one item at a time
//! at one of nine fixed home-area tiles, plus the idle-murmur table
//! (rolled only right after a successful light-toggle action, unlike the
//! merchant/bank/trader drivers' periodic per-minute throttle).
//!
//! Deviations from C (documented here, not silent):
//! - C's `struct janitor_data` also carries `light[MAXLIGHT]`/
//!   `take[MAXTAKE]`, a cache of item IDs discovered via `NT_ITEM` notify
//!   messages as the janitor patrols (`scan_item_driver`). This port
//!   recomputes the nearest matching light/take-item candidate directly
//!   from `World::items` every tick instead - the same class of
//!   simplification already established for the merchant/bank/trader
//!   greeting scans (a fresh nearest-match scan is behaviorally
//!   equivalent to C's steady-state "closest known item" selection
//!   without the extra per-character message-cache plumbing). Only
//!   `dat->cnt` (the murmur counter) is kept as genuinely persistent
//!   state - see [`crate::character_driver::JanitorDriverData`].
//! - C's bag-unstash loop reads `ch[cn].item[INVENTORYSIZE]` first (an
//!   off-by-one out-of-bounds read past the valid `0..INVENTORYSIZE`
//!   range) before falling back to `INVENTORYSIZE-1`; this port starts
//!   at the last valid index (`INVENTORY_SIZE - 1`) instead of
//!   replicating undefined behavior.
//! - `char_see_item`'s LOS/visibility gate (`take_driver`) is applied to
//!   take-item candidates; `use_driver`'s equivalent check is commented
//!   out in C for the light branch, so no visibility gate is applied
//!   there either - matching C exactly.
use super::*;

/// C `#define MAXLIGHT 150` / `#define MAXTAKE 10` bound the *cache*, not
/// the underlying selection logic (see module doc comment); this port has
/// no equivalent cache size to bound.
/// C `janitor_drop`'s nine fixed candidate tiles (`base.c:4985-5015`),
/// tried in this exact order every time the janitor needs to set an item
/// down.
const JANITOR_DROP_SPOTS: [(u16, u16); 9] = [
    (161, 180),
    (161, 179),
    (161, 178),
    (162, 178),
    (162, 179),
    (162, 180),
    (162, 181),
    (162, 182),
    (162, 183),
];

/// C `janitor_driver`'s `IF_TAKE` exclusion box (`base.c:5108`): items
/// already resting on one of the nine home-area tiles are never re-picked
/// up.
const JANITOR_HOME_MIN_X: u16 = 161;
const JANITOR_HOME_MAX_X: u16 = 162;
const JANITOR_HOME_MIN_Y: u16 = 178;
const JANITOR_HOME_MAX_Y: u16 = 183;

/// C's deep-inventory "bag" range comment on `struct char.item[]`
/// (`server.h:447`): `"0-11 worn, 12-29 spells, 30-(INVENTORYSIZE-1)
/// inventory"`.
const JANITOR_BAG_START: usize = 30;

/// C `static const char *janitor_mutterings[]` equivalent - the 18
/// `switch (RANDOM(18))` cases in `janitor_driver` (`base.c:5146-5203`).
/// Index `1` is the dynamic "N lights I turned on" counter line, handled
/// specially in [`World::janitor_murmur`] and never read from this array.
const JANITOR_MUTTERINGS: [&str; 18] = [
    "I hate my life. I hate my life! I HATE MY LIFE!",
    "",
    "Infravision potions. Yes, that's a good way to deal with the dark!",
    "I need new shoes.",
    "Filthy people. Dropping junk all over the town square.",
    "Another do-gooder looking at me with pity? I do NOT need your pity!.",
    "No one's ever going to give me a day off.",
    "If ever people could stop playing with the lights.",
    "Eddow broke it again didn't he? I knew he was trouble when he first showed up!",
    "I wonder if anyone notices how hard I work around here.",
    "Another day, another mess to clean up.",
    "Maybe I'll just leave the dirt this time... no one seems to care.",
    "The broom's wearing out, just like my patience.",
    "Why do they keep hiring new guards? I'm the one who keeps this place in order.",
    "If I had a gold coin for every time someone ignored me...",
    "Sometimes I think the dirt's the only thing that listens to me.",
    "If only I had a spell to clean up this mess instantly.",
    "I swear, this place gets messier every day.",
];

fn janitor_in_home_area(x: u16, y: u16) -> bool {
    (JANITOR_HOME_MIN_X..=JANITOR_HOME_MAX_X).contains(&x)
        && (JANITOR_HOME_MIN_Y..=JANITOR_HOME_MAX_Y).contains(&y)
}

/// C `janitor_driver`'s `NT_ITEM` handler's town-half filter
/// (`base.c:5107-5111`): only items on the same side of `y == 192` as the
/// janitor's home tile (`ch[cn].tmpy`) are ever tracked.
fn janitor_same_town_half(home_y: u16, item_y: u16) -> bool {
    if home_y < 192 && item_y > 192 {
        return false;
    }
    if home_y > 192 && item_y < 192 {
        return false;
    }
    true
}

fn highest_occupied_janitor_bag_slot(character: &Character) -> Option<usize> {
    (JANITOR_BAG_START..INVENTORY_SIZE)
        .rev()
        .find(|&slot| character.inventory[slot].is_some())
}

impl World {
    /// C `janitor_driver`'s `lightcmp`-sorted selection (`base.c:4899-4915`
    /// + `4917-4936`): the nearest known `IDR_TOYLIGHT` item, on the
    /// janitor's town half, whose `drdata[0]` (on/off state) does not
    /// already match the desired `ls` state. Ties break on the lowest
    /// `ItemId` for determinism (C's `qsort` is not stable and has no
    /// documented tie-break; there is no observable difference between
    /// two lights at the same distance).
    fn find_nearest_janitor_light(&self, janitor: &Character, ls: u8) -> Option<ItemId> {
        let mut best: Option<(usize, ItemId)> = None;
        for item in self.items.values() {
            if item.driver != IDR_TOYLIGHT {
                continue;
            }
            if item.carried_by.is_some() || item.contained_in.is_some() {
                continue;
            }
            if item.x == 0 && item.y == 0 {
                continue;
            }
            if !janitor_same_town_half(janitor.rest_y, item.y) {
                continue;
            }
            let state = item.driver_data.first().copied().unwrap_or(0);
            if state == ls {
                continue;
            }
            let dist = manhattan_distance(
                usize::from(janitor.x),
                usize::from(janitor.y),
                usize::from(item.x),
                usize::from(item.y),
            );
            if best.is_none_or(|(best_dist, best_id)| {
                dist < best_dist || (dist == best_dist && item.id.0 < best_id.0)
            }) {
                best = Some((dist, item.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// C `janitor_driver`'s `takecmp`-sorted selection (`base.c:4899-4915`):
    /// the nearest visible `IF_TAKE` item, on the janitor's town half, not
    /// already resting on one of the nine home-area tiles.
    fn find_nearest_janitor_take_item(&self, janitor: &Character) -> Option<ItemId> {
        let mut best: Option<(usize, ItemId)> = None;
        for item in self.items.values() {
            if !item.flags.contains(ItemFlags::TAKE) {
                continue;
            }
            if !janitor_same_town_half(janitor.rest_y, item.y) {
                continue;
            }
            if janitor_in_home_area(item.x, item.y) {
                continue;
            }
            if !char_see_item(janitor, item, &self.map, self.date.daylight) {
                continue;
            }
            let dist = manhattan_distance(
                usize::from(janitor.x),
                usize::from(janitor.y),
                usize::from(item.x),
                usize::from(item.y),
            );
            if best.is_none_or(|(best_dist, best_id)| {
                dist < best_dist || (dist == best_dist && item.id.0 < best_id.0)
            }) {
                best = Some((dist, item.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// C `use_driver(cn, dat->light[0], 0)`: walk to the light if not
    /// already adjacent, then use it (toggling its on/off state via
    /// `IDR_TOYLIGHT`'s item driver on action completion).
    fn janitor_use_light(
        &mut self,
        janitor_id: CharacterId,
        light_id: ItemId,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&light_id).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(janitor) = self.characters.get(&janitor_id) else {
            return false;
        };
        let direction = adjacent_use_direction(
            janitor.x,
            janitor.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        );
        if let Some(direction) = direction {
            let Some(janitor) = self.characters.get_mut(&janitor_id) else {
                return false;
            };
            do_use(janitor, &self.map, &item, direction as u8, 0).is_ok()
        } else {
            self.setup_walk_toward_use_item(
                janitor_id,
                usize::from(item.x),
                usize::from(item.y),
                item.flags,
                area_id,
            )
        }
    }

    /// C `take_driver(cn, dat->take[0])`: walk to the item if not already
    /// adjacent, then pick it up.
    fn janitor_take_item(
        &mut self,
        janitor_id: CharacterId,
        item_id: ItemId,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let Some(janitor) = self.characters.get(&janitor_id) else {
            return false;
        };
        let direction = adjacent_direction(
            janitor.x,
            janitor.y,
            usize::from(item.x),
            usize::from(item.y),
        );
        if let Some(direction) = direction {
            let Some(janitor) = self.characters.get_mut(&janitor_id) else {
                return false;
            };
            do_take(janitor, &self.map, &item, direction as u8, true).is_ok()
        } else {
            self.setup_walk_toward(
                janitor_id,
                usize::from(item.x),
                usize::from(item.y),
                1,
                area_id,
                false,
            )
        }
    }

    /// C `drop_driver(cn, x, y)`: refuse a blocked/occupied tile outright
    /// (matching C's pre-pathfind guard), otherwise walk there and drop
    /// the held (cursor) item.
    fn janitor_try_drop_at(
        &mut self,
        janitor_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    ) -> bool {
        let (tx, ty) = (usize::from(x), usize::from(y));
        let tile_item = self.map.tile(tx, ty).map(|tile| tile.item).unwrap_or(0);
        if tile_item != 0 || self.map.blocks_movement(tx, ty) {
            return false;
        }
        let Some(janitor) = self.characters.get(&janitor_id) else {
            return false;
        };
        let Some(item_id) = janitor.cursor_item else {
            return false;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        if let Some(direction) = adjacent_direction(janitor.x, janitor.y, tx, ty) {
            let Some(janitor) = self.characters.get_mut(&janitor_id) else {
                return false;
            };
            do_drop(janitor, &self.map, &item, direction as u8).is_ok()
        } else {
            self.setup_walk_toward(janitor_id, tx, ty, 1, area_id, false)
        }
    }

    /// C `janitor_drop(cn)`: try the nine fixed home-area tiles in order,
    /// stopping at the first one that accepts an action (walking closer
    /// counts as accepting, matching C's "some action happened" return).
    fn janitor_drop_held_item(&mut self, janitor_id: CharacterId, area_id: u16) -> bool {
        for &(x, y) in JANITOR_DROP_SPOTS.iter() {
            if self.janitor_try_drop_at(janitor_id, x, y, area_id) {
                return true;
            }
        }
        false
    }

    /// C `janitor_driver`'s top-of-function shift (`base.c:5109-5117`):
    /// if the janitor is holding an item on its cursor and the deep bag
    /// is not full, stash it in `item[30]`, shifting the rest of the bag
    /// up by one slot.
    fn absorb_janitor_citem_into_bag(&mut self, janitor_id: CharacterId) {
        let Some(janitor) = self.characters.get_mut(&janitor_id) else {
            return;
        };
        if janitor.cursor_item.is_none() {
            return;
        }
        if janitor.inventory[INVENTORY_SIZE - 1].is_some() {
            return;
        }
        for i in (JANITOR_BAG_START + 1..INVENTORY_SIZE).rev() {
            janitor.inventory[i] = janitor.inventory[i - 1];
        }
        janitor.inventory[JANITOR_BAG_START] = janitor.cursor_item.take();
    }

    /// C `janitor_driver`'s tail branch (`base.c:5093-5106`): pull an item
    /// out of the bag (or use the already-held cursor item) and try to
    /// drop it off; if that fails, walk home; if that fails too, idle.
    fn janitor_drop_off_or_return_home(&mut self, janitor_id: CharacterId, area_id: u16) {
        let Some(janitor) = self.characters.get(&janitor_id).cloned() else {
            return;
        };

        if janitor.cursor_item.is_none() {
            if let Some(slot) = highest_occupied_janitor_bag_slot(&janitor) {
                if let Some(item_id) = janitor.inventory[slot] {
                    if let Some(janitor_mut) = self.characters.get_mut(&janitor_id) {
                        janitor_mut.inventory[slot] = None;
                        janitor_mut.cursor_item = Some(item_id);
                    }
                    if self.janitor_drop_held_item(janitor_id, area_id) {
                        return;
                    }
                    if let Some(janitor_mut) = self.characters.get_mut(&janitor_id) {
                        janitor_mut.inventory[slot] = Some(item_id);
                        janitor_mut.cursor_item = None;
                    }
                }
            }
        } else if self.janitor_drop_held_item(janitor_id, area_id) {
            return;
        }

        if self.setup_walk_toward(
            janitor_id,
            usize::from(janitor.rest_x),
            usize::from(janitor.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }

        if let Some(character) = self.characters.get_mut(&janitor_id) {
            let _ = do_idle(character, TICKS_PER_SECOND as i32);
        }
    }

    /// C `janitor_driver`'s idle-murmur block (`base.c:5145-5203`): rolled
    /// only right after a successful light-toggle action (1-in-50 chance),
    /// unlike the merchant/bank/trader drivers' periodic per-minute
    /// throttle.
    fn janitor_murmur(&mut self, janitor_id: CharacterId) {
        let index = legacy_random_below_from_seed(&mut self.legacy_random_seed, 18);
        if index == 1 {
            let cnt = match self
                .characters
                .get(&janitor_id)
                .and_then(|character| character.driver_state.as_ref())
            {
                Some(CharacterDriverState::Janitor(data)) if data.cnt != 0 => data.cnt,
                _ => 25598,
            };
            let text = format!(
                "{cnt} lights I turned on in my life, {cnt} lights I turned on in my life..."
            );
            self.npc_murmur(janitor_id, &text);
            if let Some(CharacterDriverState::Janitor(data)) = self
                .characters
                .get_mut(&janitor_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                data.cnt = cnt + 1;
            }
            return;
        }
        let text = JANITOR_MUTTERINGS[index as usize];
        self.npc_murmur(janitor_id, text);
    }

    /// Janitor NPC tick: absorb any held item into the bag, try to pick up
    /// the nearest reachable junk item, otherwise tend the nearest
    /// mismatched light, otherwise drop off bagged junk / walk home /
    /// idle. Ports the per-tick body of C `janitor_driver`.
    fn process_janitor_tick(&mut self, janitor_id: CharacterId, area_id: u16) {
        self.absorb_janitor_citem_into_bag(janitor_id);

        let Some(janitor) = self.characters.get(&janitor_id).cloned() else {
            return;
        };

        if janitor.cursor_item.is_none() {
            if let Some(item_id) = self.find_nearest_janitor_take_item(&janitor) {
                if self.janitor_take_item(janitor_id, item_id, area_id) {
                    return;
                }
            }
        }

        // C `if (dlight > 200) ls = 0; else ls = 1;` (`base.c:5063-5067`).
        let ls: u8 = if self.date.daylight > 200 { 0 } else { 1 };
        let Some(light_id) = self.find_nearest_janitor_light(&janitor, ls) else {
            self.janitor_drop_off_or_return_home(janitor_id, area_id);
            return;
        };

        if self.janitor_use_light(janitor_id, light_id, area_id) {
            if legacy_random_below_from_seed(&mut self.legacy_random_seed, 50) == 0 {
                self.janitor_murmur(janitor_id);
            }
            return;
        }

        if let Some(character) = self.characters.get_mut(&janitor_id) {
            let _ = do_idle(character, TICKS_PER_SECOND as i32);
        }
    }

    /// Ports the per-tick dispatch loop over all live `CDR_JANITOR`
    /// characters (C `ch_driver`'s `CDR_JANITOR` case, `base.c:5990-5992`).
    pub fn process_janitor_actions(&mut self, area_id: u16) {
        let janitor_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JANITOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for janitor_id in janitor_ids {
            self.process_janitor_tick(janitor_id, area_id);
        }
    }
}
