use super::*;

pub(crate) const MAX_LQ_DOORS: usize = 256;

pub(crate) const MAX_LQ_NPCS: usize = 512;

pub(crate) const DEV_ID_LQ: u32 = 0x05;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LqDoorState {
    pub slot: usize,
    pub item_id: ItemId,
    pub nick: String,
    pub key_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LqNpcState {
    pub slot: usize,
    pub basename: String,
    pub x: u16,
    pub y: u16,
    pub dir: u8,
    pub level: u16,
    pub mode: u8,
    pub respawn_seconds: u32,
    pub name: String,
    pub description: String,
    pub nick: [String; 2],
    pub character_id: Option<CharacterId>,
    pub character_serial: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LqNpcSpawnRequest {
    pub slot: usize,
    pub basename: String,
    pub x: u16,
    pub y: u16,
    pub dir: u8,
    pub level: u16,
    pub mode: u8,
    pub name: String,
    pub description: String,
    pub nick: [String; 2],
}

pub(crate) fn write_lq_door_key_id(item: &mut Item, key_id: u32) {
    if item.driver_data.len() < 5 {
        item.driver_data.resize(5, 0);
    }
    let legacy_item_id = (DEV_ID_LQ << 24) | (key_id & 0x00ff_ffff);
    item.driver_data[1..5].copy_from_slice(&legacy_item_id.to_le_bytes());
}

impl World {
    pub fn configure_lq_npc(&mut self, mut npc: LqNpcState) -> bool {
        if npc.slot == 0 || npc.slot >= MAX_LQ_NPCS || npc.basename.is_empty() {
            return false;
        }

        npc.basename.truncate(39);
        npc.name.truncate(39);
        npc.description.truncate(159);
        npc.nick[0].truncate(39);
        npc.nick[1].truncate(39);
        if let Some(existing) = self
            .lq_npcs
            .iter_mut()
            .find(|existing| existing.slot == npc.slot)
        {
            *existing = npc;
        } else {
            self.lq_npcs.push(npc);
            self.lq_npcs.sort_by_key(|npc| npc.slot);
        }
        true
    }

    pub fn schedule_lq_npc_respawn(&mut self, slot: usize, due_tick: u64) -> bool {
        if slot == 0 || slot >= MAX_LQ_NPCS || !self.lq_npcs.iter().any(|npc| npc.slot == slot) {
            return false;
        }
        if let Some((_, existing_due_tick)) = self
            .lq_npc_respawns
            .iter_mut()
            .find(|(existing_slot, _)| *existing_slot == slot)
        {
            *existing_due_tick = due_tick;
        } else {
            self.lq_npc_respawns.push((slot, due_tick));
            self.lq_npc_respawns.sort_by_key(|(slot, _)| *slot);
        }
        true
    }

    pub fn drain_pending_lq_npc_spawns(&mut self) -> Vec<LqNpcSpawnRequest> {
        std::mem::take(&mut self.pending_lq_npc_spawns)
    }

    pub fn apply_lq_npc_spawn_result(
        &mut self,
        slot: usize,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == slot) else {
            return false;
        };
        npc.character_id = Some(character_id);
        npc.character_serial = serial;
        true
    }

    pub(crate) fn discover_lq_doors_once(&mut self) {
        if self.lq_doors_initialized {
            return;
        }
        self.lq_doors_initialized = true;

        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter_map(|(item_id, item)| {
                (item.driver == IDR_DOOR
                    && item.driver_data.get(10).copied().unwrap_or_default() != 0)
                    .then_some(*item_id)
            })
            .collect();
        item_ids.sort_by_key(|item_id| item_id.0);

        for (offset, item_id) in item_ids.into_iter().take(MAX_LQ_DOORS - 1).enumerate() {
            let slot = offset + 1;
            let Some(item) = self.items.get_mut(&item_id) else {
                continue;
            };
            let nick = item.name.chars().take(39).collect::<String>();
            write_lq_door_key_id(item, 0);
            self.lq_doors.push(LqDoorState {
                slot,
                item_id,
                nick,
                key_id: 0,
            });
        }
    }

    pub(crate) fn queue_due_lq_npc_respawns(&mut self) {
        let now = self.tick.0;
        let mut due_slots: Vec<usize> = self
            .lq_npc_respawns
            .iter()
            .filter_map(|(slot, due_tick)| (*due_tick != 0 && *due_tick <= now).then_some(*slot))
            .collect();
        if due_slots.is_empty() {
            return;
        }
        due_slots.sort_unstable();

        for slot in due_slots {
            let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
                continue;
            };
            self.pending_lq_npc_spawns.push(LqNpcSpawnRequest {
                slot: npc.slot,
                basename: npc.basename.clone(),
                x: npc.x,
                y: npc.y,
                dir: npc.dir,
                level: npc.level,
                mode: npc.mode,
                name: npc.name.clone(),
                description: npc.description.clone(),
                nick: npc.nick.clone(),
            });
            if let Some((_, due_tick)) = self
                .lq_npc_respawns
                .iter_mut()
                .find(|(existing_slot, _)| *existing_slot == slot)
            {
                *due_tick = 0;
            }
        }
        self.lq_npc_respawns.retain(|(_, due_tick)| *due_tick != 0);
    }
}
