use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FdemonCannonShot {
    start_x: u16,
    start_y: u16,
    target_x: u16,
    target_y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaligarWeightDoorResult {
    Moved,
    Locked,
    Busy,
    Noop,
}

impl World {
    pub(crate) fn move_edemon_block(
        &mut self,
        item_id: ItemId,
        target_x: u16,
        target_y: u16,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let from_x = usize::from(item.x);
        let from_y = usize::from(item.y);
        let to_x = usize::from(target_x);
        let to_y = usize::from(target_y);
        let Some(target) = self.map.tile(to_x, to_y) else {
            return false;
        };
        if target
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
            || !(12150..=12158).contains(&(target.ground_sprite & 0xffff))
        {
            return false;
        }

        if let Some(source) = self.map.tile_mut(from_x, from_y) {
            if source.item == item_id.0 {
                source.item = 0;
                source.flags.remove(MapFlags::TMOVEBLOCK);
                self.mark_dirty_sector(from_x, from_y);
            }
        }
        if let Some(target) = self.map.tile_mut(to_x, to_y) {
            target.item = item_id.0;
            target.flags.insert(MapFlags::TMOVEBLOCK);
            self.mark_dirty_sector(to_x, to_y);
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.x = target_x;
            item.y = target_y;
        }
        true
    }

    pub(crate) fn pulse_edemon_tube(&mut self, item_id: ItemId, target_x: u16, target_y: u16) {
        if target_x == 0 || target_y == 0 {
            return;
        }
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);
        let targets: Vec<_> = self
            .characters
            .values()
            .filter(|character| {
                character
                    .flags
                    .contains(CharacterFlags::USED | CharacterFlags::PLAYER)
                    && (i32::from(character.x) - item_x).abs() <= 10
                    && (i32::from(character.y) - item_y).abs() <= 10
                    && char_see_item(character, &item, &self.map, self.date.daylight)
            })
            .map(|character| character.id)
            .collect();

        for character_id in targets {
            if self.teleport_character(character_id, target_x, target_y, false) {
                self.pending_system_texts.push(WorldSystemText {
                    character_id,
                    message: "The strange tube teleported you.".to_string(),
                });
            }
        }
    }

    pub(crate) fn dungeon_door_context(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> (bool, bool, u16) {
        let Some(item) = self.items.get(&item_id) else {
            return (false, false, 0);
        };
        let key1 = read_u32_le_at(&item.driver_data, 0);
        let key2 = read_u32_le_at(&item.driver_data, 4);
        let has_key1 = key1 == 0 || self.character_has_template_id(character_id, key1);
        let has_key2 = key2 == 0 || self.character_has_template_id(character_id, key2);

        let catacomb = ((usize::from(item.x).saturating_sub(2)) / 81)
            + ((usize::from(item.y).saturating_sub(2)) / 81) * 3;
        let xf = (catacomb % 3) * 81 + 2;
        let yf = (catacomb / 3) * 81 + 2;
        let mut defenders = 0u16;
        for x in xf..xf + 80 {
            for y in yf..yf + 80 {
                let Some(tile) = self.map.tile(x, y) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                if let Some(character) =
                    self.characters.get(&CharacterId(u32::from(tile.character)))
                {
                    if !character.flags.contains(CharacterFlags::PLAYER) {
                        defenders = defenders.saturating_add(1);
                    }
                }
            }
        }

        (has_key1, has_key2, defenders)
    }

    pub(crate) fn warp_trial_door_context(
        &mut self,
        item_id: ItemId,
    ) -> Option<WarpTrialDoorContext> {
        let item = self.items.get(&item_id)?;
        let cached = item.driver_data.get(2).copied().unwrap_or(0) != 0;
        let (xs, ys, xe, ye, partner_id) = if cached {
            let xs = u16::from(item.driver_data.get(2).copied().unwrap_or(0));
            let ys = u16::from(item.driver_data.get(3).copied().unwrap_or(0));
            let xe = u16::from(item.driver_data.get(4).copied().unwrap_or(0));
            let ye = u16::from(item.driver_data.get(5).copied().unwrap_or(0));
            let partner_id = u16::from(item.driver_data.get(6).copied().unwrap_or(0))
                | (u16::from(item.driver_data.get(7).copied().unwrap_or(0)) << 8);
            (xs, ys, xe, ye, partner_id)
        } else {
            let discovered = self.discover_warp_trial_door_bounds(item_id)?;
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(8, 0);
                item.driver_data[2] = discovered.0 as u8;
                item.driver_data[3] = discovered.1 as u8;
                item.driver_data[4] = discovered.2 as u8;
                item.driver_data[5] = discovered.3 as u8;
                item.driver_data[6] = (discovered.4 & 0xff) as u8;
                item.driver_data[7] = (discovered.4 >> 8) as u8;
            }
            discovered
        };
        let partner = self.items.get(&ItemId(u32::from(partner_id)))?;
        let mut room_has_non_simple_baddy = false;
        for y in ys.saturating_add(1)..ye {
            for x in xs.saturating_add(1)..xe {
                let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                let character_id = CharacterId(u32::from(tile.character));
                if self
                    .characters
                    .get(&character_id)
                    .is_some_and(|character| character.driver != CDR_SIMPLEBADDY)
                {
                    room_has_non_simple_baddy = true;
                    break;
                }
            }
            if room_has_non_simple_baddy {
                break;
            }
        }

        Some(WarpTrialDoorContext {
            xs,
            ys,
            xe,
            ye,
            partner_x: partner.x,
            partner_y: partner.y,
            room_has_non_simple_baddy,
        })
    }

    pub(crate) fn discover_warp_trial_door_bounds(
        &self,
        item_id: ItemId,
    ) -> Option<(u16, u16, u16, u16, u16)> {
        let item = self.items.get(&item_id)?;
        let ix = i32::from(item.x);
        let iy = i32::from(item.y);
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for (dx, dy) in dirs {
            for step in 1..15 {
                let x = ix + dx * step;
                let y = iy + dy * step;
                if x < 0 || y < 0 {
                    break;
                }
                let Some(tile) = self.map.tile(x as usize, y as usize) else {
                    break;
                };
                if tile.item != 0 {
                    let partner_id = ItemId(tile.item);
                    if self
                        .items
                        .get(&partner_id)
                        .is_some_and(|partner| partner.driver == IDR_WARPTRIALDOOR)
                    {
                        return self.warp_trial_bounds_for_pair(item, partner_id, dx, dy);
                    }
                }
                if tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                {
                    break;
                }
            }
        }
        None
    }

    pub(crate) fn warp_trial_bounds_for_pair(
        &self,
        item: &Item,
        partner_id: ItemId,
        dx: i32,
        dy: i32,
    ) -> Option<(u16, u16, u16, u16, u16)> {
        let partner = self.items.get(&partner_id)?;
        let ix = i32::from(item.x);
        let iy = i32::from(item.y);
        let (xs, xe, ys, ye) = if dx != 0 {
            let xs = item.x.min(partner.x);
            let xe = item.x.max(partner.x);
            let scan_x = ix + dx;
            let ye = self.scan_warp_trial_wall(scan_x, iy, 0, 1)?;
            let ys = self.scan_warp_trial_wall(scan_x, iy, 0, -1)?;
            (xs, xe, ys, ye)
        } else if dy != 0 {
            let ys = item.y.min(partner.y);
            let ye = item.y.max(partner.y);
            let scan_y = iy + dy;
            let xe = self.scan_warp_trial_wall(ix, scan_y, 1, 0)?;
            let xs = self.scan_warp_trial_wall(ix, scan_y, -1, 0)?;
            (xs, xe, ys, ye)
        } else {
            return None;
        };
        Some((xs, ys, xe, ye, partner_id.0 as u16))
    }

    pub(crate) fn scan_warp_trial_wall(&self, x: i32, y: i32, dx: i32, dy: i32) -> Option<u16> {
        for step in 0..15 {
            let sx = x + dx * step;
            let sy = y + dy * step;
            if sx < 0 || sy < 0 {
                return None;
            }
            let tile = self.map.tile(sx as usize, sy as usize)?;
            if tile.flags.contains(MapFlags::MOVEBLOCK) {
                return if dx != 0 {
                    u16::try_from(sx).ok()
                } else {
                    u16::try_from(sy).ok()
                };
            }
        }
        None
    }

    pub(crate) fn has_matching_random_shrine_key(
        &self,
        character_id: CharacterId,
        shrine_item_id: ItemId,
    ) -> bool {
        let Some(shrine) = self.items.get(&shrine_item_id) else {
            return false;
        };
        let required_level = shrine.driver_data.get(1).copied().unwrap_or(0);
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };

        character
            .inventory
            .iter()
            .flatten()
            .copied()
            .chain(character.cursor_item)
            .any(|item_id| {
                self.items.get(&item_id).is_some_and(|item| {
                    item.template_id == IID_AREA14_SHRINEKEY
                        && item.driver_data.first().copied().unwrap_or(0) == required_level
                })
            })
    }

    pub(crate) fn clanspawn_is_contested(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        for tx in x.saturating_sub(1)..=x.saturating_add(1) {
            for ty in y.saturating_sub(1)..=y.saturating_add(1) {
                let Some(tile) = self.map.tile(tx, ty) else {
                    continue;
                };
                if tile.character == 0 || u32::from(tile.character) == character_id.0 {
                    continue;
                }
                if self
                    .characters
                    .get(&CharacterId(u32::from(tile.character)))
                    .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER))
                {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn swamp_arm_triggered(&self, item_id: ItemId) -> Option<bool> {
        let item = self.items.get(&item_id)?;
        if item.driver_data.first().copied().unwrap_or_default() != 0 {
            return Some(true);
        }
        let x = i32::from(item.x);
        let y = i32::from(item.y);
        let horizontal = [(-1, 0), (-2, 0), (1, 0), (2, 0)]
            .iter()
            .any(|(dx, dy)| self.map_character_at(x + dx, y + dy).is_some());
        if horizontal {
            return Some((self.tick.0 + u64::from(item.id.0)) % 5 == 0);
        }
        let adjacent_row = [
            (-1, -1),
            (-2, -1),
            (1, -1),
            (2, -1),
            (-1, 1),
            (-2, 1),
            (1, 1),
            (2, 1),
        ]
        .iter()
        .any(|(dx, dy)| self.map_character_at(x + dx, y + dy).is_some());
        if adjacent_row {
            return Some((self.tick.0 + u64::from(item.id.0)) % 3 == 0);
        }
        Some(false)
    }

    pub(crate) fn swamp_arm_damage_targets(&self, item_id: ItemId) -> Vec<CharacterId> {
        let Some(item) = self.items.get(&item_id) else {
            return Vec::new();
        };
        let x = i32::from(item.x);
        let y = i32::from(item.y);
        [1, 2, -1, -2]
            .into_iter()
            .filter_map(|dx| self.map_character_at(x + dx, y))
            .collect()
    }

    pub(crate) fn swamp_whisp_move_succeeds(&self, item_id: ItemId) -> Option<bool> {
        let item = self.items.get(&item_id)?;
        let frame = item.driver_data.first().copied().unwrap_or_default();
        let direction = item
            .driver_data
            .get(3)
            .copied()
            .unwrap_or(Direction::Down as u8);
        let target = match direction {
            value if value == Direction::Down as u8 && frame.wrapping_add(1) == 12 => {
                Some((usize::from(item.x), usize::from(item.y).saturating_add(1)))
            }
            value if value == Direction::Up as u8 && frame.wrapping_sub(1) == 2 => {
                Some((usize::from(item.x), usize::from(item.y).saturating_sub(1)))
            }
            value if value == Direction::Left as u8 && frame.wrapping_add(1) > 15 => {
                Some((usize::from(item.x).saturating_add(1), usize::from(item.y)))
            }
            value if value == Direction::Right as u8 && frame.wrapping_sub(1) == 6 => {
                Some((usize::from(item.x).saturating_sub(1), usize::from(item.y)))
            }
            _ => None,
        };
        let Some((x, y)) = target else {
            return Some(false);
        };
        Some(self.item_can_be_set_on_map(item, x, y))
    }

    pub(crate) fn swamp_spawn_live(&self, item_id: ItemId) -> Option<bool> {
        let item = self.items.get(&item_id)?;
        let character_id = u32::from_le_bytes([
            *item.driver_data.get(4).unwrap_or(&0),
            *item.driver_data.get(5).unwrap_or(&0),
            *item.driver_data.get(6).unwrap_or(&0),
            *item.driver_data.get(7).unwrap_or(&0),
        ]);
        let serial = u32::from_le_bytes([
            *item.driver_data.get(8).unwrap_or(&0),
            *item.driver_data.get(9).unwrap_or(&0),
            *item.driver_data.get(10).unwrap_or(&0),
            *item.driver_data.get(11).unwrap_or(&0),
        ]);
        if character_id == 0 {
            return Some(false);
        }
        Some(
            self.characters
                .get(&CharacterId(character_id))
                .is_some_and(|character| {
                    character.flags.contains(CharacterFlags::USED) && character.serial == serial
                }),
        )
    }

    pub(crate) fn swamp_spawn_player_close(&self, item_id: ItemId, distance: u16) -> Option<bool> {
        let item = self.items.get(&item_id)?;
        let x = i32::from(item.x);
        let y = i32::from(item.y);
        let distance = i32::from(distance);
        Some(self.characters.values().any(|character| {
            character.flags.contains(CharacterFlags::PLAYER)
                && (i32::from(character.x) - x).abs() <= distance
                && (i32::from(character.y) - y).abs() <= distance
        }))
    }

    pub(crate) fn swamp_spawn_ground_sprite(&self, item_id: ItemId) -> Option<u32> {
        let item = self.items.get(&item_id)?;
        self.map
            .tile(usize::from(item.x), usize::from(item.y))
            .map(|tile| tile.ground_sprite)
    }

    pub(crate) fn apply_fdemon_farm_foreground(&mut self, item_id: ItemId, foreground_sprite: u32) {
        let item_pos = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)));
        if let Some((x, y)) = item_pos {
            if let Some(tile) = self.map.tile_mut(x, y) {
                let new_foreground_sprite =
                    (tile.foreground_sprite & 0xffff) | (foreground_sprite << 16);
                if tile.foreground_sprite != new_foreground_sprite {
                    tile.foreground_sprite = new_foreground_sprite;
                    self.mark_dirty_sector(x, y);
                }
            }
        }
    }

    pub(crate) fn apply_fdemon_lava_tile(
        &mut self,
        item_id: ItemId,
        stage: u8,
    ) -> Option<CharacterId> {
        let item_pos = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)))?;
        let (x, y) = item_pos;
        let mut target_id = None;
        let mut changed = false;
        if let Some(tile) = self.map.tile_mut(x, y) {
            if tile.character != 0 {
                target_id = Some(CharacterId(u32::from(tile.character)));
            }
            if stage == 0 {
                let flags = tile.flags | MapFlags::MOVEBLOCK | MapFlags::FIRETHRU;
                if tile.flags != flags {
                    tile.flags = flags;
                    changed = true;
                }
                let foreground = tile.foreground_sprite & 0xffff;
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else if stage < 20 {
                let foreground = (tile.foreground_sprite & 0xffff) | (1024 << 16);
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else if stage < 115 {
                let foreground = tile.foreground_sprite & 0xffff;
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else {
                if tile.flags.contains(MapFlags::MOVEBLOCK) {
                    tile.flags.remove(MapFlags::MOVEBLOCK);
                    changed = true;
                }
                let foreground = (tile.foreground_sprite & 0xffff) | (1034 << 16);
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            }
        }
        if changed {
            self.mark_dirty_sector(x, y);
        }
        target_id
    }

    pub(crate) fn apply_fdemon_waypoint(
        &mut self,
        item_id: ItemId,
        spotted_enemy: bool,
        target_character_id: Option<CharacterId>,
        target_serial: Option<u32>,
    ) {
        let Some((x, y)) = self.items.get_mut(&item_id).map(|item| {
            item.driver_data.resize(12, 0);
            item.driver_data[0] = u8::from(spotted_enemy);
            item.sprite = if spotted_enemy { 14200 } else { 14202 };
            let target_id = target_character_id.map_or(0, |id| id.0);
            item.driver_data[4..8].copy_from_slice(&target_id.to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&target_serial.unwrap_or(0).to_le_bytes());
            (usize::from(item.x), usize::from(item.y))
        }) else {
            return;
        };
        self.mark_dirty_sector(x, y);
    }

    pub(crate) fn pulse_fdemon_cannon(&mut self, item_id: ItemId) -> bool {
        let Some((loader_ids, power)) = fdemon_cannon_loaders_and_power(&self.items, item_id)
        else {
            return false;
        };
        let Some(cannon) = self.items.get(&item_id).cloned() else {
            return false;
        };

        if power == 0 {
            let mut dirty = None;
            if let Some(cannon) = self.items.get_mut(&item_id) {
                if cannon.sprite & 1 != 0 {
                    cannon.sprite &= !1;
                    dirty = Some((usize::from(cannon.x), usize::from(cannon.y)));
                }
            }
            if let Some((x, y)) = dirty {
                self.mark_dirty_sector(x, y);
            }
            return true;
        }

        if let Some(shot) = self.find_fdemon_cannon_target(&cannon) {
            let shot_power = power / 50 + 1;
            self.create_edemonball_effect(
                shot.start_x,
                shot.start_y,
                shot.target_x,
                shot.target_y,
                i32::from(shot_power),
                2,
            );
            self.drain_fdemon_cannon_loaders(&loader_ids, shot_power);
        }

        let mut dirty = None;
        if let Some(cannon) = self.items.get_mut(&item_id) {
            if cannon.sprite & 1 == 0 {
                cannon.sprite |= 1;
                dirty = Some((usize::from(cannon.x), usize::from(cannon.y)));
            }
        }
        if let Some((x, y)) = dirty {
            self.mark_dirty_sector(x, y);
        }
        true
    }

    pub(crate) fn drain_fdemon_cannon_loaders(&mut self, loader_ids: &[ItemId; 3], mut drain: u16) {
        for loader_id in loader_ids {
            if drain == 0 {
                break;
            }
            let Some(loader) = self.items.get_mut(loader_id) else {
                continue;
            };
            let power = read_driver_data_u16(&loader.driver_data, 1).unwrap_or_default();
            let spent = power.min(drain);
            write_driver_data_u16(&mut loader.driver_data, 1, power - spent);
            drain -= spent;
        }
    }

    pub(crate) fn find_fdemon_cannon_target(&self, cannon: &Item) -> Option<FdemonCannonShot> {
        let (dx, dy) = Direction::try_from(cannon.driver_data.get(12).copied().unwrap_or_default())
            .ok()
            .map(|direction| direction.delta())
            .unwrap_or((0, 0));
        let mut best: Option<(u16, FdemonCannonShot)> = None;

        for target in self.characters.values() {
            if target
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
            {
                continue;
            }
            if target.x.abs_diff(cannon.x) > 17 || target.y.abs_diff(cannon.y) > 17 {
                continue;
            }

            let (ox, oy) = if target.x.abs_diff(cannon.x) > target.y.abs_diff(cannon.y) {
                (
                    i16::from(target.x > cannon.x) - i16::from(target.x < cannon.x),
                    0,
                )
            } else {
                (
                    0,
                    i16::from(target.y > cannon.y) - i16::from(target.y < cannon.y),
                )
            };
            if dx != ox || dy != oy {
                continue;
            }

            let Some(start_x) = offset_u16(cannon.x, ox) else {
                continue;
            };
            let Some(start_y) = offset_u16(cannon.y, oy) else {
                continue;
            };
            if !self.fdemon_cannon_can_hit(
                cannon.id.0,
                target.id,
                start_x,
                start_y,
                target.x,
                target.y,
            ) {
                continue;
            }

            let (target_x, target_y) = self.fdemon_cannon_predicted_target(cannon, target);
            let distance = target.x.abs_diff(cannon.x) + target.y.abs_diff(cannon.y);
            let shot = FdemonCannonShot {
                start_x,
                start_y,
                target_x,
                target_y,
            };
            if best
                .as_ref()
                .is_none_or(|(best_distance, _)| distance < *best_distance)
            {
                best = Some((distance, shot));
            }
        }

        best.map(|(_, shot)| shot)
    }

    pub(crate) fn fdemon_cannon_predicted_target(
        &self,
        cannon: &Item,
        target: &Character,
    ) -> (u16, u16) {
        if target.action != action::WALK {
            return (target.x, target.y);
        }
        let Ok(direction) = Direction::try_from(target.dir) else {
            return (target.x, target.y);
        };
        let (dx, dy) = direction.delta();
        let dist = map_dist(cannon.x, cannon.y, target.x, target.y);
        let mut eta = dist * 3 / 2;
        eta -= target.duration - target.step;
        if eta <= 0 {
            return (target.tox, target.toy);
        }
        for step in 1..10i32 {
            eta -= target.duration;
            if eta <= 0 {
                let x = i32::from(target.x) + i32::from(dx) * step;
                let y = i32::from(target.y) + i32::from(dy) * step;
                return (clamp_world_coordinate(x), clamp_world_coordinate(y));
            }
        }
        (target.x, target.y)
    }

    pub(crate) fn fdemon_cannon_can_hit(
        &self,
        cannon_raw_id: u32,
        target_id: CharacterId,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
    ) -> bool {
        let mut x = i32::from(from_x) * 1024 + 512;
        let mut y = i32::from(from_y) * 1024 + 512;
        let raw_dx = i32::from(to_x) - i32::from(from_x);
        let raw_dy = i32::from(to_y) - i32::from(from_y);
        if raw_dx.abs() < 2 && raw_dy.abs() < 2 {
            return false;
        }
        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 256 / raw_dx.abs(), raw_dy * 256 / raw_dx.abs())
        } else {
            (raw_dx * 256 / raw_dy.abs(), raw_dy * 256 / raw_dy.abs())
        };
        for _ in 0..48 {
            x += step_x;
            y += step_y;
            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if tile_x == i32::from(to_x) && tile_y == i32::from(to_y) {
                return true;
            }
            if self.edemonball_map_blocked(tile_x, tile_y) {
                let Some(tile) = usize::try_from(tile_x)
                    .ok()
                    .zip(usize::try_from(tile_y).ok())
                    .and_then(|(x, y)| self.map.tile(x, y))
                else {
                    return false;
                };
                if tile.item == cannon_raw_id || u32::from(tile.character) == target_id.0 {
                    return true;
                }
                return false;
            }
        }
        true
    }

    pub(crate) fn first_free_mine_keyholder_room(&self) -> Option<(u16, u16)> {
        for room in 0..9u16 {
            let base_x = 2 + (room % 3) * 8;
            let base_y = 231 + (room / 3) * 8;
            let mut occupied = false;
            'tiles: for x in base_x..base_x + 7 {
                for y in base_y..base_y + 7 {
                    let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) else {
                        occupied = true;
                        break 'tiles;
                    };
                    if tile.character != 0 {
                        occupied = true;
                        break 'tiles;
                    }
                    if tile.item != 0
                        && self
                            .items
                            .get(&ItemId(tile.item))
                            .is_some_and(|item| item.flags.contains(ItemFlags::TAKE))
                    {
                        occupied = true;
                        break 'tiles;
                    }
                }
            }
            if !occupied {
                return Some((base_x + 1, base_y + 3));
            }
        }
        None
    }

    pub(crate) fn teufel_arena_equipment_block(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> Option<ItemDriverOutcome> {
        let equipment = self
            .characters
            .get(&character_id)?
            .inventory
            .iter()
            .take(12)
            .flatten()
            .copied()
            .collect::<Vec<_>>();

        for worn_item_id in equipment {
            let Some(worn) = self.items.get(&worn_item_id) else {
                continue;
            };
            if (53001..=53006).contains(&worn.sprite) {
                continue;
            }
            if counted_enhancement_modifiers(worn) > 1 {
                self.pending_system_texts.push(WorldSystemText {
                    character_id,
                    message: format!(
                        "You cannot enter while wearing your {}. It has more than one enhancement.",
                        worn.name
                    ),
                });
                return Some(ItemDriverOutcome::TeufelArenaEquipmentEnhanced {
                    item_id,
                    character_id,
                });
            }
            if worn
                .flags
                .intersects(ItemFlags::QUEST | ItemFlags::BONDTAKE | ItemFlags::BONDWEAR)
            {
                self.pending_system_texts.push(WorldSystemText {
                    character_id,
                    message: format!(
                        "You cannot enter while wearing your {}. It is a quest or a bound item.",
                        worn.name
                    ),
                });
                return Some(ItemDriverOutcome::TeufelArenaEquipmentBound {
                    item_id,
                    character_id,
                });
            }
        }
        None
    }

    pub fn apply_skelraise_raise(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        raised_id: CharacterId,
        raised_serial: u32,
    ) -> bool {
        if !self.characters.contains_key(&character_id) || !self.characters.contains_key(&raised_id)
        {
            return false;
        }
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            item.driver_data[2] = 1;
            item.driver_data[4..8].copy_from_slice(&raised_id.0.to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&raised_serial.to_le_bytes());
            item.sprite += 1;
            (usize::from(item.x), usize::from(item.y))
        };
        self.destroy_item(cursor_item_id);
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        true
    }

    pub fn apply_skelraise_dust(&mut self, item_id: ItemId) -> bool {
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            item.driver_data[2] = 1;
            item.driver_data[4..12].fill(0);
            item.sprite += 1;
            (usize::from(item.x), usize::from(item.y))
        };
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        true
    }

    pub(crate) fn apply_skelraise_timer(&mut self, item_id: ItemId) -> bool {
        let (raised_id, raised_serial, active, x, y) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            let active = item.driver_data.get(2).copied().unwrap_or_default() != 0;
            let raised_id = if item.driver_data.len() >= 8 {
                CharacterId(u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]))
            } else {
                CharacterId(0)
            };
            let raised_serial = if item.driver_data.len() >= 12 {
                u32::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                    item.driver_data[10],
                    item.driver_data[11],
                ])
            } else {
                0
            };
            (
                raised_id,
                raised_serial,
                active,
                usize::from(item.x),
                usize::from(item.y),
            )
        };
        if !active {
            return true;
        }
        let still_alive = raised_id.0 != 0
            && self.characters.get(&raised_id).is_some_and(|character| {
                (raised_serial == 0 || character.serial == raised_serial)
                    && !character.flags.is_empty()
            });
        if still_alive {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
            return true;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            item.driver_data[2] = 0;
            item.sprite -= 1;
        }
        self.mark_dirty_sector(x, y);
        true
    }

    pub(crate) fn place_bone_bridge(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if character.cursor_item != Some(cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let dx = i32::from(item.x)
            .saturating_sub(i32::from(character.x))
            .signum();
        let dy = i32::from(item.y)
            .saturating_sub(i32::from(character.y))
            .signum();
        let target_x = i32::from(item.x) + dx;
        let target_y = i32::from(item.y) + dy;
        if target_x < 2
            || target_y < 2
            || target_x >= MAX_MAP as i32 - 2
            || target_y >= MAX_MAP as i32 - 2
        {
            return false;
        }
        let target_x = target_x as usize;
        let target_y = target_y as usize;
        let Some(tile) = self.map.tile(target_x, target_y) else {
            return false;
        };
        if tile.item != 0 || !tile.flags.contains(MapFlags::MOVEBLOCK) {
            return false;
        }
        let Some(cursor) = self.items.get(&cursor_item_id) else {
            return false;
        };
        if cursor.carried_by != Some(character_id) {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.item = cursor_item_id.0;
            tile.flags.remove(MapFlags::MOVEBLOCK);
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.cursor_item = None;
            character.flags.insert(CharacterFlags::ITEMS);
        }
        if let Some(cursor) = self.items.get_mut(&cursor_item_id) {
            cursor.carried_by = None;
            cursor.contained_in = None;
            cursor.x = target_x as u16;
            cursor.y = target_y as u16;
            cursor.flags.remove(ItemFlags::TAKE);
            cursor.driver_data.resize(2, 0);
            cursor.driver_data[1] = 1;
            cursor.sprite = if dx == 0 { 13045 } else { 13035 };
        }
        self.mark_dirty_sector(target_x, target_y);
        self.schedule_item_driver_timer(cursor_item_id, CharacterId(0), TICKS_PER_SECOND * 60);
        true
    }

    /// C `bonebridge`'s "bones in inventory" add-bone branch
    /// (`bones.c:236-252`): increments the carried bridge item's own bone
    /// count (the caller already checked `<= 4`), consumes one bone from
    /// the cursor stack item, and destroys the cursor item once its own
    /// count reaches zero.
    pub(crate) fn add_bone_to_bridge(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    ) {
        if let Some(item) = self.items.get_mut(&item_id) {
            if item.driver_data.is_empty() {
                item.driver_data.resize(1, 0);
            }
            item.driver_data[0] = item.driver_data[0].saturating_add(1);
            item.sprite = 13030 + i32::from(item.driver_data[0]);
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
        let exhausted = if let Some(cursor) = self.items.get_mut(&cursor_item_id) {
            if cursor.driver_data.is_empty() {
                cursor.driver_data.resize(1, 0);
            }
            cursor.driver_data[0] = cursor.driver_data[0].saturating_sub(1);
            cursor.sprite = 13030 + i32::from(cursor.driver_data[0]);
            cursor.driver_data[0] == 0
        } else {
            false
        };
        if exhausted {
            self.destroy_item(cursor_item_id);
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.cursor_item = None;
            }
        }
    }

    /// C `bonebridge`'s "bones in inventory" remove-bone branch
    /// (`bones.c:257-269`): decrements the carried bridge item's own bone
    /// count (the caller already checked `>= 2`). The new "bone" item
    /// that lands on the cursor needs a `ZoneLoader` template
    /// instantiation the server crate applies (same precedent as
    /// `BoneHolderRemoveRune`).
    pub(crate) fn remove_bone_from_bridge(&mut self, item_id: ItemId, character_id: CharacterId) {
        if let Some(item) = self.items.get_mut(&item_id) {
            if item.driver_data.is_empty() {
                item.driver_data.resize(1, 0);
            }
            item.driver_data[0] = item.driver_data[0].saturating_sub(1);
            item.sprite = 13030 + i32::from(item.driver_data[0]);
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
    }

    pub(crate) fn tick_bone_bridge(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver_data.get(1).copied().unwrap_or_default() == 0 || item.carried_by.is_some() {
            return false;
        }
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let Some(tile) = self.map.tile(x, y) else {
            return false;
        };
        if tile.item != item_id.0 {
            return false;
        }
        if tile.flags.contains(MapFlags::TMOVEBLOCK) {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            return true;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.insert(MapFlags::MOVEBLOCK);
        }
        let remove = if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(2, 0);
            item.driver_data[1] = item.driver_data[1].saturating_add(1);
            item.sprite += 1;
            item.driver_data[1] > 9
        } else {
            return false;
        };
        self.mark_dirty_sector(x, y);
        if remove {
            self.destroy_item(item_id)
        } else {
            self.schedule_item_driver_timer(item_id, CharacterId(0), 3)
        }
    }

    pub(crate) fn tick_bone_wall(&mut self, item_id: ItemId) -> bool {
        let (x, y, state) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.first().copied().unwrap_or_default(),
            )
        };
        if !self.map.legacy_inner_bounds(x, y) {
            return false;
        }

        if state == 0 {
            for (nx, ny) in [
                (x.saturating_add(1), y),
                (x.saturating_sub(1), y),
                (x, y.saturating_add(1)),
                (x, y.saturating_sub(1)),
            ] {
                let Some(tile) = self.map.tile(nx, ny) else {
                    continue;
                };
                let neighbor_id = ItemId(tile.item);
                if neighbor_id.0 == 0 {
                    continue;
                }
                let Some(neighbor) = self.items.get(&neighbor_id) else {
                    continue;
                };
                if neighbor.driver == IDR_BONEWALL
                    && neighbor.driver_data.first().copied().unwrap_or_default() == 0
                {
                    self.schedule_item_driver_timer_with_context(
                        neighbor_id,
                        CharacterId(0),
                        4,
                        false,
                    );
                }
            }
        }

        if state < 5 {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(1, 0);
            item.driver_data[0] = state.saturating_add(1);
            item.sprite = item.sprite.saturating_add(1);
            self.mark_dirty_sector(x, y);
            self.schedule_item_driver_timer(item_id, CharacterId(0), 2);
            return true;
        }

        if state == 5 {
            if let Some(tile) = self.map.tile_mut(x, y) {
                if tile.item == item_id.0 {
                    tile.item = 0;
                }
                tile.flags
                    .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            }
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.flags.remove(ItemFlags::USE);
            item.flags.insert(ItemFlags::VOID);
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 6;
            self.mark_dirty_sector(x, y);
            self.schedule_item_driver_timer(
                item_id,
                CharacterId(0),
                TICKS_PER_SECOND.saturating_mul(60),
            );
            return true;
        }

        if state == 6 {
            let blocked = self
                .map
                .tile(x, y)
                .is_some_and(|tile| tile.item != 0 || tile.flags.contains(MapFlags::TMOVEBLOCK));
            if blocked {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
                return true;
            }

            if let Some(tile) = self.map.tile_mut(x, y) {
                tile.item = item_id.0;
                tile.flags
                    .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            }
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.sprite = item.sprite.saturating_sub(5);
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 0;
            item.flags.insert(ItemFlags::USE);
            item.flags.remove(ItemFlags::VOID);
            self.mark_dirty_sector(x, y);
            return true;
        }

        false
    }

    pub(crate) fn apply_staffer_mine_dig(&mut self, item_id: ItemId) -> bool {
        let (x, y, stage) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.get(3).copied().unwrap_or_default(),
            )
        };

        if stage == 3 {
            let before = self.items.get(&item_id).cloned();
            if let Some(tile) = self.map.tile_mut(x, y) {
                tile.flags.remove(MapFlags::TSIGHTBLOCK);
            }
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::SIGHTBLOCK);
            }
            if let Some(before) = before.as_ref() {
                self.refresh_item_light_after_mutation(before, item_id);
            }
        }

        if stage == 8 {
            if let Some(tile) = self.map.tile_mut(x, y) {
                if tile.item == item_id.0 {
                    tile.item = 0;
                }
                tile.flags.remove(MapFlags::TMOVEBLOCK);
            }
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::USE);
                item.flags.insert(ItemFlags::VOID);
            }
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 60 * 5);
        }

        self.mark_dirty_sector(x, y);
        true
    }

    pub(crate) fn apply_staffer_mine_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, stage, initialized) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.get(3).copied().unwrap_or_default(),
                item.driver_data.get(4).copied().unwrap_or_default() != 0,
            )
        };

        if !initialized {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(5, 0);
                item.driver_data[4] = 1;
                item.sprite = match (u32::from(item.x) + u32::from(item.y)) % 3 {
                    0 => 15070,
                    1 => 15078,
                    _ => 15086,
                };
            }
        }

        if stage != 8 {
            return true;
        }

        let blocked = self
            .map
            .tile(x, y)
            .is_none_or(|tile| tile.flags.contains(MapFlags::TMOVEBLOCK) || tile.item != 0);
        if blocked {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            return true;
        }

        let before = self.items.get(&item_id).cloned();
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite -= 8;
            item.driver_data.resize(4, 0);
            item.driver_data[3] = 0;
            item.flags.insert(ItemFlags::USE | ItemFlags::SIGHTBLOCK);
            item.flags.remove(ItemFlags::VOID);
        }
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.item = item_id.0;
            tile.flags
                .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
        }
        if let Some(before) = before.as_ref() {
            self.refresh_item_light_after_mutation(before, item_id);
        }
        self.mark_dirty_sector(x, y);
        true
    }

    pub(crate) fn apply_staffer_block_move(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Ok(direction) = Direction::try_from(character.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let target_x_i = i32::from(item.x) + i32::from(dx);
        let target_y_i = i32::from(item.y) + i32::from(dy);
        if target_x_i < 0 || target_y_i < 0 {
            return false;
        }
        let target_x = target_x_i as usize;
        let target_y = target_y_i as usize;
        let Some(target) = self.map.tile(target_x, target_y) else {
            return false;
        };
        let gsprite = target.ground_sprite;
        let wrong_sprite =
            (gsprite < 20291 || gsprite > 20299) && gsprite != 13154 && gsprite > 13156;
        if target
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
            || wrong_sprite
        {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
            if tile.item == item_id.0 {
                tile.item = 0;
            }
        }
        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
            tile.item = item_id.0;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            item.x = target_x as u16;
            item.y = target_y as u16;
            item.driver_data[4..8].copy_from_slice(&(self.tick.0 as u32).to_le_bytes());
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        self.mark_dirty_sector(x, y);
        self.mark_dirty_sector(target_x, target_y);
        true
    }

    pub(crate) fn apply_staffer_block_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, home_x, home_y, last_touch) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            (
                usize::from(item.x),
                usize::from(item.y),
                usize::from(u16::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                ])),
                usize::from(u16::from_le_bytes([
                    item.driver_data[10],
                    item.driver_data[11],
                ])),
                u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]) as u64,
            )
        };

        if self.tick.0.saturating_sub(last_touch) > TICKS_PER_SECOND * 60 * 2
            && (home_x != x || home_y != y)
        {
            let home_free = self.map.tile(home_x, home_y).is_some_and(|tile| {
                !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                    && tile.item == 0
            });
            if home_free {
                if let Some(tile) = self.map.tile_mut(x, y) {
                    tile.flags.remove(MapFlags::TMOVEBLOCK);
                    if tile.item == item_id.0 {
                        tile.item = 0;
                    }
                }
                if let Some(tile) = self.map.tile_mut(home_x, home_y) {
                    tile.flags.insert(MapFlags::TMOVEBLOCK);
                    tile.item = item_id.0;
                }
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.x = home_x as u16;
                    item.y = home_y as u16;
                }
                self.mark_dirty_sector(x, y);
                self.mark_dirty_sector(home_x, home_y);
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    pub(crate) fn apply_caligar_weight_move(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Ok(direction) = Direction::try_from(character.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let target_x_i = i32::from(item.x) + i32::from(dx);
        let target_y_i = i32::from(item.y) + i32::from(dy);
        if target_x_i < 0 || target_y_i < 0 {
            return false;
        }
        let target_x = target_x_i as usize;
        let target_y = target_y_i as usize;
        let Some(target) = self.map.tile(target_x, target_y) else {
            return false;
        };
        let gsprite = target.ground_sprite;
        let valid_floor = (20797..=20823).contains(&gsprite)
            || gsprite == 59683
            || (20291..=20299).contains(&gsprite);
        if !valid_floor
            || target
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
        {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
            if tile.item == item_id.0 {
                tile.item = 0;
            }
        }
        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
            tile.item = item_id.0;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            item.x = target_x as u16;
            item.y = target_y as u16;
            item.driver_data[4..8].copy_from_slice(&(self.tick.0 as u32).to_le_bytes());
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        self.mark_dirty_sector(x, y);
        self.mark_dirty_sector(target_x, target_y);
        true
    }

    pub(crate) fn apply_caligar_weight_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, home_x, home_y, last_touch) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            (
                usize::from(item.x),
                usize::from(item.y),
                usize::from(u16::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                ])),
                usize::from(u16::from_le_bytes([
                    item.driver_data[10],
                    item.driver_data[11],
                ])),
                u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]) as u64,
            )
        };

        if self.tick.0.saturating_sub(last_touch) > TICKS_PER_SECOND * 60 * 5
            && (home_x != x || home_y != y)
        {
            let home_free = self.map.tile(home_x, home_y).is_some_and(|tile| {
                !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                    && tile.item == 0
            });
            if home_free {
                if let Some(tile) = self.map.tile_mut(x, y) {
                    tile.flags.remove(MapFlags::TMOVEBLOCK);
                    if tile.item == item_id.0 {
                        tile.item = 0;
                    }
                }
                if let Some(tile) = self.map.tile_mut(home_x, home_y) {
                    tile.flags.insert(MapFlags::TMOVEBLOCK);
                    tile.item = item_id.0;
                }
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.x = home_x as u16;
                    item.y = home_y as u16;
                }
                self.mark_dirty_sector(x, y);
                self.mark_dirty_sector(home_x, home_y);
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    pub(crate) fn apply_caligar_weight_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> CaligarWeightDoorResult {
        let Some(item) = self.items.get(&item_id) else {
            return CaligarWeightDoorResult::Noop;
        };
        let Some(character) = self.characters.get(&character_id) else {
            return CaligarWeightDoorResult::Noop;
        };
        let dx = i32::from(character.x) - i32::from(item.x);
        let dy = i32::from(character.y) - i32::from(item.y);
        if dx != 0 && dy != 0 {
            return CaligarWeightDoorResult::Noop;
        }

        if dy > 0 {
            let has_lock_weight = |world: &World, x: usize, y: usize| {
                let item_id = world.map.tile(x, y).map(|tile| tile.item).unwrap_or(0);
                item_id != 0
                    && world
                        .items
                        .get(&ItemId(item_id))
                        .is_some_and(|item| item.driver == IDR_CALIGAR)
            };
            if !has_lock_weight(self, 210, 184) || !has_lock_weight(self, 213, 176) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.action = 0;
                    character.step = 0;
                    character.duration = 0;
                }
                return CaligarWeightDoorResult::Locked;
            }
        }

        let target_x = i32::from(item.x) - dx;
        let target_y = i32::from(item.y) - dy;
        if target_x < 1
            || target_y < 1
            || target_x as usize > self.map.width().saturating_sub(2)
            || target_y as usize > self.map.height().saturating_sub(2)
        {
            return CaligarWeightDoorResult::Noop;
        }

        if !self.teleport_character_exact(character_id, target_x as usize, target_y as usize) {
            return CaligarWeightDoorResult::Busy;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.dir = match character.dir {
                value if value == Direction::Right as u8 => Direction::Left as u8,
                value if value == Direction::Left as u8 => Direction::Right as u8,
                value if value == Direction::Up as u8 => Direction::Down as u8,
                value if value == Direction::Down as u8 => Direction::Up as u8,
                value => value,
            };
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        CaligarWeightDoorResult::Moved
    }

    pub fn apply_caligar_skelly_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        door_index: u8,
    ) -> ItemDriverOutcome {
        let Some(item) = self.items.get(&item_id) else {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        };
        let Some(character) = self.characters.get(&character_id) else {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        };
        let dx = i32::from(character.x) - i32::from(item.x);
        let dy = i32::from(character.y) - i32::from(item.y);
        if dx != 0 && dy != 0 {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        }

        let target_x = i32::from(item.x) - dx;
        let target_y = i32::from(item.y) - dy;
        if target_x < 1
            || target_y < 1
            || target_x as usize > self.map.width().saturating_sub(2)
            || target_y as usize > self.map.height().saturating_sub(2)
        {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        }

        if !self.teleport_character_exact(character_id, target_x as usize, target_y as usize) {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.dir = match character.dir {
                value if value == Direction::Right as u8 => Direction::Left as u8,
                value if value == Direction::Left as u8 => Direction::Right as u8,
                value if value == Direction::Up as u8 => Direction::Down as u8,
                value if value == Direction::Down as u8 => Direction::Up as u8,
                value => value,
            };
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }

        ItemDriverOutcome::CaligarSkellyDoor {
            item_id,
            character_id,
            door_index,
        }
    }
}

pub(crate) fn fdemon_loader_power_for_light(
    items: &HashMap<ItemId, Item>,
    light_item_id: ItemId,
) -> Option<u16> {
    let light = items.get(&light_item_id)?;
    let mut max_power = 0u16;
    let mut found = false;

    for loader_nr in 1..=3u8 {
        let nearest = items
            .values()
            .filter(|item| {
                item.driver == IDR_FDEMONLOADER && item.driver_data.first() == Some(&loader_nr)
            })
            .min_by_key(|item| {
                i32::from(light.x).abs_diff(i32::from(item.x))
                    + i32::from(light.y).abs_diff(i32::from(item.y))
            });

        if let Some(loader) = nearest {
            found = true;
            if let Some(bytes) = loader.driver_data.get(1..3) {
                if let Ok(bytes) = <[u8; 2]>::try_from(bytes) {
                    max_power = max_power.max(u16::from_le_bytes(bytes));
                }
            }
        }
    }

    found.then_some(max_power)
}

pub(crate) fn fdemon_cannon_loaders_and_power(
    items: &HashMap<ItemId, Item>,
    cannon_item_id: ItemId,
) -> Option<([ItemId; 3], u16)> {
    let cannon = items.get(&cannon_item_id)?;
    let mut loader_ids = [ItemId(0); 3];
    let mut max_power = 0u16;

    for (index, loader_nr) in (1..=3u8).enumerate() {
        let nearest = items
            .values()
            .filter(|item| {
                item.driver == IDR_FDEMONLOADER && item.driver_data.first() == Some(&loader_nr)
            })
            .min_by_key(|item| {
                i32::from(cannon.x).abs_diff(i32::from(item.x))
                    + i32::from(cannon.y).abs_diff(i32::from(item.y))
            })?;
        loader_ids[index] = nearest.id;
        max_power =
            max_power.max(read_driver_data_u16(&nearest.driver_data, 1).unwrap_or_default());
    }

    Some((loader_ids, max_power))
}

pub(crate) fn edemon_section_power_for_light(
    items: &HashMap<ItemId, Item>,
    light_item_id: ItemId,
) -> Option<u8> {
    let light = items.get(&light_item_id)?;
    let section = light.driver_data.first().copied().unwrap_or_default();
    let mut max_power = 0u8;
    let mut found = false;

    for loader in items.values().filter(|item| {
        item.driver == IDR_EDEMONLOADER && item.driver_data.first() == Some(&section)
    }) {
        found = true;
        max_power = max_power.max(loader.driver_data.get(1).copied().unwrap_or_default());
    }

    found.then_some(max_power)
}

pub(crate) fn edemon_fire_enabled(items: &HashMap<ItemId, Item>) -> bool {
    let mut saw_switch = false;
    for switch in items
        .values()
        .filter(|item| item.driver == IDR_EDEMONSWITCH)
    {
        saw_switch = true;
        if switch.driver_data.first().copied().unwrap_or_default() != 0 {
            return true;
        }
    }
    !saw_switch
}

pub(crate) fn edemon_tube_target(
    items: &HashMap<ItemId, Item>,
    map: &MapGrid,
    tube_item_id: ItemId,
) -> Option<(u16, u16)> {
    let tube = items.get(&tube_item_id)?;
    let section = tube.driver_data.first().copied().unwrap_or_default();

    for loader in items.values().filter(|item| {
        item.driver == IDR_EDEMONLOADER && item.driver_data.first() == Some(&section)
    }) {
        let x = usize::from(loader.x);
        let y = usize::from(loader.y);
        if y < usize::from(u16::MAX) {
            if let Some(tile) = map.tile(x, y + 1) {
                if !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                {
                    return Some((loader.x, loader.y.saturating_add(1)));
                }
            }
        }
        if y > 0 {
            if let Some(tile) = map.tile(x, y - 1) {
                if !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                {
                    return Some((loader.x, loader.y.saturating_sub(1)));
                }
            }
        }
    }

    None
}
