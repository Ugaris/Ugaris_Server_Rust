use super::*;

pub const OUTCOME_ITEM_NAME_BYTES: usize = 32;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemDriverOutcome {
    LookItem {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PotionDrunk {
        item_id: ItemId,
        character_id: CharacterId,
        hp_added: i32,
        mana_added: i32,
        endurance_added: i32,
    },
    FoodEaten {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    LollipopLicked {
        item_id: ItemId,
        character_id: CharacterId,
        exp_added: u32,
        lick_count: u8,
    },
    LollipopMemories {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ChristmasPopInspected {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Teleport {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
        stop_driver: bool,
        quiet: bool,
    },
    WarpTeleportSpheres {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        x: u16,
        y: u16,
    },
    WarpTeleportMissingSphere {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpTeleportBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpTeleportBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpKeySpawn {
        item_id: ItemId,
        character_id: CharacterId,
        sphere_kind: u8,
    },
    WarpKeySpawnCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpKeyDoor {
        item_id: ItemId,
        character_id: CharacterId,
        key_item_id: ItemId,
        key_name: [u8; OUTCOME_ITEM_NAME_BYTES],
        x: u16,
        y: u16,
    },
    WarpKeyDoorMissingKey {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpKeyDoorBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpTrialDoor {
        item_id: ItemId,
        character_id: CharacterId,
        spawn_x: u16,
        spawn_y: u16,
        player_x: u16,
        player_y: u16,
        fighter_target_x: u16,
        fighter_target_y: u16,
        xs: u16,
        ys: u16,
        xe: u16,
        ye: u16,
        template: &'static str,
    },
    WarpTrialDoorWrongSide {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpTrialDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpTrialDoorBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpBonus {
        item_id: ItemId,
        character_id: CharacterId,
        location_id: u32,
        base: u32,
        next_points: u32,
        advanced: bool,
        reward_sphere_kind: Option<u8>,
        reward_level: u32,
    },
    WarpBonusFinished {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpBonusAlreadyUsed {
        item_id: ItemId,
        character_id: CharacterId,
    },
    WarpBonusNeedsSphere {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeleportDoor {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    Recall {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    CityRecall {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    TeufelArenaExit {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    TeufelArenaExitLowHealth {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelArena {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    TeufelArenaNeedsSuit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelArenaLevelTooHigh {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelArenaEquipmentEnhanced {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelArenaEquipmentBound {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelArenaBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelDoor {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    TeufelDoorNoHumans {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelDoorNoBeggars {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelDoorOnlyNobles {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelDoorBug {
        item_id: ItemId,
        character_id: CharacterId,
        x: i32,
        y: i32,
    },
    TeufelRatNestSpawn {
        item_id: ItemId,
        nest_kind: u8,
        wave: u16,
        level: u16,
        template: &'static str,
        schedule_after_ticks: u64,
    },
    TeufelRatNestDestroyed {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TeufelRatNestGuarded {
        item_id: ItemId,
        character_id: CharacterId,
    },
    DungeonTeleport {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        clan_number: u16,
    },
    ClanSpawnExit {
        item_id: ItemId,
        character_id: CharacterId,
        area_id: u16,
        x: u16,
        y: u16,
    },
    ClanSpawnExitBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    DungeonFake {
        item_id: ItemId,
        character_id: CharacterId,
        clan_number: u16,
    },
    DungeonKey {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
        key_id: u32,
        clan_number: u8,
        first_taken: bool,
    },
    DungeonKeyCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    DungeonDoorMissingKeys {
        item_id: ItemId,
        character_id: CharacterId,
        missing: u8,
        both_required: bool,
    },
    DungeonDoorTooManyDefenders {
        item_id: ItemId,
        character_id: CharacterId,
        alive: u16,
        max_allowed: u16,
    },
    DungeonDoorSolved {
        item_id: ItemId,
        character_id: CharacterId,
        clan_number: u32,
        catacomb: u8,
        first_solve: bool,
    },
    DoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
    },
    KeyedDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        key_id: u32,
        source: DoorKeySource,
        locking: bool,
    },
    DoubleDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        picked_lock: bool,
    },
    PickDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineDoorTeleport {
        item_id: ItemId,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        fallback_x: u16,
        fallback_y: u16,
    },
    MineDoorMissingTarget {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineDoorTimer {
        item_id: ItemId,
    },
    MineKeyDoor {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        golem_nr: u8,
    },
    MineKeyDoorNeedsGold {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineKeyDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    /// Synthesized only by `World::apply_item_driver_outcome` (never by
    /// the pure `mine_key_door_driver`): the door opened successfully and
    /// the player was teleported into `(room_x, room_y)` - carries the
    /// room coordinates so `ugaris-server` can spawn `CDR_GOLEMKEYHOLDER`
    /// at the matching golem position (`room_x + 4, room_y`, C
    /// `keyholder_door`'s `2 + (n%3)*8 + 5, 231 + (n/3)*8 + 3` vs. the
    /// player's `2 + (n%3)*8 + 1, 231 + (n/3)*8 + 3`, `mine.c:1187,1204-
    /// 1207`).
    MineKeyDoorOpened {
        item_id: ItemId,
        character_id: CharacterId,
        golem_nr: u8,
        room_x: u16,
        room_y: u16,
    },
    StafferSpecDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    EdemonDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        key_name: Option<[u8; OUTCOME_ITEM_NAME_BYTES]>,
        locking: bool,
    },
    EdemonDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EdemonDoorLifeless {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EdemonBlockMove {
        item_id: ItemId,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        schedule_after_ticks: Option<u64>,
    },
    EdemonBlockBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EdemonTubePulse {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        schedule_after_ticks: u64,
    },
    EdemonGateSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
        slot: usize,
        x: u16,
        y: u16,
        schedule_after_ticks: u64,
    },
    ChestSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
        x: u16,
        y: u16,
        schedule_after_ticks: u64,
    },
    ChestSpawnCheck {
        item_id: ItemId,
        character_id: CharacterId,
        spawned_character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    SwampSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
        x: u16,
        y: u16,
        schedule_after_ticks: u64,
    },
    SwampSpawnPulse {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    FdemonGateSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
        slot: usize,
        x: u16,
        y: u16,
        schedule_after_ticks: u64,
    },
    FdemonCannonPulse {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    FdemonCannonLifeless {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FreakDoorUse {
        item_id: ItemId,
        character_id: CharacterId,
        link_group: u8,
        one_way: bool,
        recursion_guard: bool,
        cached_partner_id: Option<ItemId>,
        no_target: bool,
    },
    StafferSpecDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BallTrapProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    },
    FireballMachineProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
        schedule_after_ticks: Option<u64>,
    },
    EdemonBallProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        strength: i32,
        base_sprite: i32,
        schedule_after_ticks: u64,
    },
    EdemonBallInactive {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    CaligarGunProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        direction: u8,
        schedule_after_ticks: u64,
    },
    FlameThrowerPulse {
        item_id: ItemId,
        character_id: CharacterId,
        direction: u8,
        schedule_after_ticks: u64,
    },
    FlameThrowerExtinguished {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: Option<u64>,
    },
    SpikeTrapTriggered {
        item_id: ItemId,
        character_id: CharacterId,
        damage: i32,
        reset_after_ticks: u64,
    },
    SpikeTrapReset {
        item_id: ItemId,
    },
    Extinguish {
        item_id: ItemId,
        character_id: CharacterId,
        extinguished: bool,
    },
    TriggerMapItem {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        target_character_id: CharacterId,
        delay_ticks: u64,
    },
    StepTrapDiscoverTarget {
        item_id: ItemId,
    },
    TrapdoorOpen {
        item_id: ItemId,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        schedule_after_ticks: u64,
    },
    TrapdoorBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    TrapdoorClose {
        item_id: ItemId,
    },
    TrapdoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TrapdoorNeedsStick {
        item_id: ItemId,
        character_id: CharacterId,
    },
    JunkpileSearch {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
    },
    JunkpileCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    GasTrapPulse {
        item_id: ItemId,
        character_id: CharacterId,
        power: u8,
        schedule_initial_trigger: bool,
        schedule_animation: bool,
    },
    ChestTreasure {
        item_id: ItemId,
        character_id: CharacterId,
        treasure_index: u8,
    },
    RandomChest {
        item_id: ItemId,
        character_id: CharacterId,
    },
    RatChest {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChest {
        item_id: ItemId,
        character_id: CharacterId,
        template: InfiniteChestTemplate,
        key_name: Option<[u8; OUTCOME_ITEM_NAME_BYTES]>,
    },
    InfiniteChestCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChestKeyRequired {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChestUnknown {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestSpadeFind {
        item_id: ItemId,
        character_id: CharacterId,
        find: ForestSpadeFind,
    },
    ForestSpadeCollapse {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    ForestSpadeNothing {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestSpadeCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestChest {
        item_id: ItemId,
        character_id: CharacterId,
        amount: u32,
        imp_flag_mask: u32,
    },
    ForestChestCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestChestLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChest {
        item_id: ItemId,
        character_id: CharacterId,
        template: PickChestTemplate,
    },
    PickChestCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChestLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChestBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentBossDoor {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    PentBossDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentBossDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ColorTile {
        item_id: ItemId,
        character_id: CharacterId,
        row: u8,
        color: u8,
    },
    BurndownTouch {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownTooHot {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownAlreadyBurned {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownIgnite {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownTimerTick {
        item_id: ItemId,
    },
    KeyringShow {
        item_id: ItemId,
        character_id: CharacterId,
    },
    KeyringAddCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        key_item_id: ItemId,
    },
    LightChanged {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: Option<u64>,
    },
    FdemonLoaderChanged {
        item_id: ItemId,
        character_id: CharacterId,
        consumed_cursor_item_id: Option<ItemId>,
        /// C `it[in].drdata[6]` (the loader's fixed "defense station
        /// number" tag; `0` for loaders that aren't a boss-mission gate).
        /// Only meaningful when `consumed_cursor_item_id.is_some()`.
        station_id: u8,
        ground_overlay_sprite: u32,
        sound_type: Option<u32>,
        schedule_after_ticks: Option<u64>,
    },
    FdemonLoaderBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: FdemonLoaderBlockReason,
    },
    EdemonLoaderChanged {
        item_id: ItemId,
        character_id: CharacterId,
        consumed_cursor_item_id: Option<ItemId>,
        ground_overlay_sprite: u32,
        sound_type: Option<u32>,
        schedule_after_ticks: Option<u64>,
    },
    EdemonLoaderBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: EdemonLoaderBlockReason,
    },
    FdemonFarmChanged {
        item_id: ItemId,
        character_id: CharacterId,
        foreground_sprite: u32,
        schedule_after_ticks: Option<u64>,
    },
    FdemonFarmHarvest {
        item_id: ItemId,
        character_id: CharacterId,
        template: FdemonCrystalTemplate,
        foreground_sprite: u32,
    },
    FdemonFarmCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FdemonFarmNotReady {
        item_id: ItemId,
        character_id: CharacterId,
        current: u8,
        required: u8,
    },
    FdemonFarmBug {
        item_id: ItemId,
        character_id: CharacterId,
        crystal_number: u8,
    },
    FdemonBloodBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: FdemonBloodBlockReason,
    },
    FdemonBloodDestroyedFlask {
        item_id: ItemId,
        character_id: CharacterId,
        flask_item_id: ItemId,
    },
    FdemonBloodFilled {
        item_id: ItemId,
        character_id: CharacterId,
        container_item_id: ItemId,
        amount: u8,
    },
    FdemonLavaBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: FdemonLavaBlockReason,
    },
    FdemonLavaActivated {
        item_id: ItemId,
        character_id: CharacterId,
        container_item_id: ItemId,
        amount: u8,
        schedule_after_ticks: u64,
    },
    FdemonLavaPulse {
        item_id: ItemId,
        character_id: CharacterId,
        stage: u8,
        damage: i32,
        armor_percent: i32,
        schedule_after_ticks: Option<u64>,
    },
    SwampArmPulse {
        item_id: ItemId,
        character_id: CharacterId,
        damage_now: bool,
        schedule_after_ticks: u64,
    },
    SwampWhispPulse {
        item_id: ItemId,
        character_id: CharacterId,
        moved_from: Option<(u16, u16)>,
        moved_to: Option<(u16, u16)>,
        light_changed: bool,
        schedule_after_ticks: u64,
    },
    FdemonWaypoint {
        item_id: ItemId,
        character_id: CharacterId,
        spotted_enemy: bool,
        target_character_id: Option<CharacterId>,
        target_serial: Option<u32>,
        schedule_after_ticks: u64,
    },
    IceItemSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
    },
    IceItemSpawnCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IceItemSpawnBug {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    WarmFire {
        item_id: ItemId,
        character_id: CharacterId,
        create_scroll: bool,
        removed_curse: bool,
    },
    WarmFireCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BackToFire {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    MeltingKeyTick {
        item_id: ItemId,
        character_id: CharacterId,
        melted: bool,
        started_melting: bool,
        schedule_after_ticks: Option<u64>,
    },
    EdemonSwitchStuck {
        item_id: ItemId,
        character_id: CharacterId,
    },
    OnOffLightChanged {
        item_id: ItemId,
        character_id: CharacterId,
        now_on: bool,
        remaining_off: Option<i32>,
        gates_opened: bool,
    },
    PalaceGateTick {
        item_id: ItemId,
        opened: bool,
        closed: bool,
        blocked: bool,
    },
    PalaceBombExplode {
        item_id: ItemId,
        character_id: CharacterId,
        owner_id: u32,
        x: u16,
        y: u16,
    },
    PalaceBombTimer {
        item_id: ItemId,
        character_id: CharacterId,
        armed: bool,
        schedule_after_ticks: u64,
    },
    PalaceBombToggled {
        item_id: ItemId,
        character_id: CharacterId,
        active: bool,
    },
    PalaceCapTimer {
        item_id: ItemId,
        character_id: CharacterId,
        active: bool,
        schedule_after_ticks: u64,
    },
    PalaceDoorKeyRequired {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IslenaDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IslenaDoorRespawning {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IslenaDoorResting {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PalaceDoorTick {
        item_id: ItemId,
        character_id: CharacterId,
        state: u8,
        frame: u8,
        sprite: i32,
        set_tmoveblock: Option<bool>,
        schedule_after_ticks: Option<u64>,
    },
    TorchExtinguishedUnderwater {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    TorchExpired {
        item_id: ItemId,
        character_id: CharacterId,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    ClanJewelRescheduled {
        item_id: ItemId,
        schedule_after_ticks: u64,
    },
    ClanJewelExpired {
        item_id: ItemId,
        character_id: Option<CharacterId>,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    ClanSpawnTimer {
        item_id: ItemId,
        spawned: bool,
        jewel_count: u8,
        next_spawn_seconds: u32,
        schedule_after_ticks: u64,
    },
    LqTicker {
        item_id: ItemId,
        schedule_after_ticks: u64,
    },
    LqEntranceClosed {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LqEntranceLevelBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        min_level: u16,
        max_level: u16,
    },
    LqEntranceUndefined {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LqEntrancePenalty {
        item_id: ItemId,
        character_id: CharacterId,
        remaining_seconds: u32,
    },
    ClanSpawnLevelTooHigh {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ClanSpawnContested {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ClanSpawnCountdown {
        item_id: ItemId,
        character_id: CharacterId,
        remaining_minutes: u32,
        freq_hours: u8,
        god_added: bool,
    },
    ClanSpawnAward {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
        remaining_jewels: u8,
    },
    DecayItemToggled {
        item_id: ItemId,
        character_id: CharacterId,
        active: bool,
        schedule_after_ticks: Option<u64>,
    },
    DecayItemExpired {
        item_id: ItemId,
        character_id: CharacterId,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    LabExitAnimating {
        item_id: ItemId,
        sprite: i32,
        frame: u32,
        schedule_after_ticks: u64,
    },
    LabExitExpired {
        item_id: ItemId,
    },
    LabExitUse {
        item_id: ItemId,
        character_id: CharacterId,
        lab_nr: u8,
        frame: u32,
        target_area: u16,
        target_x: u16,
        target_y: u16,
    },
    LabExitWrongOwner {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LabEntranceSolvedAll {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LabEntranceTooLow {
        item_id: ItemId,
        character_id: CharacterId,
        required_level: u16,
    },
    DeathfibrinShrineGive {
        item_id: ItemId,
        character_id: CharacterId,
    },
    DeathfibrinShrineOccupied {
        character_id: CharacterId,
    },
    DeathfibrinNeedsCarry {
        character_id: CharacterId,
    },
    DeathfibrinNoMaster {
        character_id: CharacterId,
        tile_light: u8,
    },
    DeathfibrinStrike {
        item_id: ItemId,
        character_id: CharacterId,
        master_id: CharacterId,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
        vanished: bool,
    },
    BeyondPotion {
        item_id: ItemId,
        character_id: CharacterId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
        beyond_max_mod: bool,
    },
    AlchemyFlaskPotion {
        item_id: ItemId,
        character_id: CharacterId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
    },
    TorchExtractOrb {
        item_id: ItemId,
        character_id: CharacterId,
        modifier_slot: usize,
        modifier: i16,
    },
    /// C `raise_value_exp` (`src/system/skill.c:315-361`) called once per
    /// scroll charge by the stat scroll driver
    /// (`item_driver::scrolls::stat_scroll_driver`, `base.c:6031`
    /// `IDR_STATSCROLL`). `raised`/`exp_cost` are the totals across every
    /// successful charge in the loop; `World`'s outcome handler
    /// (`world/item_outcomes.rs`) applies `check_levelup` +
    /// `update_character` once for the batch, equivalent to C's per-charge
    /// `check_levelup(cn)`/`update_char(cn)` calls since both are
    /// idempotent/monotonic on the final `exp`/`value[1]` state (the
    /// profession-unlock edge case in `check_levelup` cannot diverge here:
    /// raising `V_PROFESSION` itself requires `value[1][V_PROFESSION]` to
    /// already be non-zero, which is exactly the condition under which
    /// `check_levelup`'s unlock is a no-op).
    StatScrollUsed {
        item_id: ItemId,
        character_id: CharacterId,
        value: u8,
        raised: u8,
        exp_cost: u32,
    },
    AssembleItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        template: AssembleTemplate,
    },
    AssembleNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    AssembleDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    AssembleUnknownItem {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EnchantCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        modifier: i16,
        amount: i16,
    },
    AntiEnchantCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        modifier: i16,
        amount: i16,
        extract_orb: bool,
    },
    OrbSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        anti: bool,
        special: bool,
    },
    NomadStack {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TransportOpen {
        item_id: ItemId,
        character_id: CharacterId,
        point: u8,
    },
    TransportTravel {
        item_id: ItemId,
        character_id: CharacterId,
        spec: i32,
    },
    TransportInvalid {
        item_id: ItemId,
        character_id: CharacterId,
        point: u8,
    },
    ArenaToplist {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SpecialPotionDrunk {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        hp_delta: i32,
        mana_delta: i32,
        endurance_delta: i32,
    },
    SpecialPotionAntidote {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        poison_removed: bool,
    },
    SpecialPotionInfravision {
        item_id: ItemId,
        character_id: CharacterId,
        installed: bool,
    },
    SpecialPotionSecurity {
        item_id: ItemId,
        character_id: CharacterId,
        used: bool,
    },
    SpecialPotionProfessionReset {
        item_id: ItemId,
        character_id: CharacterId,
        used: bool,
        professions_reset: u16,
        profession_points_lowered: u16,
        exp_refunded: u32,
    },
    SpecialPotionBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SpecialShrine {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    DemonShrine {
        item_id: ItemId,
        character_id: CharacterId,
        location_id: u32,
    },
    ZombieShrine {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
    },
    ZombieShrineNeedsOffering {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
    },
    RandomShrineNeedsKey {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
        level: u8,
    },
    RandomShrineAlreadyUsed {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
        level: u8,
    },
    RandomShrineUse {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
        level: u8,
        kind: RandomShrineKind,
    },
    RandomShrineBug {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
    },
    XmasMaker {
        item_id: ItemId,
        character_id: CharacterId,
    },
    XmasTree {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PalaceKeySplit {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_part_sprite: i32,
        carried_part_sprite: i32,
    },
    PalaceKeyCombine {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    },
    PalaceKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PalaceKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EnchantNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ShrikeAmuletAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    },
    ShrikeAmuletNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ShrikeAmuletDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGatewayKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    },
    MineGatewayKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGatewayKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGateway {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    MineGatewayNeedsKey {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGatewayBug {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    ArkhataKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_template_id: u32,
        result_sprite: i32,
        final_key: bool,
    },
    ArkhataKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataPool {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    ArkhataPoolNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataPoolWrongCursor {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    ArkhataStopwatch {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u32,
    },
    BlockedByRequirements {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EmptyPotionTemplateNeeded {
        item_id: ItemId,
        character_id: CharacterId,
        empty_kind: u8,
    },
    BlockedByArea {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LibloadAreaBlocked {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
        required_area: u16,
    },
    OxygenPotion {
        item_id: ItemId,
        character_id: CharacterId,
        installed: bool,
    },
    BranningtonUnderwaterBerry {
        item_id: ItemId,
        character_id: CharacterId,
        duration_ticks: u64,
        installed: bool,
    },
    PickBerry {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        location_id: u32,
    },
    PickBerryCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickAlchemyFlower {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        location_id: u32,
    },
    PickAlchemyFlowerCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    NomadDice {
        item_id: ItemId,
        character_id: CharacterId,
        luck: u8,
    },
    FlaskIngredientAdded {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        ingredient_kind: u8,
    },
    FlaskWrongCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskFull {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskFinishedNoMoreIngredients {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskEmptyShaken {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskIngredientBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskMixed {
        item_id: ItemId,
        character_id: CharacterId,
        ingredient_counts: [u8; 29],
    },
    FlaskRuined {
        item_id: ItemId,
        character_id: CharacterId,
        ingredient_counts: [u8; 29],
    },
    LizardFlowerMixed {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
        complete: bool,
        bottle_message: bool,
    },
    LizardFlowerNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LizardFlowerDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab3YellowBerry {
        item_id: ItemId,
        character_id: CharacterId,
        duration_ticks: u64,
        installed: bool,
    },
    Lab3WhiteBerry {
        item_id: ItemId,
        character_id: CharacterId,
        light_power: i16,
        started_emit: bool,
        installed: bool,
    },
    Lab3WhiteBerryLightTick {
        item_id: ItemId,
        destroyed: bool,
    },
    Lab3BrownBerry {
        item_id: ItemId,
        character_id: CharacterId,
        duration_ticks: u64,
        installed: bool,
    },
    /// C `lab3_special`'s `drdata[0]==1` teleport-door branch, blocked
    /// path (`ppd->guard_talkstep < 20`, `lab3.c:911-914`).
    Lab3TeleportDoorLocked {
        character_id: CharacterId,
    },
    /// The same branch's `teleport_char_driver` failure path
    /// (`lab3.c:917-920`, "there is a crowd behind the door").
    Lab3TeleportDoorBusy {
        character_id: CharacterId,
    },
    /// C `lab3_special`'s `drdata[0]==1` teleport-door branch, resolved by
    /// `World::apply_item_driver_outcome` (`lab3.c:916-965`):
    /// `teleport_char_driver` plus the water/torch/bubble/lab-exit-reward
    /// tail all happen there since none of it needs `ZoneLoader`/
    /// `PlayerRuntime`. `extinguished_count` starts at `0` on the raw
    /// outcome from the item driver and is filled in with the real count
    /// once resolved.
    Lab3TeleportDoor {
        item_id: ItemId,
        character_id: CharacterId,
        dx: i8,
        dy: i8,
        password_protected: bool,
        extinguished_count: u8,
    },
    /// C `lab3_special`'s `drdata[0]==2` note-giving-skeleton branch,
    /// blocked path (`ch[cn].citem` already occupied, `lab3.c:971-974`).
    Lab3NoteGivingBlocked {
        character_id: CharacterId,
    },
    /// The same branch's success path (`lab3.c:976-994`): creates a fresh
    /// `"lab3_note_generic"` on the *using player's* cursor with
    /// `drdata[1] = note_value` copied from the special item's own
    /// `drdata[1]`.
    Lab3NoteGivingSkeleton {
        item_id: ItemId,
        character_id: CharacterId,
        note_value: u8,
    },
    /// C `lab3_special`'s `drdata[0]==3` note-reading branch
    /// (`lab3.c:1001-1067`): `note_value` is the note item's own
    /// `drdata[1]`, matched against C's `switch (drdata[1])` cases
    /// `1..=6`/`20`/`21` server-side (`20`/`21` need `PlayerRuntime` for
    /// `lab3_init_password`, so the whole switch stays there rather than
    /// splitting canned-text cases into `World`).
    Lab3NoteRead {
        item_id: ItemId,
        character_id: CharacterId,
        note_value: u8,
    },
    /// C `lab4_item`'s `drdata[0]==1` fireplace-key branch, blocked path
    /// (`ch[cn].citem` already occupied, `lab4.c:657-659`).
    Lab4FireplaceKeyBlocked {
        character_id: CharacterId,
    },
    /// The same branch's success path (`lab4.c:660-669`): creates a fresh
    /// `"lab4_mage_key"` on the using player's cursor.
    Lab4FireplaceKeyGive {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2WaterWell {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2WaterAltar {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2WaterDrink {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2WaterCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2RegenerateTick {
        item_id: ItemId,
        target_id: CharacterId,
        start_tick: u32,
        regen_percent: u8,
        schedule_after_ticks: u64,
    },
    Lab2StepActionClear {
        item_id: ItemId,
    },
    Lab2StepActionDaemonWarning {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    Lab2StepActionDaemonCheck {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab2GraveClueBook {
        item_id: ItemId,
        character_id: CharacterId,
        book: u8,
    },
    Lab2GraveClose {
        item_id: ItemId,
    },
    Lab2GraveCheckOpen {
        item_id: ItemId,
        undead_id: CharacterId,
        undead_serial: u32,
        schedule_after_ticks: u64,
    },
    Lab2GraveOpen {
        item_id: ItemId,
        character_id: CharacterId,
        fixed_item: u8,
    },
    ParkShrine {
        item_id: ItemId,
        character_id: CharacterId,
        shrine: u8,
    },
    ParkShrineBug {
        item_id: ItemId,
        character_id: CharacterId,
        shrine: u8,
    },
    CaligarTraining {
        item_id: ItemId,
        character_id: CharacterId,
        lesson: u8,
    },
    CaligarWeightMove {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightTimer {
        item_id: ItemId,
    },
    CaligarWeightBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarSkellyDoor {
        item_id: ItemId,
        character_id: CharacterId,
        door_index: u8,
    },
    CaligarSkellyDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarSkellyDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    },
    CaligarKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BookText {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        demon_value: i32,
    },
    BookcaseText {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    SkelRaiseDust {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SkelRaiseTouch {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SkelRaiseRaise {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        template: &'static str,
    },
    SkelRaiseTimer {
        item_id: ItemId,
    },
    BookcaseLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentagramActivate {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
        color: u8,
    },
    PentagramAlreadyActive {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentagramTimer {
        item_id: ItemId,
        level: u8,
        status: u8,
        area_status: u8,
    },
    BoneHint {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
        nr: u8,
        pos: u8,
    },
    StafferBookText {
        item_id: ItemId,
        character_id: CharacterId,
        page: u8,
    },
    StafferAnimationBook {
        item_id: ItemId,
        character_id: CharacterId,
        exp_added: u32,
    },
    StafferMineDig {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferMineTimer {
        item_id: ItemId,
    },
    StafferMineExhausted {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferBlockMove {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferBlockTimer {
        item_id: ItemId,
    },
    StafferBlockBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneBridgePlace {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    BoneBridgeTimerTick {
        item_id: ItemId,
    },
    /// C `bonebridge`'s "bones in inventory" add-bone branch
    /// (`bones.c:236-252`): the cursor holds another `IID_AREA18_BONE`
    /// item and the carried bridge still has room (`drdata[0] <= 4`).
    BoneBridgeAddBone {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    /// C `bonebridge:239`: the carried bridge already holds 5 bones.
    BoneBridgeFinished {
        item_id: ItemId,
        character_id: CharacterId,
    },
    /// C `bonebridge:254`: the cursor holds an item that is not a bone.
    BoneBridgeWrongCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
    },
    /// C `bonebridge`'s "bones in inventory" remove-bone branch
    /// (`bones.c:257-269`): the cursor is empty and the carried bridge
    /// has at least 2 bones, so one is pulled back out onto the cursor.
    BoneBridgeRemoveBone {
        item_id: ItemId,
        character_id: CharacterId,
    },
    /// C `bonebridge:259`: the cursor is empty but the carried bridge has
    /// fewer than 2 bones (removing would destroy the base item).
    BoneBridgeNotEnoughBones {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHolderInsertRune {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        rune: u8,
        owner_character_id: u32,
        placed_tick: u32,
        schedule_after_ticks: u32,
    },
    BoneHolderRemoveRune {
        item_id: ItemId,
        character_id: CharacterId,
        rune: u8,
    },
    BoneHolderActivate {
        item_id: ItemId,
        character_id: CharacterId,
        last_holder: bool,
    },
    /// `World`-level resolution of [`Self::BoneHolderActivate`]: the
    /// three-preceding-stand scan (C `bones.c:698-717`) has already run
    /// and cleared any matched stands, producing the concatenated
    /// combination number and up to 3 `(holder_item_id, rune)` pairs the
    /// server crate should hand each rune item back for (`ZoneLoader`
    /// instantiation of `rune{1..9}`, needed outside `ugaris-core`).
    BoneHolderActivateResolved {
        item_id: ItemId,
        character_id: CharacterId,
        last_holder: bool,
        nr: i32,
        cleared: [Option<(ItemId, u8)>; 3],
    },
    BoneHolderBadCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHolderOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHolderEmptyTouch {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHolderWrongOwner {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHolderExpired {
        item_id: ItemId,
    },
    BoneWallTick {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SaltmineDoorBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SaltmineLadderUse {
        item_id: ItemId,
        character_id: CharacterId,
        ladder_index: u8,
    },
    SaltmineSaltbagUse {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineWallInitialized {
        item_id: ItemId,
        sprite: i32,
    },
    MineWallDig {
        item_id: ItemId,
        character_id: CharacterId,
        endurance_delta: i32,
        stage: u8,
        opened: bool,
    },
    MineWallCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineWallExhausted {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineWallCollapse {
        item_id: ItemId,
        schedule_after_ticks: u32,
    },
    AccountDepotOpened {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IdentityTag {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
    },
    Noop,
    Unsupported {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
    },
}

pub fn outcome_item_name(name: &str) -> [u8; OUTCOME_ITEM_NAME_BYTES] {
    let mut bytes = [0; OUTCOME_ITEM_NAME_BYTES];
    let source = name.as_bytes();
    let len = source.len().min(OUTCOME_ITEM_NAME_BYTES);
    bytes[..len].copy_from_slice(&source[..len]);
    bytes
}
