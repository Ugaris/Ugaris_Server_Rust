use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorKeyAccess {
    pub key_id: u32,
    pub name: String,
    pub source: DoorKeySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorKeySource {
    Carried,
    Keyring,
}

/// Whether/why [`ItemDriverOutcome::StrStorageInteract`]'s carried-item
/// conversion attempt succeeded (C `storage`, `strategy.c:1161-1187`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrStorageConversion {
    /// No item was carried on the cursor, or it wasn't an `IDR_ENHANCE`
    /// mined gold/silver stack (C's outer `if` was false): no conversion
    /// attempted, no mutation, but the "This storage contains..." info
    /// message still always prints.
    None,
    /// A carried `IDR_ENHANCE` stack converted to `added` units of
    /// Platinum (C's `am` nonzero, silver at 50:1/`drdata[0]==1` or gold
    /// at 5:1/`drdata[0]==2`): `World::apply_item_driver_outcome`
    /// destroys `cursor_item_id` and credits `added` to the storage.
    Converted { cursor_item_id: ItemId, added: u32 },
    /// A carried `IDR_ENHANCE` stack existed but wasn't silver/gold, or
    /// its converted amount rounded down to zero (C's `am == 0`): prints
    /// "You can only add mined gold or silver. The exchange rate is 5 to
    /// 1 for gold and 50 to 1 for silver.", no mutation.
    WrongKind,
}

/// C `shrike_driver`'s `drdata[0]` sub-driver selector, restricted here
/// to the four "ambient day/night sprite" sub-drivers (`shrike.c:356-
/// 377`): `1`=tree, `2`=rock, `6`=pedestal (all three also swap
/// `description`), `3`=door (sprite only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShrikeAmbientKind {
    Tree,
    Rock,
    Pede,
    Door,
}

/// Which fresh amulet component template `tree_driver`/`rock_driver`/
/// `pede_driver` creates (`shrike.c:113-123`/`:200-212`/`:156-166`; item
/// keys/bits per `ugaris_data/zones/38/shrike.itm`: `shrike_amulet1` =
/// crystal (bit 1), `shrike_amulet2` = silver chain (bit 2),
/// `shrike_amulet3` = crescent charm (bit 4)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShrikeAmuletPiece {
    /// `create_item("shrike_amulet1")` (pedestal, `pede_driver`).
    Crystal,
    /// `create_item("shrike_amulet2")` (tree, `tree_driver`).
    Chain,
    /// `create_item("shrike_amulet3")` (rock, `rock_driver`).
    Charm,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDriverContext {
    pub door_key: Option<DoorKeyAccess>,
    pub cursor_template_id: Option<u32>,
    pub cursor_driver: Option<u16>,
    pub cursor_sprite: Option<i32>,
    pub cursor_drdata0: Option<u8>,
    pub cursor_drdata1_u32: Option<u32>,
    pub timer_call: bool,
    pub daylight: u8,
    pub hour: u8,
    pub fullmoon: bool,
    pub newmoon: bool,
    pub solstice: bool,
    pub equinox: bool,
    pub character_underwater: bool,
    pub current_tick: u32,
    pub edemon_fire_enabled: Option<bool>,
    pub edemon_section_power: Option<u8>,
    pub edemon_tube_target: Option<(u16, u16)>,
    pub edemon_gate_spawn: Option<EdemonGateSpawnContext>,
    pub fdemon_gate_spawn: Option<FdemonGateSpawnContext>,
    pub fdemon_loader_power: Option<u16>,
    pub bone_hint_nr: Option<u8>,
    pub bone_hint_pos: Option<u8>,
    pub has_area17_library_key: bool,
    pub has_area17_lockpick: bool,
    pub has_area17_cursor_lockpick: bool,
    pub area25_door_key: Option<(ItemId, String)>,
    pub warp_trial_door: Option<WarpTrialDoorContext>,
    pub warp_bonus_base: Option<u32>,
    pub warp_bonus_points: u32,
    pub warp_bonus_used_at_base: Option<u32>,
    pub has_dungeon_door_key1: bool,
    pub has_dungeon_door_key2: bool,
    pub dungeon_defender_count: Option<u16>,
    pub lab_solved_bits: u64,
    /// C `deathfibrin_scan(cn)` (`src/area/22/lab1.c:440-458`): the nearby
    /// `CDR_LABGNOMEDRIVER` "Immortal Master" the staff can strike, if any.
    pub deathfibrin_master: Option<CharacterId>,
    /// C `map[ch[cn].x+ch[cn].y*MAXMAP].light` (`lab1.c:523-524`), the
    /// debug light value C's own "no immortal close enough" message
    /// prints verbatim.
    pub deathfibrin_tile_light: u8,
    pub pent_last_solve_tick: Option<u32>,
    pub pent_demon_lord_access_seconds: Option<u32>,
    pub has_matching_random_shrine_key: bool,
    pub random_shrine_already_used: bool,
    pub clanspawn_max_jewel_count: Option<u8>,
    pub clanspawn_contested: bool,
    pub clanspawn_random_seconds: Option<u32>,
    pub has_curse_spell: bool,
    pub has_area11_palace_key: bool,
    pub islena_room_has_player: bool,
    pub islena_present: bool,
    pub islena_resting: bool,
    pub has_area16_robber_key: bool,
    pub has_area16_skelly_key: bool,
    pub has_mine_gateway_key: bool,
    pub mine_door_target: Option<(u16, u16, u8)>,
    pub swamp_arm_triggered: Option<bool>,
    pub swamp_whisp_move_succeeds: Option<bool>,
    pub swamp_whisp_turn_x: bool,
    pub swamp_whisp_turn_y: bool,
    pub swamp_spawn_live: Option<bool>,
    pub swamp_spawn_player_close: Option<bool>,
    pub swamp_spawn_ground_sprite: Option<u32>,
    pub lq_open: bool,
    pub lq_min_level: u16,
    pub lq_max_level: u16,
    pub lq_entrance: Option<(u16, u16)>,
    pub lq_death_penalty_seconds: Option<u32>,
    pub teufel_arena_roll: Option<u8>,
    pub teufel_ratnest_guard_active: bool,
    /// C `ppd->guard_talkstep` (`src/area/22/lab3.c:911`, `set_data(cn,
    /// DRD_LAB_PPD, ...)`): the *using character's own* password-guard
    /// challenge stage, read by `lab3_special`'s teleport-door password
    /// check (`drdata[3] && ppd->guard_talkstep < 20`). `None` (treated as
    /// `0`, matching a freshly-allocated `struct lab_ppd`) when the item
    /// isn't `IDR_LAB3_SPECIAL`.
    pub lab3_guard_talkstep: Option<u8>,
    /// C `has_potion(cn)` (`src/area/22/lab5.c:245-259`): whether the
    /// using character carries an `IDR_POTION` item in inventory slots
    /// `30..` or on the cursor. Only meaningful for `IDR_LAB5_ITEM`'s
    /// `drdata[0]==11` "no potion door" branch.
    pub has_potion: bool,
    /// C `check_chestbox(cn, in)` (`lab5.c:1000-1023`): whether *this*
    /// chestbox item has already been opened by the using character. See
    /// `PlayerRuntime::lab5_chestbox_opened`'s own doc comment for the
    /// `ItemId`-keyed deviation from C's sequential bitset.
    pub lab5_chestbox_already_opened: bool,
    /// `struct lab5_player_data.ritualdaemon` (`lab5.c:88`,
    /// `PlayerRuntime::lab5_ritual_daemon`), read before the
    /// `IDR_LAB5_ITEM` nameplate/realnameplate/entrance branches run -
    /// `None` when the item isn't `IDR_LAB5_ITEM` (treated as `0`,
    /// matching a freshly-allocated `struct lab5_player_data`).
    pub lab5_ritual_daemon: Option<u8>,
    /// `struct lab5_player_data.ritualstate` (`lab5.c:89`,
    /// `PlayerRuntime::lab5_ritual_state`), same precedent as
    /// `lab5_ritual_daemon`.
    pub lab5_ritual_state: Option<u8>,
    /// C `staffer_ppd.rouven_state` (`src/common/staffer_ppd.h:44`), read
    /// before `vault_skull` runs its `0..=5` range check
    /// (`staffer.c:339`). `None` when the item isn't `IDR_STAFFER`
    /// `drdata[0]==4`.
    pub rouven_state: Option<i32>,
    /// C `check_area_clear(in)` (`src/area/33/tunnel.c:750-762`), read
    /// before `mean_door`'s `cn == 0` automatic-call branch decides
    /// whether to `open_door`. Only computed for `IDR_TUNNELDOOR2` timer
    /// calls (`None` is treated as "not clear", never opening the door by
    /// mistake) - see `World::tunnel_mean_door_area_clear`.
    pub tunnel_door_area_clear: Option<bool>,
    /// C `is_fullnight()` (`src/area/38/shrike.c:79-81`, `moonlight &&
    /// sunlight < 100`): gates every `IDR_SHRIKE` sub-driver's player-
    /// interaction branch (tree/rock/pedestal amulet pickup, pool
    /// talisman activation) and the tree/rock/pedestal/door ambient
    /// sprite swap. Populated from `World.date.{moonlight,sunlight}` at
    /// every `IDR_SHRIKE` call (player-driven and timer-driven alike),
    /// unlike the `fullmoon`/`newmoon` fields above which are only filled
    /// in for a handful of specific drivers.
    pub is_fullnight: bool,
    /// C `cube_driver`'s player-push branch (`shrike.c:283-309`): the
    /// single tile in front of the using character, if it is currently a
    /// legal destination for the puzzle cube (`!(MF_MOVEBLOCK |
    /// MF_TMOVEBLOCK)`, no `map[m2].it`, and `map[m2].gsprite` in
    /// `59753..=59761`, the walkable shrine-floor sprite range). `None`
    /// when blocked (or the item isn't `IDR_SHRIKE` `drdata[0]==5`) -
    /// see `World::shrike_cube_push_target`.
    pub shrike_cube_push_target: Option<(u16, u16)>,
    /// C `cube_driver`'s `cn == 0` automatic-call branch (`shrike.c:
    /// 322-341`): whether the cube's *remembered origin tile* (`drdata[8..
    /// 12]`) is currently free of movement blockers/other items, i.e.
    /// C's `!(map[m2].flags & (MF_MOVEBLOCK|MF_TMOVEBLOCK)) &&
    /// !map[m2].it`. Only computed once the driver function has already
    /// determined the 15-minute idle-and-moved condition holds (`None`
    /// otherwise) - see `World::shrike_cube_origin_clear`.
    pub shrike_cube_origin_clear: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseItemError {
    IllegalCharacter,
    IllegalItem,
    Dead,
    AccessDenied,
    AccountDepotUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemDriverRequest {
    Driver {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
        spec: i32,
    },
    AccountDepot {
        item_id: ItemId,
        character_id: CharacterId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseItemOutcome {
    OpenContainer { item_id: ItemId },
    OpenDepot { item_id: ItemId },
    OpenAccountDepot { item_id: ItemId },
    Dispatch(ItemDriverRequest),
}
