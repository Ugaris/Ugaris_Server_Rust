use super::*;

pub(crate) const MAX_LQ_DOORS: usize = 256;

pub(crate) const MAX_LQ_NPCS: usize = 512;

pub(crate) const DEV_ID_LQ: u32 = 0x05;

/// C `#define MAXLQMARK 10` (`lq.c:98`): the size of `struct
/// lq_plr_data::mark[]`. Index `0` is never used by any C code path
/// (`hurt_markID`/`kill_markID` are only ever compared with `> 0`).
pub const MAXLQMARK: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LqDoorState {
    pub slot: usize,
    pub item_id: ItemId,
    pub nick: String,
    pub key_id: u32,
}

/// C `struct lq_item` (`src/area/20/lq.c:97-102`): the admin-authored spec
/// for a runtime-created "lq_<base>" quest item (`create_lq_item`), used
/// by both `lq_npc[n].carry_item` (given at spawn) and
/// `lq_npc_data.reward_item` (given on a matching quest turn-in).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LqItemSpec {
    pub base: String,
    pub name: String,
    pub description: String,
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
    /// C `lq_npc[n].sprite` (`0` means "keep the `lq_<basename>` template's
    /// own sprite", matching `spawn_npc`'s `if (lq_npc[n].sprite) ch[cn]
    /// .sprite = ...`).
    pub sprite: i32,
    /// C `lq_npc[n].greeting` - said once per newly-sighted player
    /// (`lqnpc`'s `NT_CHAR` handler).
    pub greeting: String,
    /// C `lq_npc[n].trigger[5]`/`reply[5]` - substring-matched dialogue
    /// pairs (`lqnpc`'s `NT_TEXT` handler).
    pub trigger: [String; 5],
    pub reply: [String; 5],
    /// C `lq_npc[n].want_keyID` - the `MAKE_ITEMID(DEV_ID_LQ, ..)` key a
    /// player must `NT_GIVE` this NPC to trigger `reward_item`.
    pub want_key_id: u32,
    /// C `lq_npc[n].reward_item` - handed to the giver on a matching
    /// turn-in.
    pub reward_item: LqItemSpec,
    /// C `lq_npc[n].reward_markID` - unused by `lqnpc`/`lqnpc_died`
    /// themselves (only the unported `questreward`/admin quest-lifecycle
    /// commands read it), kept for round-trip fidelity with the C struct.
    pub reward_mark_id: u32,
    /// C `lq_npc[n].kill_markID` - set on the killer's `DRD_LQ_PLR_DATA`
    /// mark array by `lqnpc_died`.
    pub kill_mark_id: u32,
    /// C `lq_npc[n].hurt_markID` - set on the attacker's mark array by
    /// `lqnpc`'s `NT_GOTHIT` handler (and again by `lqnpc_died` on the
    /// final blow).
    pub hurt_mark_id: u32,
    /// C `lq_npc[n].carry_item` - given to the NPC once at spawn time.
    pub carry_item: LqItemSpec,
    /// C `lq_npc[n].carry_gold` - `ch[cn].gold` at spawn time.
    pub carry_gold: u32,
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
    pub sprite: i32,
    pub greeting: String,
    pub trigger: [String; 5],
    pub reply: [String; 5],
    pub want_key_id: u32,
    pub reward_item: LqItemSpec,
    pub reward_mark_id: u32,
    pub kill_mark_id: u32,
    pub hurt_mark_id: u32,
    pub carry_item: LqItemSpec,
    pub carry_gold: u32,
    /// C `spawn_npc`'s `isthrall` parameter (`lq.c:1724`): `true` for the
    /// `#thrall` admin command's on-the-fly, template-detached spawns.
    /// When set, the spawned character's `DRD_LQ_NPC_DATA.n` stays `0`
    /// (not `slot`), `greeting`/`trigger`/`reply` are NOT copied (C's own
    /// `if (!isthrall) { ... }` guard, `lq.c:1819-1825`), and no
    /// `lq_npc[n].cn`/`cserial` bookkeeping happens (`World::
    /// apply_lq_npc_spawn_result` is skipped).
    pub is_thrall: bool,
    /// C `spawn_npc`'s `thrallname` parameter, copied into
    /// `dat->thrallname` only when `is_thrall` is set (`lq.c:1818`).
    pub thrall_name: String,
}

pub(crate) fn write_lq_door_key_id(item: &mut Item, key_id: u32) {
    if item.driver_data.len() < 5 {
        item.driver_data.resize(5, 0);
    }
    let legacy_item_id = make_lq_item_template_id(key_id);
    item.driver_data[1..5].copy_from_slice(&legacy_item_id.to_le_bytes());
}

/// C `MAKE_ITEMID(DEV_ID_LQ, nr)` (`src/common/item_id.h:58`): the runtime
/// item-identity value `create_lq_item` writes into `it[in].ID` - modeled
/// here as [`crate::entity::Item::template_id`] (see [`LqItemSpec`]'s own
/// doc comment and `ugaris-server`'s `area20::create_lq_item`, the only
/// place that actually instantiates one of these items).
pub fn make_lq_item_template_id(key_id: u32) -> u32 {
    (DEV_ID_LQ << 24) | (key_id & 0x00ff_ffff)
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
        npc.greeting.truncate(255);
        for trigger in npc.trigger.iter_mut() {
            trigger.truncate(39);
        }
        for reply in npc.reply.iter_mut() {
            reply.truncate(255);
        }
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
            self.pending_lq_npc_spawns
                .push(build_lq_npc_spawn_request(npc));
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

/// Builds the `ZoneLoader`-needing spawn payload for `npc` (C `spawn_npc`'s
/// `lq_npc[n]` field copy, `lq.c:1748-1834`, minus the parts only
/// `spawn_npc` itself performs). Shared by the scheduled-respawn path
/// (`queue_due_lq_npc_respawns`) and the immediate `#nspawn` admin command
/// (`world::lq_admin`).
pub(crate) fn build_lq_npc_spawn_request(npc: &LqNpcState) -> LqNpcSpawnRequest {
    LqNpcSpawnRequest {
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
        sprite: npc.sprite,
        greeting: npc.greeting.clone(),
        trigger: npc.trigger.clone(),
        reply: npc.reply.clone(),
        want_key_id: npc.want_key_id,
        reward_item: npc.reward_item.clone(),
        reward_mark_id: npc.reward_mark_id,
        kill_mark_id: npc.kill_mark_id,
        hurt_mark_id: npc.hurt_mark_id,
        carry_item: npc.carry_item.clone(),
        carry_gold: npc.carry_gold,
        is_thrall: false,
        thrall_name: String::new(),
    }
}
