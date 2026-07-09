//! `create_lab_exit` (`src/system/lab.c:137-158`): the reward-drop half of
//! the "kill a lab master, get a `labexit` gate, use it to solve the
//! level and warp to Aston" loop shared verbatim by all five
//! `src/area/22/lab*.c` files (`lab1.c:403`, `lab2.c:277`, `lab3.c:961`,
//! `lab4.c:274`, `lab5.c:451`, all `create_lab_exit(co, <level>)` calls
//! from the area's own master-kill death hook). `create_lab_exit` itself
//! needs `create_item`/`drop_item_extended` against a real
//! `ZoneLoader`-backed template, which `World` alone can't do (same
//! architectural gap as every other "spawn a fresh item" reward - see
//! `world::xmas`'s `grant_xmas_tree_gift` for the established
//! `ugaris-server`-side pattern this port reuses), so this module is only
//! the `World`-side queue: [`World::queue_lab_exit_spawn`] (called from
//! each area's own master-kill death hook, C's `if (co && (ch[co].flags &
//! CF_PLAYER))` guard) and [`World::drain_pending_lab_exit_spawns`]
//! (drained by `ugaris-server`'s `lab::create_lab_exit`, C's own function
//! of the same name, from `tick_sync::sync_phase`).
use super::*;

/// C `create_lab_exit(cn, level)`'s deferred request: `killer_id` is C's
/// `cn` parameter (the player who gets the reward gate dropped at their
/// feet), `level` is the lab-level tag the gate itself carries
/// (`it[in].drdata[4]`), read back by [`ItemDriverOutcome::LabExitUse`]
/// on use to call `set_solved_lab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabExitSpawnRequest {
    pub killer_id: CharacterId,
    pub level: u8,
}

impl World {
    /// Queues a `create_lab_exit(co, level)` reward drop - see the module
    /// doc comment for why the actual item creation is deferred to
    /// `ugaris-server`.
    pub fn queue_lab_exit_spawn(&mut self, killer_id: CharacterId, level: u8) {
        self.pending_lab_exit_spawns
            .push(LabExitSpawnRequest { killer_id, level });
    }

    /// Drains every `create_lab_exit` request queued this tick.
    pub fn drain_pending_lab_exit_spawns(&mut self) -> Vec<LabExitSpawnRequest> {
        self.pending_lab_exit_spawns.drain(..).collect()
    }

    /// C `lab3_special`'s `drdata[0]==1` teleport-door branch
    /// (`src/area/22/lab3.c:909-965`): resolves the raw
    /// `ItemDriverOutcome::Lab3TeleportDoor` outcome by actually moving the
    /// character via `teleport_char_driver`, then (if the destination tile
    /// is underwater) extinguishing carried torches and playing the
    /// bubble/"Hrgblub." flavor, then (if the door was password-protected)
    /// queuing the `create_lab_exit` reward. Called from `World::
    /// apply_item_driver_outcome` since none of this needs `ZoneLoader`/
    /// `PlayerRuntime`.
    pub(crate) fn apply_lab3_teleport_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        dx: i8,
        dy: i8,
        password_protected: bool,
    ) -> ItemDriverOutcome {
        let Some(character) = self.characters.get(&character_id) else {
            return ItemDriverOutcome::Noop;
        };
        // C `ch[cn].x + (signed char)drdata[1]` / `... + drdata[2]`
        // (`lab3.c:917`): plain signed addition onto trusted level data.
        let target_x = (i32::from(character.x) + i32::from(dx)).max(0) as u16;
        let target_y = (i32::from(character.y) + i32::from(dy)).max(0) as u16;

        if !self.teleport_char_driver(character_id, target_x, target_y) {
            return ItemDriverOutcome::Lab3TeleportDoorBusy { character_id };
        }

        let extinguished_count = self.lab3_teleport_door_water_tail(character_id);

        if password_protected {
            self.queue_lab_exit_spawn(character_id, 25);
        }

        ItemDriverOutcome::Lab3TeleportDoor {
            item_id,
            character_id,
            dx,
            dy,
            password_protected,
            extinguished_count,
        }
    }

    /// C `lab3_special:922-957`: the underwater-arrival tail (extinguish
    /// carried torches, bubble effects, "Hrgblub." talk, splash sounds).
    /// Returns the number of torches extinguished, for the caller's
    /// pluralized feedback text.
    fn lab3_teleport_door_water_tail(&mut self, character_id: CharacterId) -> u8 {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return 0;
        };
        let underwater = self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::UNDERWATER));
        if !underwater {
            return 0;
        }

        // C `WN_LHAND` slot index, same precedent as `world::robber`'s own
        // `ROBBER_TORCH_SLOT`.
        const WN_LHAND: usize = 8;
        let mut torch_item_ids: Vec<ItemId> = character.inventory[INVENTORY_START_INVENTORY..]
            .iter()
            .flatten()
            .copied()
            .collect();
        if let Some(cursor_item_id) = character.cursor_item {
            torch_item_ids.push(cursor_item_id);
        }
        if let Some(Some(lhand_item_id)) = character.inventory.get(WN_LHAND) {
            torch_item_ids.push(*lhand_item_id);
        }

        let mut extinguished_count = 0u8;
        for item_id in torch_item_ids {
            if let Some(item) = self.items.get_mut(&item_id) {
                if item.driver == IDR_TORCH && item.driver_data.first().copied().unwrap_or(0) != 0 {
                    extinguish_torch(item);
                    extinguished_count = extinguished_count.saturating_add(1);
                }
            }
        }

        // C `lab3.c:945-956`: bubbles + talk only fire when the character
        // doesn't already carry `CF_OXYGEN` (a Yellow Berry effect).
        if !character.flags.contains(CharacterFlags::OXYGEN) {
            let x = i32::from(character.x);
            let y = i32::from(character.y);
            let base_tick = self.tick.0 as i32;
            for offset in 0..5 {
                self.create_map_effect(
                    EF_BUBBLE,
                    x,
                    y,
                    base_tick + offset,
                    base_tick + offset + 1,
                    0,
                    45,
                );
            }
            for _ in 0..3 {
                let sound_type =
                    44 + legacy_random_variant_below_from_seed(&mut self.legacy_random_seed, 3);
                self.queue_sound_area(x as usize, y as usize, sound_type);
            }
            self.npc_say(character_id, "Hrgblub.");
        }

        extinguished_count
    }
}
