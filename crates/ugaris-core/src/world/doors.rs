use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoorToggleResult {
    Toggled,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StafferSpecDoorResult {
    Toggled,
    Locked,
    Blocked,
    Failed,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Area3PalaceLampState {
    pub switched_on_count: i32,
    pub switched_off_count: i32,
    pub keep_open_until_tick: u64,
}

impl World {
    pub(crate) fn toggle_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> DoorToggleResult {
        let Some(item) = self.items.get(&item_id) else {
            return DoorToggleResult::Failed;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.first().copied().unwrap_or_default() != 0;

        if x == 0 || y == 0 {
            return DoorToggleResult::Failed;
        }
        let Some(tile) = self.map.tile(x, y) else {
            return DoorToggleResult::Failed;
        };
        if tile.item != item_id.0 {
            return DoorToggleResult::Failed;
        }
        if is_open
            && tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            if character_id.0 == 0 {
                let should_retry = self.items.get_mut(&item_id).is_some_and(|item| {
                    item.driver_data.resize(40, 0);
                    item.driver_data[39] = item.driver_data[39].saturating_add(1);
                    item.driver_data[5] == 0
                });
                if should_retry {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
                }
            }
            return DoorToggleResult::Blocked;
        }

        let mut schedule_auto_close = false;
        let extended_door = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return DoorToggleResult::Failed;
            };
            item.driver_data.resize(40, 0);
            let Some(tile) = self.map.tile_mut(x, y) else {
                return DoorToggleResult::Failed;
            };

            if is_open {
                let restored = door_stored_flags(item);
                item.flags.insert(restored);
                apply_door_tile_flags(tile, item.flags);
                item.driver_data[0] = 0;
                item.sprite -= 1;
            } else {
                let stored = item.flags
                    & (ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK);
                store_door_flags(item, stored);
                item.flags.remove(
                    ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK,
                );
                tile.flags.remove(
                    MapFlags::TMOVEBLOCK
                        | MapFlags::TSIGHTBLOCK
                        | MapFlags::DOOR
                        | MapFlags::TSOUNDBLOCK,
                );
                item.driver_data[0] = 1;
                item.sprite += 1;
                item.driver_data[39] = item.driver_data[39].saturating_add(1);
                schedule_auto_close = item.driver_data[5] == 0;
            }

            item.driver_data[7] != 0
        };

        if schedule_auto_close {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        }

        if extended_door {
            self.shift_extended_door_foregrounds(x, y, if is_open { -1 } else { 1 });
        }

        self.queue_sound_area(x, y, if character_id.0 == 0 { 2 } else { 3 });

        DoorToggleResult::Toggled
    }

    pub(crate) fn toggle_pick_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> DoorToggleResult {
        let Some(item) = self.items.get(&item_id) else {
            return DoorToggleResult::Failed;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.first().copied().unwrap_or_default() != 0;

        if x == 0 || y == 0 {
            return DoorToggleResult::Failed;
        }
        let Some(tile) = self.map.tile(x, y) else {
            return DoorToggleResult::Failed;
        };
        if tile.item != item_id.0 {
            return DoorToggleResult::Failed;
        }

        if character_id.0 == 0 && !is_open {
            return DoorToggleResult::Blocked;
        }

        if is_open && self.pick_door_close_blocked(x, y) {
            if character_id.0 == 0 {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            }
            return DoorToggleResult::Blocked;
        }

        {
            let Some(item) = self.items.get_mut(&item_id) else {
                return DoorToggleResult::Failed;
            };
            item.driver_data.resize(40, 0);
            let Some(tile) = self.map.tile_mut(x, y) else {
                return DoorToggleResult::Failed;
            };

            if is_open {
                let restored = door_stored_flags(item);
                item.flags.insert(restored);
                apply_door_tile_flags(tile, item.flags);
                item.driver_data[0] = 0;
                item.sprite -= 1;
            } else {
                let stored = item.flags
                    & (ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK);
                store_door_flags(item, stored);
                item.flags.remove(
                    ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK,
                );
                tile.flags.remove(
                    MapFlags::TMOVEBLOCK
                        | MapFlags::TSIGHTBLOCK
                        | MapFlags::DOOR
                        | MapFlags::TSOUNDBLOCK,
                );
                item.driver_data[0] = 1;
                item.sprite += 1;
            }
        }

        if !is_open {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 20);
        }

        DoorToggleResult::Toggled
    }

    pub(crate) fn pick_door_close_blocked(&self, x: usize, y: usize) -> bool {
        if self.map.tile(x, y).is_some_and(|tile| {
            tile.flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        }) {
            return true;
        }

        [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .any(|(dx, dy)| {
                let Some(nx) = x.checked_add_signed(dx) else {
                    return false;
                };
                let Some(ny) = y.checked_add_signed(dy) else {
                    return false;
                };
                self.map
                    .tile(nx, ny)
                    .is_some_and(|tile| tile.character != 0)
            })
    }

    pub(crate) fn mine_door_target(&self, item_id: ItemId) -> Option<(u16, u16, u8)> {
        let item = self.items.get(&item_id)?;
        let nr = item.driver_data.first().copied().unwrap_or_default();
        let source = item.driver_data.get(1).copied().unwrap_or_default() == 0;

        self.items
            .values()
            .filter(|candidate| candidate.driver == IDR_MINEDOOR)
            .filter(|candidate| candidate.driver_data.first().copied().unwrap_or_default() == nr)
            .find(|candidate| {
                let candidate_is_target =
                    candidate.driver_data.get(1).copied().unwrap_or_default() != 0;
                if source {
                    candidate_is_target
                } else {
                    !candidate_is_target
                        && candidate.driver_data.get(3).copied().unwrap_or_default() != 0
                }
            })
            .map(|candidate| {
                (
                    candidate.x,
                    candidate.y,
                    candidate.driver_data.get(2).copied().unwrap_or_default(),
                )
            })
    }

    pub(crate) fn apply_mine_door_timer(&mut self, item_id: ItemId) -> bool {
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 30);

        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver_data.get(3).copied().unwrap_or_default() != 0 {
            return false;
        }
        let nr = item.driver_data.first().copied().unwrap_or_default();
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        if !self.mine_door_neighbors_have(x, y, MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK) {
            return false;
        }

        let current_source_id = self
            .items
            .values()
            .filter(|candidate| candidate.driver == IDR_MINEDOOR)
            .filter(|candidate| candidate.driver_data.first().copied().unwrap_or_default() == nr)
            .find(|candidate| {
                candidate.driver_data.get(1).copied().unwrap_or_default() == 0
                    && candidate.driver_data.get(3).copied().unwrap_or_default() != 0
            })
            .map(|candidate| candidate.id);

        if let Some(source_id) = current_source_id {
            let Some(source) = self.items.get(&source_id) else {
                return false;
            };
            let sx = usize::from(source.x);
            let sy = usize::from(source.y);
            if !self.mine_door_neighbors_have(sx, sy, MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK) {
                return false;
            }
            let roll = legacy_random_below_from_seed(&mut self.legacy_random_seed, 20);
            if roll != 0 {
                return false;
            }
            if let Some(dirty) = self.items.get_mut(&source_id).map(|source| {
                let dirty = (usize::from(source.x), usize::from(source.y));
                source.sprite = 15000;
                source.flags.remove(ItemFlags::USE);
                source.driver_data.resize(4, 0);
                source.driver_data[3] = 0;
                dirty
            }) {
                self.mark_dirty_sector(dirty.0, dirty.1);
            }
        }

        if let Some(dirty) = self.items.get_mut(&item_id).map(|item| {
            let dirty = (usize::from(item.x), usize::from(item.y));
            item.driver_data.resize(4, 0);
            item.sprite = match item.driver_data.get(2).copied().unwrap_or_default() {
                7 | 3 => 20124,
                1 | 5 => 20122,
                _ => item.sprite,
            };
            item.flags.insert(ItemFlags::USE);
            item.driver_data[3] = 1;
            dirty
        }) {
            self.mark_dirty_sector(dirty.0, dirty.1);
            true
        } else {
            false
        }
    }

    pub(crate) fn mine_door_neighbors_have(&self, x: usize, y: usize, flags: MapFlags) -> bool {
        [
            (1isize, 0isize),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ]
        .into_iter()
        .all(|(dx, dy)| {
            let Some(nx) = x.checked_add_signed(dx) else {
                return false;
            };
            let Some(ny) = y.checked_add_signed(dy) else {
                return false;
            };
            self.map
                .tile(nx, ny)
                .is_some_and(|tile| tile.flags.intersects(flags))
        })
    }

    pub(crate) fn ignite_burndown_barrel(&mut self, item_id: ItemId) -> bool {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let x = usize::from(before.x);
        let y = usize::from(before.y);
        if self.map.tile(x, y).is_none() {
            return false;
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 20;
            item.sprite = 51077;
            item.modifier_index[0] = CharacterValue::Light as i16;
            item.modifier_value[0] = 200;
        } else {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.foreground_sprite = 1024 << 16;
            self.mark_dirty_sector(x, y);
        }
        self.refresh_item_light_after_mutation(&before, item_id);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    pub(crate) fn tick_burndown_barrel(&mut self, item_id: ItemId) -> bool {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let x = usize::from(before.x);
        let y = usize::from(before.y);
        let Some(state) = before.driver_data.first().copied() else {
            return false;
        };
        if state == 0 {
            return false;
        }

        let mut schedule_next = false;
        let mut light_changed = false;
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(1, 0);
            item.driver_data[0] = item.driver_data[0].saturating_sub(1);
            let new_state = item.driver_data[0];
            if new_state > 15 {
                item.sprite += 1;
                schedule_next = true;
            } else if new_state == 15 {
                item.modifier_index[0] = CharacterValue::Light as i16;
                item.modifier_value[0] = 0;
                light_changed = true;
                schedule_next = true;
            } else if new_state == 0 {
                item.sprite = 21115;
            } else {
                schedule_next = true;
            }
        } else {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            if state == 16 {
                tile.foreground_sprite = 0;
            }
            self.mark_dirty_sector(x, y);
        }
        if light_changed {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        if schedule_next {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        }
        true
    }

    pub(crate) fn toggle_staffer_spec_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    ) -> StafferSpecDoorResult {
        let Some(item) = self.items.get(&item_id) else {
            return StafferSpecDoorResult::Failed;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.get(1).copied().unwrap_or_default() != 0;

        if x == 0 || y == 0 {
            return StafferSpecDoorResult::Failed;
        }
        let Some(tile) = self.map.tile(x, y) else {
            return StafferSpecDoorResult::Failed;
        };
        if tile.item != item_id.0 {
            return StafferSpecDoorResult::Failed;
        }

        if character_id.0 == 0 {
            let mut should_continue = true;
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(40, 0);
                if item.driver_data[39] != 0 {
                    item.driver_data[39] = item.driver_data[39].saturating_sub(1);
                }
                should_continue = item.driver_data[1] != 0 && item.driver_data[39] == 0;
            }
            if !should_continue {
                return StafferSpecDoorResult::Blocked;
            }
        }

        if is_open
            && tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            if character_id.0 == 0 {
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.driver_data.resize(40, 0);
                    item.driver_data[39] = item.driver_data[39].saturating_add(1);
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
            }
            return StafferSpecDoorResult::Blocked;
        }

        if character_id.0 != 0 {
            let marker = match kind {
                4 => Some((51, 234)),
                5 => Some((59, 240)),
                _ => None,
            };
            if marker
                .and_then(|(mx, my)| self.map.tile(mx, my))
                .is_some_and(|tile| tile.item == 0)
            {
                return StafferSpecDoorResult::Locked;
            }
        }

        let mut schedule_auto_close = false;
        {
            let Some(item) = self.items.get_mut(&item_id) else {
                return StafferSpecDoorResult::Failed;
            };
            item.driver_data.resize(40, 0);
            let Some(tile) = self.map.tile_mut(x, y) else {
                return StafferSpecDoorResult::Failed;
            };

            if is_open {
                let restored = door_stored_flags(item);
                item.flags.insert(restored);
                apply_door_tile_flags(tile, item.flags);
                item.driver_data[1] = 0;
                item.sprite -= 1;
            } else {
                let stored = item.flags
                    & (ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK);
                store_door_flags(item, stored);
                item.flags.remove(
                    ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK,
                );
                tile.flags.remove(
                    MapFlags::TMOVEBLOCK
                        | MapFlags::TSIGHTBLOCK
                        | MapFlags::DOOR
                        | MapFlags::TSOUNDBLOCK,
                );
                item.driver_data[1] = 1;
                item.sprite += 1;
                item.driver_data[39] = item.driver_data[39].saturating_add(1);
                schedule_auto_close = item.driver_data[5] == 0;
            }
        }

        if schedule_auto_close {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        }
        self.queue_sound_area(x, y, if character_id.0 == 0 { 2 } else { 3 });
        self.mark_dirty_sector(x, y);

        StafferSpecDoorResult::Toggled
    }

    pub(crate) fn tick_area3_palace_gate(&mut self, item_id: ItemId) -> Option<(bool, bool, bool)> {
        let item = self.items.get(&item_id)?;
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.first().copied().unwrap_or_default() != 0;
        let keep_open = self.area3_palace_lamps.keep_open_until_tick > self.tick.0;
        let tile = self.map.tile(x, y)?;
        if tile.item != item_id.0 {
            return None;
        }

        if keep_open {
            if is_open {
                return Some((false, false, false));
            }
            let item = self.items.get_mut(&item_id)?;
            item.driver_data.resize(40, 0);
            let tile = self.map.tile_mut(x, y)?;
            let stored = item.flags
                & (ItemFlags::MOVEBLOCK
                    | ItemFlags::SIGHTBLOCK
                    | ItemFlags::DOOR
                    | ItemFlags::SOUNDBLOCK);
            store_door_flags(item, stored);
            item.flags.remove(
                ItemFlags::MOVEBLOCK
                    | ItemFlags::SIGHTBLOCK
                    | ItemFlags::DOOR
                    | ItemFlags::SOUNDBLOCK,
            );
            tile.flags.remove(
                MapFlags::TMOVEBLOCK
                    | MapFlags::TSIGHTBLOCK
                    | MapFlags::DOOR
                    | MapFlags::TSOUNDBLOCK,
            );
            item.driver_data[0] = 1;
            item.sprite += 1;
            self.mark_dirty_sector(x, y);
            return Some((true, false, false));
        }

        if !is_open {
            return Some((false, false, false));
        }
        if tile
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            return Some((false, false, true));
        }

        let item = self.items.get_mut(&item_id)?;
        item.driver_data.resize(40, 0);
        let tile = self.map.tile_mut(x, y)?;
        let restored = door_stored_flags(item);
        item.flags.insert(restored);
        apply_door_tile_flags(tile, item.flags);
        item.driver_data[0] = 0;
        item.sprite -= 1;
        self.mark_dirty_sector(x, y);
        Some((false, true, false))
    }

    pub(crate) fn shift_extended_door_foregrounds(&mut self, x: usize, y: usize, delta: i32) {
        for (tile_x, tile_y) in [
            (x.saturating_add(1), y),
            (x.saturating_sub(1), y),
            (x, y.saturating_add(1)),
            (x, y.saturating_sub(1)),
        ] {
            let Some(tile) = self.map.tile_mut(tile_x, tile_y) else {
                continue;
            };
            if tile.foreground_sprite == 0 {
                continue;
            }
            if delta.is_negative() {
                tile.foreground_sprite =
                    tile.foreground_sprite.saturating_sub(delta.unsigned_abs());
            } else {
                tile.foreground_sprite = tile.foreground_sprite.saturating_add(delta as u32);
            }
        }
    }

    pub(crate) fn toggle_double_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> bool {
        let mut toggled = self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled;
        let Some((x, y, open_state)) = self.items.get(&item_id).map(|item| {
            (
                usize::from(item.x),
                usize::from(item.y),
                door_open_state(item),
            )
        }) else {
            return toggled;
        };
        if x == 0 || y == 0 {
            return toggled;
        }

        for (adjacent_x, adjacent_y) in [
            (x, y.saturating_add(1)),
            (x, y.saturating_sub(1)),
            (x.saturating_add(1), y),
            (x.saturating_sub(1), y),
        ] {
            let Some(adjacent_item_id) = self
                .map
                .tile(adjacent_x, adjacent_y)
                .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)))
            else {
                continue;
            };
            let Some(adjacent_item) = self.items.get(&adjacent_item_id) else {
                continue;
            };
            if door_open_state(adjacent_item) != open_state {
                toggled |=
                    self.toggle_door(adjacent_item_id, character_id) == DoorToggleResult::Toggled;
            }
        }

        toggled
    }

    pub(crate) fn use_freak_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        link_group: u8,
        one_way: bool,
        cached_partner_id: Option<ItemId>,
        no_target: bool,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let item_x = item.x;
        let item_y = item.y;
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let character_x = character.x;
        let character_y = character.y;

        let effective_group = if one_way { 0 } else { link_group };
        let partner_id = if effective_group == 0 {
            item_id
        } else if let Some(partner_id) = cached_partner_id.filter(|id| self.items.contains_key(id))
        {
            partner_id
        } else {
            let Some(found_id) = self.items.iter().find_map(|(candidate_id, candidate)| {
                (candidate_id != &item_id
                    && candidate.driver == crate::item_driver::IDR_FREAKDOOR
                    && candidate.driver_data.get(15).copied().unwrap_or_default() == 0
                    && candidate.driver_data.get(8).copied().unwrap_or_default() == effective_group)
                    .then_some(*candidate_id)
            }) else {
                return false;
            };
            if let Some(item) = self.items.get_mut(&item_id) {
                write_driver_data_u32(item, 10, found_id.0);
            }
            found_id
        };

        if item_x != character_x || item_y != character_y {
            let toggled = self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled;
            let opened = self.items.get(&item_id).is_some_and(door_open_state);
            let partner_closed = self
                .items
                .get(&partner_id)
                .is_some_and(|partner| !door_open_state(partner));
            if partner_id != item_id && opened && partner_closed {
                self.toggle_door(partner_id, character_id);
            }
            return toggled;
        }

        if partner_id == item_id || no_target {
            return false;
        }
        if self
            .items
            .get(&partner_id)
            .is_some_and(|partner| !door_open_state(partner))
        {
            self.toggle_door(partner_id, character_id);
        }

        let Some(partner) = self.items.get(&partner_id) else {
            return false;
        };
        let (target_x, target_y) = (partner.x, partner.y);
        let (dx, dy) = self
            .characters
            .get(&character_id)
            .map(|character| {
                (
                    i32::from(character.tox) - i32::from(character.x),
                    i32::from(character.toy) - i32::from(character.y),
                )
            })
            .unwrap_or((0, 0));

        if let Some(partner) = self.items.get_mut(&partner_id) {
            partner.driver_data.resize(10, 0);
            partner.driver_data[9] = 1;
        }
        let teleported = self.teleport_character(character_id, target_x, target_y, false);
        if let Some(partner) = self.items.get_mut(&partner_id) {
            partner.driver_data.resize(10, 0);
            partner.driver_data[9] = 0;
        }
        if teleported && (dx != 0 || dy != 0) {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.tox = (i32::from(character.x) + dx).clamp(0, u16::MAX as i32) as u16;
                character.toy = (i32::from(character.y) + dy).clamp(0, u16::MAX as i32) as u16;
            }
        }
        teleported
    }
}

pub(crate) fn door_stored_flags(item: &Item) -> ItemFlags {
    let mut bytes = [0; 8];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = item
            .driver_data
            .get(30 + offset)
            .copied()
            .unwrap_or_default();
    }
    ItemFlags::from_bits_retain(u64::from_le_bytes(bytes))
}

pub(crate) fn door_open_state(item: &Item) -> bool {
    item.driver_data.first().copied().unwrap_or_default() != 0
}

pub(crate) fn store_door_flags(item: &mut Item, flags: ItemFlags) {
    item.driver_data.resize(40, 0);
    item.driver_data[30..38].copy_from_slice(&flags.bits().to_le_bytes());
}

pub(crate) fn apply_door_tile_flags(tile: &mut crate::map::MapTile, item_flags: ItemFlags) {
    if item_flags.contains(ItemFlags::MOVEBLOCK) {
        tile.flags.insert(MapFlags::TMOVEBLOCK);
    }
    if item_flags.contains(ItemFlags::SIGHTBLOCK) {
        tile.flags.insert(MapFlags::TSIGHTBLOCK);
    }
    if item_flags.contains(ItemFlags::SOUNDBLOCK) {
        tile.flags.insert(MapFlags::TSOUNDBLOCK);
    }
    if item_flags.contains(ItemFlags::DOOR) {
        tile.flags.insert(MapFlags::DOOR);
    }
}
