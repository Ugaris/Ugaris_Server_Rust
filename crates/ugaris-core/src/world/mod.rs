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

mod actions;
mod admin_flag;
mod allow;
mod anticheat;
mod area_mech;
mod assembly;
mod bones;
mod character_values;
mod combat;
mod complain;
mod consistency;
mod date;
mod death;
mod doors;
mod effect_tick;
mod effects;
mod exp;
mod exterminate;
mod fdemon;
mod helpers;
mod hurt;
mod item_outcomes;
mod items;
mod jail;
mod lab;
mod lastseen;
mod light;
mod lockname;
mod look;
mod loot;
mod lq;
mod lq_admin;
pub use lq_admin::{LqNspawnDispatch, LqThrallDispatch};
mod lq_quest_admin;
mod lq_quest_file;
pub use lq_quest_file::{LqQuestFile, LqQuestFileDispatch, LqQuestSnapshot};
pub mod lq_usurp;
mod mining;
pub mod npc;
mod npc_fight;
mod npc_idle;
mod npc_messages;
mod pents;
mod player_driver;
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
mod strategy;
mod strategy_ai;
mod strategy_ai_main;
mod strategy_ai_tasks;
mod strategy_special;
mod strategy_worker;
mod teleport;
mod text;
mod traps_hazards;
mod tunnel;
mod turn_seyan;
mod tutorial;
mod values;
mod weather;

pub use actions::*;
pub use admin_flag::*;
pub use allow::*;
pub use anticheat::*;
pub(crate) use area_mech::*;
pub(crate) use assembly::*;
pub(crate) use character_values::*;
pub(crate) use combat::*;
pub use complain::*;
pub use consistency::*;
pub use death::*;
pub use doors::*;
#[allow(unused_imports)]
pub(crate) use effect_tick::*;
#[allow(unused_imports)]
pub(crate) use effects::*;
pub use exp::*;
pub use exterminate::*;
pub use fdemon::*;
pub(crate) use helpers::*;
pub use hurt::*;
#[allow(unused_imports)]
pub(crate) use item_outcomes::*;
pub(crate) use items::*;
pub use jail::*;
pub use lab::*;
#[allow(unused_imports)]
pub use lastseen::*;
pub(crate) use light::*;
pub use lockname::*;
pub use look::*;
pub use loot::*;
pub use lq::*;
pub use mining::*;
pub use npc::*;
pub use npc_fight::*;
#[allow(unused_imports)]
pub(crate) use npc_idle::*;
#[allow(unused_imports)]
pub(crate) use npc_messages::*;
pub use pents::*;
pub use punish::*;
pub use querystats::*;
#[allow(unused_imports)]
pub(crate) use regen::*;
pub use rename::*;
pub use rmdeath::*;
pub use skills::*;
#[allow(unused_imports)]
pub(crate) use spawn::*;
pub use special_item::RandomShrineWeldingResult;
#[allow(unused_imports)]
pub(crate) use spells::*;
pub use steal::*;
pub use strategy::*;
pub use strategy_ai::*;
pub use strategy_ai_main::AiMainOutcome;
pub use strategy_ai_tasks::{AiEguardSpawnPlan, AiWorkerSpawnPlan};
pub use strategy_special::*;
pub use strategy_worker::*;
#[allow(unused_imports)]
pub(crate) use teleport::*;
pub use text::*;
#[allow(unused_imports)]
pub(crate) use traps_hazards::*;
pub use tunnel::*;
pub use tutorial::*;
pub use values::*;

#[cfg(test)]
mod tests;

// Names used only by test modules under `world::tests` via `use super::*`.
#[allow(unused_imports)]
use crate::character_driver::{
    CDR_BANK, CDR_GATE_FIGHT, CDR_JANITOR, CDR_LAB2UNDEAD, CDR_MILITARY_ADVISOR,
    CDR_MILITARY_MASTER, CDR_TRADER, NTID_GATEKEEPER, NT_CREATE,
};

use std::collections::HashMap;

use crate::{
    area_sound::AreaSoundSpecial,
    attack::{attack_skill, reduce_hurt_by_armor, spell_average},
    character_driver::{
        add_simple_baddy_enemy, add_simple_baddy_enemy_unchecked, execute_character_died_driver,
        process_simple_baddy_messages,
        remove_simple_baddy_enemy as remove_simple_baddy_enemy_state, CharacterDriverMessage,
        CharacterDriverOutcome, CharacterDriverState, FightDriverData, SimpleBaddyEnemy,
        SimpleBaddyMessageOutcome, CDR_ACLERK, CDR_ARKHATAPRISON, CDR_CALIGARGUARD2,
        CDR_CALIGARSKELLY, CDR_CAMERON_FORESTMONSTER, CDR_CENTINEL, CDR_DUNGEONFIGHTER,
        CDR_FORESTMONSTER, CDR_GATE_WELCOME, CDR_LOSTCON, CDR_MERCHANT, CDR_MISSIONFIGHT,
        CDR_PENTER, CDR_SIMPLEBADDY, CDR_SMUGGLELEAD, CDR_SWAMPMONSTER, CDR_TEUFELDEMON,
        CDR_TEUFELRAT, CDR_TWOROBBER, CDR_WHITEROBBERBOSS, FDEMON_MSG_WAYPOINT, NTID_FDEMON,
        NTID_LAB2_DEAMONCHECK, NTID_LABGNOMETORCH, NTID_TWOCITY_PICK, NT_CHAR, NT_DEAD, NT_DIDHIT,
        NT_GIVE, NT_GOTHIT, NT_ITEM, NT_NPC, NT_SEEHIT, NT_SPELL, NT_TEXT,
    },
    clan::ClanRegistry,
    club::ClubRegistry,
    direction::Direction,
    do_action::{
        act_attack, act_drop, act_heal, act_magicshield, act_take, act_use, act_walk,
        advance_action_step, can_attack, can_attack_in_area, can_attack_in_area_with_clan_policy,
        do_attack, do_ball, do_bless, do_drop, do_earthmud, do_fireball, do_flash, do_freeze,
        do_heal, do_idle, do_magicshield, do_pulse, do_take, do_use, do_walk, do_warcry,
        edemon_reduction, endurance_cost, reset_action_after_act, speed_ticks, speed_ticks_inverse,
        turn, ClanAttackPolicy, ItemUseRequest, DUR_MISC_ACTION,
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
        execute_item_driver_with_context, extinguish_torch, reset_flask_empty_state, use_item,
        EdemonGateSpawnContext, FdemonGateSpawnContext, ItemDriverContext, ItemDriverOutcome,
        ItemDriverRequest, StrStorageConversion, UseItemError, UseItemOutcome,
        WarpTrialDoorContext, IDR_BONEWALL, IDR_CALIGAR, IDR_CALIGARFLAME, IDR_CLANJEWEL,
        IDR_CLANSPAWN, IDR_DEATHFIBRIN, IDR_DOOR, IDR_DUNGEONDOOR, IDR_EDEMONBALL, IDR_EDEMONBLOCK,
        IDR_EDEMONDOOR, IDR_EDEMONGATE, IDR_EDEMONLIGHT, IDR_EDEMONLOADER, IDR_EDEMONSWITCH,
        IDR_EDEMONTUBE, IDR_FDEMONCANNON, IDR_FDEMONFARM, IDR_FDEMONGATE, IDR_FDEMONLIGHT,
        IDR_FDEMONLOADER, IDR_FLAMETHROW, IDR_FLASK, IDR_FORESTCHEST, IDR_LAB2_WATER,
        IDR_LAB3_PLANT, IDR_LAB5_ITEM, IDR_LABTORCH, IDR_LQ_TICKER, IDR_MINEDOOR, IDR_MINEGATEWAY,
        IDR_NIGHTLIGHT, IDR_ONOFFLIGHT, IDR_PALACEDOOR, IDR_PENT, IDR_POTION, IDR_RANDOMSHRINE,
        IDR_RECALL, IDR_STEPTRAP, IDR_STR_DEPOT, IDR_STR_MINE, IDR_STR_SPAWNER, IDR_STR_STORAGE,
        IDR_STR_TICKER, IDR_SWAMPARM, IDR_SWAMPSPAWN, IDR_SWAMPWHISP, IDR_TORCH, IDR_TOYLIGHT,
        IDR_TUNNELDOOR2, IDR_WARPKEYDOOR, IDR_WARPTELEPORT, IDR_WARPTRIALDOOR,
        IID_AREA11_PALACEKEY, IID_AREA14_SHRINEKEY, IID_AREA16_ROBBERKEY, IID_AREA16_SKELLYKEY,
        IID_AREA25_DOORKEY, IID_AREA25_TELEKEY, IID_GENERIC_SPECIAL, IID_MINEGATEWAY,
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
        emote_message, murmur_message, quiet_say_message, say_message, shout_message,
        whisper_message, LOG_TALK,
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

/// C `RANDOM(n)` (`src/system/tool.h`, an `lrand48`-free LCG the legacy
/// server seeds once at startup): visible crate-wide (rather than
/// `world`-private like most of this module's internals) so
/// `crate::macro_daemon`'s pure decision functions - which cannot be
/// `impl World` methods since they don't touch `World` at all, see that
/// module's doc comment - can still reproduce the exact same random
/// sequence a real `World`-driven caller's `legacy_random_seed` would
/// produce, instead of duplicating this two-line LCG a second time.
pub(crate) fn legacy_random_below_from_seed(seed: &mut u32, below: u32) -> u32 {
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
    /// C `struct lamp lamp[MAXLAMP]`'s `cn`/`cost` claim fields
    /// (`area3.c:2601-2607`), keyed directly by `ItemId` instead of a
    /// `lamp[]` slot index - see `world::npc::area3::lampghost`'s module
    /// doc comment. `in`/registration membership is already `Item::
    /// driver_data[6]` (see `Area3PalaceLampState`'s own doc comment).
    pub area3_lamp_claims: HashMap<ItemId, (CharacterId, i32)>,
    /// C `int namecoordx[4]`/`int namecoordy[4]` (`src/area/22/lab5.c:
    /// 105-107`): dynamic overrides of the Master Demons' nameplate/
    /// entrance/mage-spawn coordinates, indexed by daemon number
    /// (`namecoordx[0]`/`namecoordy[0]` is the mage's own spawn tile,
    /// `[1..=3]` are the three nameplate items' positions). `None` means
    /// "still C's static initializer default" - see
    /// [`crate::world::npc::area22::lab5_mage::LAB5_NAMECOORD_DEFAULTS`]
    /// and `World::lab5_namecoord`. Only index 0 (written by the mage's
    /// own `NT_CREATE`) is wired today; `IDR_LAB5_ITEM`'s nameplate
    /// branch (`drdata[0]==5`, indices 1-3) is not yet ported.
    pub lab5_namecoords: [Option<(i32, i32)>; 4],
    pub legacy_random_seed: u32,
    pub lq_doors_initialized: bool,
    pub lq_doors: Vec<LqDoorState>,
    pub lq_npcs: Vec<LqNpcState>,
    pub lq_npc_respawns: Vec<(usize, u64)>,
    /// C's single-instance `struct lq_data lq_data` (`src/area/20/lq.c:162`)
    /// - see [`LqData`]'s own doc comment.
    pub lq_data: LqData,
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
    /// C `src/area/4/pents.c`'s file-static pentagram-quest solve state
    /// (`solve_serial`/`active_pentagrams`/`power_levels`/etc). See
    /// [`PentagramQuestState`]'s doc comment for what is/isn't ported.
    pub pentagram_quest: PentagramQuestState,
    /// C's file-static `struct waypoint wp[MAXWAY]`/`maxway`
    /// (`src/area/8/fdemon.c:2492-2503`) - see [`FdemonWaypoint`]'s and
    /// `world::fdemon`'s module doc comments. Empty means "not built yet"
    /// (matching C's `maxway==1` sentinel, ported as index `0` being an
    /// always-present unused sentinel once built).
    pub fdemon_waypoints: Vec<FdemonWaypoint>,
    /// C's file-static `struct str_area area[MAX_STR_AREA]`/`int
    /// area_init` (`src/area/23_24/strategy.c:154-155`) - see
    /// [`StrategyAreaRegistry`]'s doc comment and
    /// `World::ensure_strategy_areas_initialized` (the `init_areas` port).
    pub strategy_areas: StrategyAreaRegistry,
    /// C's file-static `struct jumppoint jp[MAXJUMP]`/`int special_init`
    /// (`src/area/23_24/strategy.c:2994-2995`) - see
    /// [`StrategyJumpPointRegistry`]'s doc comment and `World::
    /// ensure_strategy_jump_points_initialized`.
    pub strategy_jump_points: StrategyJumpPointRegistry,
    /// C's file-static `struct ai_data ai_data[MAX_AI]` (`src/area/23_24/
    /// strategy.c:1787`), one entry per AI-controlled battleground party -
    /// see [`crate::world::strategy_ai::AiData`]'s own doc comment. Keyed
    /// directly by the party's `code` (C's own `ai_data[code -
    /// STR_OWNER_AI_BASE]` index) rather than a fixed-size array; a
    /// missing entry means "this AI party's `ai_init` hasn't run yet" (C's
    /// own `!ad->ai_init` gate, `strategy.c:2450`), which [`World::
    /// ai_main`] uses exactly that way. In-memory only, no DB persistence -
    /// same "resets on restart" precedent as every other server-wide
    /// registry above (e.g. [`Self::arena_toplist`]); C's own AI armies
    /// are just as ephemeral (rebuilt from the still-live place graph and
    /// `CDR_STRATEGY` roster the next time `ai_main` runs after a
    /// restart).
    pub ai_parties: HashMap<u32, AiData>,
    /// `World::str_spawner_ambient_tick`'s queue of pending AI worker
    /// spawn plans (`ai_main`'s "create new workers" tail, `strategy.c:
    /// 2644-2672`), paired with the party `code` `ugaris-server` must
    /// hand back to `World::register_ai_worker` once the character is
    /// actually built (`AiWorkerSpawnPlan::group` is `u16`-narrowed and
    /// can't round-trip the real `code`, see that field's own doc
    /// comment). `World` can't build the character itself (needs
    /// `ZoneLoader`), same "pure `World` queues, `ugaris-server` drains"
    /// precedent as [`Self::pending_lq_npc_spawns`].
    pending_ai_worker_spawns: Vec<(u32, AiWorkerSpawnPlan)>,
    /// Same as [`Self::pending_ai_worker_spawns`] for `ai_main`'s "place
    /// eternal guards" tail (`strategy.c:2892-2916`); the `usize` is the
    /// place index `World::register_ai_eguard` needs back.
    pending_ai_eguard_spawns: Vec<(u32, usize, AiEguardSpawnPlan)>,
    /// `World::str_reward_winner`'s queue of pending `reward_winner`
    /// (`strategy.c:428-454`) `ppd` mutations - `World` can't reach
    /// session-owned `PlayerRuntime::strategy` directly, so `ugaris-
    /// server`'s `strategy::apply_strategy_reward_events` drains this and
    /// applies `crate::world::apply_strategy_mission_win` for real.
    pending_strategy_rewards: Vec<StrategyRewardEvent>,
    pending_npc_respawns: Vec<NpcRespawnRequest>,
    pending_kill_exp: Vec<KillExpAward>,
    pending_kill_achievements: Vec<KillAchievementAward>,
    pending_first_kill_checks: Vec<FirstKillCheck>,
    pending_military_mission_checks: Vec<MilitaryMissionKillCheck>,
    pending_level_achievements: Vec<LevelAchievementCheck>,
    pending_lq_npc_spawns: Vec<LqNpcSpawnRequest>,
    /// C `cmd_wimp`'s `ppd->last_lq_death = realtime;` write (`lq.c:2332`,
    /// `world::lq_usurp`) - needs `PlayerRuntime`, drained by
    /// `ugaris-server`'s area 20 glue. Holds character ids, not
    /// timestamps; the server stamps `current_realtime_seconds()` itself
    /// at drain time (same "server owns the clock" precedent as every
    /// other `realtime_seconds` parameter threaded in from `ugaris-server`).
    pending_lq_wimps: Vec<CharacterId>,
    pending_look_maps: Vec<LookMapRequest>,
    pending_sound_specials: Vec<WorldSoundSpecial>,
    pending_player_specials: Vec<WorldPlayerSpecial>,
    pending_system_texts: Vec<WorldSystemText>,
    pending_system_text_bytes: Vec<WorldSystemTextBytes>,
    pending_area_texts: Vec<WorldAreaText>,
    pending_area_text_bytes: Vec<WorldAreaTextBytes>,
    pending_channel_broadcasts: Vec<WorldChannelBroadcast>,
    pending_hurt_events: Vec<LegacyHurtEvent>,
    /// `CharacterId`s that took nonzero hp damage this call while
    /// `CF_PLAYER`+`CDR_LOSTCON`, matching C `death.c:1214`'s
    /// `(ch[cn].flags & CF_PLAYER) && ch[cn].driver == CDR_LOSTCON` gate
    /// around `player_use_potion`/`player_use_recall`. `World` has no
    /// access to the session-owned `PlayerRuntime` that holds the `no*`
    /// toggles those two C functions read, so - unlike every other
    /// `pending_*` queue in this list, which is drained and fully handled
    /// inside `ugaris-core` - this one is drained by `ugaris-server`
    /// (`World::drain_lostcon_hurt_events`), which calls back into
    /// `World::process_player_use_potion`/`process_player_use_recall`
    /// with suppressions built from the stashed `PlayerRuntime` for each
    /// currently-lingering character. See `world/lostcon.rs`'s module doc
    /// comment for the resulting (disclosed) ordering deviation from C:
    /// this reacts once per tick to whatever damage accumulated by the
    /// time `ugaris-server`'s per-tick lostcon block runs, not literally
    /// inline between the sound-effect check and the death-threshold
    /// check of the exact same `hurt()` call C calls it from.
    pending_lostcon_hurt_events: Vec<CharacterId>,
    /// `CharacterId`s that just gained a positive amount of experience via
    /// [`World::give_exp`] (C `give_exp`'s trailing `if (addedExp > 0)
    /// macro_track_exp_gain(cn)`, `src/system/tool.c:1427-1429`). `World`
    /// has no access to the session-owned `PlayerRuntime` that owns
    /// `MacroPpd::last_exp_gain`, so `ugaris-server`'s
    /// `apply_macro_activity_events`
    /// (`crates/ugaris-server/src/macro_daemon.rs`) drains this and stamps
    /// the matching field, mirroring `pending_lostcon_hurt_events`'s same
    /// architectural gap.
    pending_exp_gain_events: Vec<CharacterId>,
    /// `CharacterId`s that took nonzero combat damage via
    /// [`World::apply_legacy_hurt`] (C `hurt`'s leading `if (dam > 0) {
    /// macro_track_combat(cn); if (cc > 0) macro_track_combat(cc); }`,
    /// `src/system/death.c:1112-1117`) - both the defender and, if
    /// present, the attacker. Drained the same way as
    /// [`Self::pending_exp_gain_events`].
    pending_combat_events: Vec<CharacterId>,
    /// `CharacterId`s whose gold changed by a nonzero amount via
    /// [`World::gate_give_money_silent`] (C `give_money_silent`'s trailing
    /// `if (val != 0) macro_track_gold_change(cn)`, `src/system/tool.c:
    /// 1441-1449`). Drained the same way as
    /// [`Self::pending_exp_gain_events`]. C's other gold-granting entry
    /// point, `give_money`, already runs server-side with direct
    /// `PlayerRuntime` access (`ugaris-server/src/achievement.rs::
    /// give_money`) and stamps `MacroPpd::last_gold_change` inline there
    /// instead of queuing through here.
    pending_gold_change_events: Vec<CharacterId>,
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
    /// Macro-daemon "challenge room" banishments/returns whose
    /// destination area differs from this area server's own `area_id` -
    /// see `world/macro_npc.rs`'s module doc comment.
    pending_macro_cross_area_transfers: Vec<MacroCrossAreaTransfer>,
    /// `gwendylon_driver`'s `IID_CALIGARLETTER` hand-off to area 36 (C
    /// `change_area(co, 36, 240, 10)`, `src/area/1/gwendylon.c:637`) - see
    /// `world/gwendylon.rs`'s module doc comment.
    pending_gwendylon_cross_area_transfers: Vec<GwendylonCrossAreaTransfer>,
    /// `create_lab_exit(co, level)` reward-gate drops queued by every
    /// area-22 lab master's own death hook - see `world/lab.rs`'s module
    /// doc comment.
    pending_lab_exit_spawns: Vec<LabExitSpawnRequest>,
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
    /// `/allow <name>` async DB round trips - see `world/allow.rs`'s
    /// module doc comment.
    pending_allow_requests: Vec<AllowRequest>,
    /// Pentagram activations (`IDR_PENT` `PentagramActivate` outcomes)
    /// queued for `ugaris-server`'s `pents` module to apply the
    /// per-player half of C's reward pipeline - see
    /// [`pents::PentagramActivationEvent`]'s doc comment.
    pending_pentagram_activations: Vec<PentagramActivationEvent>,
    /// Planned pentagram demon spawns (C `spawn_demons_at_pentagram`
    /// calls from `handle_pentagram_interaction`) queued for
    /// `ugaris-server`'s `pents` module to instantiate from `penterN`
    /// zone templates - see [`pents::PentagramDemonSpawnRequest`]'s doc
    /// comment.
    pending_pentagram_demon_spawns: Vec<PentagramDemonSpawnRequest>,
    /// `CharacterId`s of players who just landed the killing blow on a
    /// `CDR_PENTER` demon whose class fell in C's demon-lord power-
    /// reduction range (`handle_demon_death`'s `258..=305`/`404..=411`
    /// gate, `pents.c:1379`) - the `ACHIEVEMENT_DEMON_LORDS_DEMISE`
    /// one-shot award needs the async DB-backed achievement repository
    /// `World` doesn't have, same architectural split as every other
    /// `pending_*_achievement*`-shaped queue.
    pending_penter_demon_lords_demise_awards: Vec<CharacterId>,
    /// `CharacterId`s of players who just landed the killing blow on
    /// `CDR_PALACEISLENA` for the first time (`islena_dead`'s `else`
    /// branch, `src/area/11/palace.c:751-766`) - the `ACHIEVEMENT_
    /// LADYKILLER` one-shot award needs the async DB-backed achievement
    /// repository `World` doesn't have, same architectural split as
    /// `pending_penter_demon_lords_demise_awards` above.
    pending_islena_ladykiller_awards: Vec<CharacterId>,
    /// C `lab3_passguard_driver`'s `static int talk` (`src/area/22/
    /// lab3.c:83`): process-lifetime, not per-character - the *first*
    /// `CDR_LAB3PASSGUARD` ever created (server-wide) latches its own
    /// `dat->talk = 1` forever; every later creation (e.g. a respawn
    /// after death) sees `talk` already `1` and never sets its own fresh
    /// `dat->talk`, so it stays permanently mute. A real, reproduced C
    /// quirk (there is exactly one `lab3.chr` guard instance in the whole
    /// game, so this only matters across that guard's own respawns) - see
    /// `world::npc::area22::lab3_passguard`'s module doc comment.
    pub(crate) lab3_passguard_talk_latched: bool,
}

impl Default for Tick {
    fn default() -> Self {
        Self(0)
    }
}
