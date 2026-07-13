//! `World`-level mutations for `IDR_SHRIKE` (`src/area/38/shrike.c`)
//! that the pure item-driver boundary (`item_driver/area38_shrike.rs`)
//! can't do itself: map/sprite mutation for the four ambient day/night
//! sub-drivers (tree/rock/door/pedestal), the puzzle-cube push/auto-reset
//! map relocation, and the Pool of the Moon's talisman transformation.
//! The two "hand a fresh amulet component to the player" outcomes
//! (`ShrikeGiveAmuletPiece`/`ShrikeRockDigSuccess`) are deliberately
//! *not* applied here - they need `ZoneLoader::instantiate_item_template`,
//! which `World` cannot see, so `ugaris-server`'s `tick_item_use_shrike`
//! handles them directly (same precedent as `VaultShelfSearch`,
//! `crates/ugaris-server/src/tick_item_use_keyassembly.rs`).

use super::*;
use crate::item_driver::{drdata_u16, set_drdata_u16, set_drdata_u32, ShrikeAmbientKind};

/// C `shrike.c:283-284`'s valid "cube floor" ground-sprite range (the
/// puzzle room's walkable tiles the cube may slide across).
const SHRIKE_CUBE_FLOOR_SPRITE_MIN: u32 = 59753;
const SHRIKE_CUBE_FLOOR_SPRITE_MAX: u32 = 59761;

/// `drdata` byte offsets `cube_driver` uses (`shrike.c:283-341`): `[4..8]`
/// is the last-touched `ticker` (u32), `[8..10]`/`[10..12]` are the
/// remembered origin `x`/`y` (u16 each, `0` meaning "not yet recorded").
const SHRIKE_CUBE_LAST_TOUCH_OFFSET: usize = 4;
const SHRIKE_CUBE_ORIGIN_X_OFFSET: usize = 8;
const SHRIKE_CUBE_ORIGIN_Y_OFFSET: usize = 10;

impl World {
    /// C `tree_driver`/`rock_driver`/`pede_driver`/`door_driver`'s shared
    /// `!cn` automatic-call branch: swap sprite (and, for tree/rock/
    /// pedestal, description) if it changed, then unconditionally
    /// reschedule.
    pub(crate) fn apply_shrike_ambient_refresh(
        &mut self,
        item_id: ItemId,
        x: u16,
        y: u16,
        kind: ShrikeAmbientKind,
        night: bool,
        schedule_after_ticks: u64,
    ) {
        let (sprite, description): (i32, Option<&'static str>) = match (kind, night) {
            (ShrikeAmbientKind::Tree, true) => (
                51631,
                Some("A silver chain is hanging from one twig of this three."),
            ),
            (ShrikeAmbientKind::Tree, false) => (16006, Some("A dead tree.")),
            (ShrikeAmbientKind::Rock, true) => (
                51632,
                Some("A piece of silver seems to be stuck below this rock."),
            ),
            (ShrikeAmbientKind::Rock, false) => (59763, Some("A big rock.")),
            (ShrikeAmbientKind::Pede, true) => (
                51636,
                Some("A crystal is floating above a pedestal of stone."),
            ),
            (ShrikeAmbientKind::Pede, false) => (51637, Some("A pedestal made of rock.")),
            (ShrikeAmbientKind::Door, true) => (51625, None),
            (ShrikeAmbientKind::Door, false) => (20122, None),
        };

        if let Some(item) = self.items.get_mut(&item_id) {
            if item.sprite != sprite {
                item.sprite = sprite;
                if let Some(desc) = description {
                    item.description = desc.to_string();
                }
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
    }

    /// C `door_driver`'s success branch (`shrike.c:243-247`): `change_
    /// area(cn, 38, 8, 92)`. Ported as a same-area teleport - this door
    /// only exists on area 38's own server (one area-server process per
    /// area, see `AGENTS.md`).
    pub(crate) fn apply_shrike_door_enter(&mut self, character_id: CharacterId) -> bool {
        self.teleport_char_driver(character_id, 8, 92)
    }

    /// C `pool_driver`'s success branch (`shrike.c:276-280`): `it[in2].ID
    /// = IID_SHRIKE_TALISMAN; sprintf(it[in2].description, "The Talisman
    /// of the Moon.");`.
    pub(crate) fn apply_shrike_pool_talisman(&mut self, cursor_item_id: ItemId) -> bool {
        let Some(item) = self.items.get_mut(&cursor_item_id) else {
            return false;
        };
        item.template_id = crate::item_driver::IID_SHRIKE_TALISMAN;
        item.description = "The Talisman of the Moon.".to_string();
        true
    }

    /// C `cube_driver`'s player-push branch (`shrike.c:262-282`): clears
    /// `MF_TMOVEBLOCK`/`it` at the old tile, sets both at the new tile,
    /// and records the touch tick (`drdata[4..8] = ticker`) used by the
    /// auto-reset timer.
    pub(crate) fn apply_shrike_cube_push(
        &mut self,
        item_id: ItemId,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
    ) {
        self.move_shrike_cube(item_id, from_x, from_y, to_x, to_y);
        if let Some(item) = self.items.get_mut(&item_id) {
            let tick = self.tick.0 as u32;
            set_drdata_u32(item, SHRIKE_CUBE_LAST_TOUCH_OFFSET, tick);
        }
    }

    /// C `cube_driver`'s `cn == 0` automatic-call branch (`shrike.c:
    /// 312-343`): remembers the origin tile the first time it ticks, and
    /// slides the cube back home once it has sat idle away from it for
    /// 15 minutes. Always reschedules itself.
    pub(crate) fn apply_shrike_cube_ambient_tick(
        &mut self,
        item_id: ItemId,
        set_origin: Option<(u16, u16)>,
        reset_to: Option<(u16, u16)>,
        schedule_after_ticks: u64,
    ) {
        if let (Some((ox, oy)), Some(item)) = (set_origin, self.items.get_mut(&item_id)) {
            set_drdata_u16(item, SHRIKE_CUBE_ORIGIN_X_OFFSET, ox);
            set_drdata_u16(item, SHRIKE_CUBE_ORIGIN_Y_OFFSET, oy);
        }
        if let Some((tx, ty)) = reset_to {
            if let Some((fx, fy)) = self.items.get(&item_id).map(|item| (item.x, item.y)) {
                self.move_shrike_cube(item_id, fx, fy, tx, ty);
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
    }

    fn move_shrike_cube(
        &mut self,
        item_id: ItemId,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
    ) {
        if let Some(tile) = self.map.tile_mut(usize::from(from_x), usize::from(from_y)) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
            tile.item = 0;
        }
        self.mark_dirty_sector(usize::from(from_x), usize::from(from_y));

        if let Some(tile) = self.map.tile_mut(usize::from(to_x), usize::from(to_y)) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
            tile.item = item_id.0;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.x = to_x;
            item.y = to_y;
        }
        self.mark_dirty_sector(usize::from(to_x), usize::from(to_y));
    }

    /// C `cube_driver`'s player-push validity check (`shrike.c:262-268`):
    /// the single tile the using character is facing must be free of
    /// movement blockers and other items, and its ground sprite must be
    /// in the puzzle room's walkable-floor range.
    pub(crate) fn shrike_cube_push_target(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Option<(u16, u16)> {
        let character = self.characters.get(&character_id)?;
        let item = self.items.get(&item_id)?;
        let direction = Direction::try_from(character.dir).ok()?;
        let (dx, dy) = direction.delta();
        let target_x = i32::from(item.x) + i32::from(dx);
        let target_y = i32::from(item.y) + i32::from(dy);
        if target_x < 0 || target_y < 0 {
            return None;
        }
        let (target_x, target_y) = (target_x as u16, target_y as u16);
        let tile = self
            .map
            .tile(usize::from(target_x), usize::from(target_y))?;
        if tile
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || tile.item != 0
            || tile.ground_sprite < SHRIKE_CUBE_FLOOR_SPRITE_MIN
            || tile.ground_sprite > SHRIKE_CUBE_FLOOR_SPRITE_MAX
        {
            return None;
        }
        Some((target_x, target_y))
    }

    /// C `cube_driver`'s auto-reset target-tile check (`shrike.c:335-
    /// 336`): `!(map[m2].flags & (MF_MOVEBLOCK|MF_TMOVEBLOCK)) &&
    /// !map[m2].it`, evaluated against the cube's *remembered origin*
    /// tile (read straight from `drdata`, since only `World` can see the
    /// map to validate it).
    pub(crate) fn shrike_cube_origin_clear(&self, item_id: ItemId) -> Option<bool> {
        let item = self.items.get(&item_id)?;
        let ox = drdata_u16(item, SHRIKE_CUBE_ORIGIN_X_OFFSET);
        let oy = drdata_u16(item, SHRIKE_CUBE_ORIGIN_Y_OFFSET);
        if ox == 0 && oy == 0 {
            return None;
        }
        let tile = self.map.tile(usize::from(ox), usize::from(oy))?;
        Some(
            !tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                && tile.item == 0,
        )
    }
}
