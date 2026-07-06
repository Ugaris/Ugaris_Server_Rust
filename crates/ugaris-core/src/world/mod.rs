//! Live game world state and mutation.
//!
//! `World` owns the map grid, characters, items, effects, timers, and the
//! action tick. Submodules port the corresponding legacy C systems:
//! `actions` (do.c/act.c), `combat`+`hurt` (act.c/death.c), `spells`
//! (do.c/act.c/tool.c/poison.c), `effects`+`effect_tick` (effect.c),
//! `npc_*` (drvlib.c fight driver + module/simple_baddy.c), `doors`,
//! `items`, `light` (light.c/sector.c), `spawn` (create.c slices), `text`
//! (talk.c fan-out), `lq` (area/20), `lab2_undead` (area/22), `exp`
//! (tool.c experience/level-up math), and `area_mech`/`assembly`/
//! `traps_hazards` for area-specific machinery.

mod aclerk;
mod actions;
mod admin_flag;
mod anticheat;
mod area_mech;
mod arena;
mod assembly;
mod bank;
mod character_values;
mod clanclerk;
mod clanmaster;
mod clubmaster;
mod combat;
mod complain;
mod consistency;
mod date;
mod death;
mod doors;
mod dungeon_fighter;
mod dungeon_master;
mod effect_tick;
mod effects;
mod exp;
mod exterminate;
mod gate_fight;
mod gatekeeper;
mod helpers;
mod hurt;
mod item_outcomes;
mod items;
mod jail;
mod janitor;
mod lab2_undead;
mod lastseen;
mod light;
mod lockname;
mod look;
mod loot;
mod lostcon;
mod lq;
mod merchant;
mod military;
mod npc_fight;
mod npc_idle;
mod npc_messages;
mod punish;
mod querystats;
mod regen;
mod rename;
mod rmdeath;
mod skills;
mod spawn;
mod special_item;
mod speed;
mod spells;
mod steal;
mod teleport;
mod text;
mod trader;
mod traps_hazards;
mod turn_seyan;
mod values;
mod weather;

pub use actions::*;
pub use admin_flag::*;
pub use anticheat::*;
pub(crate) use area_mech::*;
pub use arena::*;
pub(crate) use assembly::*;
pub use bank::*;
pub(crate) use character_values::*;
pub use clanclerk::*;
pub use clanmaster::*;
pub use clubmaster::*;
pub(crate) use combat::*;
pub use complain::*;
pub use consistency::*;
pub use death::*;
pub use doors::*;
pub use dungeon_master::*;
#[allow(unused_imports)]
pub(crate) use effect_tick::*;
#[allow(unused_imports)]
pub(crate) use effects::*;
pub use exp::*;
pub use exterminate::*;
pub use gatekeeper::*;
pub(crate) use helpers::*;
pub use hurt::*;
#[allow(unused_imports)]
pub(crate) use item_outcomes::*;
pub(crate) use items::*;
pub use jail::*;
#[allow(unused_imports)]
pub(crate) use lab2_undead::*;
pub use lastseen::*;
pub(crate) use light::*;
pub use lockname::*;
pub use look::*;
pub use loot::*;
pub use lq::*;
pub use merchant::*;
pub use military::*;
pub(crate) use npc_fight::*;
#[allow(unused_imports)]
pub(crate) use npc_idle::*;
#[allow(unused_imports)]
pub(crate) use npc_messages::*;
pub use punish::*;
pub use querystats::*;
#[allow(unused_imports)]
pub(crate) use regen::*;
pub use rename::*;
pub use rmdeath::*;
pub use skills::*;
#[allow(unused_imports)]
pub(crate) use spawn::*;
#[allow(unused_imports)]
pub(crate) use spells::*;
pub use steal::*;
#[allow(unused_imports)]
pub(crate) use teleport::*;
pub use text::*;
pub use trader::*;
#[allow(unused_imports)]
pub(crate) use traps_hazards::*;
pub use values::*;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::{
    area_sound::AreaSoundSpecial,
    attack::{attack_skill, reduce_hurt_by_armor, spell_average},
    character_driver::{
        add_simple_baddy_enemy, add_simple_baddy_enemy_unchecked, execute_character_died_driver,
        process_simple_baddy_messages,
        remove_simple_baddy_enemy as remove_simple_baddy_enemy_state, BankDriverData,
        CharacterDriverMessage, CharacterDriverOutcome, CharacterDriverState, Lab2UndeadDriverData,
        LostconDriverData, SimpleBaddyEnemy, SimpleBaddyMessageOutcome, CDR_ACLERK, CDR_BANK,
        CDR_DUNGEONFIGHTER, CDR_GATE_FIGHT, CDR_GATE_WELCOME, CDR_JANITOR, CDR_LAB2UNDEAD,
        CDR_LOSTCON, CDR_MERCHANT, CDR_MILITARY_ADVISOR, CDR_MILITARY_MASTER, CDR_SIMPLEBADDY,
        CDR_SWAMPMONSTER, CDR_TRADER, FDEMON_MSG_WAYPOINT, NTID_FDEMON, NTID_GATEKEEPER,
        NTID_LAB2_DEAMONCHECK, NTID_LABGNOMETORCH, NTID_TWOCITY_PICK, NT_CHAR, NT_CREATE, NT_DEAD,
        NT_DIDHIT, NT_GIVE, NT_GOTHIT, NT_ITEM, NT_NPC, NT_SEEHIT, NT_SPELL, NT_TEXT,
    },
    clan::ClanRegistry,
    club::ClubRegistry,
    direction::Direction,
    do_action::{
        act_attack, act_drop, act_heal, act_magicshield, act_take, act_use, act_walk,
        advance_action_step, can_attack, can_attack_in_area, can_attack_in_area_with_clan_policy,
        do_attack, do_ball, do_bless, do_drop, do_earthmud, do_fireball, do_flash, do_freeze,
        do_heal, do_idle, do_magicshield, do_pulse, do_take, do_use, do_walk, do_warcry,
        endurance_cost, reset_action_after_act, speed_ticks, speed_ticks_inverse, turn,
        ClanAttackPolicy, ItemUseRequest, DUR_MISC_ACTION,
    },
    drvlib::{char_dist, map_dist, step_char_dist, tile_char_dist},
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode,
        CHARACTER_VALUE_COUNT, INVENTORY_SIZE, MAX_MODIFIERS, POWERSCALE, V_FIREBALL,
    },
    game_settings::{
        GameSettings, SP_FEW_CONST, SP_MANY_CONST, SP_RARE_CONST, SP_SOME_CONST, SP_ULTRA_CONST,
    },
    game_time::GameDate,
    ids::{CharacterId, ItemId},
    item_driver::{
        execute_item_driver_with_context, reset_flask_empty_state, use_item,
        EdemonGateSpawnContext, FdemonGateSpawnContext, ItemDriverContext, ItemDriverOutcome,
        ItemDriverRequest, UseItemError, UseItemOutcome, WarpTrialDoorContext, IDR_BONEWALL,
        IDR_CALIGAR, IDR_CALIGARFLAME, IDR_CLANJEWEL, IDR_CLANSPAWN, IDR_DOOR, IDR_DUNGEONDOOR,
        IDR_EDEMONBALL, IDR_EDEMONBLOCK, IDR_EDEMONDOOR, IDR_EDEMONGATE, IDR_EDEMONLIGHT,
        IDR_EDEMONLOADER, IDR_EDEMONSWITCH, IDR_EDEMONTUBE, IDR_FDEMONCANNON, IDR_FDEMONFARM,
        IDR_FDEMONGATE, IDR_FDEMONLIGHT, IDR_FDEMONLOADER, IDR_FLAMETHROW, IDR_FLASK,
        IDR_FORESTCHEST, IDR_LAB2_WATER, IDR_LAB3_PLANT, IDR_LABTORCH, IDR_MINEDOOR,
        IDR_MINEGATEWAY, IDR_NIGHTLIGHT, IDR_ONOFFLIGHT, IDR_PALACEDOOR, IDR_POTION,
        IDR_RANDOMSHRINE, IDR_STEPTRAP, IDR_SWAMPARM, IDR_SWAMPSPAWN, IDR_SWAMPWHISP, IDR_TORCH,
        IDR_TOYLIGHT, IDR_WARPKEYDOOR, IDR_WARPTELEPORT, IDR_WARPTRIALDOOR, IID_AREA11_PALACEKEY,
        IID_AREA14_SHRINEKEY, IID_AREA16_ROBBERKEY, IID_AREA16_SKELLYKEY, IID_AREA25_DOORKEY,
        IID_AREA25_TELEKEY, IID_GENERIC_SPECIAL, IID_MINEGATEWAY,
    },
    item_ops::{consume_item, give_item_to_character, GiveItemFlags, GiveItemResult},
    legacy::{
        action, profession, worn_slot, DIST_MAX, INVENTORY_LAST_SPELLS, INVENTORY_START_INVENTORY,
        INVENTORY_START_SPELLS, MAX_FIELD, MAX_MAP, SAY_DIST,
    },
    light::{
        add_character_light, add_effect_light, add_item_light, compute_dlight, compute_groundlight,
        compute_shadow_with_random, remove_character_light, remove_effect_light, remove_item_light,
        reset_dlight, LIGHT_DISTANCE,
    },
    log_text::{
        emote_message, murmur_message, quiet_say_message, say_message, whisper_message, LOG_TALK,
    },
    map::{manhattan_distance, MapFlags, MapGrid},
    path::{pathfinder, pathfinder_ignore_characters},
    player::{PlayerActionCode, PlayerRuntime},
    scheduler::{TaskScheduler, TimerPayload, TimerQueue},
    sector::{DirtySectors, SoundSectors},
    see::{char_see_char, char_see_item},
    spell::{
        add_same_spell_slot, fireball_damage, freeze_speed_modifier, is_timed_spell_driver,
        may_add_spell, pulse_damage, pulse_spend, read_spell_expire_tick, spell_power,
        strike_damage, warcry_damage, warcry_speed_modifier, BLESS_COST, BLESS_DURATION, EF_BALL,
        EF_BLESS, EF_BUBBLE, EF_BURN, EF_CAP, EF_CURSE, EF_EARTHMUD, EF_EARTHRAIN, EF_EDEMONBALL,
        EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE, EF_HEAL, EF_MAGICSHIELD,
        EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE, EF_WARCRY, FIREBALL_COST,
        FLASH_COST, FLASH_DURATION, FREEZE_COST, FREEZE_DURATION, IDR_ARMOR, IDR_BLESS, IDR_CURSE,
        IDR_FIRERING, IDR_FLASH, IDR_FREEZE, IDR_HP, IDR_INFRARED, IDR_MANA, IDR_NONOMAGIC,
        IDR_OXYGEN, IDR_POISON0, IDR_POISON3, IDR_POTION_SP, IDR_UWTALK, IDR_WARCRY, IDR_WEAPON,
        POISON_DURATION, SPELL_SLOT_END, SPELL_SLOT_START, WARCRY_DURATION,
    },
    tick::TICKS_PER_SECOND,
    zone::ZoneLoader,
    Tick,
};

const LEGACY_EQUIPMENT_SLOTS: std::ops::Range<usize> = 0..12;

const ITEM_DRIVER_TIMER: &str = "item_driver";

const REMOVE_SPELL_TIMER: &str = "remove_spell";

const POISON_CALLBACK_TIMER: &str = "poison_callback";

fn legacy_random_below_from_seed(seed: &mut u32, below: u32) -> u32 {
    if below == 0 {
        return 0;
    }
    *seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    *seed % below
}

fn legacy_random_variant_below_from_seed(seed: &mut u32, below: u32) -> u32 {
    if below == 0 {
        return 0;
    }
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed % below
}

#[derive(Debug, Default)]
pub struct World {
    /// The C server's `areaID` (`src/server.c`, set from the area
    /// server's launch config and constant for the process lifetime -
    /// this Rust server is one process per area). Defaults to `0` (no
    /// area loaded yet / test scaffolding); the runtime sets it once at
    /// startup from `ServerConfig::area_id`. Used by
    /// [`crate::world::character_values::recompute_character_values`]
    /// for the `P_CLAN`/catacombs (`areaID == 13`) bonus check (C
    /// `create.c:1856`).
    pub area_id: u16,
    pub tick: Tick,
    pub date: GameDate,
    /// Clan identity/membership/relation registry (`src/system/clan.c`).
    /// Not yet persisted (`ClanRegistry`'s own doc comment) and not yet
    /// mutated by any live `/clan` command - see the "Clan system" P3
    /// task in `PORTING_TODO.md`. Wired into combat's
    /// [`crate::do_action::ClanAttackPolicy`] via
    /// `world::combat::RuntimePlayerAttackPolicy` so `can_attack`'s
    /// clan-relation checks reflect real founded clans once that command
    /// exists; today every character's `clan` field defaults
    /// to `0` (no clan), which this registry treats as "not a member of
    /// any clan pair", so this wiring is a behavior no-op until clans are
    /// actually founded.
    pub clan_registry: ClanRegistry,
    /// Club identity/serial registry (`src/system/club.c`), the parallel
    /// larger-scale variant of [`Self::clan_registry`] distinguished by
    /// `Character.clan >= CLUB_OFFSET`. Not yet persisted and not yet
    /// mutated by any live `/joinclub`/`/killclub`/`/renclub` command or
    /// the `CDR_CLUBMASTER` founding NPC - see the "Clan system" P3 task
    /// in `PORTING_TODO.md` and `crate::club`'s module doc comment. Today
    /// only reachable from `world::clanmaster::is_club_member`'s
    /// membership gate.
    pub club_registry: ClubRegistry,
    /// Military Master NPC-scoped storage blobs (`src/module/
    /// military.c`'s `struct military_master_storage`), keyed by each
    /// NPC's zone-file `storage=N;` id. In-memory only, no DB
    /// persistence yet - see [`crate::world::MilitaryMasterStorageRegistry`]'s
    /// doc comment.
    pub military_master_storage: MilitaryMasterStorageRegistry,
    /// Military Advisor NPC-scoped sales-economy storage blobs
    /// (`struct military_advisor_data`'s `cost_data storage_data[5]`),
    /// keyed by each NPC's zone-file `storage=N;` id. In-memory only, no
    /// DB persistence yet - see
    /// [`crate::world::MilitaryAdvisorStorageRegistry`]'s doc comment.
    pub military_advisor_storage: MilitaryAdvisorStorageRegistry,
    pub show_attack_debug: bool,
    pub timers: TimerQueue,
    pub scheduler: TaskScheduler,
    pub map: MapGrid,
    pub dirty_sectors: DirtySectors,
    pub characters: HashMap<CharacterId, Character>,
    pub items: HashMap<ItemId, Item>,
    pub effects: HashMap<u32, Effect>,
    pub settings: GameSettings,
    pub area3_palace_lamps: Area3PalaceLampState,
    pub legacy_random_seed: u32,
    pub lq_doors_initialized: bool,
    pub lq_doors: Vec<LqDoorState>,
    pub lq_npcs: Vec<LqNpcState>,
    pub lq_npc_respawns: Vec<(usize, u64)>,
    pub npc_respawn_slots: Vec<NpcRespawnSlot>,
    pub merchant_stores: HashMap<CharacterId, MerchantStore>,
    /// Parsed death/spawn-mode loot tables (`src/system/loot/loot.c`'s
    /// `tables[]`/`n_tables` plus `pity_counters[]`) - see
    /// [`LootRegistry`]'s doc comment for the `ugaris-server`
    /// file-scanning split. In-memory only; reset on restart, same as
    /// every other server-wide registry above.
    pub loot_registry: LootRegistry,
    /// Server-wide arena tournament ranking table (C's `static struct
    /// toplist *tops`, `arena.c:255`). In-memory only, no DB/storage-blob
    /// persistence yet (resets on restart) - same architectural gap as
    /// `MilitaryMasterStorageRegistry`, documented in the "Arena rankings"
    /// P3 task in `PORTING_TODO.md`. Lazily grown to
    /// [`arena::ARENA_TOPLIST_SIZE`] entries by
    /// [`World::arena_update_toplist`]; empty (no entries yet) reads back
    /// as "no rankings" via [`World::arena_toplist_entries`].
    pub arena_toplist: Vec<ArenaToplistRecord>,
    pending_npc_respawns: Vec<NpcRespawnRequest>,
    pending_kill_exp: Vec<KillExpAward>,
    pending_kill_achievements: Vec<KillAchievementAward>,
    pending_first_kill_checks: Vec<FirstKillCheck>,
    pending_military_mission_checks: Vec<MilitaryMissionKillCheck>,
    pending_level_achievements: Vec<LevelAchievementCheck>,
    pending_lq_npc_spawns: Vec<LqNpcSpawnRequest>,
    pending_look_maps: Vec<LookMapRequest>,
    pending_sound_specials: Vec<WorldSoundSpecial>,
    pending_system_texts: Vec<WorldSystemText>,
    pending_system_text_bytes: Vec<WorldSystemTextBytes>,
    pending_area_texts: Vec<WorldAreaText>,
    pending_channel_broadcasts: Vec<WorldChannelBroadcast>,
    pending_hurt_events: Vec<LegacyHurtEvent>,
    pending_bank_events: Vec<BankEvent>,
    pending_trader_events: Vec<TraderEvent>,
    pending_clanmaster_events: Vec<ClanmasterEvent>,
    pending_clanclerk_events: Vec<ClanclerkEvent>,
    pending_clubmaster_events: Vec<ClubmasterEvent>,
    pending_military_master_events: Vec<MilitaryMasterEvent>,
    pending_military_advisor_events: Vec<MilitaryAdvisorEvent>,
    pending_arena_master_events: Vec<ArenaMasterEvent>,
    pending_dungeon_raid_builds: Vec<DungeonRaidBuildRequest>,
    pending_dungeon_jewel_steals: Vec<DungeonJewelStealEvent>,
    pending_death_loot_rolls: Vec<PendingDeathLootRoll>,
    pending_lastseen_lookups: Vec<LastSeenLookup>,
    pending_complain_lookups: Vec<ComplainLookup>,
    /// `/jail`/`/unjail` targets not found among the currently loaded
    /// characters yet - see `world/jail.rs`'s module doc comment.
    pending_jail_lookups: Vec<JailLookup>,
    /// `/jail`/`/unjail` mutations whose destination area differs from
    /// this area server's own `area_id` - see `world/jail.rs`'s module
    /// doc comment.
    pending_jail_cross_area_transfers: Vec<JailCrossAreaTransfer>,
    /// `/rmdeath` targets not found among the currently loaded characters
    /// yet - see `world/rmdeath.rs`'s module doc comment.
    pending_rmdeath_lookups: Vec<RmdeathLookup>,
    /// `/god`/`/setsir`/`/staff`/`/emaster`/`/devel`/`/hardcore`/
    /// `/qmaster` targets not found among the currently loaded
    /// characters - see `world/admin_flag.rs`'s module doc comment.
    pending_admin_flag_toggles: Vec<AdminFlagToggle>,
    /// `#acstatus <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_status_lookups: Vec<AcStatusLookup>,
    /// `#aclist` async DB round trips - see `world/anticheat.rs`'s module
    /// doc comment.
    pending_ac_list_lookups: Vec<AcListLookup>,
    /// `#acstats` async DB round trips - see `world/anticheat.rs`'s module
    /// doc comment.
    pending_ac_stats_lookups: Vec<AcStatsLookup>,
    /// `#acsuspicious` async DB round trips - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_suspicious_lookups: Vec<AcSuspiciousLookup>,
    /// `#accleanup <days>` async DB round trips - see
    /// `world/anticheat.rs`'s module doc comment.
    pending_ac_cleanup_lookups: Vec<AcCleanupLookup>,
    /// `#acreset <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_reset_lookups: Vec<AcResetLookup>,
    /// `#acflag <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_flag_lookups: Vec<AcFlagLookup>,
    /// `#acunflag <name>` async DB round trips (session id already
    /// resolved synchronously by the caller; the "is not flagged" status
    /// gate itself happens later, after the round trip) - see
    /// `world/anticheat.rs`'s module doc comment.
    pending_ac_unflag_lookups: Vec<AcUnflagLookup>,
    /// `#actrust <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_trust_lookups: Vec<AcTrustLookup>,
    /// `#acuntrust <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_untrust_lookups: Vec<AcUntrustLookup>,
    /// `#acwarn <name> [reason]` async DB round trips (session id and
    /// target character id already resolved synchronously by the
    /// caller) - see `world/anticheat.rs`'s module doc comment.
    pending_ac_warn_lookups: Vec<AcWarnLookup>,
    /// `#acsessions <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_sessions_lookups: Vec<AcSessionsLookup>,
    /// `#acviolations <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_violations_lookups: Vec<AcViolationsLookup>,
    /// `#achistory <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_history_lookups: Vec<AcHistoryLookup>,
    /// `#acsiglist` async DB round trips - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_siglist_lookups: Vec<AcSiglistLookup>,
    /// `#acsigadd <type> <value> <name>` async DB round trips (parsed and
    /// validated synchronously by the caller) - see
    /// `world/anticheat.rs`'s module doc comment.
    pending_ac_sigadd_lookups: Vec<AcSigaddLookup>,
    /// `#acsigdel <id>` async DB round trips (id parsed synchronously by
    /// the caller) - see `world/anticheat.rs`'s module doc comment.
    pending_ac_sigdel_lookups: Vec<AcSigdelLookup>,
    /// `#acsharedip <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_sharedip_lookups: Vec<AcSharedIpLookup>,
    /// `#acsharedhw <name>` async DB round trips (session id already
    /// resolved synchronously by the caller) - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_sharedhw_lookups: Vec<AcSharedHwLookup>,
    /// `#achighrisk` async DB round trips - see `world/anticheat.rs`'s
    /// module doc comment.
    pending_ac_highrisk_lookups: Vec<AcHighriskLookup>,
    /// `#aclookup <subscriber_id>` async DB round trips (id parsed
    /// synchronously by the caller) - see `world/anticheat.rs`'s module
    /// doc comment.
    pending_ac_lookup_lookups: Vec<AcLookupLookup>,
    /// `#querystats`/`/querystats` async DB round trips - see
    /// `world/querystats.rs`'s module doc comment.
    pending_querystats_lookups: Vec<QueryStatsLookup>,
    /// `/rename <from> <to>` async DB round trips - see
    /// `world/rename.rs`'s module doc comment.
    pending_rename_lookups: Vec<RenameLookup>,
    /// `/lockname <name>` async DB round trips - see
    /// `world/lockname.rs`'s module doc comment.
    pending_lockname_lookups: Vec<LockNameLookup>,
    /// `/unlockname <name>` async DB round trips - see
    /// `world/lockname.rs`'s module doc comment.
    pending_unlockname_lookups: Vec<UnlockNameLookup>,
    /// `/punish <name> <level> <reason>` async DB round trips - see
    /// `world/punish.rs`'s module doc comment.
    pending_punish_requests: Vec<PunishRequest>,
    /// `/unpunish <name> <note id>` async DB round trips - see
    /// `world/punish.rs`'s module doc comment.
    pending_unpunish_requests: Vec<UnpunishRequest>,
    /// `#look <name>` async DB round trips (name resolution, then a
    /// per-character notes list) - see `world/look.rs`'s module doc
    /// comment.
    pending_look_requests: Vec<LookRequest>,
    /// `#klog` async DB round trips (no name to resolve - just the
    /// caller's own id to reply to) - see `world/look.rs`'s module doc
    /// comment.
    pending_klog_requests: Vec<CharacterId>,
    /// `build_remove_tile`'s evicted-player rescue whose `rest_area`
    /// differs from this area server's own `area_id` - see
    /// `world/dungeon_master.rs`'s module doc comment.
    pending_dungeon_eviction_transfers: Vec<DungeonEvictionTransfer>,
    /// `/exterminate <name>` async DB round trips - see
    /// `world/exterminate.rs`'s module doc comment.
    pending_exterminate_requests: Vec<ExterminateRequest>,
    /// `/showvalues <name>` async DB round trips - see
    /// `world/values.rs`'s module doc comment.
    pending_showvalues_requests: Vec<ShowValuesRequest>,
    /// `/values <name>` async DB round trips - see `world/values.rs`'s
    /// module doc comment.
    pending_values_requests: Vec<ValuesRequest>,
}

impl Default for Tick {
    fn default() -> Self {
        Self(0)
    }
}
