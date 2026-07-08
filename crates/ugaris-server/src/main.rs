//! Ugaris area server runtime.
//!
//! `main` owns startup plus the legacy tick loop. Server-side behavior is
//! split by concern: client `login`/`snapshots`, per-session `map_sync` and
//! `effects_sync` caches, `player_actions` queueing, `commands_*` legacy text
//! commands, item-outcome application (`item_apply`, `chests`, `keyring`,
//! `stacks`, `area_apply`, `xmas`, `transport`, `weather`), NPC/`spawns`,
//! `world_events` drains, `depot`/`containers`/`inventory` views, and `zone`
//! startup loading.

mod achievement;
mod area1;
mod area11;
mod area3;
mod area8;
mod area8_army;
mod area_apply;
mod auction;
mod chests;
mod clan_command;
mod clan_log;
mod commands_admin;
mod commands_chat;
mod commands_player;
mod constants;
mod containers;
mod cross_area;
mod depot;
mod dungeon;
mod effects_sync;
mod events;
mod inventory;
mod item_apply;
mod keyring;
mod legacy_backfill;
mod login;
mod loot;
mod lostcon;
mod macro_daemon;
mod map_sync;
mod merchants;
mod military;
mod pents;
mod player_actions;
mod resource_sync;
mod rng;
mod shutdown;
mod snapshots;
mod spawns;
mod stacks;
mod tick_client_actions;
mod tick_item_use_books_potions;
mod tick_item_use_burndown;
mod tick_item_use_caligar;
mod tick_item_use_chests;
mod tick_item_use_clan_lq_arena;
mod tick_item_use_completion;
mod tick_item_use_crafting;
mod tick_item_use_dig_pick;
mod tick_item_use_dungeon;
mod tick_item_use_edemon_fdemon;
mod tick_item_use_ice;
mod tick_item_use_keyassembly;
mod tick_item_use_lab;
mod tick_item_use_minewall;
mod tick_item_use_shrines;
mod tick_item_use_skelraise;
mod tick_item_use_teufel;
mod tick_item_use_transport;
mod tick_item_use_warp;
mod tick_item_use_xmas_swamp;
mod tick_npc;
mod tick_sync;
mod tick_world;
mod transport;
mod tutorial;
mod weather;
mod world_events;
mod xmas;
mod zone;

pub(crate) use achievement::*;
pub(crate) use area1::*;
pub(crate) use area3::*;
pub(crate) use area_apply::*;
pub(crate) use chests::*;
pub(crate) use commands_admin::*;
pub(crate) use commands_chat::*;
pub(crate) use commands_player::*;
pub(crate) use constants::*;
pub(crate) use containers::*;
pub(crate) use cross_area::*;
pub(crate) use depot::*;
// Only consumed by `tests::dungeon` today - `build_warrior`/`build_mage`/
// `build_seyan` aren't wired into any runtime call site yet (see that
// module's doc comment).
#[allow(unused_imports)]
pub(crate) use dungeon::*;
pub(crate) use effects_sync::*;
pub(crate) use inventory::*;
pub(crate) use item_apply::*;
pub(crate) use keyring::*;
pub(crate) use login::*;
pub(crate) use loot::*;
pub(crate) use lostcon::*;
pub(crate) use macro_daemon::*;
pub(crate) use map_sync::*;
pub(crate) use merchants::*;
pub(crate) use military::*;
pub(crate) use player_actions::*;
pub(crate) use resource_sync::*;
pub(crate) use rng::*;
pub(crate) use shutdown::*;
pub(crate) use snapshots::*;
pub(crate) use spawns::*;
pub(crate) use stacks::*;
pub(crate) use transport::*;
pub(crate) use weather::*;
pub(crate) use world_events::*;
pub(crate) use xmas::*;
pub(crate) use zone::*;

#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;

use tokio::{sync::mpsc, time};

use tracing::{debug, info, warn};

use tracing_subscriber::{fmt, EnvFilter};

use ugaris_core::{
    achievement::{
        achievement_def, check_exploration, check_level, check_login_streak, check_profession,
        clear_all, fix_all_stat_thresholds, AccountAchievements, AchievementStats, AchievementType,
        PentArea, ACHIEVEMENT_TYPE_COUNT,
    },
    area_section::{section_at, section_look_text, section_name_by_id},
    area_sound::area_sound_special,
    character_driver::{
        needs_next_lab, CharacterDriverState, CDR_ASTRO1, CDR_ASTRO2, CDR_ASTURIN,
        CDR_BIGBADSPIDER, CDR_BREDEL, CDR_BRITHILDIE, CDR_CALIGARSKELLY, CDR_CAMERON_FORESTMONSTER,
        CDR_CAMHERMIT, CDR_CARLOS, CDR_DUNGEONMASTER, CDR_GATE_FIGHT, CDR_GATE_WELCOME,
        CDR_GREETER, CDR_GUIWYNN, CDR_JAMES, CDR_JESSICA, CDR_KASSIM, CDR_KELLY, CDR_LAB2UNDEAD,
        CDR_LAMPGHOST, CDR_LOGAIN, CDR_LOSTCON, CDR_LQNPC, CDR_LYDIA, CDR_MERCHANT, CDR_NOOK,
        CDR_PALACEISLENA, CDR_RIVERBEAST, CDR_SEYMOUR, CDR_SIMPLEBADDY, CDR_SIRJONES, CDR_SUPERMAX,
        CDR_SWAMPMONSTER, CDR_TERION, CDR_TEUFELRAT, CDR_THOMAS, CDR_VAMPIRE, CDR_VAMPIRE2,
        NTID_GATEKEEPER, NT_NPC,
    },
    clan::{ClanRelations, ClanTreasuryEvent},
    direction::Direction,
    do_action::{
        can_attack_in_area, can_attack_in_area_with_clan_policy, ClanAttackPolicy, ItemUseRequest,
    },
    drvlib::char_dist,
    dungeon_maze::{create_maze, MazeCell, MAZE_XSIZE, MAZE_YSIZE},
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode,
        CHARACTER_VALUE_NAMES, POWERSCALE,
    },
    game_settings::GameSettings,
    game_time::{
        GameDate, DAYS_PER_MOON_CYCLE, DAYS_PER_YEAR, DAY_LEN, FALL_EQUINOX_DAY, HALF_MOON_CYCLE,
        HOUR_LEN, MIN_LEN, SPRING_EQUINOX_DAY, START_TIME, SUMMER_SOLSTICE_DAY,
    },
    ids::{CharacterId, ItemId},
    item_driver::{
        legacy_lucky_die_from_rolls, ForestSpadeFind, IDR_ACCOUNT_DEPOT, IDR_ARKHATA, IDR_BOOKCASE,
        IDR_DECAYITEM, IDR_DEMONCHIP, IDR_DEMONSHRINE, IDR_ENHANCE, IDR_FOOD, IDR_ISLENADOOR,
        IDR_KEY_RING, IDR_LAB2_GRAVE, IDR_MELTINGKEY, IDR_PICKCHEST, IDR_PICKDOOR, IDR_RATCHEST,
        IDR_SPECIAL_POTION, IDR_TORCH, IDR_WARMFIRE, IDR_WARPBONUS, IID_AREA17_LIBRARYKEY,
        IID_AREA17_LOCKPICK, IID_AREA25_DOORKEY, IID_AREA2_SUN1, IID_AREA2_SUN12, IID_AREA2_SUN123,
        IID_AREA2_SUN13, IID_AREA2_SUN2, IID_AREA2_SUN23, IID_AREA2_SUN3, IID_AREA2_ZOMBIESKULL1,
        IID_AREA2_ZOMBIESKULL2, IID_AREA2_ZOMBIESKULL3,
    },
    item_ops::{
        can_use_inventory_slot, consume_item, give_item_to_character, replace_item_in_character,
        GiveItemFlags, GiveItemResult,
    },
    key_registry::{is_registered_key, REGISTERED_KEY_IDS},
    legacy::{action, profession, worn_slot, INVENTORY_START_INVENTORY, SAY_DIST},
    log_text::{
        emote_message, holler_message, sanitize_log_bytes, say_message, shout_message,
        whisper_message,
    },
    map::{MapFlags, MapTile},
    player::{
        CaligarSkellyDeathResult, CommandAlias, DemonShrineResult, IgnoreToggleResult,
        KeyringAddResult, PentagramDebugData, PlayerActionCode, PlayerConnectionState,
        PlayerRuntime, QueuedAction, XmasTreeResult, ARENA_PPD_NEWCOMER_SCORE,
        DEFERRED_ACHIEVEMENTS, DEFERRED_AUCTION, LEGACY_SWEAR_PPD_SIZE, MACRO_HISTORY_SIZE,
        MAX_TUNNEL_LEVEL, MAX_TUNNEL_USES, MILITARY_PPD_MAXADVISOR, MIN_TUNNEL_LEVEL,
        SWEAR_SENTENCE_COUNT, SWEAR_SENTENCE_LEN,
    },
    quest::{QuestLog, QuestReopenResult, QF_OPEN, QLOG_BRITHILDIE},
    spell::{
        is_one_carry_driver, EF_BALL, EF_BLESS, EF_BUBBLE, EF_BURN, EF_CAP, EF_CURSE, EF_EARTHMUD,
        EF_EARTHRAIN, EF_EDEMONBALL, EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE,
        EF_HEAL, EF_LAG, EF_MAGICSHIELD, EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE,
        EF_WARCRY, IDR_ARMOR, IDR_CURSE, IDR_HP, IDR_MANA, IDR_WEAPON,
    },
    tell::tell_not_listening_message,
    text::{
        runtime_color, COL_DARK_GRAY, COL_LIGHT_BLUE, COL_LIGHT_GREEN, COL_LIGHT_RED,
        COL_LIGHT_VIOLET, COL_MAUVE, COL_ORANGE, COL_RESET, COL_VIOLET, COL_YELLOW,
    },
    tick::TICKS_PER_SECOND,
    world::{
        ac_status_string, apply_punishment, apply_unpunishment, army_rank_for_points,
        army_rank_name, compute_paid_till, decode_punishment_note, encode_punishment_note,
        exp2level, legacy_save_number, level2exp, level2maxitem, level_value, merchant_buy_price,
        merchant_sales_price, show_values_lines, values_lines, AcOnlineTarget, ArenaMasterEvent,
        BankEvent, ClanclerkEvent, ClanmasterEvent, ClubmasterEvent, DungeonRaidBuildRequest,
        FirstKillCheck, GateWelcomeOutcomeEvent, GateWelcomePlayerFacts, LegacyHurtEvent,
        LookMapRequest, LootKiller, LootRegistry, MerchantTradeResult, PendingDeathLootRoll,
        PunishmentNote, RaiseSkillOutcome, StealOutcome, StoreWare, TraderEvent, TutorialHintKind,
        TutorialOutcome, TutorialPlayerFacts, WorldActionCompletion, AC_STATUS_FLAGGED,
        AC_STATUS_SUSPICIOUS, AC_STATUS_VERIFIED, MERCHANT_STORE_SIZE, PUNISHMENT_NOTE_KIND,
    },
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
};

use ugaris_db::{
    AntiCheatRepository, AreaRepository, AreaServerRecord, AuctionRepository, CharacterRepository,
    CharacterSaveMode, CharacterSaveRequest, CharacterSnapshot, ClanRegistryRepository,
    LegacyBlobRow, LoginOutcome, LoginRequest, MerchantRepository, MerchantStoreSnapshot,
    MerchantWareSnapshot, MilitaryAdvisorStorageRepository, MilitaryMasterStorageRepository,
    NotesRepository,
};

use ugaris_net::{NetServer, SessionCommand, SessionEvent};

use ugaris_protocol::{
    mod_sfx::{
        sv_sfx_packet, SFX_COLOR_DEFAULT, SFX_COLOR_WHITE, SFX_LIGHTNING_STRIKE, SFX_POS_SCREEN,
        SFX_SCREEN_FLASH,
    },
    mod_weather::{sv_weather_packet, MOD_WEATHER_EFFECT_INDOOR},
    packet::{
        CharacterMapAction, CharacterMapStatus, MapLayer, MapPosition, PacketBuilder,
        CMF_SINK_ANKLE, CMF_SINK_BELLY, CMF_SINK_CHEST, CMF_SINK_KNEE, CMF_TAKE, CMF_UNDERWATER,
        CMF_USE, MAP_CHARACTER_CLEAR, SV_SCROLL_DOWN, SV_SCROLL_LEFT, SV_SCROLL_LEFTDOWN,
        SV_SCROLL_LEFTUP, SV_SCROLL_RIGHT, SV_SCROLL_RIGHTDOWN, SV_SCROLL_RIGHTUP, SV_SCROLL_UP,
    },
    ClientAction, LoginBlock, SpellAction, MAX_LEGACY_TICK_PAYLOAD,
};

#[derive(Debug, Parser)]
#[command(version, about = "Rust Ugaris area server compatibility rewrite")]
struct Args {
    #[arg(long, env = "UGARIS_BIND_ADDR", default_value = "0.0.0.0:5556")]
    bind_addr: SocketAddr,

    // C `server_addr`/`server_port` (`src/system/io.c:433-437`, config-file
    // `inet_addr(ipstring)`): the address *advertised* to other area
    // servers and to clients redirected here via `SV_SERVER`, which is not
    // necessarily the same as `bind_addr` (usually `0.0.0.0` to listen on
    // every interface). Defaults to `bind_addr` for single-host/dev setups
    // where that coincidentally is routable; a real multi-area deployment
    // must set this explicitly per area-server process. See the "Cross-
    // area transfer" design plan in `PORTING_LEDGER.md`.
    #[arg(long, env = "UGARIS_PUBLIC_ADDR")]
    public_addr: Option<SocketAddr>,

    #[arg(short = 'a', long, env = "UGARIS_AREA_ID", default_value_t = 1)]
    area_id: u16,

    #[arg(short = 'm', long, env = "UGARIS_MIRROR_ID", default_value_t = 1)]
    mirror_id: u16,

    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    #[arg(long, env = "UGARIS_ZONE_ROOT")]
    zone_root: Option<PathBuf>,
}

#[derive(Debug)]
struct ServerRuntime {
    players: HashMap<u64, PlayerRuntime>,
    sessions: HashMap<u64, mpsc::Sender<SessionCommand>>,
    map_caches: HashMap<u64, VisibleMapCache>,
    effect_caches: HashMap<u64, ClientEffectCache>,
    account_depots: HashMap<CharacterId, AccountDepotState>,
    merchant_views: HashMap<CharacterId, CharacterId>,
    /// C `kick_player`/`CDR_LOSTCON`: the session-owned `PlayerRuntime` for
    /// a character that lost its connection, stashed here (instead of
    /// dropped) while the character lingers on the map so a reconnect
    /// within `lagout_time` (`lostcon::reclaim_lostcon_on_login`) or the
    /// eventual save-and-despawn (`lostcon::take_expired_lostcon_characters`)
    /// can recover it.
    lostcon_players: HashMap<CharacterId, PlayerRuntime>,
    tick_out: HashMap<u64, Vec<bytes::BytesMut>>,
    staff_codes: HashMap<CharacterId, String>,
    action_queue: VecDeque<(u64, ClientAction)>,
    next_character_id: u32,
    dlight_override: i32,
    show_attack: bool,
    hardcore_kill_exp_bonus: f64,
    xmas_special_override: Option<i32>,
    item_decay_time: i32,
    player_body_decay_time: i32,
    npc_body_decay_time: i32,
    npc_body_decay_time_area32: i32,
    npc_respawn_timer: i32,
    sewer_item_respawn_time: i32,
    lagout_time: i32,
    regen_time: i32,
    holler_dist: i32,
    shout_dist: i32,
    say_dist: i32,
    emote_dist: i32,
    quietsay_dist: i32,
    whisper_dist: i32,
    holler_cost: i32,
    shout_cost: i32,
    weather: WeatherState,
    /// C `src/module/events/events.c`'s recurring boosted-rate event
    /// scheduler state (`events::RecurringEventKind`'s five events).
    recurring_events: events::RecurringEventsState,
    /// C `src/module/events/seasonal/easter_event.c`'s `easter_event`/
    /// `event_data` file-statics.
    easter_event: events::EasterEventState,
    /// C `backup_players`'s static `int n` (`player.c:3707-3721`): a
    /// round-robin cursor over currently-connected players, advanced by
    /// one entry each time a backup save is triggered (`/saveall`,
    /// `command.c:7460-7473`; also the periodic 85s `maintenance_60s_task`
    /// sweep in C, not yet ported here - see `next_backup_rotation_target`).
    /// C indexes into the raw `player[]` connection-slot array in
    /// insertion order; Rust has no equivalent stable slot order, so this
    /// walks a deterministic sort-by-`CharacterId` list instead (a
    /// documented simplification, not a behavioral requirement of the
    /// feature).
    backup_rotation_cursor: usize,
    /// C `shutdown_at` (`server.c:112`): absolute wall-clock second
    /// (`current_realtime_seconds`) the server should exit at, or `0` when
    /// no shutdown is scheduled. Set by `/shutdown` (`shutdown::
    /// apply_shutdown_command`), checked every tick by `shutdown::
    /// tick_shutdown_scheduler`.
    shutdown_at: i64,
    /// C `shutdown_down` (`server.c:112`): the advertised downtime in
    /// minutes, shown in every countdown broadcast.
    shutdown_down_minutes: i64,
    /// C `shutdown_warn`'s `static int shutdown_last` (`system/tool.c:
    /// 3117`): the last remaining-minutes value that was broadcast, so
    /// unchanged minutes don't re-broadcast every tick.
    shutdown_warned_minutes: i64,
    /// C `nologin` (`server.c:112`): blocks new logins (`LoginOutcome::
    /// Shutdown`) once the countdown drops under 3 minutes, or forever
    /// until `/shutdown` schedules or cancels again.
    nologin: bool,
}

impl Default for ServerRuntime {
    fn default() -> Self {
        let settings = GameSettings::default();
        Self {
            players: HashMap::new(),
            sessions: HashMap::new(),
            map_caches: HashMap::new(),
            effect_caches: HashMap::new(),
            account_depots: HashMap::new(),
            merchant_views: HashMap::new(),
            lostcon_players: HashMap::new(),
            tick_out: HashMap::new(),
            staff_codes: HashMap::new(),
            action_queue: VecDeque::new(),
            next_character_id: 0,
            dlight_override: 0,
            show_attack: false,
            hardcore_kill_exp_bonus: settings.hardcore_kill_exp_bonus,
            xmas_special_override: None,
            item_decay_time: settings.item_decay_time,
            player_body_decay_time: settings.player_body_decay_time,
            npc_body_decay_time: settings.npc_body_decay_time,
            npc_body_decay_time_area32: settings.npc_body_decay_time_area32,
            npc_respawn_timer: settings.npc_respawn_timer,
            sewer_item_respawn_time: settings.sewer_item_respawn_time,
            lagout_time: settings.lagout_time,
            regen_time: settings.regen_time,
            holler_dist: settings.holler_dist,
            shout_dist: settings.shout_dist,
            say_dist: settings.say_dist,
            emote_dist: settings.emote_dist,
            quietsay_dist: settings.quietsay_dist,
            whisper_dist: settings.whisper_dist,
            holler_cost: settings.holler_cost,
            shout_cost: settings.shout_cost,
            weather: WeatherState::default(),
            recurring_events: events::RecurringEventsState::default(),
            easter_event: events::EasterEventState::default(),
            backup_rotation_cursor: 0,
            shutdown_at: 0,
            shutdown_down_minutes: 0,
            shutdown_warned_minutes: 0,
            nologin: false,
        }
    }
}

impl ServerRuntime {
    fn connect(
        &mut self,
        session_id: u64,
        commands: mpsc::Sender<SessionCommand>,
        current_tick: u64,
    ) {
        self.sessions.insert(session_id, commands);
        self.players.insert(
            session_id,
            PlayerRuntime::connected(session_id, current_tick),
        );
    }

    fn login(&mut self, session_id: u64, login: &LoginBlock, current_tick: u64) -> CharacterId {
        let new_character_id = self
            .players
            .get(&session_id)
            .and_then(|player| player.character_id)
            .unwrap_or_else(|| self.allocate_character_id());
        let player = self
            .players
            .entry(session_id)
            .or_insert_with(|| PlayerRuntime::connected(session_id, current_tick));
        player.mark_login_parsed(login.client_version, current_tick);
        player.state = PlayerConnectionState::Normal;
        player.character_id = Some(new_character_id);
        player.character_number = new_character_id.0;
        // C `read_login` (`player.c:576-578`) sets `deferred_init =
        // DEFERRED_ACHIEVEMENTS` unconditionally right after login, then
        // (`player.c:618-629`, the `!(ch[cn].flags & CF_AREACHANGE)`
        // branch - a fresh login, as opposed to a cross-area transfer, not
        // yet implemented here, see `login.rs`'s `LoginOutcome::NewArea`
        // comment) also defers a login-time auction delivery notice.
        // `DEFERRED_MOTD` isn't ported yet (see PORTING_TODO.md).
        player.deferred_init |= DEFERRED_ACHIEVEMENTS | DEFERRED_AUCTION;
        new_character_id
    }

    fn disconnect(&mut self, session_id: u64) -> Option<PlayerRuntime> {
        let player = self.players.remove(&session_id);
        self.sessions.remove(&session_id);
        self.map_caches.remove(&session_id);
        self.effect_caches.remove(&session_id);
        self.tick_out.remove(&session_id);
        self.action_queue.retain(|(id, _)| *id != session_id);
        if let Some(player) = &player {
            if let Some(character_id) = player.character_id {
                self.account_depots.remove(&character_id);
                self.weather
                    .elemental_debuff_last_notify
                    .remove(&character_id);
            }
        }
        player
    }

    /// Queue a payload for the session's next tick frame. The legacy client
    /// advances its clock once per received frame, so payloads accumulate
    /// per tick and `flush_session_frames` sends as few frames as possible.
    fn send_to_session(&mut self, session_id: u64, payload: bytes::BytesMut) -> bool {
        if !self.sessions.contains_key(&session_id) {
            return false;
        }
        self.tick_out.entry(session_id).or_default().push(payload);
        true
    }

    fn send_many_to_session(&mut self, session_id: u64, payloads: Vec<bytes::BytesMut>) -> bool {
        payloads
            .into_iter()
            .all(|payload| self.send_to_session(session_id, payload))
    }

    /// Flush one session's buffered payloads as tick frames, greedily packed
    /// under the legacy frame size limit.
    fn flush_session(&mut self, session_id: u64) {
        let payloads = self
            .tick_out
            .get_mut(&session_id)
            .map(std::mem::take)
            .unwrap_or_default();
        if payloads.is_empty() {
            return;
        }
        let Some(commands) = self.sessions.get(&session_id) else {
            return;
        };
        let mut frame = bytes::BytesMut::new();
        for payload in payloads {
            if !frame.is_empty()
                && frame.len() + payload.len() > ugaris_protocol::frame::MAX_LEGACY_TICK_PAYLOAD
            {
                let full = std::mem::take(&mut frame);
                let _ = commands.try_send(SessionCommand::Send(full));
            }
            if payload.len() > ugaris_protocol::frame::MAX_LEGACY_TICK_PAYLOAD {
                // Oversized single payload: send alone (the session layer
                // reports the error); should not happen for tick diffs.
                let _ = commands.try_send(SessionCommand::Send(payload));
                continue;
            }
            frame.extend_from_slice(&payload);
        }
        if !frame.is_empty() {
            let _ = commands.try_send(SessionCommand::Send(frame));
        }
    }

    /// End-of-tick flush: every logged-in session receives exactly one tick
    /// frame (empty when nothing changed) so the lockstep client clock keeps
    /// advancing at the legacy rate. `send_empties` is false for
    /// out-of-tick flushes (session events) to avoid injecting fake ticks.
    fn flush_tick_frames(&mut self, send_empties: bool) {
        let session_ids: Vec<u64> = self.sessions.keys().copied().collect();
        for session_id in session_ids {
            let has_data = self
                .tick_out
                .get(&session_id)
                .is_some_and(|payloads| !payloads.is_empty());
            if has_data {
                self.flush_session(session_id);
            } else if send_empties
                && self
                    .players
                    .get(&session_id)
                    .is_some_and(|player| player.state == PlayerConnectionState::Normal)
            {
                if let Some(commands) = self.sessions.get(&session_id) {
                    let _ = commands.try_send(SessionCommand::Send(bytes::BytesMut::new()));
                }
            }
        }
    }

    fn allocate_character_id(&mut self) -> CharacterId {
        if self.next_character_id == 0 {
            self.next_character_id = 1;
        }
        let id = self.next_character_id;
        self.next_character_id = self.next_character_id.saturating_add(1).max(1);
        CharacterId(id)
    }

    fn set_next_character_id(&mut self, next_character_id: u32) {
        self.next_character_id = next_character_id.max(1);
    }

    fn queue_action(
        &mut self,
        session_id: u64,
        action: ClientAction,
        current_tick: u64,
        characters: &HashMap<CharacterId, Character>,
    ) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.last_command_tick = current_tick;
            apply_player_action(player, &action, current_tick, characters);
        }
        self.action_queue.push_back((session_id, action));
    }

    fn drain_actions_for_tick(&mut self) -> Vec<(u64, ClientAction)> {
        self.action_queue.drain(..).collect()
    }

    fn setup_world_actions(&mut self, world: &mut World, area_id: u16) -> usize {
        let mut count = 0;

        for player in self.players.values_mut() {
            let Some(character_id) = player.character_id else {
                continue;
            };
            player.apply_deferred_fightback(world.tick.0);
            if world
                .characters
                .get(&character_id)
                .is_none_or(|character| character.action != 0)
            {
                continue;
            }
            if world.apply_player_action_setup(player, area_id) {
                count += 1;
            }
        }

        count
    }

    fn sessions_for_character(&self, character_id: CharacterId) -> Vec<(u64, usize)> {
        self.players
            .iter()
            .filter_map(|(session_id, player)| {
                (player.character_id == Some(character_id))
                    .then_some((*session_id, player.view_distance))
            })
            .collect()
    }

    fn refresh_known_character_name(
        &mut self,
        world: &World,
        pk_relations: &PkRelationSnapshot,
        character: &Character,
    ) -> Vec<(u64, bytes::BytesMut)> {
        let character_id = client_character_id(character);
        let mut sessions = Vec::new();
        let viewer_packets: HashMap<u64, bytes::BytesMut> = self
            .players
            .iter()
            .filter_map(|(session_id, player)| {
                let viewer = player
                    .character_id
                    .and_then(|viewer_id| world.characters.get(&viewer_id))?;
                Some((
                    *session_id,
                    character_name_packet_for_viewer(pk_relations, viewer, character),
                ))
            })
            .collect();
        for (session_id, cache) in &mut self.map_caches {
            if cache.known_character_names.contains_key(&character_id) {
                let packet = viewer_packets
                    .get(session_id)
                    .cloned()
                    .unwrap_or_else(|| character_name_packet(character));
                cache
                    .known_character_names
                    .insert(character_id, packet.to_vec());
                sessions.push((*session_id, packet));
            }
        }
        sessions
    }

    fn sessions_for_area_message(
        &self,
        world: &World,
        origin_character_id: CharacterId,
        maxdist: u16,
    ) -> Vec<(u64, CharacterId)> {
        let Some(origin) = world.characters.get(&origin_character_id) else {
            return Vec::new();
        };
        let min_x = origin.x.saturating_sub(maxdist);
        let max_x = origin.x.saturating_add(maxdist);
        let min_y = origin.y.saturating_sub(maxdist);
        let max_y = origin.y.saturating_add(maxdist);

        self.players
            .iter()
            .filter_map(|(session_id, player)| {
                let character_id = player.character_id?;
                let character = world.characters.get(&character_id)?;
                (character.x >= min_x
                    && character.x <= max_x
                    && character.y >= min_y
                    && character.y <= max_y)
                    .then_some((*session_id, character_id))
            })
            .collect()
    }

    fn player_for_character_mut(
        &mut self,
        character_id: CharacterId,
    ) -> Option<&mut PlayerRuntime> {
        self.players
            .values_mut()
            .find(|player| player.character_id == Some(character_id))
    }

    fn player_for_character(&self, character_id: CharacterId) -> Option<&PlayerRuntime> {
        self.players
            .values()
            .find(|player| player.character_id == Some(character_id))
    }

    fn ensure_account_depot(&mut self, character_id: CharacterId) -> &mut AccountDepotState {
        self.account_depots.entry(character_id).or_default()
    }

    /// C `backup_players` (`player.c:3707-3721`): advances the round-robin
    /// cursor by one and returns the next connected player to back up, or
    /// `None` if nobody is currently connected (matching C's `while (n <
    /// MAXPLAYER)` falling through without saving anyone). See the
    /// `backup_rotation_cursor` field doc comment for the deterministic-
    /// sort-order deviation from C's raw connection-slot order.
    fn next_backup_rotation_target(&mut self) -> Option<CharacterId> {
        let mut connected: Vec<CharacterId> = self
            .players
            .values()
            .filter_map(|player| player.character_id)
            .collect();
        if connected.is_empty() {
            return None;
        }
        connected.sort_unstable_by_key(|character_id| character_id.0);
        if self.backup_rotation_cursor >= connected.len() {
            self.backup_rotation_cursor = 0;
        }
        let target = connected[self.backup_rotation_cursor];
        self.backup_rotation_cursor += 1;
        Some(target)
    }
}

use depot::legacy_account_depot_codec::{
    decode_legacy_account_depot_blob, encode_legacy_account_depot_blob,
};

#[cfg(test)]
use depot::legacy_account_depot_codec::{
    encode_legacy_account_depot_item, LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET,
    LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET, LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX,
    LEGACY_ACCOUNT_DEPOT_ITEM_SIZE, LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET,
    LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET, LEGACY_ACCOUNT_DEPOT_NAME_OFFSET,
    LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET, LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET,
    LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET,
};

/// Real wall-clock seconds since the Unix epoch, matching C `time(NULL)`
/// (`time_now = time(NULL);` in `src/server.c:616`, consumed by `tick_date()`
/// immediately afterward).
fn current_unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// Converts an IPv4 address to the raw 32-bit representation C's
/// `inet_addr()` produces (`src/system/io.c:433`), for storage in
/// `area_servers.server_addr`/encoding into an `SV_SERVER` redirect
/// packet. C's value has the first dotted octet at the lowest memory
/// address regardless of host endianness (`inet_addr` builds it that
/// way), which on this codebase's little-endian target is bit-for-bit
/// `u32::from_le_bytes(octets)` - the same convention already used for
/// `LoginBlock::his_ip` (`ugaris-protocol/src/login.rs`), so a value
/// this function returns round-trips through `PacketBuilder::
/// server_redirect` (which writes it back out via `put_u32_le`) with the
/// exact same wire bytes a real C client/server would produce.
fn legacy_ipv4_addr(addr: std::net::Ipv4Addr) -> u32 {
    u32::from_le_bytes(addr.octets())
}

/// C `area_alive(0)` (`src/system/database/database_area.c:31-75`):
/// upserts this area server's `area` (here: `area_servers`) row as alive
/// at the given public address. Shared by the one-time startup call and
/// the periodic re-mark in the game loop below (C re-runs the same
/// function from `maintenance_60s_task` every 85 seconds via
/// `add_scheduled_task(maintenance_60s_task, 85, "Maintenance", true)`)
/// so other area servers' `get_area` lookups (and `read_login`'s
/// cross-area redirect) keep resolving to a live target instead of only
/// a startup snapshot.
async fn mark_area_alive(
    repository: &ugaris_db::PgAreaRepository,
    area_id: u16,
    mirror_id: u16,
    public_addr: std::net::SocketAddr,
) {
    match public_addr.ip() {
        std::net::IpAddr::V4(ipv4) => {
            let server_addr = legacy_ipv4_addr(ipv4) as i32;
            if let Err(err) = repository
                .mark_alive(
                    i32::from(area_id),
                    i32::from(mirror_id),
                    server_addr,
                    i32::from(public_addr.port()),
                )
                .await
            {
                warn!(error = %err, "failed to mark this area server alive");
            }
        }
        std::net::IpAddr::V6(_) => {
            warn!(
                "public-addr is IPv6; the legacy SV_SERVER redirect protocol only carries \
                 an IPv4 address, skipping area-liveness registration"
            );
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let args = Args::parse();

    let config = ServerConfig {
        bind_addr: args.bind_addr,
        area_id: args.area_id,
        mirror_id: args.mirror_id,
        ..ServerConfig::default()
    };

    let (
        character_repository,
        merchant_repository,
        auction_repository,
        achievement_repository,
        clan_repository,
        clan_log_repository,
        military_master_storage_repository,
        military_advisor_storage_repository,
        anticheat_repository,
        notes_repository,
        area_repository,
        pentagram_record_repository,
    ) = if let Some(database_url) = args.database_url.as_deref() {
        let db = ugaris_db::Database::connect(database_url, 8).await?;
        db.ping().await?;
        db.run_migrations().await?;
        info!("connected to PostgreSQL and applied pending migrations");
        let characters = db.characters();
        // "Retire legacy blob writes" (PORTING_TODO.md): decode any
        // pre-migration-0020 rows still carrying only `ppd_blob`/
        // `subscriber_blob` and write their typed `player_state_json`
        // document back, once, before any session can touch them.
        match legacy_backfill::backfill_legacy_player_state(&characters).await {
            Ok(0) => {}
            Ok(count) => {
                info!(
                    count,
                    "backfilled legacy ppd_blob/subscriber_blob rows into player_state_json"
                );
            }
            Err(err) => {
                warn!(error = %err, "failed to backfill legacy player state rows");
            }
        }
        let auctions = db.auctions();
        // C `init_auction_house` (`auction_house.c:37-47`): clean up
        // any auctions that expired while the server was down, before
        // the game loop (and its periodic `update_auction_house`
        // equivalent, below) starts.
        if let Err(err) = auctions.cleanup_expired_auctions().await {
            warn!(error = %err, "failed to clean up expired auctions at startup");
        }
        (
            Some(characters),
            Some(db.merchants()),
            Some(auctions),
            Some(db.achievements()),
            Some(db.clans()),
            Some(db.clan_log()),
            Some(db.military_master_storage()),
            Some(db.military_advisor_storage()),
            Some(db.anticheat()),
            Some(db.notes()),
            Some(db.areas()),
            Some(db.pentagram_record()),
        )
    } else {
        warn!("DATABASE_URL not set; starting without persistence");
        (
            None, None, None, None, None, None, None, None, None, None, None, None,
        )
    };

    let (events_tx, mut events_rx) = mpsc::channel(1024);
    let (listener_ready_tx, listener_ready_rx) = tokio::sync::oneshot::channel();
    let net = NetServer::new(config.bind_addr);
    tokio::spawn(async move {
        if let Err(err) = net.run(events_tx, Some(listener_ready_tx)).await {
            tracing::error!(error = %err, "legacy TCP listener stopped");
        }
    });

    match listener_ready_rx.await {
        Ok(Ok(status)) => {
            info!(addr = %status.bind_addr, listeners = status.listener_count, "legacy TCP listener ready");
        }
        Ok(Err(err)) => {
            anyhow::bail!("legacy TCP listener failed to bind: {err}");
        }
        Err(_) => {
            anyhow::bail!("legacy TCP listener task exited before reporting readiness");
        }
    }

    // C `area_alive(0)` called once before the game loop starts
    // (`server.c:586`), matching `mark_area_alive`'s doc comment above.
    if let Some(repository) = &area_repository {
        let public_addr = args.public_addr.unwrap_or(args.bind_addr);
        mark_area_alive(repository, config.area_id, config.mirror_id, public_addr).await;
    }

    let mut world = World::default();
    world.area_id = config.area_id;
    let mut zone_loader = ZoneLoader::new();
    if let Some(zone_root) = resolve_zone_root(args.zone_root.as_deref()) {
        match load_area_zone(&mut world, &mut zone_loader, &zone_root, config.area_id) {
            Ok(summary) => {
                info!(
                    root = %summary.root.display(),
                    map_file = %summary.map_file.display(),
                    item_templates = summary.item_templates,
                    character_templates = summary.character_templates,
                    skipped_template_files = summary.skipped_template_files,
                    placed_items = summary.placed_items,
                    placed_characters = summary.placed_characters,
                    ground_tiles = summary.ground_tiles,
                    blocked_tiles = summary.blocked_tiles,
                    scheduled_light_timers = summary.scheduled_light_timers,
                    "loaded area zone map"
                );
            }
            Err(err) => {
                warn!(root = %zone_root.display(), area_id = config.area_id, error = %err, "failed to load area zone map; using empty scaffold map");
            }
        }
    } else {
        warn!("zone root not found; using empty scaffold map");
    }
    // C `init_loot` (`server.c:541`): scan `ugaris_data/loot/` for JSON
    // loot tables before any character/loot roll can reference them.
    if let Some(loot_root) = resolve_loot_root(None) {
        let summary = load_loot_tables(&mut world.loot_registry, &loot_root);
        info!(
            root = %loot_root.display(),
            files_scanned = summary.files_scanned,
            tables_added = summary.tables_added,
            warnings = summary.warnings,
            "loaded loot tables"
        );
    } else {
        warn!("loot root not found; loot tables unavailable");
    }
    let spawn_tile = choose_spawn_tile(&world);
    info!(
        x = spawn_tile.0,
        y = spawn_tile.1,
        "selected login spawn tile"
    );
    let mut runtime = ServerRuntime::default();
    let next_character_id = next_available_character_id(&world);
    runtime.set_next_character_id(next_character_id);
    info!(
        next_character_id,
        "initialized scaffold player character id allocator"
    );
    // Restart-persistence for `world.clan_registry` (no C equivalent as a
    // standalone table - see `crates/ugaris-db/src/clan.rs`'s doc comment):
    // load whatever was last saved before the game loop starts, so clan
    // identity/relations survive a restart instead of always starting
    // empty.
    if let Some(repository) = &clan_repository {
        match repository.load_registry().await {
            Ok(Some(loaded)) => {
                info!("loaded clan registry from database");
                world.clan_registry = loaded;
            }
            Ok(None) => {
                info!("no persisted clan registry found; starting with an empty one");
            }
            Err(err) => {
                warn!(error = %err, "failed to load clan registry from database; starting with an empty one");
            }
        }
    }
    // Restart-persistence for `world.military_master_storage` (no C
    // equivalent as a standalone table - see
    // `crates/ugaris-db/src/military.rs`'s doc comment): load whatever
    // was last saved before the game loop starts, so Military Master
    // clan-points/quest-stat counters survive a restart instead of
    // always starting empty.
    if let Some(repository) = &military_master_storage_repository {
        match repository.load_registry().await {
            Ok(loaded) => {
                info!("loaded military master storage registry from database");
                world.military_master_storage = loaded;
            }
            Err(err) => {
                warn!(error = %err, "failed to load military master storage registry from database; starting with an empty one");
            }
        }
    }
    // Restart-persistence for `world.military_advisor_storage` (no C
    // equivalent as a standalone table - see
    // `crates/ugaris-db/src/military.rs`'s doc comment): load whatever
    // was last saved before the game loop starts, so Military Advisor
    // sales-economy counters survive a restart instead of always
    // starting empty.
    if let Some(repository) = &military_advisor_storage_repository {
        match repository.load_registry().await {
            Ok(loaded) => {
                info!("loaded military advisor storage registry from database");
                world.military_advisor_storage = loaded;
            }
            Err(err) => {
                warn!(error = %err, "failed to load military advisor storage registry from database; starting with an empty one");
            }
        }
    }
    // C `initialize_pentagram_system`'s `load_pentagram_record` call
    // (`pents.c:369`): load this area's lifetime pentagram-activation
    // record before the game loop starts, so the very first solve's
    // "you broke the record" comparison sees the persisted value instead
    // of always starting at `0`/`"Nobody"`.
    pents::load_pentagram_record_at_startup(&mut world, &pentagram_record_repository).await;
    let mut tick = time::interval(TickRate::default().interval());
    // C `tick_date()` (`src/system/date.c:267`) runs once before the very
    // first `tick_char()` in the game loop (`src/server.c:618`), so players
    // logging in before the first tick still see a live game clock.
    world.advance_date(
        current_unix_time(),
        config.area_id,
        (runtime.dlight_override != 0).then_some(runtime.dlight_override),
    );
    // C `init_weather` (`src/module/weather/weather.c:204-256`): seed the
    // autonomous cycle's season tracking and first change time from the
    // live game date/tick before the loop starts, so the very first
    // `update_weather_tick` call doesn't see a spurious season change or
    // fire the periodic-change branch instantly (`weather_change_time`
    // defaults to `0`, which is always in the past).
    runtime.weather.seasonal_influence = current_season(&world.date);
    runtime.weather.weather_change_time = world.tick.0
        + WEATHER_DURATION_MIN
        + u64::from(
            runtime_random_below((WEATHER_DURATION_MAX - WEATHER_DURATION_MIN) as i32).max(0)
                as u32,
        );
    info!(
        area_id = config.area_id,
        mirror_id = config.mirror_id,
        "entering Rust game loop"
    );

    loop {
        tokio::select! {
            _ = tick.tick() => {
                tick_world::world_step(
                    &mut world,
                    &mut runtime,
                    &mut zone_loader,
                    &config,
                    &args,
                    &achievement_repository,
                    &character_repository,
                )
                .await;
                tick_client_actions::process_queued_client_actions(
                    &mut world,
                    &mut runtime,
                    &mut zone_loader,
                    &config,
                    &achievement_repository,
                    &character_repository,
                    &area_repository,
                    &clan_log_repository,
                    &merchant_repository,
                    &auction_repository,
                    &pentagram_record_repository,
                )
                .await;
                // C `player_driver.c:1067-1070`'s autobless/autopulse
                // consumer, run for every connected (non-lostcon) player
                // once their previous action has finished
                // (`character.action == 0`, mirroring C's own
                // `char_driver`-is-only-called-when-`ch[n].action` was
                // just reset invocation contract, `act.c:2223-2242`) and
                // before `setup_world_actions` dispatches whatever is
                // queued next - matching C's own ordering, where a
                // successful autobless/autopulse `return`s before the
                // queued-action `switch` ever runs.
                let mut autobless_autopulse_casts = 0;
                for player in runtime.players.values() {
                    let Some(character_id) = player.character_id else {
                        continue;
                    };
                    if !player.autobless_enabled && !player.autopulse_enabled {
                        continue;
                    }
                    if world
                        .characters
                        .get(&character_id)
                        .is_none_or(|character| character.action != 0)
                    {
                        continue;
                    }
                    if world.process_player_autobless_autopulse(
                        character_id,
                        player.autobless_enabled,
                        player.autopulse_enabled,
                    ) {
                        autobless_autopulse_casts += 1;
                    }
                }
                if autobless_autopulse_casts != 0 {
                    info!(autobless_autopulse_casts, tick = world.tick.0, "queued player autobless/autopulse actions");
                }

                // C `tutorial()` (`player_driver.c:402-711`): the newbie
                // in-window hint system, run for every connected player
                // (own internal `ppd->timer` throttle skips players not
                // yet due). See `ugaris_core::world::tutorial`'s module
                // doc comment.
                let tutorial_now = current_unix_time().max(0) as u64;
                let tutorial_facts = tutorial::tutorial_player_facts(&runtime, tutorial_now);
                let tutorial_outcomes = world.process_tutorial_hints(
                    &tutorial_facts,
                    &mut zone_loader,
                    config.area_id,
                    tutorial_now,
                );
                let tutorial_outcomes_applied =
                    tutorial::apply_tutorial_outcomes(&mut runtime, tutorial_outcomes, tutorial_now);
                if tutorial_outcomes_applied != 0 {
                    info!(
                        tutorial_outcomes_applied,
                        tick = world.tick.0,
                        "applied tutorial hint state updates"
                    );
                }

                let setup_count = runtime.setup_world_actions(&mut world, config.area_id);
                if setup_count != 0 {
                    info!(count = setup_count, tick = world.tick.0, "prepared player actions");
                }
                let look_map_requests = world.drain_look_map_requests();
                if !look_map_requests.is_empty() {
                    let mut look_sessions = 0;
                    for request in look_map_requests {
                        let payloads = look_map_payloads(&world, config.area_id, request);
                        for (session_id, _) in runtime.sessions_for_character(request.character_id) {
                            if runtime.send_many_to_session(session_id, payloads.clone()) {
                                look_sessions += 1;
                            }
                        }
                    }
                    info!(look_sessions, tick = world.tick.0, "queued look-map feedback");
                }
                let clan_relations: ClanRelations = world.clan_registry.relations().clone();
                let mut completed_actions = world.tick_basic_actions_with_attack_policy(|caster_id, caster, target, map| {
                    if let Some(player) = runtime.player_for_character_mut(caster_id) {
                        let attack_policy = RuntimePlayerAttackPolicy {
                            attacker_runtime: &*player,
                            clan_relations: &clan_relations,
                        };
                        let can_attack = can_attack_in_area_with_clan_policy(
                            caster,
                            target,
                            map,
                            config.area_id,
                            &attack_policy,
                        );
                        if !can_attack {
                            remove_stale_pvp_hate_if_effect_check_fails(
                                player,
                                caster,
                                target,
                                config.area_id,
                            );
                        }
                        can_attack
                    } else {
                        can_attack_in_area(caster, target, map, config.area_id)
                    }
                });
                tick_item_use_completion::process_completed_action_outcomes(
                    &mut world,
                    &mut runtime,
                    &mut zone_loader,
                    &config,
                    &args,
                    &mut completed_actions,
                    &achievement_repository,
                    &character_repository,
                    &area_repository,
                )
                .await;

                // Per-NPC and queued-event tick passes live in
                // `tick_npc/` (one fn per legacy driver, grouped
                // by area); `run_all` keeps the original order.
                tick_npc::run_all(&mut world, &mut runtime, &mut zone_loader, &config, &args, &completed_actions, &achievement_repository, &character_repository, &area_repository, &clan_repository, &clan_log_repository, &merchant_repository, &military_master_storage_repository, &military_advisor_storage_repository, &notes_repository, &anticheat_repository, &auction_repository).await;

                // C `add_scheduled_task(save_pentagram_record_scheduled,
                // 3600 * 4, "PentagramRecords", 1)`
                // (`database_pent_record.c:128`): re-save this area's
                // lifetime pentagram-activation record every 4 hours.
                if world.tick.0 % (TICKS_PER_SECOND * 3600 * 4) == 0 {
                    pents::save_pentagram_record_scheduled(&world, &pentagram_record_repository)
                        .await;
                }

                // Per-tick sync phase (PK hate updates, shutdown scheduler,
                // pending text/channel broadcast drains, resource sync,
                // periodic map/action diffs, final frame flush) lives in
                // `tick_sync::sync_phase`; it returns whether the scheduled
                // shutdown time was reached this tick.
                if tick_sync::sync_phase(&mut world, &mut runtime, &zone_loader) {
                    break;
                }
            }
            Some(event) = events_rx.recv() => {
                match event {
                    SessionEvent::Connected { id, peer_addr, commands } => {
                        runtime.connect(id.0, commands, world.tick.0);
                        info!(%id, %peer_addr, "session registered");
                    }
                    SessionEvent::Login { id, login } => {
                        let mut character_id = runtime.login(id.0, &login, world.tick.0);
                        if let Some(player) = runtime.players.get_mut(&id.0) {
                            player.set_current_mirror(u32::from(config.mirror_id));
                        }
                        let mut loaded_from_database = false;
                        // C `read_login` (`src/system/player.c:396-444`): a
                        // non-`Ready` `find_login` result rejects the
                        // connection with the matching legacy `SV_EXIT`
                        // reason instead of continuing to a scaffold spawn.
                        let mut login_reject: Option<&'static str> = None;
                        if let Some(repository) = &character_repository {
                            let request = LoginRequest {
                                name: login.name.clone(),
                                password: login.password.clone(),
                                vendor: login.vendor,
                                unique: login.unique,
                                ip: login.his_ip,
                                area_id: i32::from(config.area_id),
                                mirror_id: i32::from(config.mirror_id),
                                // C `nologin` (`server.c:112`), set by
                                // `/shutdown`'s countdown crossing under 3
                                // minutes (`shutdown::tick_shutdown_
                                // scheduler`) or by an already-pending
                                // shutdown - rejects new logins with
                                // `LoginOutcome::Shutdown`.
                                no_login: runtime.nologin,
                            };
                            match repository.begin_login(request).await {
                                Ok(LoginOutcome::Ready { character_id: db_character_id, character_number, mirror, login_session_id, account_id, .. }) => {
                                    character_id = db_character_id;
                                    if let Some(player) = runtime.players.get_mut(&id.0) {
                                        player.character_id = Some(db_character_id);
                                        player.character_number = if character_number == 0 { db_character_id.0 } else { character_number };
                                        player.set_current_mirror(mirror.max(0) as u32);
                                    }
                                    // C `ac_player_login`
                                    // (`src/module/anticheat/anticheat.c:
                                    // 173-220`): create the anti-cheat
                                    // session as soon as the character
                                    // identity is known. Only the session-
                                    // lifecycle half is ported here
                                    // (creation on login, `end_session` on
                                    // disconnect below) - the detection
                                    // engine (heartbeat/state/challenge/
                                    // anomaly subsystems) that would
                                    // populate `bot_score`/violation
                                    // counters is not ported, so every
                                    // session is created and closed with
                                    // every counter at its SQL default (0).
                                    if let Some(repository) = &anticheat_repository {
                                        match repository
                                            .create_session(ugaris_db::AntiCheatSessionCreate {
                                                login_session_id: Some(login_session_id),
                                                account_id: Some(account_id),
                                                character_id: Some(db_character_id),
                                                ip_address: login.his_ip as i32,
                                                area_id: i32::from(config.area_id),
                                            })
                                            .await
                                        {
                                            Ok(session_id) => {
                                                if let Some(player) = runtime.players.get_mut(&id.0) {
                                                    player.anticheat_session_id = Some(session_id);
                                                }
                                            }
                                            Err(err) => {
                                                warn!(%id, character_id = db_character_id.0, error = %err, "failed to create anti-cheat session");
                                            }
                                        }
                                    }
                                    // C `tick_login()`
                                    // (`database_character.c:1164`): a
                                    // character still loaded in memory under
                                    // `CDR_LOSTCON` is reclaimed in place
                                    // instead of being re-read from the
                                    // (stale, pre-disconnect) database
                                    // snapshot.
                                    let reclaim_tick = world.tick.0;
                                    if reclaim_lostcon_on_login(&mut world, &mut runtime, id.0, db_character_id, reclaim_tick) {
                                        loaded_from_database = true;
                                        info!(%id, character_id = db_character_id.0, "reclaimed lostcon-lingering character on reconnect");
                                    } else {
                                        match repository.load_character_snapshot(db_character_id).await {
                                            Ok(Some(snapshot)) => {
                                                if let Some(player) = runtime.players.get_mut(&id.0) {
                                                    let login_realtime_seconds =
                                                        world.tick.0 / TICKS_PER_SECOND;
                                                    let snapshot_result = apply_character_snapshot(
                                                        &mut world,
                                                        player,
                                                        snapshot,
                                                        spawn_tile.0,
                                                        spawn_tile.1,
                                                        login_realtime_seconds,
                                                    );
                                                    loaded_from_database = snapshot_result.loaded;
                                                    if let Some(account_depot) = snapshot_result.account_depot {
                                                        runtime.account_depots.insert(db_character_id, account_depot);
                                                    }
                                                }
                                                if loaded_from_database {
                                                    info!(%id, character_id = db_character_id.0, mirror, "loaded DB-backed character snapshot");
                                                }
                                            }
                                            Ok(None) => {
                                                warn!(%id, character_id = db_character_id.0, "DB login succeeded but no character snapshot was available; using scaffold");
                                            }
                                            Err(err) => {
                                                warn!(%id, character_id = db_character_id.0, error = %err, "failed to load DB character snapshot; using scaffold");
                                            }
                                        }
                                    }
                                }
                                // C `read_login`'s area-routing branch
                                // (`src/system/player.c:445-465`): if the
                                // target area server is registered and
                                // alive, redirect the client there via
                                // `SV_SERVER` (C `player_to_server`)
                                // instead of always falling through to the
                                // "target area server is down" reject text
                                // (which stays the fallback for a genuinely
                                // unregistered/offline target, matching
                                // C's own `else` branch).
                                Ok(outcome @ LoginOutcome::NewArea { area_id: target_area_id, mirror: target_mirror, .. }) => {
                                    let mut redirect_payload = None;
                                    if let Some(area_repo) = &area_repository {
                                        match area_repo.get_area(target_area_id, target_mirror).await {
                                            Ok(record) => redirect_payload = area_redirect_payload(record.as_ref()),
                                            Err(err) => {
                                                warn!(%id, area_id = target_area_id, mirror = target_mirror, error = %err, "failed to look up target area server for cross-area login redirect");
                                            }
                                        }
                                    }
                                    if let Some(payload) = redirect_payload {
                                        runtime.send_to_session(id.0, payload);
                                        runtime.flush_session(id.0);
                                        if let Some(commands) = runtime.sessions.get(&id.0) {
                                            let _ = commands.try_send(SessionCommand::Disconnect);
                                        }
                                        info!(%id, name = %login.name, area_id = target_area_id, mirror = target_mirror, "redirecting login to target area server");
                                        continue;
                                    }
                                    login_reject = login_reject_message(&outcome);
                                    warn!(%id, code = outcome.legacy_find_login_code(), reject = login_reject.is_some(), "target area server not registered or offline; rejecting cross-area login");
                                }
                                Ok(outcome) => {
                                    login_reject = login_reject_message(&outcome);
                                    warn!(%id, code = outcome.legacy_find_login_code(), reject = login_reject.is_some(), "DB login did not return a local ready character");
                                }
                                Err(err) => {
                                    login_reject = Some(LOGIN_REJECT_INTERNAL_ERROR);
                                    warn!(%id, error = %err, "DB login failed; rejecting connection");
                                }
                            }
                        }
                        if let Some(reason) = login_reject {
                            // C `player_client_exit` (`src/system/player.c:260`):
                            // send `SV_EXIT` with the reject text, then drop
                            // the connection (`ST_EXIT`) instead of spawning
                            // any character.
                            let mut builder = PacketBuilder::new();
                            builder.exit(reason);
                            runtime.send_to_session(id.0, builder.into_payload());
                            runtime.flush_session(id.0);
                            if let Some(commands) = runtime.sessions.get(&id.0) {
                                let _ = commands.try_send(SessionCommand::Disconnect);
                            }
                            info!(%id, name = %login.name, reason, "login rejected");
                            continue;
                        }
                        let reclaim_tick = world.tick.0;
                        if !loaded_from_database
                            && reclaim_lostcon_on_login(&mut world, &mut runtime, id.0, character_id, reclaim_tick)
                        {
                            // No DB repository configured: still honor an
                            // in-memory `CDR_LOSTCON` reclaim so the
                            // scaffold path matches C's `tick_login` reclaim
                            // instead of falling through to a fresh spawn.
                            loaded_from_database = true;
                            info!(%id, character_id = character_id.0, "reclaimed lostcon-lingering character on reconnect (no DB repository)");
                        }
                        if !loaded_from_database && !world.characters.contains_key(&character_id) {
                            let (character, inventory_items) = login_character_from_template(
                                &mut zone_loader,
                                character_id,
                                &login,
                                config.area_id,
                                spawn_tile.0,
                                spawn_tile.1,
                            )
                            .unwrap_or_else(|err| {
                                warn!(template = DEFAULT_PLAYER_TEMPLATE, error = %err, "failed to instantiate player template; using hard-coded login scaffold");
                                (
                                    login_character(
                                        character_id,
                                        &login,
                                        config.area_id,
                                        spawn_tile.0,
                                        spawn_tile.1,
                                    ),
                                    Vec::new(),
                                )
                            });
                            if !world.spawn_character(character, spawn_tile.0, spawn_tile.1) {
                                warn!(%id, ?character_id, "failed to spawn login character");
                            } else {
                                for item in inventory_items {
                                    world.add_item(item);
                                }
                                // C `login_ok` (`database_character.c:1512`):
                                // `update_char(cn)` once the new character's
                                // starting equipment is in place.
                                world.update_character(character_id);
                            }
                        }
                        // C `login_ok` (`src/system/player.c:659`):
                        // `questlog_init(cn)` runs unconditionally on every
                        // successful login (new character, DB-loaded, or
                        // reclaimed lostcon), lazily seeding any area's
                        // questlog entries that haven't been initialized yet.
                        // Idempotent via the `quest[MAXQUEST-1].done == 55`
                        // sentinel, so calling it every login (not just the
                        // very first) matches C exactly.
                        if let Some(player) = runtime.players.get_mut(&id.0) {
                            player.init_questlog();
                        }
                        let view_distance = runtime
                            .players
                            .get(&id.0)
                            .map(|player| player.view_distance)
                            .unwrap_or(ugaris_core::legacy::DIST_OLD);
                        let pk_relations = PkRelationSnapshot::from_runtime(&runtime);
                        let payloads = world
                            .characters
                            .get(&character_id)
                            .map(|character| {
                                runtime.map_caches.insert(
                                    id.0,
                                    visible_map_cache(&world, character, &pk_relations, view_distance),
                                );
                                login_bootstrap_payloads(
                                    &world,
                                    character,
                                    &pk_relations,
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
                                    runtime.effect_caches.entry(id.0).or_default(),
                                    &runtime.weather,
                                )
                            })
                            .unwrap_or_else(|| {
                                let fallback_character = login_character(
                                    character_id,
                                    &login,
                                    config.area_id,
                                    spawn_tile.0,
                                    spawn_tile.1,
                                );
                                runtime.map_caches.insert(
                                    id.0,
                                    visible_map_cache(
                                        &world,
                                        &fallback_character,
                                        &pk_relations,
                                        view_distance,
                                    ),
                                );
                                login_bootstrap_payloads(
                                    &world,
                                    &fallback_character,
                                    &pk_relations,
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
                                    runtime.effect_caches.entry(id.0).or_default(),
                                    &runtime.weather,
                                )
                            });
                        let payload_count = payloads.len();
                        if !runtime.send_many_to_session(id.0, payloads) {
                            warn!(%id, "failed to queue complete login bootstrap for session");
                        }
                        info!(%id, name = %login.name, client_version = ?login.client_version, payload_count, "login accepted by compatibility scaffold");
                    }
                    SessionEvent::Action { id, command_kind, action } => {
                        runtime.queue_action(id.0, action, world.tick.0, &world.characters);
                        info!(%id, command = command_kind, "action queued for gameplay port");
                    }
                    SessionEvent::Disconnected { id } => {
                        let account_depot = runtime
                            .players
                            .get(&id.0)
                            .and_then(|player| player.character_id)
                            .and_then(|character_id| runtime.account_depots.get(&character_id).cloned());
                        if let Some(player) = runtime.disconnect(id.0) {
                            let anticheat_session_id = player.anticheat_session_id;
                            if let Some(character_id) = player.character_id {
                                // C `kick_player`: the character is not
                                // despawned on disconnect. It is detached
                                // from the socket and lingers under
                                // `CDR_LOSTCON` for `lagout_time` ticks
                                // (attackable, reclaimable on reconnect)
                                // before the tick loop's expiry check saves
                                // and removes it.
                                let lagout_time = runtime.lagout_time;
                                let current_tick = world.tick.0;
                                if let Some(leftover_player) = enter_lostcon_on_disconnect(
                                    &mut world,
                                    &mut runtime,
                                    character_id,
                                    player,
                                    account_depot.clone(),
                                    current_tick,
                                    lagout_time,
                                ) {
                                    if let Some(repository) = &character_repository {
                                        if let Some(character) = world.characters.get(&character_id) {
                                            let request = character_save_request(
                                                &world,
                                                &leftover_player,
                                                character,
                                                account_depot.as_ref(),
                                                config.area_id,
                                                config.mirror_id,
                                            );
                                            match repository.save_character_snapshot(request).await {
                                                Ok(true) => {
                                                    info!(%id, character_id = character_id.0, "saved DB-backed character snapshot on logout");
                                                }
                                                Ok(false) => {
                                                    warn!(%id, character_id = character_id.0, "DB character snapshot save was skipped by area guard");
                                                }
                                                Err(err) => {
                                                    warn!(%id, character_id = character_id.0, error = %err, "failed to save DB-backed character snapshot on logout");
                                                }
                                            }
                                        }
                                    }
                                    world.remove_character(character_id);
                                } else {
                                    info!(%id, character_id = character_id.0, "character entered lostcon linger on disconnect");
                                }
                            }
                            // C `ac_player_disconnect`
                            // (`src/module/anticheat/anticheat.c:87-172`):
                            // ends the anti-cheat session tied to the
                            // physical connection unconditionally on
                            // disconnect, independent of whether the
                            // character itself lingers under `CDR_LOSTCON`
                            // (a still-online character that reconnects on
                            // a new socket gets a brand-new anti-cheat
                            // session at the login site instead, matching
                            // `PlayerRuntime::reclaim_for_session` clearing
                            // the old id). Only the session lifecycle +
                            // lifetime-rollup halves are ported - no bot-
                            // score/violation summary exists yet (no
                            // detection engine ported), so the final
                            // `bot_score` is always 0.0 and the rollup
                            // below always accumulates zero-valued
                            // counters, exactly like the session row it
                            // reads from.
                            if let (Some(repository), Some(session_id)) =
                                (&anticheat_repository, anticheat_session_id)
                            {
                                // C reads `player[nr]->ac`'s fields before
                                // `db_ac_session_end`/`db_ac_update_player_
                                // stats` touch anything; this port takes
                                // the same pre-mutation snapshot via
                                // `find_session` (the row `#acstatus`
                                // already reads) before ending the session.
                                let session_info =
                                    repository.find_session(session_id).await.unwrap_or(None);
                                if let Err(err) = repository.end_session(session_id, 0.0).await {
                                    warn!(%id, session_id, error = %err, "failed to end anti-cheat session");
                                }
                                if let Some(info) = session_info {
                                    match repository.account_id_for_session(session_id).await {
                                        Ok(Some(subscriber_id)) => {
                                            if let Err(err) = repository
                                                .update_player_stats(
                                                    subscriber_id,
                                                    info.bot_score,
                                                    info.status,
                                                    info.heartbeat_violations,
                                                    info.state_violations,
                                                    info.challenge_failures,
                                                    0,
                                                )
                                                .await
                                            {
                                                warn!(%id, session_id, error = %err, "failed to update anti-cheat player stats");
                                            }
                                        }
                                        Ok(None) => {}
                                        Err(err) => {
                                            warn!(%id, session_id, error = %err, "failed to resolve anti-cheat subscriber id");
                                        }
                                    }
                                }
                            }
                        }
                        info!(%id, "session removed");
                    }
                }
                // Session events (login bootstrap, etc.) flush their
                // buffered payloads immediately instead of waiting a tick.
                runtime.flush_tick_frames(false);
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown requested");
                break;
            }
        }
    }

    // C `shutdown_auction_house` (`auction_house.c:1334-1340`): one last
    // expired-auction sweep before exit.
    if let Some(repository) = &auction_repository {
        if let Err(err) = repository.cleanup_expired_auctions().await {
            warn!(error = %err, "failed to clean up expired auctions at shutdown");
        }
    }

    // C `area_alive(1)` (`database_area.c:31-75`, called from the exit
    // path): mark this area server's row down so `get_area`/the cross-area
    // login redirect stop pointing other servers/clients at a server that
    // is about to stop accepting connections.
    if let Some(repository) = &area_repository {
        if let Err(err) = repository
            .mark_down(i32::from(config.area_id), i32::from(config.mirror_id))
            .await
        {
            warn!(error = %err, "failed to mark this area server down at shutdown");
        }
    }

    Ok(())
}
