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
mod login;
mod loot;
mod lostcon;
mod macro_daemon;
mod map_sync;
mod merchants;
mod military;
mod player_actions;
mod resource_sync;
mod rng;
mod shutdown;
mod snapshots;
mod spawns;
mod stacks;
mod tick_client_actions;
mod tick_item_use_chests;
mod tick_item_use_dungeon;
mod tick_item_use_ice;
mod tick_item_use_teufel;
mod tick_item_use_warp;
mod tick_npc;
mod tick_sync;
mod tick_world;
mod transport;
mod weather;
mod world_events;
mod xmas;
mod zone;

pub(crate) use achievement::*;
pub(crate) use area1::*;
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
        needs_next_lab, CharacterDriverState, CDR_BIGBADSPIDER, CDR_BREDEL, CDR_CALIGARSKELLY,
        CDR_CAMERON_FORESTMONSTER, CDR_DUNGEONMASTER, CDR_GATE_FIGHT, CDR_GATE_WELCOME,
        CDR_LAB2UNDEAD, CDR_LOSTCON, CDR_LQNPC, CDR_MERCHANT, CDR_PALACEISLENA, CDR_RIVERBEAST,
        CDR_SIMPLEBADDY, CDR_SWAMPMONSTER, CDR_TEUFELRAT, NTID_GATEKEEPER, NT_NPC,
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
        IID_AREA17_LOCKPICK, IID_AREA25_DOORKEY, IID_AREA2_ZOMBIESKULL1, IID_AREA2_ZOMBIESKULL2,
        IID_AREA2_ZOMBIESKULL3,
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
        PunishmentNote, RaiseSkillOutcome, StealOutcome, StoreWare, TraderEvent,
        WorldActionCompletion, AC_STATUS_FLAGGED, AC_STATUS_SUSPICIOUS, AC_STATUS_VERIFIED,
        MERCHANT_STORE_SIZE, PUNISHMENT_NOTE_KIND,
    },
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
};

use ugaris_db::{
    AntiCheatRepository, AreaRepository, AreaServerRecord, AuctionRepository, CharacterRepository,
    CharacterSaveMode, CharacterSaveRequest, CharacterSnapshot, ClanRegistryRepository,
    LoginOutcome, LoginRequest, MerchantRepository, MerchantStoreSnapshot, MerchantWareSnapshot,
    MilitaryAdvisorStorageRepository, MilitaryMasterStorageRepository, NotesRepository,
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
    ) = if let Some(database_url) = args.database_url.as_deref() {
        let db = ugaris_db::Database::connect(database_url, 8).await?;
        db.ping().await?;
        db.run_migrations().await?;
        info!("connected to PostgreSQL and applied pending migrations");
        let auctions = db.auctions();
        // C `init_auction_house` (`auction_house.c:37-47`): clean up
        // any auctions that expired while the server was down, before
        // the game loop (and its periodic `update_auction_house`
        // equivalent, below) starts.
        if let Err(err) = auctions.cleanup_expired_auctions().await {
            warn!(error = %err, "failed to clean up expired auctions at startup");
        }
        (
            Some(db.characters()),
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
        )
    } else {
        warn!("DATABASE_URL not set; starting without persistence");
        (
            None, None, None, None, None, None, None, None, None, None, None,
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
                if !completed_actions.is_empty() {
                    info!(count = completed_actions.len(), tick = world.tick.0, "completed world actions");
                    let mut auto_keyring_feedback = Vec::new();
                    let mut auto_keyring_added = 0;
                    let mut auto_keyring_kept = 0;
                    let mut auto_keyring_failed = 0;
                    for completion in &completed_actions {
                        if !completion.ok
                            || completion.action_id != ugaris_core::legacy::action::TAKE
                        {
                            continue;
                        }
                        let Some(item_id) = completion.action_item_id else {
                            continue;
                        };
                        let keyring_result = apply_keyring_auto_add_pickup(
                            &mut world,
                            runtime.player_for_character_mut(completion.character_id),
                            completion.character_id,
                            item_id,
                        );
                        // C `act_take` (`act.c:305-327`): the stone-pickup
                        // achievement check only runs when
                        // `keyring_try_auto_add` did NOT consume the item
                        // (that branch `free_item`s it and `return`s early
                        // in C before reaching this check).
                        let stone_check_allowed = !matches!(
                            keyring_result,
                            Some(KeyringAutoAddPickupResult::Added { .. })
                        );
                        match keyring_result {
                            Some(KeyringAutoAddPickupResult::Added { key_name }) => {
                                auto_keyring_feedback.push((
                                    completion.character_id,
                                    format!("{key_name} added to keyring."),
                                ));
                                auto_keyring_added += 1;
                            }
                            Some(KeyringAutoAddPickupResult::Duplicate { key_name }) => {
                                auto_keyring_feedback.push((
                                    completion.character_id,
                                    format!("{key_name} already in keyring, added to inventory."),
                                ));
                                auto_keyring_kept += 1;
                            }
                            Some(KeyringAutoAddPickupResult::Full { key_name }) => {
                                auto_keyring_feedback.push((
                                    completion.character_id,
                                    format!("Keyring full, {key_name} added to inventory."),
                                ));
                                auto_keyring_kept += 1;
                            }
                            Some(
                                KeyringAutoAddPickupResult::MissingPlayer
                                | KeyringAutoAddPickupResult::MissingCursorItem,
                            ) => {
                                auto_keyring_failed += 1;
                            }
                            None => {}
                        }
                        if stone_check_allowed {
                            if let Some(item) = world.items.get(&item_id) {
                                if item.template_id == ugaris_core::item_driver::IID_ALCHEMY_INGREDIENT {
                                    let stone_drdata =
                                        item.driver_data.first().copied().unwrap_or_default();
                                    award_stone_pickup_achievement(
                                        &mut world,
                                        &mut runtime,
                                        &achievement_repository,
                                        completion.character_id,
                                        stone_drdata,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    if !auto_keyring_feedback.is_empty() {
                        let mut feedback_sessions = 0;
                        for (character_id, message) in auto_keyring_feedback {
                            let payload = ugaris_protocol::packet::system_text(&message);
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    feedback_sessions += 1;
                                }
                            }
                        }
                        info!(auto_keyring_added, auto_keyring_kept, auto_keyring_failed, feedback_sessions, tick = world.tick.0, "processed keyring pickup auto-add");
                    }
                    let item_use_requests: Vec<_> = completed_actions
                        .iter()
                        .enumerate()
                        .filter_map(|(index, completion)| {
                            completion.item_use.map(|request| (index, request))
                        })
                        .collect();
                    if !item_use_requests.is_empty() {
                        let mut opened = 0;
                        let mut executed = 0;
                        let mut unsupported = 0;
                        let mut deferred_templates = 0;
                        let mut blocked = 0;
                        let mut failed = 0;
                        let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                        let mut feedback = Vec::new();
                        let mut feedback_bytes = Vec::new();
                        let mut special_feedback = Vec::new();
                        let mut area_feedback = Vec::new();
                        let mut container_refresh = Vec::new();
                        for (completion_index, request) in item_use_requests {
                            let use_character_id = request.character_id;
                            match world.use_item_request(request, true) {
                                Ok(ugaris_core::item_driver::UseItemOutcome::OpenContainer { .. })
                                | Ok(ugaris_core::item_driver::UseItemOutcome::OpenDepot { .. }) => {
                                    if let Some(completion) = completed_actions.get_mut(completion_index) {
                                        completion.legacy_return_code = 1;
                                    }
                                    container_refresh.push(use_character_id);
                                    opened += 1;
                                }
                                Ok(ugaris_core::item_driver::UseItemOutcome::OpenAccountDepot { .. }) => {
                                    if let Some(completion) = completed_actions.get_mut(completion_index) {
                                        completion.legacy_return_code = 1;
                                    }
                                    runtime.ensure_account_depot(use_character_id);
                                    container_refresh.push(use_character_id);
                                    opened += 1;
                                }
                                Ok(ugaris_core::item_driver::UseItemOutcome::Dispatch(request)) => {
                                    let driver = match request {
                                        ugaris_core::item_driver::ItemDriverRequest::Driver { driver, .. } => Some(driver),
                                        ugaris_core::item_driver::ItemDriverRequest::AccountDepot { .. } => None,
                                    };
                                    let is_chest_request = matches!(
                                        request,
                                        ugaris_core::item_driver::ItemDriverRequest::Driver {
                                            driver: ugaris_core::item_driver::IDR_CHEST,
                                            ..
                                        }
                                    );
                                    let request_character_id = match request {
                                        ugaris_core::item_driver::ItemDriverRequest::Driver { character_id, .. }
                                        | ugaris_core::item_driver::ItemDriverRequest::AccountDepot { character_id, .. } => character_id,
                                    };
                                    let driver_context = item_driver_context_for_request(
                                        &world,
                                        runtime.player_for_character(request_character_id),
                                        &request,
                                    );
                                    let outcome = world.execute_item_driver_request_with_context(request, config.area_id, &driver_context);
                                    if let Some(completion) = completed_actions.get_mut(completion_index) {
                                        completion.legacy_return_code = ugaris_core::item_driver::legacy_item_driver_return_code(driver, &outcome);
                                    }
                                    match outcome {
                                        outcome @ (ugaris_core::item_driver::ItemDriverOutcome::ChestTreasure { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::RandomChest { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::RatChest { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChest { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestKeyRequired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestUnknown { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ForestChest { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ForestChestCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ForestChestLocked { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PickChest { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PickChestCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PickChestLocked { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PickChestBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ChestSpawnCheck { .. }) => {
                                            tick_item_use_chests::dispatch_chest_outcome(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                &achievement_repository,
                                                &config,
                                                realtime_seconds,
                                                outcome,
                                                &mut feedback,
                                                &mut executed,
                                                &mut blocked,
                                                &mut failed,
                                            )
                                            .await;
                                        }
                                        outcome @ (ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarmFireCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarmFire { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BackToFire { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::MeltingKeyTick { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorKeyRequired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorBusy { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorRespawning { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorResting { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorTick { .. }) => {
                                            tick_item_use_ice::dispatch_ice_outcome(
                                                &mut world,
                                                &mut zone_loader,
                                                outcome,
                                                &mut feedback,
                                                &mut executed,
                                                &mut blocked,
                                                &mut failed,
                                            );
                                        }
                                        outcome @ (ugaris_core::item_driver::ItemDriverOutcome::DungeonTeleport { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonFake { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonKey { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonKeyCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorMissingKeys { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorTooManyDefenders { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorSolved { .. }) => {
                                            tick_item_use_dungeon::dispatch_dungeon_outcome(
                                                &mut world,
                                                &mut zone_loader,
                                                outcome,
                                                &mut feedback,
                                                &mut executed,
                                                &mut blocked,
                                                &mut failed,
                                            )
                                            .await;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeFind { item_id, character_id, find } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0);
                                            match apply_forest_spade_find(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                character_id,
                                                find,
                                                realtime_seconds,
                                                random_seed,
                                            ) {
                                                ForestSpadeApplyResult::Found { item_name } => {
                                                    feedback.push((character_id, format!("You found a {item_name}.")));
                                                    executed += 1;
                                                }
                                                ForestSpadeApplyResult::FoundMoney { amount } => {
                                                    feedback.push((character_id, format!("You found a Money ({:.2}G).", f64::from(amount) / 100.0)));
                                                    executed += 1;
                                                }
                                                ForestSpadeApplyResult::AlreadyDug => {
                                                    feedback.push((character_id, "You've already dug here. The treasure hasn't regrown yet.".to_string()));
                                                    blocked += 1;
                                                }
                                                ForestSpadeApplyResult::Nothing => {
                                                    feedback.push((character_id, "You dug a nice deep hole but you didn't find anything. Embarrassed you stop digging and fill the hole again.".to_string()));
                                                    blocked += 1;
                                                }
                                                ForestSpadeApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                ForestSpadeApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCollapse { character_id, .. } => {
                                            feedback.push((character_id, "The floor collapses below your feet and you fall...".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeNothing { character_id, .. } => {
                                            feedback.push((character_id, "You dug a nice deep hole but you didn't find anything. Embarrassed you stop digging and fill the hole again.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::JunkpileSearch { item_id, character_id, level } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0);
                                            match apply_junkpile_search(
                                                &mut world,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                level,
                                                random_seed,
                                            ) {
                                                JunkpileApplyResult::Found { .. }
                                                | JunkpileApplyResult::FoundMoney { .. } => {
                                                    feedback.push((character_id, "You found something between all that junk.".to_string()));
                                                    executed += 1;
                                                }
                                                JunkpileApplyResult::Nothing => {
                                                    executed += 1;
                                                }
                                                JunkpileApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                JunkpileApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::JunkpileCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickDoorToggle { character_id, picked_lock, .. } => {
                                            if picked_lock {
                                                feedback.push((character_id, "You pick the lock.".to_string()));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickDoorLocked { character_id, .. } => {
                                            feedback.push((character_id, "The door is locked and you don't have the right key.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BurndownTooHot { character_id, .. } => {
                                            feedback.push((character_id, "It is too hot to touch.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BurndownAlreadyBurned { character_id, .. } => {
                                            feedback.push((character_id, "It was burned down already.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BurndownTouch { character_id, .. } => {
                                            feedback.push((character_id, "You touch the barrel.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BurndownIgnite { character_id, .. } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                player.mark_twocity_burndown_kill();
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BurndownTimerTick { .. } => {
                                            executed += 1;
                                        }
                                        outcome @ (ugaris_core::item_driver::ItemDriverOutcome::TeufelArena { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExit { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaNeedsSuit { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaLevelTooHigh { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentEnhanced { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentBound { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaBusy { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExitLowHealth { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoor { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoHumans { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoBeggars { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorOnlyNobles { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBusy { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestSpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestDestroyed { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestGuarded { .. }) => {
                                            tick_item_use_teufel::dispatch_teufel_outcome(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                outcome,
                                                &mut feedback,
                                                &mut executed,
                                                &mut blocked,
                                                &mut failed,
                                            );
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseDust { item_id, character_id } => {
                                            world.apply_skelraise_dust(item_id);
                                            feedback.push((character_id, "The skeleton crumbles to dust as you touch it.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTouch { character_id, .. } => {
                                            feedback.push((character_id, "You touch the chair.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseRaise { item_id, character_id, cursor_item_id, template } => {
                                            if raise_skeleton_from_template(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                item_id,
                                                character_id,
                                                cursor_item_id,
                                                template,
                                            ) {
                                                feedback.push((character_id, "The skeleton comes to life as you pour the blood over it.".to_string()));
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTimer { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ColorTile { character_id, row, color, .. } => {
                                            let matched = if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                let colors = player.ensure_twocity_goodtile_with(|| {
                                                    runtime_random_below(6) as u8 + 1
                                                });
                                                colors
                                                    .get(usize::from(row))
                                                    .is_some_and(|expected| *expected == color)
                                            } else {
                                                false
                                            };
                                            if matched {
                                                executed += 1;
                                            } else {
                                                if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                    for goodtile in &mut player.twocity_goodtile {
                                                        *goodtile = runtime_random_below(6) as u8 + 1;
                                                    }
                                                }
                                                feedback.push((character_id, "You see colors dancing before your eyes, and you sense that something has changed.".to_string()));
                                                if world.teleport_character_same_area(character_id, 5, 250, true) {
                                                    executed += 1;
                                                } else {
                                                    blocked += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::OrbSpawn { item_id, character_id, anti, special } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0);
                                            match apply_orb_spawn(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                item_id,
                                                character_id,
                                                config.area_id,
                                                realtime_seconds,
                                                anti,
                                                special,
                                                random_seed,
                                            ) {
                                                OrbSpawnApplyResult::Granted { item_name, special } => {
                                                    let prefix = if special { "An extracting" } else { "An" };
                                                    feedback.push((character_id, format!("{prefix} {item_name} was created.")));
                                                    executed += 1;
                                                }
                                                OrbSpawnApplyResult::Cooldown { days_left } => {
                                                    feedback.push((character_id, format!("Nothing happens, days left: {days_left}")));
                                                    blocked += 1;
                                                }
                                                OrbSpawnApplyResult::Nothing => {
                                                    feedback.push((character_id, "Nothing happens.".to_string()));
                                                    blocked += 1;
                                                }
                                                OrbSpawnApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                OrbSpawnApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TorchExtractOrb {
                                            item_id,
                                            character_id,
                                            modifier_slot,
                                            modifier,
                                        } => {
                                            let granted = instantiate_orb_with_modifier(
                                                &mut zone_loader,
                                                character_id,
                                                modifier,
                                            )
                                            .is_some_and(|orb| {
                                                world.apply_torch_extract_orb(
                                                    item_id,
                                                    character_id,
                                                    modifier_slot,
                                                    orb,
                                                )
                                            });
                                            if granted {
                                                executed += 1;
                                            } else {
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::NomadStack { item_id, character_id } => {
                                            match apply_nomad_stack(&mut world, &mut zone_loader, item_id, character_id) {
                                                NomadStackApplyResult::Split { left, right, unit } => {
                                                    feedback.push((character_id, format!("Split into {left} {unit}s and {right} {unit}s.")));
                                                    executed += 1;
                                                }
                                                NomadStackApplyResult::Merged { count, unit } => {
                                                    feedback.push((character_id, format!("{count} {unit}s.")));
                                                    executed += 1;
                                                }
                                                NomadStackApplyResult::CannotSplitOne { unit } => {
                                                    feedback.push((character_id, format!("You cannot split 1 {unit}.")));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::CannotMix => {
                                                    feedback.push((character_id, "You cannot mix those.".to_string()));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::EnhanceNeedsSilver => {
                                                    feedback.push((character_id, "To enhance this item, you need silver.".to_string()));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::EnhanceNeedsGold => {
                                                    feedback.push((character_id, "This item has already been enhanced once. For further enhancements, you need gold.".to_string()));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::EnhanceNotEnough { material, need } => {
                                                    feedback.push((character_id, format!("You do not have enough {material} to enhance this item. You need {need} units.")));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::EnhanceConfirmUnusable => {
                                                    feedback.push((character_id, "Enhancing this item would make it unusable for you. Click again if this is what you want.".to_string()));
                                                    blocked += 1;
                                                }
                                                NomadStackApplyResult::Enhanced { used, target_name } => {
                                                    feedback.push((character_id, format!("You used {used} units to enhance your {target_name}.")));
                                                    executed += 1;
                                                }
                                                NomadStackApplyResult::Bug(message) => {
                                                    feedback.push((character_id, message.to_string()));
                                                    failed += 1;
                                                }
                                                NomadStackApplyResult::MissingPlayer
                                                | NomadStackApplyResult::MissingItem => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TransportOpen { character_id, point, .. } => {
                                            let Some(player) = runtime.player_for_character_mut(character_id) else {
                                                failed += 1;
                                                continue;
                                            };
                                            let newly_seen = if point == ugaris_core::item_driver::LEGACY_TRANSPORT_CLAN_EXIT {
                                                false
                                            } else {
                                                player.touch_transport(point)
                                            };
                                            let seen = player.transport_seen;
                                            if newly_seen {
                                                feedback.push((character_id, "You have reached a new transportation point.".to_string()));
                                            }
                                            let clan_access = transport_clan_access(&world, character_id);
                                            let payload = bytes::BytesMut::from(
                                                &ugaris_protocol::packet::transport(seen, clan_access)[..],
                                            );
                                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                                runtime.send_to_session(session_id, payload.clone());
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TransportInvalid { character_id, point, .. } => {
                                            feedback.push((character_id, format!("Nothing happens - BUG ({point},#1).")));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TransportTravel { character_id, spec, .. } => {
                                            let Some(player) = runtime.player_for_character(character_id) else {
                                                failed += 1;
                                                continue;
                                            };
                                            match apply_transport_travel(&mut world, player, character_id, config.area_id, spec) {
                                                TransportTravelResult::SameArea { mirror, .. } => {
                                                    if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                        player.set_current_mirror(mirror);
                                                    }
                                                    let mut builder = PacketBuilder::new();
                                                    builder.mirror(mirror);
                                                    let payload = builder.into_payload();
                                                    for (session_id, _) in runtime.sessions_for_character(character_id) {
                                                        runtime.send_to_session(session_id, payload.clone());
                                                    }
                                                    executed += 1;
                                                }
                                                TransportTravelResult::CrossArea { area, x, y, mirror } => {
                                                    let transferred = attempt_cross_area_transfer(
                                                        &mut world,
                                                        &mut runtime,
                                                        &character_repository,
                                                        &area_repository,
                                                        config.area_id,
                                                        config.mirror_id,
                                                        character_id,
                                                        area,
                                                        mirror,
                                                        x,
                                                        y,
                                                    ).await;
                                                    if transferred {
                                                        executed += 1;
                                                    } else {
                                                        feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                        blocked += 1;
                                                    }
                                                }
                                                TransportTravelResult::Busy => {
                                                    feedback.push((character_id, "Please try again soon. Target is busy".to_string()));
                                                    blocked += 1;
                                                }
                                                TransportTravelResult::Blocked(message)
                                                | TransportTravelResult::Bug(message) => {
                                                    feedback.push((character_id, message));
                                                    blocked += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExit { character_id, area_id, x, y, .. } => {
                                            if area_id != config.area_id {
                                                let transferred = attempt_cross_area_transfer(
                                                    &mut world,
                                                    &mut runtime,
                                                    &character_repository,
                                                    &area_repository,
                                                    config.area_id,
                                                    config.mirror_id,
                                                    character_id,
                                                    area_id,
                                                    u32::from(config.mirror_id),
                                                    x,
                                                    y,
                                                ).await;
                                                if transferred {
                                                    executed += 1;
                                                } else {
                                                    feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                    blocked += 1;
                                                }
                                            } else {
                                                executed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExitBusy { character_id, .. } => {
                                            feedback.push((character_id, "Please try again soon. Target is busy".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnLevelTooHigh { character_id, .. } => {
                                            feedback.push((character_id, "Thou mayest not use this clan spawner for thy level is too great.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnContested { character_id, .. } => {
                                            feedback.push((character_id, "Thou mayest not use this clan spawner while others can touch it.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnCountdown { character_id, remaining_minutes, freq_hours, god_added, .. } => {
                                            if god_added {
                                                feedback.push((character_id, "A jewel has been added to the clan spawner.".to_string()));
                                            }
                                            feedback.push((character_id, format!(
                                                "{:02}:{:02} to go, about one jewel every {} hours.",
                                                remaining_minutes / 60,
                                                remaining_minutes % 60,
                                                freq_hours
                                            )));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnAward { character_id, level, .. } => {
                                            // C fires the "won a Jewel" broadcast/clan-log
                                            // (`clanmaster.c:1373-1397`) unconditionally, before
                                            // even calling `award_clan_jewel` - it never checks
                                            // that call's return value, so the announcement
                                            // still fires even if item delivery fails (e.g. a
                                            // full inventory).
                                            world.resolve_clan_spawn_jewel_award(character_id, level);
                                            if grant_clan_jewel(&mut world, &mut zone_loader, character_id) {
                                                executed += 1;
                                            } else {
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnTimer { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LqTicker { item_id, schedule_after_ticks } => {
                                            world.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceClosed { character_id, .. } => {
                                            feedback.push((character_id, "No quest is in progress, you may not enter.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceLevelBlocked { character_id, min_level, max_level, .. } => {
                                            feedback.push((character_id, format!("This quest is for levels {min_level} to {max_level}, you may not enter.")));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceUndefined { character_id, .. } => {
                                            feedback.push((character_id, "No entrance defined, bad quest.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LqEntrancePenalty { character_id, remaining_seconds, .. } => {
                                            feedback.push((character_id, format!(
                                                "You may not enter again yet. Your remaining penalty is: {:.2} minutes.",
                                                remaining_seconds as f64 / 60.0
                                            )));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArenaToplist { character_id, .. } => {
                                            // C `toplist_driver` (`arena.c:1045-1087`): top-10 lines,
                                            // a +/-5 window around the reader's own rank, then their
                                            // own score/wins/losses summary line.
                                            if let Some(player) = runtime.player_for_character(character_id) {
                                                let entries = world.arena_toplist_entries();
                                                let lines = ugaris_core::item_driver::arena_toplist_lines(
                                                    &entries,
                                                    player.arena_score(),
                                                    player.arena_wins(),
                                                    player.arena_losses(),
                                                    player.arena_fights(),
                                                );
                                                for line in lines {
                                                    feedback.push((character_id, line));
                                                }
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ZombieShrine { item_id, character_id, shrine_type } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0);
                                            match apply_zombie_shrine(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                shrine_type,
                                                random_seed,
                                                u32::from(args.area_id),
                                            ) {
                                                ZombieShrineApplyResult::Gift(_) => {
                                                    feedback.push((character_id, "You received a gift.".to_string()));
                                                    executed += 1;
                                                }
                                                ZombieShrineApplyResult::Experience(_) => {
                                                    feedback.push((character_id, "You have been blessed with experience.".to_string()));
                                                    executed += 1;
                                                }
                                                ZombieShrineApplyResult::Bonus { message, .. } => {
                                                    feedback.push((character_id, message.to_string()));
                                                    executed += 1;
                                                }
                                                ZombieShrineApplyResult::NeedsOffering(shrine_type) => {
                                                    feedback.push((character_id, zombie_shrine_offering_message(shrine_type).to_string()));
                                                    blocked += 1;
                                                }
                                                ZombieShrineApplyResult::MissingGift => {
                                                    failed += 1;
                                                }
                                                ZombieShrineApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ZombieShrineNeedsOffering { character_id, shrine_type, .. } => {
                                            feedback.push((character_id, zombie_shrine_offering_message(shrine_type).to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineNeedsKey { character_id, .. } => {
                                            feedback.push((character_id, "Nothing happens. You seem to need some kind of magical item to invoke the powers of the shrine.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineAlreadyUsed { character_id, .. } => {
                                            feedback.push((character_id, "The magic of this place will only work once.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineBug { character_id, .. } => {
                                            feedback.push((character_id, "You have found bug #2116a.".to_string()));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineUse { character_id, shrine_type, level, kind, .. } => {
                                            match kind {
                                                ugaris_core::item_driver::RandomShrineKind::Security => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_security(player, character, shrine_type),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineSecurityApplyResult::Used { saves } => {
                                                            feedback.push((character_id, "A scared voice whispers: 'Thou shalt be secure.'".to_string()));
                                                            feedback.push((
                                                                character_id,
                                                                format!(
                                                                    "Thou hast {} save{}.",
                                                                    legacy_save_number(saves),
                                                                    if saves == 1 { "" } else { "s" }
                                                                ),
                                                            ));
                                                            executed += 1;
                                                        }
                                                        RandomShrineSecurityApplyResult::SecureAlready => {
                                                            feedback.push((character_id, "A scared voice whispers: 'Thou art secure already.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                        RandomShrineSecurityApplyResult::Hardcore => {
                                                            feedback.push((character_id, "A scared voice whispers: 'Thou wilt never be secure.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Jobless => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_jobless(player, character, shrine_type),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineJoblessApplyResult::Used => {
                                                            feedback.push((character_id, "A bored voice says: 'Thou shalt be jobless.'".to_string()));
                                                            executed += 1;
                                                        }
                                                        RandomShrineJoblessApplyResult::AlreadyJobless => {
                                                            feedback.push((character_id, "A bored voice says: 'Thou art jobless already.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Edge => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_edge(player, character, shrine_type, level),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineEdgeApplyResult::Used { exp } => {
                                                            // C `shrine_edge` (`random.c:2038`) grants
                                                            // `bonus` via `give_exp(cn, bonus)`.
                                                            world.give_exp(character_id, i64::from(exp), u32::from(args.area_id));
                                                            feedback.push((character_id, "A booming voice declares: 'Living on the edge has its merits - and its dangers!'".to_string()));
                                                            feedback.push((character_id, "Thou hast no saves left.".to_string()));
                                                            executed += 1;
                                                        }
                                                        RandomShrineEdgeApplyResult::AlreadyOnEdge => {
                                                            feedback.push((character_id, "A booming voice declares: 'Thou art living on the edge already!'".to_string()));
                                                            blocked += 1;
                                                        }
                                                        RandomShrineEdgeApplyResult::NoExp => {
                                                            feedback.push((character_id, "A deadly voice says: 'Thou canst live on the edge as long as thou has /noexp turned on.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Kindness => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_kindness(player, character, shrine_type),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineKindnessApplyResult::Used => {
                                                            feedback.push((character_id, "A tender voice whispers: 'Mayest thou find other ways to amuse thyself. Thou art not a killer henceforth.'".to_string()));
                                                            executed += 1;
                                                        }
                                                        RandomShrineKindnessApplyResult::AlreadyKind => {
                                                            feedback.push((character_id, "A tender voice whispers: 'But thou art a kind soul already...'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Death => {
                                                    match runtime.player_for_character_mut(character_id) {
                                                        Some(player) => player.mark_random_shrine_used(shrine_type),
                                                        None => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    }
                                                    if let Some(character) = world.characters.get_mut(&character_id) {
                                                        character.saves = 0;
                                                    }
                                                    feedback.push((character_id, "You hear a manical laugh.".to_string()));
                                                    world.apply_legacy_hurt(character_id, None, i32::MAX / 4, 1, 100, 100);
                                                    executed += 1;
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Vitality => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_vitality(player, character, shrine_type),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineVitalityApplyResult::Used { cost, .. } => {
                                                            // C `shrine_vitality` (`random.c:2109-2110`)
                                                            // grants `cost` via `give_exp(cn, cost)` then
                                                            // `update_char(cn)`.
                                                            world.give_exp(character_id, i64::from(cost), u32::from(args.area_id));
                                                            world.update_character(character_id);
                                                            executed += 1;
                                                        }
                                                        RandomShrineVitalityApplyResult::NoExp => {
                                                            feedback.push((character_id, "A lively voice says: 'Thou canst improve thine vitality any more as long as thou has /noexp turned on.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                        RandomShrineVitalityApplyResult::Capped => {
                                                            feedback.push((character_id, "A lively voice says: 'Thou canst improve thine vitality any more.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Braveness => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_braveness(player, character, shrine_type, level),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineBravenessApplyResult::Used { exp, .. } => {
                                                            // C `shrine_braveness` (`random.c:2193`) grants
                                                            // `cost` via `give_exp(cn, cost)`.
                                                            world.give_exp(character_id, i64::from(exp), u32::from(args.area_id));
                                                            feedback.push((character_id, "A triumphant voice says: 'Thou art brave indeed!'".to_string()));
                                                            executed += 1;
                                                        }
                                                        RandomShrineBravenessApplyResult::Coward => {
                                                            feedback.push((character_id, "An insulting voice says: 'Thou art a coward, bother me not!".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Continuity => {
                                                    let result = match (
                                                        runtime.player_for_character_mut(character_id),
                                                        world.characters.get_mut(&character_id),
                                                    ) {
                                                        (Some(player), Some(character)) => apply_random_shrine_continuity(player, character, level),
                                                        _ => {
                                                            failed += 1;
                                                            continue;
                                                        }
                                                    };
                                                    match result {
                                                        RandomShrineContinuityApplyResult::Used { exp, opens_gate } => {
                                                            // C `shrine_continuity` (`random.c:2154`) grants
                                                            // `cost` via `give_exp(cn, cost)` before the
                                                            // level-99 gate teleport.
                                                            world.give_exp(character_id, i64::from(exp), u32::from(args.area_id));
                                                            feedback.push((character_id, "A steady voice says: 'Continuity is power.'".to_string()));
                                                            if opens_gate {
                                                                if world.teleport_character_same_area(character_id, 41, 250, false) {
                                                                    feedback.push((character_id, "Thy continuity has opened a gate...".to_string()));
                                                                } else {
                                                                    feedback.push((character_id, "Target is busy, please try again soon.".to_string()));
                                                                }
                                                            }
                                                            executed += 1;
                                                        }
                                                        RandomShrineContinuityApplyResult::AlreadyVisited { opens_gate } => {
                                                            if opens_gate {
                                                                if world.teleport_character_same_area(character_id, 41, 250, false) {
                                                                    feedback.push((character_id, "Thy continuity has opened a gate...".to_string()));
                                                                } else {
                                                                    feedback.push((character_id, "Target is busy, please try again soon.".to_string()));
                                                                }
                                                            } else {
                                                                feedback.push((character_id, "A steady voice says: 'Thou hast visited me already.'".to_string()));
                                                            }
                                                            blocked += 1;
                                                        }
                                                        RandomShrineContinuityApplyResult::NeedYoungerBrother => {
                                                            feedback.push((character_id, "A steady voice says: 'Thou must visit mine younger brother first.'".to_string()));
                                                            blocked += 1;
                                                        }
                                                    }
                                                }
                                                ugaris_core::item_driver::RandomShrineKind::Dormant => {
                                                    executed += 1;
                                                }
                                                _ => {
                                                    feedback.push((character_id, "Nothing happens.".to_string()));
                                                    blocked += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialShrine { character_id, kind, .. } => {
                                            let result = match (
                                                runtime.player_for_character_mut(character_id),
                                                world.characters.get_mut(&character_id),
                                            ) {
                                                (Some(player), Some(character)) => player.touch_special_shrine(
                                                    character,
                                                    kind,
                                                    realtime_seconds,
                                                ),
                                                _ => {
                                                    failed += 1;
                                                    continue;
                                                }
                                            };
                                            match result {
                                                ugaris_core::player::SpecialShrineResult::NothingHere => {
                                                    feedback.push((character_id, "A mild voice speaks: There is nothing for thee here.".to_string()));
                                                    blocked += 1;
                                                }
                                                ugaris_core::player::SpecialShrineResult::ConfirmRequired => {
                                                    feedback.push((character_id, "A mild voice says: I can remove the perils of living on the edge from thee. If this is your wish, touch me again.".to_string()));
                                                    blocked += 1;
                                                }
                                                ugaris_core::player::SpecialShrineResult::HardcoreRemoved => {
                                                    feedback.push((character_id, "A mild voice speaks: Thou art no longer living on the edge, Ishtar will again save thee when thou art in need. The benefits of a hardcore character shant be thine any more.".to_string()));
                                                    executed += 1;
                                                }
                                                ugaris_core::player::SpecialShrineResult::Unsupported => {
                                                    blocked += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DemonShrine { character_id, location_id, .. } => {
                                            let result = match (
                                                runtime.player_for_character_mut(character_id),
                                                world.characters.get_mut(&character_id),
                                            ) {
                                                (Some(player), Some(character)) => player.touch_demonshrine(
                                                    character,
                                                    location_id,
                                                ),
                                                _ => {
                                                    failed += 1;
                                                    continue;
                                                }
                                            };
                                            match result {
                                                DemonShrineResult::Learned { exp_added } => {
                                                    // C `demonshrine_driver` (`base.c:3231-3235`):
                                                    // `update_char(cn)` after the Demon value bump,
                                                    // then `give_exp(cn, ...)`.
                                                    world.update_character(character_id);
                                                    world.give_exp(character_id, i64::from(exp_added), u32::from(args.area_id));
                                                    feedback.push((character_id, "You study the old book and learn something about the ancient tribes. Your Ancient Knowledge went up by one and you gained experience.".to_string()));
                                                    executed += 1;
                                                }
                                                DemonShrineResult::AlreadyKnown => {
                                                    feedback.push((character_id, "You've been here before. You cannot learn more from this book.".to_string()));
                                                    blocked += 1;
                                                }
                                                DemonShrineResult::Full => {
                                                    feedback.push((character_id, "Bug 771".to_string()));
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::XmasMaker { character_id, .. } => {
                                            if apply_xmasmaker(&mut world, &mut zone_loader, character_id) {
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SwampSpawn { item_id, character_id: _, template, x, y, .. } => {
                                            if spawn_swampspawn_character(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                item_id,
                                                template,
                                                x,
                                                y,
                                            ) {
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SwampSpawnPulse { .. } => {}
                                        ugaris_core::item_driver::ItemDriverOutcome::XmasTree { character_id, .. } => {
                                            let (is_xmas, event_year) = runtime_effective_xmas_event(&runtime);
                                            let gift_seed = world.tick.0;
                                            let result = match runtime.player_for_character_mut(character_id) {
                                                Some(player) => apply_xmastree(
                                                    &mut world,
                                                    &mut zone_loader,
                                                    player,
                                                    character_id,
                                                    args.area_id,
                                                    is_xmas,
                                                    event_year,
                                                    gift_seed,
                                                ),
                                                None => XmasTreeApplyResult::MissingPlayer,
                                            };
                                            match result {
                                                XmasTreeApplyResult::Dormant => {
                                                    feedback.push((character_id, "The tree seems dormant outside the holiday season.".to_string()));
                                                    blocked += 1;
                                                }
                                                XmasTreeApplyResult::AlreadyGranted => {
                                                    feedback.push((character_id, "The tree's magic has already granted you a gift.".to_string()));
                                                    blocked += 1;
                                                }
                                                XmasTreeApplyResult::NeedsHolidayTreat => {
                                                    feedback.push((character_id, "The tree awaits a special holiday treat before bestowing its gift.".to_string()));
                                                    blocked += 1;
                                                }
                                                XmasTreeApplyResult::GiftGranted(item_name) => {
                                                    feedback.push((character_id, format!("The tree glows brightly as you receive a {item_name}!")));
                                                    executed += 1;
                                                }
                                                XmasTreeApplyResult::NoSpace => {
                                                    feedback.push((character_id, "You need more space in your inventory for the gift!".to_string()));
                                                    blocked += 1;
                                                }
                                                XmasTreeApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                            if is_chest_request =>
                                        {
                                            feedback.push((
                                                character_id,
                                                chest_blocked_message(&world, item_id, character_id).to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged {
                                            character_id,
                                            now_on: true,
                                            remaining_off: Some(0),
                                            gates_opened: true,
                                            ..
                                        } => {
                                            if character_id.0 != 0 {
                                                feedback.push((
                                                    character_id,
                                                    "The light has returned to the palace and the gates open.".to_string(),
                                                ));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged {
                                            character_id,
                                            now_on: true,
                                            remaining_off: Some(remaining),
                                            ..
                                        } => {
                                            if character_id.0 != 0 {
                                                feedback.push((character_id, format!("{} remaining", remaining)));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonSwitchStuck {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "The lever seems stuck.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "You need a key to use this door.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLifeless {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "The door won't move. It seems somehow lifeless.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockBlocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "It won't move.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockMove {
                                            ..
                                        } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonTubePulse {
                                            ..
                                        } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderBlocked {
                                            character_id,
                                            reason,
                                            ..
                                        } => {
                                            let text = match reason {
                                                ugaris_core::item_driver::FdemonLoaderBlockReason::CrystalAlreadyPresent => "There is already a crystal, you cannot add another item.",
                                                ugaris_core::item_driver::FdemonLoaderBlockReason::CrystalStuck => "The crystal is stuck.",
                                                ugaris_core::item_driver::FdemonLoaderBlockReason::NeedsCrystal => "Nothing happens.",
                                                ugaris_core::item_driver::FdemonLoaderBlockReason::WrongCrystal => "That doesn't fit.",
                                            };
                                            feedback.push((character_id, text.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonCannonLifeless {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "It seems lifeless.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonLoaderBlocked {
                                            character_id,
                                            reason,
                                            ..
                                        } => {
                                            let text = match reason {
                                                ugaris_core::item_driver::EdemonLoaderBlockReason::CrystalAlreadyPresent => "There is already a crystal, you cannot add another item.",
                                                ugaris_core::item_driver::EdemonLoaderBlockReason::CrystalStuck => "The crystal is stuck.",
                                                ugaris_core::item_driver::EdemonLoaderBlockReason::NeedsCrystal => "Nothing happens.",
                                                ugaris_core::item_driver::EdemonLoaderBlockReason::WrongCrystal => "That doesn't fit.",
                                            };
                                            feedback.push((character_id, text.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmHarvest {
                                            character_id,
                                            template,
                                            ..
                                        } => {
                                            if grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                template.as_str(),
                                            )
                                            .is_some()
                                            {
                                                executed += 1;
                                            } else {
                                                feedback.push((
                                                    character_id,
                                                    format!(
                                                        "BUG # 31992 mark {}",
                                                        template.legacy_number()
                                                    ),
                                                ));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmCursorOccupied {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Please empty your hand (mouse cursor) first."
                                                    .to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmNotReady {
                                            character_id,
                                            current,
                                            required,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                format!(
                                                    "There's nothing to take yet ({} of {}).",
                                                    current, required
                                                ),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmBug {
                                            character_id,
                                            crystal_number,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                format!("BUG # 31992 mark {}", crystal_number),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodBlocked {
                                            character_id,
                                            reason,
                                            ..
                                        } => {
                                            let text = match reason {
                                                ugaris_core::item_driver::FdemonBloodBlockReason::BareHands => "You do not want to touch the liquid with your bare hands.",
                                                ugaris_core::item_driver::FdemonBloodBlockReason::WrongItem => "Hu?",
                                                ugaris_core::item_driver::FdemonBloodBlockReason::ContainerFull => "The container is full already.",
                                            };
                                            feedback.push((character_id, text.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodDestroyedFlask {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "The liquid burns through the flask and shatters it."
                                                    .to_string(),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodFilled {
                                            character_id,
                                            ..
                                        } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.advance_farmy_blood_stage() {
                                                    feedback.push((
                                                        character_id,
                                                        "That's it. Now report to the commander.".to_string(),
                                                    ));
                                                }
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaBlocked {
                                            character_id,
                                            reason,
                                            ..
                                        } => {
                                            let text = match reason {
                                                ugaris_core::item_driver::FdemonLavaBlockReason::BareHands => "You do not want to touch burning lava with your bare hands, do you?",
                                                ugaris_core::item_driver::FdemonLavaBlockReason::WrongItem => "Hu?",
                                                ugaris_core::item_driver::FdemonLavaBlockReason::EmptyContainer => "The container is empty, and it cannot hold lava.",
                                            };
                                            feedback.push((character_id, text.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaActivated {
                                            character_id,
                                            ..
                                        } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.advance_farmy_lava_stage() {
                                                    feedback.push((
                                                        character_id,
                                                        "You got it. Now report to the commander.".to_string(),
                                                    ));
                                                }
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PotionDrunk {
                                            character_id,
                                            ..
                                        } => {
                                            area_feedback.push((
                                                character_id,
                                                potion_area_message(&world, character_id),
                                                10,
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorToggle {
                                            character_id,
                                            key_name: Some(key_name),
                                            locking,
                                            ..
                                        } => {
                                            let action = if locking { "lock" } else { "unlock" };
                                            let key_name = outcome_item_name_text(&key_name);
                                            feedback.push((
                                                character_id,
                                                format!("You use {key_name} to {action} the door."),
                                            ));
                                            executed += 1;
                                        }
                                        outcome @ (ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportMissingSphere { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBusy { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportSpheres { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusFinished { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusAlreadyUsed { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusNeedsSphere { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpBonus { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawnCursorOccupied { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorMissingKey { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoor { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorWrongSide { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBusy { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBug { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoor { .. }) => {
                                            tick_item_use_warp::dispatch_warp_outcome(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                &achievement_repository,
                                                &args,
                                                outcome,
                                                &mut feedback,
                                                &mut feedback_bytes,
                                                &mut executed,
                                                &mut blocked,
                                                &mut failed,
                                            )
                                            .await;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StatScrollUsed {
                                            character_id,
                                            value,
                                            ..
                                        } => {
                                            // C `raise_value_exp` (`src/
                                            // system/skill.c:311-373`,
                                            // called by the `IDR_STAT_SCROLL`
                                            // driver): `if (ch[cn].flags &
                                            // CF_PLAYER) {
                                            // achievement_check_skill(cn, v,
                                            // ch[cn].value[1][v]); }` after
                                            // each successful raise - use the
                                            // post-charge bare value already
                                            // applied to `world.characters`.
                                            if let Some(level) = world
                                                .characters
                                                .get(&character_id)
                                                .map(|character| character.values[1][value as usize])
                                            {
                                                award_skill_achievement(
                                                    &mut world,
                                                    &mut runtime,
                                                    &achievement_repository,
                                                    character_id,
                                                    value as i32,
                                                    level as i32,
                                                )
                                                .await;
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FoodEaten { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoubleDoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FreakDoorUse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Teleport { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeleportDoor { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::MineDoorTeleport { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::MineDoorTimer { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Recall { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::CityRecall { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FireballMachineProjectile { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BallTrapProjectile { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::EdemonBallProjectile { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::EdemonGateSpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FdemonCannonPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FdemonGateSpawn { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderChanged { .. }
                                           | ugaris_core::item_driver::ItemDriverOutcome::FdemonWaypoint { .. }
                                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonLoaderChanged { .. }
                                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmChanged { .. }
                                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaPulse { .. }
                                           | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerExtinguished { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapTriggered { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapReset { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SwampArmPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SwampWhispPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TriggerMapItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StepTrapDiscoverTarget { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderInsertRune { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderRemoveRune { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderActivate { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderExpired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneWallTick { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LightChanged { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceGateTick { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DecayItemToggled { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitAnimating { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitExpired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitUse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BeyondPotion { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::AlchemyFlaskPotion { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::OxygenPotion { .. }
                                          | ugaris_core::item_driver::ItemDriverOutcome::EnchantCursorItem { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::AntiEnchantCursorItem { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoor { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble { final_key: false, .. }
                                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyCombine { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::AccountDepotOpened { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LookItem { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineDoorMissingTarget { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::NomadDice {
                                            item_id,
                                            character_id,
                                            luck,
                                        } => {
                                            if let Some(character) = world.characters.get(&character_id) {
                                                let seed = world
                                                    .tick
                                                    .0
                                                    .wrapping_mul(1_048_573)
                                                    .wrapping_add(u64::from(character_id.0))
                                                    .wrapping_add(u64::from(item_id.0) << 16);
                                                let ([d1, d2, d3], total) = legacy_nomad_dice_roll(seed, luck);
                                                area_feedback.push((
                                                    character_id,
                                                    format!(
                                                        "{} rolled {}, {} and {} for a total of {}.",
                                                        character.name, d1, d2, d3, total
                                                    ),
                                                    8,
                                                ));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LollipopLicked {
                                            character_id,
                                            ..
                                        } => {
                                            area_feedback.push((
                                                character_id,
                                                lollipop_area_message(&world, character_id),
                                                10,
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LollipopMemories {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Ahh memories, sweet memories.".to_string(),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ChristmasPopInspected {
                                            character_id,
                                            ..
                                        } => {
                                            for message in christmas_pop_inspection_messages() {
                                                feedback.push((character_id, message.to_string()));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionDrunk {
                                            character_id,
                                            kind,
                                            ..
                                        } => {
                                            if let Some(message) = special_potion_fun_message(&world, character_id, kind) {
                                                area_feedback.push((character_id, message, 16));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionAntidote {
                                            character_id,
                                            poison_removed,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                if poison_removed {
                                                    "You feel better."
                                                } else {
                                                    "It didn't have any effect."
                                                }
                                                .to_string(),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionInfravision {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Your eyes start to itch.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionSecurity {
                                            character_id,
                                            used,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                if used {
                                                    "You feel secure."
                                                } else {
                                                    "You don't feel like drinking this potion now."
                                                }
                                                .to_string(),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionProfessionReset {
                                            character_id,
                                            used,
                                            ..
                                        } => {
                                            if !used {
                                                feedback.push((
                                                    character_id,
                                                    "You don't feel like drinking this potion now.".to_string(),
                                                ));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionBug {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Please report bug #1734.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BookText {
                                            character_id,
                                            kind,
                                            demon_value,
                                            ..
                                        } => {
                                            let lines = if kind == ugaris_core::item_driver::BOOK_NOOK_JOKES {
                                                ugaris_core::item_driver::book_nook_joke_line_bytes(
                                                    runtime_random_below(5) as u32,
                                                )
                                            } else {
                                                ugaris_core::item_driver::book_text_line_bytes_for_reader_id(
                                                    kind,
                                                    demon_value,
                                                    character_id.0,
                                                )
                                            };
                                            for line in lines {
                                                feedback_bytes.push((character_id, line));
                                            }
                                            if let Some(special_type) =
                                                ugaris_core::item_driver::book_special_effect(kind)
                                            {
                                                special_feedback.push((
                                                    character_id,
                                                    bytes::BytesMut::from(
                                                        &ugaris_protocol::packet::special(
                                                            special_type,
                                                            0,
                                                            0,
                                                        )[..],
                                                    ),
                                                ));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BookcaseText {
                                            character_id,
                                            kind,
                                            ..
                                        } => {
                                            let mut random_index = runtime_random_below(26) as u8;
                                            let mut color = 1;
                                            let mut solved_library = false;
                                            let mut grant_library_exp = false;
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                let colors = player.ensure_twocity_goodtile_with(|| {
                                                    runtime_random_below(6) as u8 + 1
                                                });
                                                color = match kind {
                                                    2..=6 => colors[usize::from(kind - 2)],
                                                    _ => 1,
                                                };
                                                solved_library = player.twocity_solved_library;
                                                if kind == 1 && !player.twocity_solved_library {
                                                    player.twocity_solved_library = true;
                                                    grant_library_exp = true;
                                                }
                                            }
                                            if grant_library_exp {
                                                // C `bookcase` (`area/17/two.c:2622`) grants the
                                                // library-solved exp via `give_exp(cn, ...)`, not a
                                                // raw mutation.
                                                if let Some(level) =
                                                    world.characters.get(&character_id).map(|character| character.level)
                                                {
                                                    let exp_added = ugaris_core::item_driver::bookcase_library_exp(level);
                                                    world.give_exp(
                                                        character_id,
                                                        i64::from(exp_added),
                                                        u32::from(args.area_id),
                                                    );
                                                }
                                            }
                                            if kind != 0 {
                                                random_index = 0;
                                            }
                                            feedback_bytes.push((
                                                character_id,
                                                ugaris_core::item_driver::bookcase_text_line_bytes(
                                                    kind,
                                                    random_index,
                                                    color,
                                                    solved_library,
                                                ),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BookcaseLocked {
                                            character_id,
                                            ..
                                        } => {
                                            for line in ugaris_core::item_driver::bookcase_locked_text_lines() {
                                                feedback.push((character_id, line.to_string()));
                                            }
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferBookText {
                                            character_id,
                                            page,
                                            ..
                                        } => {
                                            if let Some(line) = ugaris_core::item_driver::staffer_book_text(page) {
                                                feedback.push((character_id, line.to_string()));
                                            }
                                            if let Some(line) = ugaris_core::item_driver::staffer_book_continue_text(page) {
                                                feedback.push((character_id, line.to_string()));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferAnimationBook {
                                            character_id,
                                            exp_added,
                                            ..
                                        } => {
                                            let grant_exp = runtime
                                                .player_for_character_mut(character_id)
                                                .map(|player| player.mark_staffer_animation_book_seen())
                                                .unwrap_or(false);
                                            if grant_exp {
                                                // C `staffer_animation_book`
                                                // (`area/29/brannington.c:521`) grants exp via
                                                // `give_exp(cn, ...)`, not a raw mutation.
                                                world.give_exp(
                                                    character_id,
                                                    i64::from(exp_added),
                                                    u32::from(args.area_id),
                                                );
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferMineExhausted {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "You're too exhausted to continue digging.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferBlockBlocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "It won't move.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightBlocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "It won't move.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorLocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "The door is locked.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorBusy {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Please try again soon. Target is busy.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorLocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "The door is locked.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::StafferMineDig { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StafferMineTimer { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockMove { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockTimer { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightMove { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoor { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightTimer { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CaligarGunProjectile { .. }
                                          | ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorToggle { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SaltmineDoorBlocked {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Thou canst not enter there.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SaltmineLadderUse {
                                            character_id,
                                            ladder_index,
                                            ..
                                        } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.saltmine_ladder_ready(ladder_index, realtime_seconds) {
                                                    player.mark_saltmine_ladder_used(ladder_index, realtime_seconds);
                                                    feedback.push((character_id, "Thou signalst the monks to gather salt from this ladder.".to_string()));
                                                    executed += 1;
                                                } else {
                                                    feedback.push((character_id, "Thou already got all the Salt out of this, so thou have to wait until it is refilled again.".to_string()));
                                                    blocked += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::SaltmineSaltbagUse {
                                            character_id,
                                            ..
                                        } => {
                                            if world
                                                .characters
                                                .get(&character_id)
                                                .is_some_and(|character| character.cursor_item.is_some())
                                            {
                                                blocked += 1;
                                                continue;
                                            }
                                            let units = runtime
                                                .player_for_character(character_id)
                                                .map(|player| player.saltmine_pending_salt.saturating_mul(1000))
                                                .unwrap_or(0);
                                            if units == 0 {
                                                feedback.push((character_id, "Thou feelst thou should bring salt to the monastery, before rewarding thinself.".to_string()));
                                                blocked += 1;
                                            } else if grant_salt_to_cursor(&mut world, &mut zone_loader, character_id, units) {
                                                if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                    player.saltmine_pending_salt = 0;
                                                }
                                                feedback.push((character_id, format!("Thou took {units} units of salt, feeling thou have earned it.")));
                                                executed += 1;
                                            } else {
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneHint {
                                            character_id,
                                            level,
                                            nr,
                                            pos,
                                            ..
                                        } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                match player.bone_hint(level, nr, pos, |limit| {
                                                    runtime_random_below(limit as i32).max(0) as u32
                                                }) {
                                                    ugaris_core::player::BoneHintResult::Hint { page, rune, position } => {
                                                        feedback.push((character_id, format!("Rune Diary, Page {page}:")));
                                                        feedback.push((character_id, format!("Used the rune {rune} in the {position} position.")));
                                                    }
                                                    ugaris_core::player::BoneHintResult::Bug { level, nr, pos, value } => {
                                                        feedback.push((character_id, format!("You found bug #197-{level}-{nr}-{pos}-{value}")));
                                                    }
                                                }
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::KeyringShow { character_id, .. } => {
                                            for message in keyring_show_messages(runtime.player_for_character(character_id)) {
                                                feedback.push((character_id, message));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Extinguish {
                                            character_id,
                                            extinguished,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                if extinguished {
                                                    "You extinguish the flames."
                                                } else {
                                                    "Ahh. Sweet and refreshing."
                                                }
                                                .to_string(),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::KeyedDoorToggle {
                                            character_id,
                                            key_id,
                                            source,
                                            locking,
                                            ..
                                        } => {
                                            if source == ugaris_core::item_driver::DoorKeySource::Keyring {
                                                let action = if locking { "lock" } else { "unlock" };
                                                let key_name = driver_context
                                                    .door_key
                                                    .as_ref()
                                                    .map(|key| key.name.as_str())
                                                    .unwrap_or("a key");
                                                feedback.push((
                                                    character_id,
                                                    format!(
                                                        "You use {key_name} (ID: {key_id:08X}) from your keyring to {action} the door."
                                                    ),
                                                ));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::KeyringAddCursorItem { character_id, key_item_id, .. } => {
                                            match apply_keyring_add_cursor_item(
                                                &mut world,
                                                runtime.player_for_character_mut(character_id),
                                                character_id,
                                                key_item_id,
                                            ) {
                                                KeyringAddApplyResult::Added { key_name } => {
                                                    feedback.push((character_id, format!("You add {key_name} to your keyring.")));
                                                    executed += 1;
                                                }
                                                KeyringAddApplyResult::Duplicate => {
                                                    feedback.push((character_id, "This key is already on your keyring.".to_string()));
                                                    blocked += 1;
                                                }
                                                KeyringAddApplyResult::Full => {
                                                    feedback.push((character_id, "Your keyring is full.".to_string()));
                                                    blocked += 1;
                                                }
                                                KeyringAddApplyResult::NotAKey => {
                                                    feedback.push((character_id, "You can only add keys to the keyring.".to_string()));
                                                    blocked += 1;
                                                }
                                                KeyringAddApplyResult::MissingPlayer
                                                | KeyringAddApplyResult::MissingCursorItem => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::AssembleItem {
                                            item_id,
                                            character_id,
                                            cursor_item_id,
                                            template,
                                        } => {
                                            match apply_assemble_item(
                                                &mut world,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                cursor_item_id,
                                                template.as_str(),
                                            ) {
                                                AssembleApplyResult::Assembled => {
                                                    executed += 1;
                                                }
                                                AssembleApplyResult::TemplateUnavailable => {
                                                    feedback.push((character_id, "That doesn't seem to fit.".to_string()));
                                                    blocked += 1;
                                                }
                                                AssembleApplyResult::MissingPlayer
                                                | AssembleApplyResult::MissingItem => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::AssembleNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "You can only use this item with another item.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::AssembleDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "That doesn't seem to fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::AssembleUnknownItem { character_id, .. } => {
                                            feedback.push((character_id, "Bug # 42556".to_string()));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeySplit {
                                            item_id,
                                            character_id,
                                            cursor_part_sprite,
                                            carried_part_sprite,
                                        } => {
                                            match apply_palace_key_split(
                                                &mut world,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                cursor_part_sprite,
                                                carried_part_sprite,
                                            ) {
                                                AssembleApplyResult::Assembled => {
                                                    executed += 1;
                                                }
                                                AssembleApplyResult::TemplateUnavailable => {
                                                    feedback.push((character_id, "That doesn't fit.".to_string()));
                                                    blocked += 1;
                                                }
                                                AssembleApplyResult::MissingPlayer
                                                | AssembleApplyResult::MissingItem => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyNeedsCursor { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "The only thing you can think of to do with this key part is to add another key part to it."
                                                    .to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "That doesn't fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EnchantNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "You have to use another item on this one.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "You can only use this item with another item.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "It doesn't fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "Use, yes, but use it with what?".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "Interesting idea. Really. Doesn't work, though.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGateway { character_id, area_id, x, y, .. } => {
                                            if area_id != config.area_id {
                                                let transferred = attempt_cross_area_transfer(
                                                    &mut world,
                                                    &mut runtime,
                                                    &character_repository,
                                                    &area_repository,
                                                    config.area_id,
                                                    config.mirror_id,
                                                    character_id,
                                                    area_id,
                                                    u32::from(config.mirror_id),
                                                    x,
                                                    y,
                                                ).await;
                                                if transferred {
                                                    executed += 1;
                                                } else {
                                                    feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                    blocked += 1;
                                                }
                                            } else {
                                                executed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayNeedsKey { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "The door won't open. You notice an inscription: \"This door leads to the Dwarven town Grimroot. Only those who have proven their abilities as miners and fighters may enter.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayBug { character_id, x, y, area_id, .. } => {
                                            let name = world
                                                .characters
                                                .get(&character_id)
                                                .map(|character| character.name.as_str())
                                                .unwrap_or("Someone");
                                            feedback.push((
                                                character_id,
                                                format!("{name} touches a teleport object but nothing happens - BUG ({x},{y},{area_id})."),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorNeedsGold { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You'll need to use 2000 gold units as a key to open the door.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorBusy { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You hear fighting noises from behind the door. It won't open while the fight lasts.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "You can only use this item with another item.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "This doesn't seem to fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPool {
                                            character_id,
                                            cursor_item_id,
                                            ..
                                        } => {
                                            match apply_arkhata_pool(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                cursor_item_id,
                                                runtime_random_seed(),
                                            ) {
                                                ArkhataPoolApplyResult::Gift(item_name) => {
                                                    feedback.push((character_id, format!("You got a {}.", item_name)));
                                                    executed += 1;
                                                }
                                                ArkhataPoolApplyResult::Vanished => {
                                                    feedback.push((character_id, "It vanished in the pool. You sense that the idea was right, but more of the same is needed for a result.".to_string()));
                                                    executed += 1;
                                                }
                                                ArkhataPoolApplyResult::MissingGift => {
                                                    failed += 1;
                                                }
                                                ArkhataPoolApplyResult::MissingPlayer
                                                | ArkhataPoolApplyResult::MissingCursor => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "You sense that you have to use the pool with another item (put it on your mouse cursor).".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolWrongCursor { character_id, cursor_item_id, .. } => {
                                            let cursor_name = world.items.get(&cursor_item_id).map(|item| item.name.as_str()).unwrap_or("item");
                                            feedback.push((character_id, format!("Strangely, the {} floats on the surface of the pool. Since nothing happens to it, you take it back.", cursor_name)));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ArkhataStopwatch { character_id, .. } => {
                                            if character_id.0 != 0 {
                                                if let Some(player) = runtime.player_for_character(character_id) {
                                                    let text = arkhata_stopwatch_feedback(player, realtime_seconds);
                                                    feedback.push((character_id, text));
                                                    executed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderBadCursor { character_id, .. } => {
                                            feedback.push((character_id, "That does not fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderOccupied { character_id, .. } => {
                                            feedback.push((character_id, "There is a rune already.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderEmptyTouch { character_id, .. } => {
                                            feedback.push((character_id, "You touch the stand.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderWrongOwner { character_id, .. } => {
                                            feedback.push((character_id, "This rune does not belong to you. You cannot take it.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble {
                                            item_id,
                                            character_id,
                                            cursor_item_id,
                                            final_key: true,
                                            ..
                                        } => {
                                            match apply_caligar_key_final(
                                                &mut world,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                cursor_item_id,
                                            ) {
                                                AssembleApplyResult::Assembled => {
                                                    executed += 1;
                                                }
                                                AssembleApplyResult::TemplateUnavailable => {
                                                    feedback.push((character_id, "This does not seem to fit.".to_string()));
                                                    blocked += 1;
                                                }
                                                AssembleApplyResult::MissingPlayer
                                                | AssembleApplyResult::MissingItem => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "Nothing happens.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "This does not seem to fit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoor {
                                            item_id,
                                            character_id,
                                            door_index,
                                        } => {
                                            if runtime
                                                .player_for_character(character_id)
                                                .is_some_and(|player| player.caligar_skelly_door_unlocked(door_index))
                                            {
                                                match world.apply_caligar_skelly_door(
                                                    item_id,
                                                    character_id,
                                                    door_index,
                                                ) {
                                                    ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoor { .. } => {
                                                        executed += 1;
                                                    }
                                                    ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorBusy { character_id, .. } => {
                                                        feedback.push((character_id, "Please try again soon. Target is busy.".to_string()));
                                                        blocked += 1;
                                                    }
                                                    _ => {
                                                        failed += 1;
                                                    }
                                                }
                                            } else {
                                                feedback.push((character_id, "The door appears to be locked by some strange mechanism. It seems you need to open three seperate locks.".to_string()));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorLocked { character_id, .. } => {
                                            feedback.push((character_id, "The door appears to be locked by some strange mechanism. It seems you need to open three seperate locks.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorBusy { character_id, .. } => {
                                            feedback.push((character_id, "Please try again soon. Target is busy.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ParkShrine { character_id, shrine, .. } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.memorize_park_shrine(shrine).unwrap_or(false) {
                                                    feedback.push((character_id, "You memorize the location of the shrine.".to_string()));
                                                } else {
                                                    feedback.push((character_id, "This shrine seems familar.".to_string()));
                                                }
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ParkShrineBug { character_id, .. } => {
                                            feedback.push((character_id, "BUG #55343, please report".to_string()));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::CaligarTraining { character_id, lesson, .. } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.observe_caligar_training(lesson).unwrap_or(false) {
                                                    let text = match lesson {
                                                        1 => "You observe the skeletons fighting techniques: Melee.",
                                                        2 => "You observe the vampires fighting techniques: Magic and Melee.",
                                                        3 => "You observe the zombies fighting techniques: Magic.",
                                                        _ => "",
                                                    };
                                                    if !text.is_empty() {
                                                        feedback.push((character_id, text.to_string()));
                                                    }
                                                }
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickBerry {
                                            character_id,
                                            kind,
                                            location_id,
                                            ..
                                        } => {
                                            match apply_pick_berry(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                character_id,
                                                kind,
                                                location_id,
                                                realtime_seconds,
                                            ) {
                                                PickBerryApplyResult::Picked(_) => {
                                                    executed += 1;
                                                }
                                                PickBerryApplyResult::NotRipe => {
                                                    feedback.push((character_id, "It's not ripe yet.".to_string()));
                                                    blocked += 1;
                                                }
                                                PickBerryApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                PickBerryApplyResult::Bug => {
                                                    feedback.push((character_id, "Bug # 4111c".to_string()));
                                                    failed += 1;
                                                }
                                                PickBerryApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickBerryCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlower {
                                            character_id,
                                            kind,
                                            location_id,
                                            ..
                                        } => {
                                            match apply_pick_alchemy_flower(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                character_id,
                                                kind,
                                                location_id,
                                                realtime_seconds,
                                            ) {
                                                PickBerryApplyResult::Picked(_) => {
                                                    award_gathering_achievement(&mut world, &mut runtime, &achievement_repository, character_id, kind).await;
                                                    executed += 1;
                                                }
                                                PickBerryApplyResult::NotRipe => {
                                                    feedback.push((character_id, "It's not ripe yet.".to_string()));
                                                    blocked += 1;
                                                }
                                                PickBerryApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                PickBerryApplyResult::Bug => {
                                                    feedback.push((character_id, "Bug # 4111".to_string()));
                                                    failed += 1;
                                                }
                                                PickBerryApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlowerCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientAdded {
                                            item_id,
                                            character_id,
                                            cursor_item_id,
                                            ingredient_kind,
                                        } => {
                                            if let Some(name) = apply_flask_ingredient_added(
                                                &mut world,
                                                character_id,
                                                item_id,
                                                cursor_item_id,
                                                ingredient_kind,
                                            ) {
                                                feedback.push((character_id, format!("You put {name} into the flask.")));
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskWrongCursor { character_id, .. } => {
                                            feedback.push((character_id, "That's not an ingredient you can use in a flask.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskFull { character_id, .. } => {
                                            feedback.push((character_id, "The Flask is full.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskFinishedNoMoreIngredients { character_id, .. } => {
                                            feedback.push((character_id, "This potion is finished. You cannot add more ingredients.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskEmptyShaken { character_id, .. } => {
                                            feedback.push((character_id, "You shake the empty bottle, but nothing happens.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientBug { character_id, .. } => {
                                            feedback.push((character_id, "BUG # 231...".to_string()));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskMixed { character_id, ingredient_counts, .. } => {
                                            for message in flask_ingredient_feedback(ingredient_counts) {
                                                feedback.push((character_id, message));
                                            }
                                            feedback.push((character_id, "The potion seems finished.".to_string()));
                                            award_potion_brewed_achievement(&mut world, &mut runtime, &achievement_repository, character_id).await;
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::FlaskRuined { character_id, ingredient_counts, .. } => {
                                            for message in flask_ingredient_feedback(ingredient_counts) {
                                                feedback.push((character_id, message));
                                            }
                                            feedback.push((character_id, "You shake the bottle and create a stinking liquid which you throw away.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerMixed {
                                            character_id,
                                            complete,
                                            bottle_message,
                                            ..
                                        } => {
                                            if bottle_message {
                                                feedback.push((
                                                    character_id,
                                                    "A bottle pops out of thin air as you try to combine the flowers. You're stunned for a moment, but then you mix the flowers in the bottle."
                                                        .to_string(),
                                                ));
                                            }
                                            if complete {
                                                feedback.push((character_id, "The potion seems finished.".to_string()));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerNeedsCursor { character_id, .. } => {
                                            feedback.push((character_id, "No, eating this berry isn't a good idea.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerDoesNotFit { character_id, .. } => {
                                            feedback.push((character_id, "This cannot be used together. Try something else.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BranningtonUnderwaterBerry { installed, .. } => {
                                            if installed {
                                                executed += 1;
                                            } else {
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab3YellowBerry { character_id, installed, .. } => {
                                            if installed {
                                                executed += 1;
                                            } else {
                                                feedback.push((character_id, "Due to some strange reasons thou canst not eat those berries now.".to_string()));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerry { character_id, installed, .. } => {
                                            if installed {
                                                executed += 1;
                                            } else {
                                                feedback.push((character_id, "Due to some strange reasons thou canst not eat those berries now.".to_string()));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerryLightTick { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab3BrownBerry { character_id, installed, .. } => {
                                            if installed {
                                                executed += 1;
                                            } else {
                                                feedback.push((character_id, "Thou art still chewing a brown berry.".to_string()));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterWell { character_id, .. } => {
                                            if let Some(item_name) = grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                "lab2_waterbowl",
                                            ) {
                                                feedback.push((character_id, format!("You received a {item_name}.")));
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterAltar { character_id, .. } => {
                                            match apply_lab2_water_altar(&mut world, &mut zone_loader, character_id) {
                                                Lab2WaterApplyResult::Converted(0) => {
                                                    feedback.push((character_id, "You feel the holyness of the Altar. Water would be holy now, if you had some.".to_string()));
                                                    blocked += 1;
                                                }
                                                Lab2WaterApplyResult::Converted(1) => {
                                                    feedback.push((character_id, "The water inside your bowl is holy now.".to_string()));
                                                    executed += 1;
                                                }
                                                Lab2WaterApplyResult::Converted(count) => {
                                                    feedback.push((character_id, format!("The water inside your {count} bowls is holy now.")));
                                                    executed += 1;
                                                }
                                                Lab2WaterApplyResult::MissingPlayer | Lab2WaterApplyResult::TemplateMissing => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterDrink { character_id, .. } => {
                                            feedback.push((character_id, "Skoll!".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "You won't throw this into the water, will you?".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionClear { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonCheck { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonWarning { x, y, .. } => {
                                            let character_id = runtime.allocate_character_id();
                                            match zone_loader.instantiate_character_template("lab2_daemon", character_id) {
                                                Ok((daemon, inventory_items)) => {
                                                    if world.spawn_character(daemon, usize::from(x), usize::from(y)) {
                                                        for item in inventory_items {
                                                            world.items.insert(item.id, item);
                                                        }
                                                        executed += 1;
                                                    } else {
                                                        failed += 1;
                                                    }
                                                }
                                                _ => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClueBook { character_id, book, .. } => {
                                            let text = lab2_grave_clue_text(&mut runtime, character_id, book);
                                            feedback.push((character_id, text));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClose { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveCheckOpen { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveOpen { item_id, character_id, fixed_item } => {
                                            if apply_lab2_grave_open(
                                                &mut world,
                                                &mut runtime,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                fixed_item,
                                            ) {
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceSolvedAll { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You have solved all existing labyrinths already. You can now fight the gatekeeper.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceTooLow { character_id, required_level, .. } => {
                                            feedback.push((
                                                character_id,
                                                format!("You may not enter before reaching level {required_level}."),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LabExitWrongOwner { character_id, .. } => {
                                            feedback.push((character_id, "This gate has not been created for you. You cannot use it.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EmptyPotionTemplateNeeded {
                                            item_id,
                                            character_id,
                                            empty_kind,
                                        } => {
                                            if apply_empty_potion_drink(
                                                &mut world,
                                                &mut zone_loader,
                                                item_id,
                                                character_id,
                                                empty_kind,
                                            ) {
                                                area_feedback.push((
                                                    character_id,
                                                    potion_area_message(&world, character_id),
                                                    10,
                                                ));
                                                executed += 1;
                                            } else {
                                                deferred_templates += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { item_id, character_id }
                                            if is_no_potion_area_blocked_item(&world, item_id) =>
                                        {
                                            feedback.push((character_id, "You sense that the potion would not work.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::LibloadAreaBlocked { character_id, .. } => {
                                            feedback.push((character_id, "This does not work outside its area.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                            if is_timed_potion_source_item(&world, item_id) =>
                                        {
                                            let message = if character_has_active_beyond_potion(&world, character_id) {
                                                "Another potion is still active."
                                            } else {
                                                "You do not meet the requirements needed to use this potion."
                                            };
                                            feedback.push((
                                                character_id,
                                                message.to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                            if is_torch_item(&world, item_id) =>
                                        {
                                            feedback.push((character_id, TORCH_UNDERWATER_MESSAGE.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                            if is_demonshrine_item(&world, item_id) =>
                                        {
                                            feedback.push((character_id, "You're not powerful enough to read this book.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PentBossDoor { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PentagramActivate { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PentagramTimer { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PentagramAlreadyActive { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PentBossDoorLocked { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "The door won't open. It seems it is only accessible directly after a solve.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PentBossDoorBusy { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Please try again soon. Target is busy.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TrapdoorOpen { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TrapdoorBlocked { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TrapdoorClose { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::GasTrapPulse { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EdemonBallInactive { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TrapdoorBusy { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You cannot do anything with it now.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TrapdoorNeedsStick { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You'd need something like a hard stick to lock the door.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneBridgePlace { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeTimerTick { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineWallInitialized { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineWallDig {
                                            character_id,
                                            endurance_delta,
                                            opened,
                                            ..
                                        } => {
                                            if let Some(character) = world.characters.get_mut(&character_id) {
                                                character.endurance = character.endurance.saturating_add(endurance_delta);
                                            }
                                            if opened {
                                                deferred_templates += 1;
                                            } else {
                                                executed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineWallCursorOccupied { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Please empty your hand (mouse cursor) first.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineWallExhausted { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You're too exhausted to continue digging.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MineWallCollapse {
                                            item_id,
                                            schedule_after_ticks,
                                        } => {
                                            world.schedule_item_driver_timer(
                                                item_id,
                                                CharacterId(0),
                                                u64::from(schedule_after_ticks),
                                            );
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IdentityTag { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Unsupported { .. } => {
                                            unsupported += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TorchExpired { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanJewelRescheduled { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Lab2RegenerateTick { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanJewelExpired { character_id, item_name, .. } => {
                                            if let Some(character_id) = character_id {
                                                let item_name = String::from_utf8_lossy(&item_name)
                                                    .trim_end_matches('\0')
                                                    .to_string();
                                                feedback.push((character_id, format!("Your {item_name} expired.")));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DecayItemExpired { character_id, item_name, .. } => {
                                            let item_name = String::from_utf8_lossy(&item_name)
                                                .trim_end_matches('\0')
                                                .to_string();
                                            feedback.push((character_id, format!("Your {item_name} expired.")));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceBombExplode { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceBombTimer { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceBombToggled { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceCapTimer { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Noop => {
                                            failed += 1;
                                        }
                                    }
                                }
                                // C `use_item` (`src/system/do.c:1504-
                                // 1508`): `log_char(cn, LOG_SYSTEM, 0,
                                // "Permission denied.");` - the
                                // grave-container access-denied reply.
                                Err(ugaris_core::item_driver::UseItemError::AccessDenied) => {
                                    feedback.push((
                                        use_character_id,
                                        "Permission denied.".to_string(),
                                    ));
                                    blocked += 1;
                                }
                                Err(_) => {
                                    failed += 1;
                                }
                            }
                        }
                        let mut feedback_sessions = 0;
                        for (character_id, message) in feedback {
                            let payload = ugaris_protocol::packet::system_text(&message);
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    feedback_sessions += 1;
                                }
                            }
                        }
                        for (character_id, message) in feedback_bytes {
                            let payload = ugaris_protocol::packet::system_text_bytes(&message);
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    feedback_sessions += 1;
                                }
                            }
                        }
                        for (character_id, payload) in special_feedback {
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    feedback_sessions += 1;
                                }
                            }
                        }
                        for (character_id, message, maxdist) in area_feedback {
                            let payload = ugaris_protocol::packet::system_text(&message);
                            for (session_id, _) in runtime.sessions_for_area_message(&world, character_id, maxdist) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    feedback_sessions += 1;
                                }
                            }
                        }
                        let mut container_sessions = 0;
                        container_refresh.sort_unstable_by_key(|id| id.0);
                        container_refresh.dedup();
                        for character_id in container_refresh {
                            let Some(payload) = current_container_payload(
                                &world,
                                runtime.account_depots.get(&character_id),
                                runtime
                                    .player_for_character(character_id)
                                    .map(|player| player.depot.as_slice()),
                                character_id,
                            ) else {
                                continue;
                            };
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                if runtime.send_to_session(session_id, payload.clone()) {
                                    container_sessions += 1;
                                }
                            }
                        }
                        info!(opened, executed, unsupported, deferred_templates, blocked, failed, feedback_sessions, container_sessions, tick = world.tick.0, "processed item-use requests");
                    }
                    clear_completed_use_actions(&mut runtime, &completed_actions);
                    let mut refreshed_sessions = 0;
                    for completion in &completed_actions {
                        let Some(character) = world.characters.get(&completion.character_id) else {
                            continue;
                        };
                        let walk_section_payload = if completion.ok
                            && completion.action_id == ugaris_core::legacy::action::WALK
                        {
                            runtime
                                .player_for_character_mut(completion.character_id)
                                .and_then(|player| walk_section_payload(config.area_id, player, character))
                        } else {
                            None
                        };
                        let pk_relations = PkRelationSnapshot::from_runtime(&runtime);
                        for (session_id, view_distance) in runtime.sessions_for_character(completion.character_id) {
                            let mut payloads = if completion.ok
                                && completion.action_id == ugaris_core::legacy::action::WALK
                            {
                                // Shift the session cache like the client
                                // shifts its map; the per-tick diff pass
                                // fills fringe and LOS changes afterwards.
                                let scroll_payload = runtime
                                    .map_caches
                                    .get_mut(&session_id)
                                    .and_then(|cache| {
                                        movement_scroll_payload(
                                            character,
                                            completion.old_x,
                                            completion.old_y,
                                            view_distance,
                                            cache,
                                        )
                                    });
                                match scroll_payload {
                                    Some(payload) => vec![payload],
                                    None => {
                                        let payloads = map_refresh_payloads(&world, character, &pk_relations, view_distance);
                                        runtime.map_caches.insert(
                                            session_id,
                                            visible_map_cache(&world, character, &pk_relations, view_distance),
                                        );
                                        payloads
                                    }
                                }
                            } else {
                                match runtime.map_caches.get_mut(&session_id) {
                                    Some(cache) => map_diff_payloads(
                                        &world,
                                        character,
                                        &pk_relations,
                                        view_distance,
                                        cache,
                                    ),
                                    None => {
                                        let payloads =
                                            map_refresh_payloads(&world, character, &pk_relations, view_distance);
                                        runtime.map_caches.insert(
                                            session_id,
                                            visible_map_cache(&world, character, &pk_relations, view_distance),
                                        );
                                        payloads
                                    }
                                }
                            };
                            if completion.action_id != ugaris_core::legacy::action::WALK {
                                payloads.push(inventory_snapshot_payload(&world, character));
                            }
                            if let Some(payload) = &walk_section_payload {
                                payloads.push(payload.clone());
                            }
                            if completion.ok {
                                if let Some(payload) = area_sound_payload(
                                    config.area_id,
                                    character,
                                    world.date.hour,
                                    world.tick
                                        .0
                                        .wrapping_add(u64::from(completion.character_id.0) << 32),
                                ) {
                                    payloads.push(bytes::BytesMut::from(&payload[..]));
                                }
                            }
                            payloads.extend(client_effect_payloads(
                                &world,
                                character,
                                view_distance,
                                runtime.effect_caches.entry(session_id).or_default(),
                            ));
                            if runtime.send_many_to_session(session_id, payloads) {
                                refreshed_sessions += 1;
                            }
                        }
                    }
                    if refreshed_sessions != 0 {
                        info!(refreshed_sessions, tick = world.tick.0, "queued map refreshes for completed actions");
                    }

                    let mut sound_sessions = 0;
                    for sound in world.drain_pending_sound_specials() {
                        let payload = ugaris_protocol::packet::special(
                            sound.special.special_type,
                            sound.special.opt1 as u32,
                            sound.special.opt2 as u32,
                        );
                        for (session_id, _) in runtime.sessions_for_character(sound.character_id) {
                            if runtime.send_to_session(
                                session_id,
                                bytes::BytesMut::from(&payload[..]),
                            ) {
                                sound_sessions += 1;
                            }
                        }
                    }
                    if sound_sessions != 0 {
                        info!(sound_sessions, tick = world.tick.0, "queued legacy sound-area specials");
                    }
                }

                // Per-NPC and queued-event tick passes live in
                // `tick_npc/` (one fn per legacy driver, grouped
                // by area); `run_all` keeps the original order.
                tick_npc::run_all(&mut world, &mut runtime, &mut zone_loader, &config, &args, &completed_actions, &achievement_repository, &character_repository, &area_repository, &clan_repository, &clan_log_repository, &merchant_repository, &military_master_storage_repository, &military_advisor_storage_repository, &notes_repository, &anticheat_repository, &auction_repository).await;

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
