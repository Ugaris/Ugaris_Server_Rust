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
mod depot;
mod dungeon;
mod effects_sync;
mod inventory;
mod item_apply;
mod keyring;
mod login;
mod lostcon;
mod map_sync;
mod merchants;
mod military;
mod player_actions;
mod resource_sync;
mod rng;
mod snapshots;
mod spawns;
mod stacks;
mod transport;
mod weather;
mod world_events;
mod xmas;
mod zone;

pub(crate) use achievement::*;
pub(crate) use area_apply::*;
pub(crate) use chests::*;
pub(crate) use commands_admin::*;
pub(crate) use commands_chat::*;
pub(crate) use commands_player::*;
pub(crate) use constants::*;
pub(crate) use containers::*;
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
pub(crate) use lostcon::*;
pub(crate) use map_sync::*;
pub(crate) use merchants::*;
pub(crate) use military::*;
pub(crate) use player_actions::*;
pub(crate) use resource_sync::*;
pub(crate) use rng::*;
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
        needs_next_lab, CharacterDriverState, CDR_CALIGARSKELLY, CDR_GATE_FIGHT, CDR_GATE_WELCOME,
        CDR_LAB2UNDEAD, CDR_LOSTCON, CDR_LQNPC, CDR_PALACEISLENA, CDR_SIMPLEBADDY,
        CDR_SWAMPMONSTER, CDR_TEUFELRAT, NTID_GATEKEEPER, NT_NPC,
    },
    clan::{ClanRelations, ClanTreasuryEvent},
    direction::Direction,
    do_action::{
        can_attack_in_area, can_attack_in_area_with_clan_policy, ClanAttackPolicy, ItemUseRequest,
    },
    drvlib::char_dist,
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
        KeyringAddResult, PlayerActionCode, PlayerConnectionState, PlayerRuntime, QueuedAction,
        XmasTreeResult, ARENA_PPD_NEWCOMER_SCORE, DEFERRED_ACHIEVEMENTS, DEFERRED_AUCTION,
        LEGACY_SWEAR_PPD_SIZE, MILITARY_PPD_MAXADVISOR, SWEAR_SENTENCE_COUNT, SWEAR_SENTENCE_LEN,
    },
    quest::{QuestReopenResult, QF_OPEN},
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
        army_rank_for_points, army_rank_name, exp2level, legacy_save_number, level2exp,
        level2maxitem, level_value, merchant_buy_price, merchant_sales_price, ArenaMasterEvent,
        BankEvent, ClanclerkEvent, ClanmasterEvent, ClubmasterEvent, FirstKillCheck,
        GateWelcomeOutcomeEvent, GateWelcomePlayerFacts, LegacyHurtEvent, LookMapRequest,
        MerchantTradeResult, RaiseSkillOutcome, StoreWare, TraderEvent, WorldActionCompletion,
        MERCHANT_STORE_SIZE,
    },
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
};

use ugaris_db::{
    AuctionRepository, CharacterRepository, CharacterSaveMode, CharacterSaveRequest,
    CharacterSnapshot, ClanRegistryRepository, LoginOutcome, LoginRequest, MerchantRepository,
    MerchantStoreSnapshot, MerchantWareSnapshot, MilitaryAdvisorStorageRepository,
    MilitaryMasterStorageRepository,
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
    ) = if let Some(database_url) = args.database_url.as_deref() {
        let db = ugaris_db::Database::connect(database_url, 8).await?;
        db.ping().await?;
        info!("connected to PostgreSQL");
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
        )
    } else {
        warn!("DATABASE_URL not set; starting without persistence");
        (None, None, None, None, None, None, None, None)
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
                world.advance();
                world.advance_date(
                    current_unix_time(),
                    config.area_id,
                    (runtime.dlight_override != 0).then_some(runtime.dlight_override),
                );
                world.regenerate_characters(runtime.regen_time, config.area_id);
                // C `server.c:210`'s `update_weather()` + `act.c:2268`'s
                // per-player `apply_weather_effects` (`src/module/weather/
                // weather.c`): advance the autonomous seasonal weather
                // cycle every tick, broadcast an `SV_MOD2`/`SV_VIS_WEATHER`
                // packet to every connected player when it changes, and
                // roll the periodic outdoor damage tick.
                let weather_changed = update_weather_tick(
                    &mut runtime.weather,
                    &world.date,
                    world.tick.0,
                    runtime_random_below,
                );
                if weather_changed {
                    broadcast_weather_packet(&world, &mut runtime, config.area_id);
                }
                // C `modify_movement_speed` (`module/weather/weather.c:
                // 477-493`): refresh the live movement-slow percent every
                // tick so `do_walk` (via `World.settings.
                // weather_movement_percent`) applies it exactly like C's
                // `speed()` call folds it in. Gated on `area_has_weather`
                // like the damage roll below - no-weather areas (indoor/
                // underground/arena) never apply the autonomous cycle's
                // current weather type to movement.
                world.settings.weather_movement_percent =
                    if area_has_weather(i64::from(config.area_id)) {
                        current_movement_percent(&runtime.weather)
                    } else {
                        100
                    };
                if area_has_weather(i64::from(config.area_id)) {
                    let player_character_ids: Vec<CharacterId> = world
                        .characters
                        .values()
                        .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
                        .map(|character| character.id)
                        .collect();
                    if runtime.weather.weather_effects & WEATHER_EFFECT_DAMAGE != 0 {
                        let damage = weather_damage_amount(
                            runtime.weather.current_weather,
                            runtime.weather.weather_intensity,
                        );
                        if damage > 0 {
                            // C `handle_weather_damage` (`weather.c:435-471`):
                            // each player rolls its own independent "Only
                            // apply damage occasionally (every ~12 seconds)"
                            // `RANDOM(TICKS * 12)` check every tick (the C
                            // call site is inside the per-character
                            // `tick_char` loop), so every player gets its own
                            // chance rather than all-or-nothing for the
                            // whole area. On an actual hit, also queues the
                            // matching per-weather-type `log_char` message -
                            // previously missing from this port, see
                            // `weather_damage_message`.
                            for &character_id in &player_character_ids {
                                if runtime_random_below((TICKS_PER_SECOND * 12) as i32) == 0
                                    && world.apply_weather_damage(character_id, damage).is_some()
                                {
                                    if let Some(message) =
                                        weather_damage_message(runtime.weather.current_weather)
                                    {
                                        world.queue_system_text(character_id, message);
                                    }
                                }
                            }
                        }
                    }
                    // C `handle_lightning_strike` (`weather.c:534-575`),
                    // called from the same per-player `apply_weather_effects`
                    // tick hook as the damage roll above: an independent
                    // per-player `RANDOM(100*TICKS*60) < lightning_chance*100`
                    // roll (only `MOD_WEATHER_STORM` ever has a nonzero
                    // `lightning_chance`), gated on the same
                    // `character_weather_eligible` guards (player-only,
                    // never gods/immortals, never indoors) *before* the RNG
                    // call so ineligible characters never consume a roll,
                    // matching C's guard-before-roll order exactly.
                    if runtime.weather.weather_effects & WEATHER_EFFECT_LIGHTNING != 0 {
                        let lightning_chance = lightning_strike_chance(
                            runtime.weather.current_weather,
                            runtime.weather.weather_intensity,
                        );
                        if lightning_chance > 0 {
                            for &character_id in &player_character_ids {
                                if !world.character_weather_eligible(character_id) {
                                    continue;
                                }
                                if runtime_random_below(100 * TICKS_PER_SECOND as i32 * 60)
                                    >= lightning_chance * 100
                                {
                                    continue;
                                }
                                let base_damage = lightning_strike_damage_amount(
                                    runtime.weather.weather_intensity,
                                    &mut runtime_random_below,
                                );
                                if world
                                    .apply_lightning_strike_damage(character_id, base_damage)
                                    .is_some()
                                {
                                    world.queue_system_text(
                                        character_id,
                                        "CRACK! Lightning strikes you!",
                                    );
                                    if let Some(character) =
                                        world.characters.get(&character_id)
                                    {
                                        let (x, y) = (character.x, character.y);
                                        let weather_intensity = runtime.weather.weather_intensity;
                                        broadcast_weather_thunder_effect(
                                            &world,
                                            &mut runtime,
                                            x,
                                            y,
                                            12,
                                            weather_intensity,
                                        );
                                    }
                                    // C's own nearby-players text broadcast
                                    // (`log_char(co, LOG_INFO, 0, "Lightning
                                    // strikes nearby with a thunderous
                                    // crack!")`, `weather.c:606-608`) is
                                    // intentionally NOT ported: `log_char`'s
                                    // own `LOG_INFO` gate is `if (type ==
                                    // LOG_INFO && !char_see_char(cn, dat1))
                                    // return 0;`, and this call site hardcodes
                                    // `dat1 = 0`, so `char_see_char(co, 0)`
                                    // always returns `0` (its own `co == 0`
                                    // early-return) - the gate always fails
                                    // and the message is *never* delivered to
                                    // anyone in the real C server. Verified:
                                    // no other C caller passes `dat1 = 0` to
                                    // `LOG_INFO`; every other `LOG_INFO` call
                                    // site passes a real acting character id.
                                }
                            }
                        }
                    }
                    // C `apply_elemental_debuffs` (`weather.c:614-655`),
                    // called from the same per-player `apply_weather_effects`
                    // tick hook as the damage/lightning rolls above: a
                    // periodic (at most once per 10 real seconds per
                    // character) flavor-text notification while standing in
                    // wet/cold/scorching weather. Gated on the same
                    // `character_weather_eligible` guards - see
                    // `elemental_debuff_message`'s doc comment for why only
                    // this notification (not the persistent debuff/expire
                    // state) is ported.
                    if runtime.weather.weather_effects & WEATHER_EFFECT_ELEMENTAL != 0 {
                        if let Some(message) = elemental_debuff_message(elemental_debuff_type(
                            runtime.weather.current_weather,
                            runtime.weather.weather_intensity,
                        )) {
                            for &character_id in &player_character_ids {
                                if !world.character_weather_eligible(character_id) {
                                    continue;
                                }
                                let last_notify = runtime
                                    .weather
                                    .elemental_debuff_last_notify
                                    .get(&character_id)
                                    .copied()
                                    .unwrap_or(0);
                                if should_notify_elemental_debuff(last_notify, world.tick.0) {
                                    runtime
                                        .weather
                                        .elemental_debuff_last_notify
                                        .insert(character_id, world.tick.0);
                                    world.queue_system_text(character_id, message);
                                }
                            }
                        }
                    }
                }
                // C `lostcon_driver`'s `!ch[cn].player && ticker >
                // dat->timeout` branch + `exit_char`/`kick_char`: save and
                // despawn characters whose disconnect linger expired
                // without being reclaimed by a reconnect.
                let expired_lostcon =
                    take_expired_lostcon_characters(&world, &mut runtime, world.tick.0);
                if !expired_lostcon.is_empty() {
                    let expired_count = expired_lostcon.len();
                    for (character_id, player, account_depot) in expired_lostcon {
                        if let Some(repository) = &character_repository {
                            if let Some(character) = world.characters.get(&character_id) {
                                let request = character_save_request(
                                    &world,
                                    &player,
                                    character,
                                    account_depot.as_ref(),
                                    config.area_id,
                                    config.mirror_id,
                                );
                                match repository.save_character_snapshot(request).await {
                                    Ok(true) => {
                                        info!(character_id = character_id.0, "saved DB-backed character snapshot on lostcon expiry");
                                    }
                                    Ok(false) => {
                                        warn!(character_id = character_id.0, "DB character snapshot save was skipped by area guard on lostcon expiry");
                                    }
                                    Err(err) => {
                                        warn!(character_id = character_id.0, error = %err, "failed to save DB-backed character snapshot on lostcon expiry");
                                    }
                                }
                            }
                        }
                        world.remove_character(character_id);
                    }
                    info!(expired_count, tick = world.tick.0, "despawned expired lostcon characters");
                }
                let clan_relations: ClanRelations = world.clan_registry.relations().clone();
                world.tick_effects_with_attack_policy(|caster_id, caster, target, map| {
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
                let timer_outcomes = world.process_due_timers(config.area_id);
                if !timer_outcomes.is_empty() {
                    info!(count = timer_outcomes.len(), tick = world.tick.0, "processed timer callbacks");
                }
                let mut edemon_gate_spawns = 0;
                let mut fdemon_gate_spawns = 0;
                let mut chest_spawns = 0;
                let mut swamp_spawns = 0;
                for outcome in &timer_outcomes {
                    if let ugaris_core::item_driver::ItemDriverOutcome::EdemonGateSpawn {
                        item_id,
                        template,
                        slot,
                        x,
                        y,
                        ..
                    } = outcome
                    {
                        if spawn_edemon_gate_character(
                            &mut world,
                            &mut zone_loader,
                            &mut runtime,
                            *item_id,
                            template,
                            *slot,
                            *x,
                            *y,
                        ) {
                            edemon_gate_spawns += 1;
                        }
                    }
                    if let ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn {
                        item_id,
                        template,
                        x,
                        y,
                        ..
                    } = outcome
                    {
                        if spawn_chestspawn_character(
                            &mut world,
                            &mut zone_loader,
                            &mut runtime,
                            *item_id,
                            template,
                            *x,
                            *y,
                        ) {
                            chest_spawns += 1;
                        }
                    }
                    if let ugaris_core::item_driver::ItemDriverOutcome::SwampSpawn {
                        item_id,
                        template,
                        x,
                        y,
                        ..
                    } = outcome
                    {
                        if spawn_swampspawn_character(
                            &mut world,
                            &mut zone_loader,
                            &mut runtime,
                            *item_id,
                            template,
                            *x,
                            *y,
                        ) {
                            swamp_spawns += 1;
                        }
                    }
                    if let ugaris_core::item_driver::ItemDriverOutcome::FdemonGateSpawn {
                        item_id,
                        level,
                        slot,
                        x,
                        y,
                        ..
                    } = outcome
                    {
                        if spawn_fdemon_gate_character(
                            &mut world,
                            &mut zone_loader,
                            &mut runtime,
                            *item_id,
                            *level,
                            *slot,
                            *x,
                            *y,
                        ) {
                            fdemon_gate_spawns += 1;
                        }
                    }
                }
                if edemon_gate_spawns != 0 {
                    info!(count = edemon_gate_spawns, tick = world.tick.0, "spawned edemon gate characters");
                }
                if chest_spawns != 0 {
                    info!(count = chest_spawns, tick = world.tick.0, "spawned chestspawn characters");
                }
                if swamp_spawns != 0 {
                    info!(count = swamp_spawns, tick = world.tick.0, "spawned swampspawn characters");
                }
                if fdemon_gate_spawns != 0 {
                    info!(count = fdemon_gate_spawns, tick = world.tick.0, "spawned fdemon gate characters");
                }
                let lq_spawn_requests = world.drain_pending_lq_npc_spawns();
                if !lq_spawn_requests.is_empty() {
                    let mut lq_spawns = 0;
                    for request in &lq_spawn_requests {
                        if spawn_lq_npc_character(
                            &mut world,
                            &mut zone_loader,
                            &mut runtime,
                            request,
                        ) {
                            lq_spawns += 1;
                        }
                    }
                    if lq_spawns != 0 {
                        info!(count = lq_spawns, tick = world.tick.0, "spawned LQ NPC characters");
                    }
                }
                // C respawn_callback: recreate dead template NPCs at their
                // spawn tile, retrying every ten seconds while blocked.
                let respawn_requests = world.drain_pending_npc_respawns();
                if !respawn_requests.is_empty() {
                    let mut respawned = 0;
                    for request in &respawn_requests {
                        if respawn_npc_character(&mut world, &mut zone_loader, &mut runtime, request)
                        {
                            respawned += 1;
                        } else {
                            world.schedule_npc_respawn_retry(request.slot);
                        }
                    }
                    if respawned != 0 {
                        info!(count = respawned, tick = world.tick.0, "respawned NPC characters");
                    }
                }
                // C kill_char give_exp: route kill experience through the
                // shared runtime EXP modifiers.
                for award in world.drain_pending_kill_exp() {
                    let area_id = args.area_id;
                    give_exp_with_runtime_modifiers(
                        &mut world,
                        award.killer_id,
                        i64::from(award.exp),
                        u32::from(area_id),
                    );
                }
                // C kill_char achievement_add_enemy_killed/achievement_add_demons.
                for award in world.drain_pending_kill_achievements() {
                    award_enemy_killed_achievement(
                        &mut world,
                        &mut runtime,
                        &achievement_repository,
                        award.killer_id,
                        award.area_id,
                        award.target_is_demon,
                    )
                    .await;
                }
                // C check_levelup achievement_check_level.
                for check in world.drain_pending_level_achievements() {
                    award_level_achievement(
                        &mut world,
                        &mut runtime,
                        &achievement_repository,
                        check.character_id,
                        check.level as i32,
                        check.is_hardcore,
                    )
                    .await;
                }
                // C kill_char give_first_kill.
                for check in world.drain_pending_first_kill_checks() {
                    apply_first_kill_check(
                        &mut world,
                        &mut runtime,
                        &achievement_repository,
                        i32::from(args.area_id),
                        check,
                    )
                    .await;
                }
                // C kill_char check_military_solve.
                for check in world.drain_pending_military_mission_checks() {
                    apply_military_mission_kill_check(&mut world, &mut runtime, check);
                }
                let timer_feedback = timer_outcome_feedback(&timer_outcomes);
                if !timer_feedback.is_empty() {
                    let mut feedback_sessions = 0;
                    for (character_id, message) in timer_feedback {
                        let payload = ugaris_protocol::packet::system_text(&message);
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                feedback_sessions += 1;
                            }
                        }
                    }
                    info!(feedback_sessions, tick = world.tick.0, "queued timer feedback");
                }
                let due_tasks = world.scheduler.due_tasks(world.tick.0);
                if !due_tasks.is_empty() {
                    info!(count = due_tasks.len(), tick = world.tick.0, "scheduled tasks are due");
                }
                let queued = runtime.drain_actions_for_tick();
                if !queued.is_empty() {
                    info!(count = queued.len(), tick = world.tick.0, "drained queued client actions");
                }
                let mut command_feedback = Vec::new();
                let mut command_feedback_bytes = Vec::new();
                let mut command_inventory_refresh = Vec::new();
                let mut command_container_refresh = Vec::new();
                let mut command_name_refresh = Vec::new();
                for (character_id, message) in
                    drain_expired_tell_feedback(&world, &mut runtime, world.tick.0)
                {
                    command_feedback_bytes.push((character_id, message));
                }
                let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                for (character_id, message) in
                    drain_expired_shutup_feedback(&mut world, &mut runtime, realtime_seconds)
                {
                    command_feedback_bytes.push((character_id, message));
                }
                for (session_id, action) in queued {
                    let Some(player) = runtime.players.get(&session_id) else {
                        continue;
                    };
                    let Some(character_id) = player.character_id else {
                        continue;
                    };
                    match action {
                        ClientAction::Text(bytes) => {
                            let Some(mut command) = normalize_text_command(&bytes) else {
                                continue;
                            };
                            {
                                let Some(player) = runtime.players.get_mut(&session_id) else {
                                    continue;
                                };
                                if let Some(result) = apply_alias_command(player, &command) {
                                    for message in result.messages {
                                        command_feedback.push((character_id, message));
                                    }
                                    continue;
                                }
                                command = player.expand_aliases(&command);
                            }
                            if command.eq_ignore_ascii_case("sort") {
                                inventory_sort(&mut world, character_id);
                                command_inventory_refresh.push(character_id);
                                continue;
                            }
                            if command.eq_ignore_ascii_case("accountdepotsort") {
                                if account_depot_sort_if_open(&mut world, &mut runtime, character_id) {
                                    command_container_refresh.push(character_id);
                                    command_feedback.push((character_id, "Account depot sorted.".to_string()));
                                } else {
                                    command_feedback.push((character_id, "You must have the account depot open to use this command.".to_string()));
                                }
                                continue;
                            }
                            let character_flags = world
                                .characters
                                .get(&character_id)
                                .map(|character| character.flags)
                                .unwrap_or_else(CharacterFlags::empty);
                            let weather_before_admin_command = runtime.weather.clone();
                            if let Some(result) = apply_weather_admin_command(
                                &world,
                                character_id,
                                &mut runtime.weather,
                                &command,
                            ) {
                                // C `cmd_setweather`/`cmd_clearweather`/
                                // `cmd_setareaweather` (`command.c`) each
                                // call `broadcast_weather_packet()`
                                // immediately on success (not just on the
                                // next `update_weather()` tick).
                                if runtime.weather != weather_before_admin_command {
                                    broadcast_weather_packet(&world, &mut runtime, config.area_id);
                                }
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_weather_command(
                                &world,
                                character_id,
                                config.area_id,
                                &runtime.weather,
                                &command,
                            ) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_time_command(world.date, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_help_command(&command, character_flags, u32::from(config.area_id)) {
                                if result.message_bytes.is_empty() {
                                    for message in result.messages {
                                        command_feedback.push((character_id, message));
                                    }
                                } else {
                                    for message in result.message_bytes {
                                        command_feedback_bytes.push((character_id, message));
                                    }
                                }
                                continue;
                            }
                            if let Some(result) = apply_color_command(&mut world, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.name_changed {
                                    command_name_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = apply_description_command(&mut world, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_create_orb_command(
                                &mut world,
                                &mut zone_loader,
                                character_id,
                                &command,
                            ) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = apply_create_command(
                                &mut world,
                                &mut zone_loader,
                                character_id,
                                &command,
                            ) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = apply_admin_character_command(
                                &mut world,
                                &mut runtime,
                                character_id,
                                &command,
                                u32::from(config.area_id),
                            ) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                                if result.name_changed {
                                    command_name_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = apply_achievement_command(
                                &mut world,
                                &mut runtime,
                                &achievement_repository,
                                character_id,
                                &command,
                                current_unix_time(),
                            )
                            .await
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                for message in result.message_bytes {
                                    command_feedback_bytes.push((character_id, message));
                                }
                                for (target_id, message) in result.target_message_bytes {
                                    command_feedback_bytes.push((target_id, message));
                                }
                                continue;
                            }
                            if let Some(result) =
                                apply_shutup_command(
                                    &mut world,
                                    &mut runtime,
                                    character_id,
                                    &command,
                                    realtime_seconds,
                                )
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                for (target_id, message) in result.target_message_bytes {
                                    command_feedback_bytes.push((target_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_notells_command(&mut world, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_channels_command(&command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            let character_flags = world
                                .characters
                                .get(&character_id)
                                .map(|character| character.flags)
                                .unwrap_or_else(CharacterFlags::empty);
                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                if let Some(result) =
                                    apply_join_leave_chat_command(player, character_flags, &command)
                                {
                                    for message in result.messages {
                                        command_feedback.push((character_id, message));
                                    }
                                    continue;
                                }
                            }
                            if let Some(result) = apply_clearignore_command(&mut runtime, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_ignore_command(&world, &mut runtime, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_tell_command(
                                &world,
                                &mut runtime,
                                character_id,
                                &command,
                                world.tick.0,
                                u64::from(current_realtime_seconds()),
                            ) {
                                for message in result.sender_messages {
                                    command_feedback.push((character_id, message));
                                }
                                for (target_id, message) in result.delivered_messages {
                                    command_feedback.push((target_id, message));
                                }
                                for (target_id, message) in result.delivered_message_bytes {
                                    command_feedback_bytes.push((target_id, message));
                                }
                                continue;
                            }
                            let current_tick = world.tick.0;
                            if let Some(result) = apply_local_speech_command(
                                &mut world,
                                &mut runtime,
                                character_id,
                                &command,
                                current_tick,
                                u64::from(current_realtime_seconds()),
                            ) {
                                for message in result.sender_messages {
                                    command_feedback.push((character_id, message));
                                }
                                for (target_id, message) in result.delivered_message_bytes {
                                    command_feedback_bytes.push((target_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_chat_command(
                                &world,
                                &mut runtime,
                                character_id,
                                &command,
                                config.area_id,
                                u64::from(current_realtime_seconds()),
                            ) {
                                for message in result.sender_messages {
                                    command_feedback.push((character_id, message));
                                }
                                for (target_id, message) in result.delivered_message_bytes {
                                    command_feedback_bytes.push((target_id, message));
                                }
                                continue;
                            }
                            if let Some(result) =
                                apply_nowho_command(&mut world, character_id, &command)
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) =
                                apply_who_command(&world, Some(&runtime), character_flags, &command)
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            let Some(player) = runtime.players.get_mut(&session_id) else {
                                continue;
                            };
                            let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                            if let Some(result) = apply_pk_hate_command(&mut world, player, character_id, &command, realtime_seconds) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                command_name_refresh.extend(result.name_refresh);
                                continue;
                            }
                            if let Some(result) = apply_maxlag_command(player, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_hints_command(player, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_wimp_command(&command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(character) = world.characters.get(&character_id) {
                                if let Some(result) = apply_autoturn_command(character, player, &command) {
                                    for message in result.messages {
                                        command_feedback.push((character_id, message));
                                    }
                                    continue;
                                }
                            }
                            if let Some(result) = apply_lag_command(&mut world, player, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(character) = world.characters.get(&character_id) {
                                if let Some(result) = apply_status_command(character, player, &command) {
                                    for message in result.messages {
                                        command_feedback.push((character_id, message));
                                    }
                                    continue;
                                }
                            }
                            if let Some(result) = apply_gold_command(&mut world, &mut zone_loader, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = apply_laugh_command(&mut world, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                continue;
                            }
                            if let Some(result) = apply_keyring_command(&mut world, &mut zone_loader, player, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                                continue;
                            }
                            if let Some(result) = auction::apply_auction_command(
                                &mut world,
                                &auction_repository,
                                character_id,
                                current_unix_time(),
                                &command,
                            )
                            .await
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                for message in result.message_bytes {
                                    command_feedback_bytes.push((character_id, message));
                                }
                                for (target_id, message) in result.other_messages {
                                    command_feedback.push((target_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                            }
                            if let Some(result) = clan_log::apply_clan_log_command(
                                &mut world,
                                &clan_log_repository,
                                character_id,
                                current_unix_time(),
                                &command,
                            )
                            .await
                            {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                for message in result.message_bytes {
                                    command_feedback_bytes.push((character_id, message));
                                }
                            }
                            if let Some(result) = clan_command::apply_clan_command(
                                &mut world,
                                character_id,
                                &command,
                                current_unix_time(),
                            ) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                for message in result.message_bytes {
                                    command_feedback_bytes.push((character_id, message));
                                }
                            }
                        }
                        ClientAction::Container { .. } | ClientAction::LookContainer { .. } => {
                            // C cl_container: validate and prefer the active
                            // merchant store before item containers.
                            world.check_merchant(character_id);
                            let active_merchant = world
                                .characters
                                .get(&character_id)
                                .and_then(|character| character.merchant);
                            if let Some(merchant_id) = active_merchant {
                                let result = apply_merchant_container_command(
                                    &mut world,
                                    character_id,
                                    merchant_id,
                                    &action,
                                );
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.changed {
                                    command_inventory_refresh.push(character_id);
                                    command_container_refresh.push(character_id);
                                    save_merchant_store_if_configured(
                                        &world,
                                        &merchant_repository,
                                        merchant_id,
                                    )
                                    .await;
                                }
                                continue;
                            }
                            let current_container = world
                                .characters
                                .get(&character_id)
                                .and_then(|character| character.current_container);
                            let result = if current_container.is_some_and(|container_id| {
                                world
                                    .items
                                    .get(&container_id)
                                    .is_some_and(|item| item.driver == IDR_ACCOUNT_DEPOT)
                            }) {
                                let depot = runtime.ensure_account_depot(character_id);
                                apply_account_depot_command(&mut world, depot, character_id, &action)
                            } else {
                                apply_item_container_command(&mut world, character_id, &action)
                            };
                            match result {
                                AccountDepotCommandResult::Changed => {
                                    command_inventory_refresh.push(character_id);
                                    command_container_refresh.push(character_id);
                                }
                                AccountDepotCommandResult::Look(message)
                                | AccountDepotCommandResult::Blocked(message) => {
                                    command_feedback.push((character_id, message));
                                }
                                AccountDepotCommandResult::Ignored => {}
                            }
                        }
                        ClientAction::FastSell { slot } => {
                            // C `cl_fastsell`: quick-sell an inventory slot
                            // straight to the active merchant.
                            let result =
                                apply_fast_sell(&mut world, character_id, usize::from(slot));
                            for message in result.messages {
                                command_feedback.push((character_id, message));
                            }
                            if result.inventory_changed {
                                command_inventory_refresh.push(character_id);
                            }
                            if result.sold {
                                command_container_refresh.push(character_id);
                                let merchant_id = world
                                    .characters
                                    .get(&character_id)
                                    .and_then(|character| character.merchant);
                                if let Some(merchant_id) = merchant_id {
                                    save_merchant_store_if_configured(
                                        &world,
                                        &merchant_repository,
                                        merchant_id,
                                    )
                                    .await;
                                }
                            }
                        }
                        ClientAction::Swap { .. }
                        | ClientAction::UseInventory { .. }
                        | ClientAction::LookInventory { .. }
                        | ClientAction::LookItem { .. } => {
                            let result = apply_inventory_client_action(
                                &mut world,
                                runtime.player_for_character(character_id),
                                character_id,
                                &action,
                                config.area_id,
                            );
                            match result {
                                InventoryCommandResult::Changed => {
                                    command_inventory_refresh.push(character_id);
                                }
                                InventoryCommandResult::MoneyConverted { price } => {
                                    command_inventory_refresh.push(character_id);
                                    award_swap_money_converted_achievement(
                                        &mut world,
                                        &mut runtime,
                                        &achievement_repository,
                                        character_id,
                                        price,
                                    )
                                    .await;
                                }
                                InventoryCommandResult::ContainerOpened { account_depot } => {
                                    if account_depot {
                                        runtime.ensure_account_depot(character_id);
                                    }
                                    command_inventory_refresh.push(character_id);
                                    command_container_refresh.push(character_id);
                                }
                                InventoryCommandResult::Look(message) => {
                                    command_feedback.push((character_id, message));
                                }
                                InventoryCommandResult::Ignored => {}
                            }
                        }
                        ClientAction::TakeGold { .. } | ClientAction::DropGold => {
                            if apply_gold_client_action(
                                &mut world,
                                &mut zone_loader,
                                character_id,
                                &action,
                            ) {
                                command_inventory_refresh.push(character_id);
                            }
                        }
                        ClientAction::JunkItem => {
                            if apply_junk_item_client_action(&mut world, character_id) {
                                command_inventory_refresh.push(character_id);
                            }
                        }
                        ClientAction::Speed { mode } => {
                            // C `cl_speed` (`src/system/player.c`): silently
                            // ignores invalid mode bytes and fast-mode
                            // requests without enough endurance - no
                            // feedback packet either way.
                            world.set_speed_mode(character_id, mode);
                        }
                        ClientAction::FightMode { .. } => {
                            // C `cl_fightmode` (`src/system/player.c`) is a
                            // no-op stub (`return;`); `ch[cn].fight_mode` is
                            // otherwise unused in the C tree. Consume the
                            // packet without acting on it, matching C.
                        }
                        ClientAction::Raise { value } => {
                            // C `cl_raise` (`src/system/player.c`) calls
                            // `raise_value` and discards the result - no
                            // feedback packet on failure, only the updated
                            // value/exp on success.
                            if let RaiseSkillOutcome::Raised {
                                value,
                                bare,
                                effective,
                                exp,
                                exp_used,
                            } = world.raise_skill(character_id, value)
                            {
                                let mut builder = PacketBuilder::new();
                                builder
                                    .set_value0(value as u8, effective)
                                    .set_value1(value as u8, bare)
                                    .exp(exp)
                                    .exp_used(exp_used);
                                runtime.send_to_session(session_id, builder.into_payload());
                                // C `raise_value` (`src/system/skill.c:256-
                                // 259`): `if (ch[cn].flags & CF_PLAYER) {
                                // achievement_check_skill(cn, v,
                                // ch[cn].value[1][v]); }`.
                                award_skill_achievement(
                                    &mut world,
                                    &mut runtime,
                                    &achievement_repository,
                                    character_id,
                                    value as i32,
                                    bare as i32,
                                )
                                .await;
                            }
                        }
                        ClientAction::LookCharacter { character } => {
                            // C `cl_look_char` (`src/system/player.c`):
                            // bounds-checks the target, gates on
                            // `char_see_char`, then `look_char`
                            // (`src/system/tool.c`) sends `#1`/`#2` text
                            // plus the `SV_LOOKINV` paperdoll. `character
                            // == 0` mirrors C's `co < 1` bounds check.
                            if character != 0 {
                                let target_id = CharacterId(u32::from(character));
                                let target_is_brave = runtime
                                    .player_for_character(target_id)
                                    .is_some_and(|player| player.has_used_random_shrine(51));
                                let target_mirror = runtime
                                    .player_for_character(target_id)
                                    .map(|player| u32::from(player.current_mirror_id))
                                    .unwrap_or(0);
                                if let Some(text) = world.look_character_text(
                                    character_id,
                                    target_id,
                                    target_is_brave,
                                    target_mirror,
                                ) {
                                    command_feedback.push((character_id, text.header));
                                    if let Some(paperdoll) =
                                        world.look_character_paperdoll(target_id)
                                    {
                                        let mut builder = PacketBuilder::new();
                                        builder.look_inventory(
                                            paperdoll.sprite,
                                            paperdoll.colors,
                                            paperdoll.worn_sprites,
                                        );
                                        runtime.send_to_session(session_id, builder.into_payload());
                                    }
                                    command_feedback.push((character_id, text.body));
                                }
                            }
                        }
                        ClientAction::GetQuestLog => {
                            if let Some(player) = runtime.players.get(&session_id) {
                                let payload = legacy_questlog_payload(player);
                                runtime.send_to_session(session_id, payload);
                            }
                        }
                        ClientAction::ReopenQuest { quest } => {
                            // C `questlog_reopen` (`src/system/questlog.c:613-826`):
                            // `sendquestlog` fires unconditionally once the
                            // generic preconditions pass (`Reopened`,
                            // `SeriesConflict`, and `NoEffect` all reach the
                            // per-quest switch), even when the switch leaves
                            // `ret` falsy and nothing actually reopens.
                            let result_and_payload = runtime.players.get_mut(&session_id).map(|player| {
                                let result = player.reopen_quest_legacy(quest as usize);
                                let payload = (!matches!(
                                    result,
                                    QuestReopenResult::CannotOpenAgain
                                        | QuestReopenResult::CannotOpenNow
                                        | QuestReopenResult::InvalidQuest
                                ))
                                .then(|| legacy_questlog_payload(player));
                                (result, payload)
                            });
                            if let Some((result, payload)) = result_and_payload {
                                if let Some(payload) = payload {
                                    runtime.send_to_session(session_id, payload);
                                }
                                // C `questlog_reopen` (`src/system/
                                // questlog.c:815-822`): when `ret` stayed
                                // truthy (our `Reopened` case) and the
                                // character is a player (always true here),
                                // `achievement_award(cn,
                                // ACHIEVEMENT_QUESTER, 1)` fires.
                                if matches!(result, QuestReopenResult::Reopened) {
                                    let name = world
                                        .characters
                                        .get(&character_id)
                                        .map(|character| character.name.clone());
                                    if let (Some(name), Some(player)) =
                                        (name, runtime.player_for_character_mut(character_id))
                                    {
                                        let now = current_unix_time();
                                        if player.achievement_data.award(
                                            AchievementType::Quester,
                                            &name,
                                            now,
                                        ) {
                                            let payload = achievement_unlock_payload(
                                                AchievementType::Quester,
                                                now,
                                            );
                                            for (sid, _) in
                                                runtime.sessions_for_character(character_id)
                                            {
                                                runtime.send_to_session(sid, payload.clone());
                                            }
                                            record_achievement_firsts_and_announce(
                                                &mut world,
                                                &achievement_repository,
                                                character_id,
                                                &name,
                                                &[AchievementType::Quester],
                                            )
                                            .await;
                                        }
                                    }
                                }
                                match result {
                                    QuestReopenResult::Reopened | QuestReopenResult::NoEffect => {}
                                    QuestReopenResult::SeriesConflict => command_feedback.push((
                                        character_id,
                                        "Cannot re-open more than one quest from a series."
                                            .to_string(),
                                    )),
                                    QuestReopenResult::CannotOpenAgain => command_feedback.push((
                                        character_id,
                                        "You cannot open this quest again.".to_string(),
                                    )),
                                    QuestReopenResult::CannotOpenNow => command_feedback.push((
                                        character_id,
                                        "You cannot open this quest at the moment.".to_string(),
                                    )),
                                    QuestReopenResult::InvalidQuest => {}
                                }
                            }
                        }
                        ClientAction::Ping { value } => {
                            // C `cl_ping` (`src/system/player.c`) blindly
                            // echoes the client's opaque 4-byte value back
                            // prefixed with `SV_PING` - no character/world
                            // state involved, pure transport round trip.
                            let mut builder = PacketBuilder::new();
                            builder.ping(value);
                            runtime.send_to_session(session_id, builder.into_payload());
                        }
                        ClientAction::Nop => {
                            // C `cl_nop` (`src/system/player.c`) is a
                            // genuine no-op used only as a keep-alive
                            // filler packet - no logging in C either.
                        }
                        ClientAction::ClientInfo(_) => {
                            // C `cl_clientinfo` (`src/system/player.c`)
                            // has its entire body commented out: the
                            // `client_info` payload (skip/idle counters,
                            // sysmem/vidmem, display surfaces) is parsed
                            // off the wire and discarded. Matches C.
                        }
                        ClientAction::Log(bytes) => {
                            // C `cl_log` (`src/system/player.c`) writes
                            // the client-supplied message to the server
                            // log via `charlog`. Port that as a `debug`
                            // trace line instead of silently dropping it.
                            let name = world
                                .characters
                                .get(&character_id)
                                .map(|character| character.name.as_str())
                                .unwrap_or("ILLEGAL CN");
                            let message = String::from_utf8_lossy(&bytes);
                            debug!(
                                target: "client_log",
                                "{}",
                                format_client_log_message(name, character_id.0, &message)
                            );
                        }
                        ClientAction::ModPacket {
                            packet_type,
                            subtype,
                            ..
                        } => {
                            // C `cl_mod1`/`cl_mod3` (`src/system/player.c`)
                            // route known handshake subtypes (0x01-0x0F:
                            // mod version/ready/pong) to a blind
                            // acknowledge ("For now, just acknowledge we
                            // received them"); other subtypes get an
                            // `SV_MOD1`/`SV_SYS_ERROR` reply via
                            // `mod_send_error_by_slot`, not ported yet.
                            // Log and no-op for now, matching the C
                            // oracle's own "future work" stub.
                            debug!(
                                character_id = character_id.0,
                                packet_type, subtype, "mod packet received (not yet implemented, logged no-op)"
                            );
                        }
                        _ => {}
                    }
                }
                if !command_feedback.is_empty() || !command_feedback_bytes.is_empty() || !command_inventory_refresh.is_empty() || !command_container_refresh.is_empty() || !command_name_refresh.is_empty() {
                    let mut feedback_sessions = 0;
                    for (character_id, message) in command_feedback {
                        let payload = ugaris_protocol::packet::system_text(&message);
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                feedback_sessions += 1;
                            }
                        }
                    }
                    for (character_id, message) in command_feedback_bytes {
                        let payload = ugaris_protocol::packet::system_text_bytes(&message);
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                feedback_sessions += 1;
                            }
                        }
                    }
                    let mut inventory_sessions = 0;
                    command_inventory_refresh.sort_unstable_by_key(|id| id.0);
                    command_inventory_refresh.dedup();
                    for character_id in command_inventory_refresh {
                        let Some(character) = world.characters.get(&character_id) else {
                            continue;
                        };
                        let payload = inventory_snapshot_payload(&world, character);
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                inventory_sessions += 1;
                            }
                        }
                    }
                    let mut container_sessions = 0;
                    command_container_refresh.sort_unstable_by_key(|id| id.0);
                    command_container_refresh.dedup();
                    for character_id in command_container_refresh {
                        // Active merchant stores refresh with prices instead
                        // of the ordinary container view.
                        world.check_merchant(character_id);
                        let payload = if world
                            .characters
                            .get(&character_id)
                            .is_some_and(|character| character.merchant.is_some())
                        {
                            merchant_store_payload(&mut world, character_id)
                        } else {
                            if !check_current_container(&mut world, character_id) {
                                continue;
                            }
                            current_container_payload(
                                &world,
                                runtime.account_depots.get(&character_id),
                                character_id,
                            )
                        };
                        let Some(payload) = payload else { continue };
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                container_sessions += 1;
                            }
                        }
                    }
                    let mut name_sessions = 0;
                    command_name_refresh.sort_unstable_by_key(|id| id.0);
                    command_name_refresh.dedup();
                    let pk_relations = PkRelationSnapshot::from_runtime(&runtime);
                    for character_id in command_name_refresh {
                        let Some(character) = world.characters.get(&character_id).cloned() else {
                            continue;
                        };
                        for (session_id, payload) in
                            runtime.refresh_known_character_name(&world, &pk_relations, &character)
                        {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                name_sessions += 1;
                            }
                        }
                    }
                    info!(feedback_sessions, inventory_sessions, container_sessions, name_sessions, tick = world.tick.0, "processed text/container commands");
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ChestTreasure { item_id, character_id, treasure_index } => {
                                            match apply_chest_treasure(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                item_id,
                                                character_id,
                                                treasure_index,
                                                realtime_seconds,
                                            ) {
                                                ChestTreasureApplyResult::Granted { item_name, key_name } => {
                                                    if let Some(key_name) = key_name {
                                                        feedback.push((character_id, format!("You use {key_name} to unlock the chest.")));
                                                    }
                                                    feedback.push((character_id, format!("You got a {item_name}.")));
                                                    executed += 1;
                                                    award_chest_opened_achievement(&mut world, &mut runtime, &achievement_repository, character_id, Some(treasure_index)).await;
                                                }
                                                ChestTreasureApplyResult::Empty => {
                                                    feedback.push((character_id, CHEST_EMPTY_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                ChestTreasureApplyResult::KeyRequired => {
                                                    feedback.push((character_id, CHEST_KEY_REQUIRED_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                ChestTreasureApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, CHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                ChestTreasureApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RandomChest { item_id, character_id } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0);
                                            match apply_random_chest(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                item_id,
                                                character_id,
                                                config.area_id,
                                                realtime_seconds,
                                                random_seed,
                                            ) {
                                                RandomChestApplyResult::Money { amount } => {
                                                    feedback.push((character_id, format!("You found some money ({:.2}G)!", f64::from(amount) / 100.0)));
                                                    executed += 1;
                                                    award_chest_opened_achievement(&mut world, &mut runtime, &achievement_repository, character_id, None).await;
                                                }
                                                RandomChestApplyResult::Item { item_name } => {
                                                    feedback.push((character_id, format!("You found a {item_name}.")));
                                                    executed += 1;
                                                    award_chest_opened_achievement(&mut world, &mut runtime, &achievement_repository, character_id, None).await;
                                                }
                                                RandomChestApplyResult::Empty => {
                                                    feedback.push((character_id, RANDCHEST_EMPTY_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                RandomChestApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, RANDCHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                RandomChestApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::RatChest { item_id, character_id } => {
                                            let random_seed = world.tick.0
                                                ^ (u64::from(item_id.0) << 16)
                                                ^ u64::from(character_id.0)
                                                ^ 0x5241_5443_4845_5354;
                                            match apply_rat_chest(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                item_id,
                                                character_id,
                                                config.area_id,
                                                realtime_seconds,
                                                random_seed,
                                            ) {
                                                RatChestApplyResult::Money { amount } => {
                                                    feedback.push((character_id, format!("You found some money ({:.2}G)!", f64::from(amount) / 100.0)));
                                                    executed += 1;
                                                }
                                                RatChestApplyResult::Treasure { item_name } => {
                                                    feedback.push((character_id, format!("You found a {item_name}.")));
                                                    executed += 1;
                                                }
                                                RatChestApplyResult::Empty => {
                                                    feedback.push((character_id, RANDCHEST_EMPTY_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                RatChestApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, RANDCHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                                                    blocked += 1;
                                                }
                                                RatChestApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawn { character_id, template, .. } => {
                                            match grant_ice_itemspawn_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                template,
                                            ) {
                                                IceItemSpawnGrantResult::Granted { item_name } => {
                                                    feedback.push((character_id, format!("You got a {item_name}.")));
                                                    executed += 1;
                                                }
                                                IceItemSpawnGrantResult::OneCarry { item_name } => {
                                                    feedback.push((character_id, format!("You can only carry one {item_name} at a time!")));
                                                    blocked += 1;
                                                }
                                                IceItemSpawnGrantResult::CannotCarry => {
                                                    blocked += 1;
                                                }
                                                IceItemSpawnGrantResult::Bug => {
                                                    feedback.push((
                                                        character_id,
                                                        "Congratulations, you have just discovered bug #4244C, please report it to the authorities!".to_string(),
                                                    ));
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnCursorOccupied { character_id, .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::WarmFireCursorOccupied { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Please empty your 'hand' (mouse cursor) first.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnBug { character_id, kind, .. } => {
                                            feedback.push((
                                                character_id,
                                                format!(
                                                    "Congratulations, you have just discovered bug #4244B-{kind}, please report it to the authorities!"
                                                ),
                                            ));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarmFire { character_id, create_scroll, removed_curse, .. } => {
                                            if create_scroll && grant_warmfire_scroll_to_cursor(&mut world, &mut zone_loader, character_id).is_some() {
                                                feedback.push((
                                                    character_id,
                                                    "Next to the fire, you find an ancient scroll. It seems to be a scroll of teleport which will take you back here.".to_string(),
                                                ));
                                            }
                                            if removed_curse {
                                                feedback.push((
                                                    character_id,
                                                    "You move close to the heat of the fire, and you feel the demon's cold leave you.".to_string(),
                                                ));
                                            } else {
                                                feedback.push((character_id, "You warm your hands on the fire.".to_string()));
                                            }
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BackToFire { character_id, .. } => {
                                            feedback.push((character_id, "The scroll vanished.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::MeltingKeyTick { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorKeyRequired { character_id, .. } => {
                                            feedback.push((character_id, "You need a key to open this gate.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorBusy { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "You hear fighting behind the door. It seems Islena is killing somebody else at the moment. Please come back later so she can take care of you, too.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorRespawning { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Islena is being re-incarnated. Please try again soon.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorResting { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Islena is resting after killing your predecessor. Being well mannered, you wait for her.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorTick { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChest { character_id, template, key_name, .. } => {
                                            match grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                template.as_str(),
                                            ) {
                                                Some(item_name) => {
                                                    if let Some(key_name) = key_name {
                                                        let key_name = outcome_item_name_text(&key_name);
                                                        feedback.push((character_id, format!("You use {key_name} to open the chest.")));
                                                    }
                                                    feedback.push((character_id, format!("You got a {item_name}.")));
                                                    executed += 1;
                                                }
                                                None => {
                                                    feedback.push((
                                                        character_id,
                                                        "Congratulations, you have just discovered bug #4744C, please report it to the authorities!".to_string(),
                                                    ));
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, CHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestKeyRequired { character_id, .. } => {
                                            feedback.push((character_id, CHEST_KEY_REQUIRED_MESSAGE.to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestUnknown { character_id, .. } => {
                                            feedback.push((
                                                character_id,
                                                "Congratulations, you have just discovered bug #4744B, please report it to the authorities!".to_string(),
                                            ));
                                            failed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonTeleport { item_id, character_id, x, y, .. } => {
                                            let teleported = world.teleport_character_same_area(character_id, x, y, false)
                                                || world.teleport_character_same_area(character_id, 240, 250, false)
                                                || world.teleport_character_same_area(character_id, 235, 250, false)
                                                || world.teleport_character_same_area(character_id, 230, 250, false);
                                            if teleported {
                                                world.destroy_item(item_id);
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonFake { item_id, .. } => {
                                            if world.destroy_item(item_id) {
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonKey { character_id, template, key_id, .. } => {
                                            match grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                template,
                                            ) {
                                                Some(_) => {
                                                    if let Some(cursor_item_id) = world
                                                        .characters
                                                        .get(&character_id)
                                                        .and_then(|character| character.cursor_item)
                                                    {
                                                        if let Some(cursor_item) = world.items.get_mut(&cursor_item_id) {
                                                            cursor_item.template_id = key_id;
                                                        }
                                                    }
                                                    executed += 1;
                                                }
                                                None => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonKeyCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your 'hand' (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorMissingKeys { character_id, missing, both_required, .. } => {
                                            if both_required {
                                                feedback.push((
                                                    character_id,
                                                    format!("You need {missing} more key{}.", if missing > 1 { "s" } else { "" }),
                                                ));
                                            } else {
                                                feedback.push((character_id, "You need a key.".to_string()));
                                            }
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorTooManyDefenders { character_id, alive, max_allowed, .. } => {
                                            feedback.push((
                                                character_id,
                                                format!("Too many Defenders are still alive ({alive} vs {max_allowed})."),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorSolved { .. } => {
                                            executed += 1;
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestChest { character_id, amount, imp_flag_mask, .. } => {
                                            match apply_forest_chest(
                                                &mut world,
                                                &mut zone_loader,
                                                runtime.player_for_character_mut(character_id),
                                                character_id,
                                                amount,
                                                imp_flag_mask,
                                            ) {
                                                ForestChestApplyResult::FoundMoney { .. } => {
                                                    feedback.push((character_id, "You found a nice sum of money!".to_string()));
                                                    executed += 1;
                                                }
                                                ForestChestApplyResult::Empty => {
                                                    feedback.push((character_id, "The chest is empty.".to_string()));
                                                    blocked += 1;
                                                }
                                                ForestChestApplyResult::CursorOccupied => {
                                                    feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                                    blocked += 1;
                                                }
                                                ForestChestApplyResult::MissingPlayer => {
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestChestCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::ForestChestLocked { character_id, .. } => {
                                            feedback.push((character_id, "The chest is locked and you don't have the right key.".to_string()));
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
                                        ugaris_core::item_driver::ItemDriverOutcome::PickChest { character_id, template, .. } => {
                                            match grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                template.as_str(),
                                            ) {
                                                Some(item_name) => {
                                                    world.notify_twocity_pick_from_character(character_id);
                                                    feedback.push((character_id, "You pick the lock.".to_string()));
                                                    feedback.push((character_id, format!("You found a {}.", item_name.to_ascii_lowercase())));
                                                    executed += 1;
                                                }
                                                None => {
                                                    feedback.push((character_id, "You've found bug #8331.".to_string()));
                                                    failed += 1;
                                                }
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickChestCursorOccupied { character_id, .. } => {
                                            feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickChestLocked { item_id, character_id } => {
                                            let item_name = world
                                                .items
                                                .get(&item_id)
                                                .map(|item| item.name.to_ascii_lowercase())
                                                .unwrap_or_else(|| "chest".to_string());
                                            feedback.push((character_id, format!("The {item_name} is locked and you don't have the right key.")));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::PickChestBug { character_id, .. } => {
                                            feedback.push((character_id, "You've found bug #8331.".to_string()));
                                            failed += 1;
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
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArena { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExit { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaNeedsSuit { character_id, .. } => {
                                            feedback.push((character_id, "You need to wear an earth demon suit.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaLevelTooHigh { character_id, .. } => {
                                            feedback.push((character_id, "Max Level 38, sorry.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentEnhanced { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentBound { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaBusy { character_id, .. } => {
                                            feedback.push((character_id, "Please try again soon. Target is busy.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExitLowHealth { character_id, .. } => {
                                            feedback.push((character_id, "You cannot leave with less than full health.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoor { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoHumans { character_id, .. } => {
                                            feedback.push((character_id, "A demon looks through the view-hole in the door and shouts: \"No humans allowed!\"".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoBeggars { character_id, .. } => {
                                            feedback.push((character_id, "A demon looks through the view-hole in the door and shouts: \"No beggars allowed!\"".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorOnlyNobles { character_id, .. } => {
                                            feedback.push((character_id, "A demon looks through the view-hole in the door and shouts: \"Only nobles allowed!\"".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBusy { character_id, .. } => {
                                            feedback.push((character_id, "Please try again soon. Target is busy.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBug { character_id, x, y, .. } => {
                                            feedback.push((character_id, format!("You touch a teleport object but nothing happens - BUG ({x},{y}).")));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestSpawn { item_id, level, template, schedule_after_ticks, .. } => {
                                            world.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                                            if spawn_teufel_ratnest_character(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                item_id,
                                                level,
                                                template,
                                            ) {
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestDestroyed { character_id, .. } => {
                                            feedback.push((character_id, "You destroy the rat nest.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestGuarded { character_id, .. } => {
                                            feedback.push((character_id, "You need a moment of peace to destroy the nest. There is still a guard left, distracting you.".to_string()));
                                            blocked += 1;
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
                                                TransportTravelResult::CrossArea { .. } => {
                                                    feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                    blocked += 1;
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExit { character_id, area_id, .. } => {
                                            if area_id != config.area_id {
                                                feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                blocked += 1;
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnAward { character_id, .. } => {
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn { item_id, character_id: _, template, x, y, .. } => {
                                            if spawn_chestspawn_character(
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
                                        ugaris_core::item_driver::ItemDriverOutcome::ChestSpawnCheck { .. } => {}
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
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportMissingSphere {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Nothing happened.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBug {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "You found BUG #31as5.".to_string()));
                                            feedback.push((character_id, "Target is busy, please try again soon.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBusy {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Target is busy, please try again soon.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportSpheres {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Your spheres vanished.".to_string()));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusFinished {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "You're done. Finished. It's over. You're there. You've solved the final level."
                                                    .to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusAlreadyUsed {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((character_id, "Nothing happened.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusNeedsSphere {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Nothing happened. You sense that you'll need one of the spheres this time."
                                                    .to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpBonus {
                                            character_id,
                                            location_id,
                                            base,
                                            next_points,
                                            advanced,
                                            reward_sphere_kind,
                                            reward_level,
                                            ..
                                        } => {
                                            if let Some(player) = runtime.player_for_character_mut(character_id) {
                                                if player.warp_base <= 0 {
                                                    player.warp_base = 40;
                                                }
                                                let slot = player
                                                    .warp_bonus_ids
                                                    .iter()
                                                    .position(|stored| *stored == location_id as i32)
                                                    .or_else(|| {
                                                        player
                                                            .warp_bonus_last_used
                                                            .iter()
                                                            .enumerate()
                                                            .min_by_key(|(_, used)| **used)
                                                            .map(|(index, _)| index)
                                                    });
                                                if let Some(slot) = slot {
                                                    if slot >= player.warp_bonus_ids.len() {
                                                        player.warp_bonus_ids.resize(slot + 1, 0);
                                                    }
                                                    if slot >= player.warp_bonus_last_used.len() {
                                                        player.warp_bonus_last_used.resize(slot + 1, 0);
                                                    }
                                                    player.warp_bonus_ids[slot] = location_id as i32;
                                                    player.warp_bonus_last_used[slot] = base as i32;
                                                }
                                                player.warp_points = next_points as i32;
                                                if advanced {
                                                    player.warp_base = base as i32 + 5;
                                                    player.warp_nostepexp = 0;
                                                    if player.warp_base > 139 {
                                                        feedback.push((
                                                            character_id,
                                                            "You've finished the final level.".to_string(),
                                                        ));
                                                    } else if player.warp_base > 134 {
                                                        feedback.push((
                                                            character_id,
                                                            "You've reached the final level.".to_string(),
                                                        ));
                                                    } else {
                                                        feedback.push((
                                                            character_id,
                                                            "You advanced a level! Take care!".to_string(),
                                                        ));
                                                    }
                                                }
                                                let current_base = player.warp_base.max(40) as u32;
                                                let current_points = player.warp_points.max(0) as u32;
                                                let no_step_exp = player.warp_nostepexp != 0;
                                                if advanced {
                                                    // C `warpbonus_driver` (`area/25/warped.c:423-449`)
                                                    // grants the sphere-kind-1 case's exp via
                                                    // `give_exp(cn, ...)`, not a raw mutation.
                                                    match reward_sphere_kind {
                                                        Some(1) => {
                                                            world.give_exp(
                                                                character_id,
                                                                i64::from(level_value(reward_level) / 7),
                                                                u32::from(args.area_id),
                                                            );
                                                            feedback.push((
                                                                character_id,
                                                                "You received experience.".to_string(),
                                                            ));
                                                        }
                                                        Some(2) => {
                                                            if let Some(character) =
                                                                world.characters.get_mut(&character_id)
                                                            {
                                                                if character.saves < 10
                                                                    && !character
                                                                        .flags
                                                                        .contains(CharacterFlags::HARDCORE)
                                                                {
                                                                    character.saves += 1;
                                                                    feedback.push((
                                                                        character_id,
                                                                        "You received a save.".to_string(),
                                                                    ));
                                                                }
                                                            }
                                                        }
                                                        Some(3) => {
                                                            // C `warpbonus_driver` (`area/25/
                                                            // warped.c:432-434`): `log_char(cn, ...,
                                                            // "You received military rank.");
                                                            // give_military_pts_no_npc(cn, level, 0);`
                                                            // - the fixed message first, then the
                                                            // shared point-award/promotion helper
                                                            // (`World::give_military_pts`, `crates/
                                                            // ugaris-core/src/world/military.rs`),
                                                            // which queues its own "You've been
                                                            // promoted..." feedback (and the above-
                                                            // Sergeant-Major server broadcast) if the
                                                            // grant crosses a rank threshold.
                                                            feedback.push((
                                                                character_id,
                                                                "You received military rank.".to_string(),
                                                            ));
                                                            world.give_military_pts(
                                                                character_id,
                                                                reward_level as i32,
                                                                0,
                                                                u32::from(args.area_id),
                                                            );
                                                        }
                                                        Some(4) => {
                                                            // C `warpbonus_driver` (`area/25/
                                                            // warped.c:434-436`): `give_money(cn,
                                                            // level * level * 10, "Warped area
                                                            // reward")`.
                                                            achievement::give_money(
                                                                &mut world,
                                                                &mut runtime,
                                                                &achievement_repository,
                                                                character_id,
                                                                reward_level
                                                                    .saturating_mul(reward_level)
                                                                    .saturating_mul(10),
                                                                &mut feedback_bytes,
                                                            )
                                                            .await;
                                                        }
                                                        Some(5) => {
                                                            if grant_template_item_smart(
                                                                &mut world,
                                                                &mut zone_loader,
                                                                character_id,
                                                                "lollipop",
                                                            )
                                                            .is_some()
                                                            {
                                                                feedback.push((
                                                                    character_id,
                                                                    "You received a lollipop.".to_string(),
                                                                ));
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                } else if !no_step_exp {
                                                    // C `warpbonus_driver` (`area/25/warped.c:453`)
                                                    // grants the step exp via `give_exp(cn, ...)`.
                                                    world.give_exp(
                                                        character_id,
                                                        i64::from(level_value(reward_level) / 70),
                                                        u32::from(args.area_id),
                                                    );
                                                }
                                                if current_base <= 139 {
                                                    feedback.push((
                                                        character_id,
                                                        format!(
                                                            "You are at level {}, and you have {} of {} points.",
                                                            (current_base - 35) / 5,
                                                            current_points,
                                                            current_base / 4
                                                        ),
                                                    ));
                                                }
                                                executed += 1;
                                            } else {
                                                failed += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawnCursorOccupied {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Please empty your hand (mouse cursor) first.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawn {
                                            character_id,
                                            sphere_kind,
                                            ..
                                        } => {
                                            let template = format!("warped_teleport_key{sphere_kind}");
                                            if grant_template_item_to_cursor(
                                                &mut world,
                                                &mut zone_loader,
                                                character_id,
                                                &template,
                                            )
                                            .is_some()
                                            {
                                                feedback.push((
                                                    character_id,
                                                    "You got a glowing half sphere.".to_string(),
                                                ));
                                                executed += 1;
                                            } else {
                                                feedback.push((
                                                    character_id,
                                                    "It won't come off.".to_string(),
                                                ));
                                                blocked += 1;
                                            }
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorMissingKey {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "The door is locked and you do not have the right key.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorBug {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Bug #329i, sorry.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoor {
                                            character_id,
                                            key_name,
                                            ..
                                        } => {
                                            let key_name = outcome_item_name_text(&key_name);
                                            feedback.push((
                                                character_id,
                                                format!("A {key_name} vanished."),
                                            ));
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorWrongSide {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "You cannot open the door from this side.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBusy {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "You hear fighting noises and the door won't open.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBug {
                                            character_id,
                                            ..
                                        } => {
                                            feedback.push((
                                                character_id,
                                                "Bug #319i, sorry.".to_string(),
                                            ));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoor {
                                            character_id,
                                            spawn_x,
                                            spawn_y,
                                            template,
                                            ..
                                        } => {
                                            if spawn_warp_trial_fighter(
                                                &mut world,
                                                &mut zone_loader,
                                                &mut runtime,
                                                template,
                                                spawn_x,
                                                spawn_y,
                                            ) {
                                                executed += 1;
                                            } else {
                                                feedback.push((
                                                    character_id,
                                                    "Bug #319i, sorry.".to_string(),
                                                ));
                                                failed += 1;
                                            }
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
                                        ugaris_core::item_driver::ItemDriverOutcome::MineGateway { character_id, area_id, .. } => {
                                            if area_id != config.area_id {
                                                feedback.push((character_id, "Nothing happens - target area server is down.".to_string()));
                                                blocked += 1;
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
                            let Some(depot) = runtime.account_depots.get(&character_id) else {
                                continue;
                            };
                            let payload = account_depot_payload(depot);
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

                let npc_message_characters: Vec<_> = world
                    .characters
                    .iter()
                    .filter_map(|(&character_id, character)| {
                        (!character.driver_messages.is_empty()
                            && (character.driver == CDR_SIMPLEBADDY
                                || character.driver == CDR_LAB2UNDEAD
                                || matches!(
                                    character.driver_state.as_ref(),
                                    Some(
                                        CharacterDriverState::SimpleBaddy(_)
                                            | CharacterDriverState::Lab2Undead(_)
                                    )
                                )))
                        .then_some(character_id)
                    })
                    .collect();
                if !npc_message_characters.is_empty() {
                    let mut simple_baddy_outcomes = 0;
                    let mut lab2_undead_outcomes = 0;
                    for character_id in npc_message_characters {
                        let driver_state = world
                            .characters
                            .get(&character_id)
                            .and_then(|character| character.driver_state.as_ref())
                            .cloned();
                        match driver_state {
                            Some(CharacterDriverState::SimpleBaddy(_)) => {
                                simple_baddy_outcomes += world
                                    .process_simple_baddy_message_actions(character_id, config.area_id)
                                    .len();
                            }
                            Some(CharacterDriverState::Lab2Undead(_)) => {
                                lab2_undead_outcomes +=
                                    world.process_lab2_undead_message_actions(character_id);
                            }
                            _ => {
                                if world
                                    .characters
                                    .get(&character_id)
                                    .is_some_and(|character| character.driver == CDR_SIMPLEBADDY)
                                {
                                    simple_baddy_outcomes += world
                                        .process_simple_baddy_message_actions(
                                            character_id,
                                            config.area_id,
                                        )
                                        .len();
                                } else if world
                                    .characters
                                    .get(&character_id)
                                    .is_some_and(|character| character.driver == CDR_LAB2UNDEAD)
                                {
                                    lab2_undead_outcomes +=
                                        world.process_lab2_undead_message_actions(character_id);
                                }
                            }
                        }
                    }
                    info!(
                        simple_baddy_outcomes,
                        lab2_undead_outcomes,
                        tick = world.tick.0,
                        "processed NPC driver messages"
                    );
                }

                let simple_baddy_attacks = world.process_simple_baddy_attack_actions_with_random(
                    config.area_id,
                    |limit| runtime_random_below(limit as i32).max(0) as u32,
                );
                if simple_baddy_attacks != 0 {
                    info!(simple_baddy_attacks, tick = world.tick.0, "queued simple-baddy attack actions");
                }

                let simple_baddy_noncombat = world
                    .process_simple_baddy_noncombat_actions_with_completions(
                        config.area_id,
                        &completed_actions,
                    );
                if simple_baddy_noncombat != 0 {
                    info!(simple_baddy_noncombat, tick = world.tick.0, "queued simple-baddy noncombat actions");
                }

                let lab2_undead_cathedral = world.process_lab2_undead_cathedral_self_destructions();
                if lab2_undead_cathedral != 0 {
                    info!(
                        lab2_undead_cathedral,
                        tick = world.tick.0,
                        "processed Lab 2 undead cathedral self-destruction"
                    );
                }

                let lab2_undead_crypt_doors = world.process_lab2_undead_crypt_door_actions();
                if lab2_undead_crypt_doors != 0 {
                    info!(
                        lab2_undead_crypt_doors,
                        tick = world.tick.0,
                        "processed Lab 2 undead crypt door closures"
                    );
                }

                let lab2_undead_patrol = world.process_lab2_undead_patrol_actions(config.area_id);
                if lab2_undead_patrol != 0 {
                    info!(lab2_undead_patrol, tick = world.tick.0, "queued Lab 2 undead patrol actions");
                }

                // C merchant_driver: store creation, greetings, and trade
                // activation, then push store views to players whose active
                // merchant changed.
                let merchants_before_tick: std::collections::HashSet<CharacterId> =
                    world.merchant_stores.keys().copied().collect();
                world.process_merchant_actions();
                // C `aclerk_driver` (`CDR_ACLERK`): the Cameron arena
                // clerk's store creation, welcome greeting, and idle chatter
                // (`src/module/merchants/merchant.c`).
                world.process_aclerk_actions();
                // C `bank_driver`: greetings, small talk, and
                // deposit/withdraw/balance text commands (`src/module/
                // bank.c`).
                world.process_bank_actions(config.area_id);
                let bank_events_applied = apply_bank_events(&mut runtime, &mut world);
                if bank_events_applied != 0 {
                    info!(bank_events_applied, tick = world.tick.0, "applied bank account events");
                }
                // C `trader_driver`: player-to-player trade middleman NPC
                // (`src/module/base.c`).
                world.process_trader_actions();
                let trader_events_applied =
                    apply_trader_events(&mut world, &mut runtime, &achievement_repository).await;
                if trader_events_applied != 0 {
                    info!(trader_events_applied, tick = world.tick.0, "applied trader item-look events");
                }
                // C `clanmaster_driver`: the clan foundations NPC
                // (`src/area/30/clanmaster.c`).
                world.process_clanmaster_actions(config.area_id, current_unix_time());
                let clanmaster_events_applied = apply_clanmaster_events(
                    &mut world,
                    &mut runtime,
                    &achievement_repository,
                    &clan_log_repository,
                    &character_repository,
                    current_unix_time(),
                )
                .await;
                if clanmaster_events_applied != 0 {
                    info!(
                        clanmaster_events_applied,
                        tick = world.tick.0,
                        "applied clanmaster founding/membership events"
                    );
                }
                // C `clanclerk_driver`: the clan administration/treasury
                // NPC (`src/area/30/clanmaster.c`).
                world.process_clanclerk_actions(config.area_id, current_unix_time());
                let clanclerk_events_applied =
                    apply_clanclerk_events(&mut world, &clan_log_repository, current_unix_time())
                        .await;
                // C `clubmaster_driver`: the club foundations/
                // administration NPC (`src/system/clubmaster.c`).
                world.process_clubmaster_actions(config.area_id, current_unix_time());
                let clubmaster_events_applied = apply_clubmaster_events(
                    &mut world,
                    &mut runtime,
                    &achievement_repository,
                    &character_repository,
                )
                .await;
                if clubmaster_events_applied != 0 {
                    info!(
                        clubmaster_events_applied,
                        tick = world.tick.0,
                        "applied clubmaster founding/membership events"
                    );
                }
                // C `military_master_driver`: the mission-giving Military
                // Master NPC (`src/module/military.c`).
                world.process_military_master_actions(config.area_id, current_unix_time());
                let military_master_events_applied = apply_military_master_events(
                    &mut world,
                    &mut runtime,
                    &achievement_repository,
                    config.area_id,
                )
                .await;
                if military_master_events_applied != 0 {
                    info!(
                        military_master_events_applied,
                        tick = world.tick.0,
                        "applied military master mission-dialogue events"
                    );
                }
                // C `military_advisor_driver`: the paid mission-
                // recommendation Military Advisor NPC
                // (`src/module/military.c`).
                world.process_military_advisor_actions(config.area_id);
                let military_advisor_events_applied =
                    apply_military_advisor_events(&mut world, &mut runtime);
                if military_advisor_events_applied != 0 {
                    info!(
                        military_advisor_events_applied,
                        tick = world.tick.0,
                        "applied military advisor favor/mission-recommendation events"
                    );
                }
                if clanclerk_events_applied != 0 {
                    info!(
                        clanclerk_events_applied,
                        tick = world.tick.0,
                        "applied clanclerk treasury/admin events"
                    );
                }
                // C `tick_clan` states 3/4 (`clan.c:358-436,936-1182`):
                // the daily relation escalation/de-escalation tick, the
                // weekly treasury tick (bonus affordability, upkeep,
                // debt, bankrupt-clan deletion), and the hourly dungeon
                // training-score decay tick.
                let clan_economy_events_applied =
                    apply_clan_economy_tick(&mut world, &clan_log_repository, current_unix_time())
                        .await;
                if clan_economy_events_applied != 0 {
                    info!(
                        clan_economy_events_applied,
                        tick = world.tick.0,
                        "applied clan relation/treasury economy events"
                    );
                }
                // C `master_driver`: the arena tournament master NPC
                // (`src/system/arena.c`) - pairs registered contenders,
                // watches the fight, and (via `apply_arena_master_events`
                // below) scores the result.
                world.process_arena_master_actions(config.area_id, |character_id| {
                    runtime
                        .player_for_character(character_id)
                        .map(|player| player.arena_score())
                        .unwrap_or(ARENA_PPD_NEWCOMER_SCORE)
                });
                // C `fighter_driver`: the autonomous tournament practice-bot
                // (`CDR_ARENAFIGHTER`) - walks home/to the master, registers/
                // enters/fights on its own, entirely self-contained within
                // `World` (its own local win/loss ledger lives on
                // `ArenaFighterDriverData`, not `PlayerRuntime`).
                world.process_arena_fighter_actions(config.area_id);
                // C `manager_driver`: the arena-rental NPC (`CDR_ARENAMANAGER`)
                // - `rent`/`invite:`/`enter`/`leave`, entirely self-contained
                // within `World` (never touches `PlayerRuntime`).
                world.process_arena_manager_actions(config.area_id);
                let arena_master_events_applied =
                    apply_arena_master_events(&mut world, &mut runtime, current_unix_time());
                if arena_master_events_applied != 0 {
                    info!(
                        arena_master_events_applied,
                        tick = world.tick.0,
                        "applied arena tournament fight-scoring events"
                    );
                }
                // C `gate_welcome_driver`: the Ishtar labyrinth gatekeeper
                // greeter NPC (`src/system/gatekeeper.c`).
                let gate_welcome_facts = gate_welcome_player_facts(&runtime);
                let gate_welcome_events =
                    world.process_gate_welcome_actions(&gate_welcome_facts, config.area_id);
                let gate_welcome_events_applied = apply_gate_welcome_events(
                    &mut runtime,
                    &mut world,
                    &mut zone_loader,
                    gate_welcome_events,
                );
                if gate_welcome_events_applied != 0 {
                    info!(
                        gate_welcome_events_applied,
                        tick = world.tick.0,
                        "applied gate-welcome dialogue state updates"
                    );
                }
                // C `gate_fight_driver`: the private-room duel opponent
                // spawned by `EnterTestReady` (`src/system/gatekeeper.c`).
                // Its `gate_fight_dead` death-reward tail is wired via
                // `apply_gate_fight_death_from_hurt_event`, called from
                // `apply_pk_hate_from_hurt_events` below.
                let gate_fight_acted = world.process_gate_fight_actions(config.area_id);
                if gate_fight_acted != 0 {
                    info!(gate_fight_acted, tick = world.tick.0, "processed gate-fight opponent actions");
                }
                // C `janitor_driver`: lamp-lighting/item-tidying NPC
                // (`src/module/base.c`).
                world.process_janitor_actions(config.area_id);
                // C `merchant_driver`: seed/refresh "special" enchanted-item
                // stock (`add_special_store`, every 12h).
                let special_store_updates = world.refresh_special_stores(&mut zone_loader);
                for merchant_id in special_store_updates {
                    save_merchant_store_if_configured(&world, &merchant_repository, merchant_id)
                        .await;
                }
                if let Some(repository) = &merchant_repository {
                    // C `create_store`: `load_merchant_inventory` on first
                    // creation, or an initial `queue_merchant_full_save` if
                    // nothing was persisted yet for this merchant.
                    let newly_created_stores: Vec<CharacterId> = world
                        .merchant_stores
                        .keys()
                        .copied()
                        .filter(|id| !merchants_before_tick.contains(id))
                        .collect();
                    for merchant_id in newly_created_stores {
                        let Some((name, x, y)) = world
                            .characters
                            .get(&merchant_id)
                            .map(|merchant| (merchant.name.clone(), merchant.x, merchant.y))
                        else {
                            continue;
                        };
                        match repository.load_store(&name, i32::from(x), i32::from(y)).await {
                            Ok(Some(snapshot)) => {
                                apply_merchant_store_snapshot(&mut world, merchant_id, snapshot);
                                info!(merchant = %name, x, y, "loaded merchant store from database");
                            }
                            Ok(None) => {
                                if let Some(snapshot) =
                                    merchant_store_snapshot(&world, merchant_id)
                                {
                                    match repository.save_store(&snapshot).await {
                                        Ok(()) => info!(merchant = %name, x, y, "saved initial merchant store to database"),
                                        Err(err) => warn!(merchant = %name, x, y, error = %err, "failed to save initial merchant store"),
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(merchant = %name, x, y, error = %err, "failed to load merchant store from database");
                            }
                        }
                    }
                }
                // C `maintenance_60s_task` (`server.c:197-210`):
                // `update_auction_house()` delivers expired auctions'
                // items/gold to their winners (or returns them to the
                // seller if unsold) roughly once a minute, not every tick.
                if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 {
                    if let Some(repository) = &auction_repository {
                        match repository.cleanup_expired_auctions().await {
                            Ok(processed) if processed > 0 => {
                                info!(processed, tick = world.tick.0, "processed expired auctions");
                            }
                            Ok(_) => {}
                            Err(err) => {
                                warn!(error = %err, "failed to process expired auctions");
                            }
                        }
                    }
                }
                // Restart-persistence for `world.clan_registry`: C has no
                // direct equivalent (its clan data rides along inside the
                // whole-server memory-image save, not a dedicated flush
                // task), so this reuses the same once-a-minute cadence as
                // the auction/play-time maintenance above rather than
                // C's own `update_state`-driven storage state machine.
                // Gated on `ClanRegistry::dirty` (mirroring C's own
                // `clan_changed` check, `clan.c:415-418`) so an unchanged
                // registry doesn't get rewritten every minute.
                if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 && world.clan_registry.dirty() {
                    if let Some(repository) = &clan_repository {
                        match repository.save_registry(&world.clan_registry).await {
                            Ok(()) => world.clan_registry.clear_dirty(),
                            Err(err) => {
                                warn!(error = %err, "failed to save clan registry to database")
                            }
                        }
                    }
                }
                // Restart-persistence for `world.military_master_storage`:
                // same once-a-minute cadence and `dirty`-gating as the
                // clan registry save above, for the same reason (no C
                // equivalent flush task to mirror - see
                // `crates/ugaris-db/src/military.rs`'s doc comment).
                if world.tick.0 % (TICKS_PER_SECOND * 60) == 0
                    && world.military_master_storage.dirty()
                {
                    if let Some(repository) = &military_master_storage_repository {
                        match repository
                            .save_registry(&world.military_master_storage)
                            .await
                        {
                            Ok(()) => world.military_master_storage.clear_dirty(),
                            Err(err) => {
                                warn!(error = %err, "failed to save military master storage registry to database")
                            }
                        }
                    }
                }
                // Restart-persistence for `world.military_advisor_storage`:
                // same once-a-minute cadence and `dirty`-gating as the
                // Military Master storage save above.
                if world.tick.0 % (TICKS_PER_SECOND * 60) == 0
                    && world.military_advisor_storage.dirty()
                {
                    if let Some(repository) = &military_advisor_storage_repository {
                        match repository
                            .save_registry(&world.military_advisor_storage)
                            .await
                        {
                            Ok(()) => world.military_advisor_storage.clear_dirty(),
                            Err(err) => {
                                warn!(error = %err, "failed to save military advisor storage registry to database")
                            }
                        }
                    }
                }
                // C `player_update` (`player.c:3448-3462`): every player
                // slot gets `achievement_add_play_time(cn, 1)` (plus
                // `stats_update`, unported - see PORTING_TODO.md) once
                // per real-time minute, staggered across ticks via
                // `nr % (TICKS * 60)`. Rust has no stable per-player slot
                // index to replicate that stagger, so this fires for all
                // logged-in characters on the same once-a-minute tick
                // gate already used for auction cleanup above - same net
                // rate (1 minute credited per minute of uptime), just
                // synchronized instead of spread across the 60 ticks.
                if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 {
                    let play_time_characters: Vec<CharacterId> = runtime
                        .players
                        .values()
                        .filter_map(|player| player.character_id)
                        .collect();
                    for character_id in play_time_characters {
                        award_play_time_minute(&mut world, &mut runtime, &achievement_repository, character_id).await;
                    }
                }
                // C `tick_player`'s deferred-init sweep (`player.c:3660-
                // 3676`): `ticks >= 2 && (deferred_init &
                // DEFERRED_ACHIEVEMENTS)` fires `achievement_sync_all` +
                // `achievement_award(ACHIEVEMENT_STARTED_UGARIS)` +
                // `achievement_check_level`/`_exploration`/
                // `_login_streak`. Each newly-unlocked achievement sends
                // its own `SV_ACH_UNLOCK` (`achievement_send_to_client`,
                // called from inside C's `achievement_award`).
                // `DEFERRED_MOTD`'s own gate is not ported here - MOTD
                // doesn't exist yet (see PORTING_TODO.md).
                {
                    let due_achievement_notices: Vec<CharacterId> = runtime
                        .players
                        .values()
                        .filter(|player| {
                            player.deferred_init & DEFERRED_ACHIEVEMENTS != 0
                                && world.tick.0.saturating_sub(player.login_tick) >= 2
                        })
                        .filter_map(|player| player.character_id)
                        .collect();
                    for character_id in due_achievement_notices {
                        let character_info = world.characters.get(&character_id).map(|character| {
                            (
                                character.name.clone(),
                                character.level as i32,
                                character.flags.contains(CharacterFlags::HARDCORE),
                            )
                        });
                        let area_id = world.area_id as i32;
                        let mut payloads: Vec<bytes::BytesMut> = Vec::new();
                        if let Some(player) = runtime.player_for_character_mut(character_id) {
                            player.deferred_init &= !DEFERRED_ACHIEVEMENTS;
                            payloads = achievement_sync_payloads(
                                &player.achievement_data,
                                &player.achievement_stats,
                            );
                            if let Some((name, level, is_hardcore)) = character_info {
                                let now = current_unix_time();
                                let mut unlocked = Vec::new();
                                if player.achievement_data.award(
                                    AchievementType::StartedUgaris,
                                    &name,
                                    now,
                                ) {
                                    unlocked.push(AchievementType::StartedUgaris);
                                }
                                unlocked.extend(check_level(
                                    &mut player.achievement_data,
                                    level,
                                    is_hardcore,
                                    &name,
                                    now,
                                ));
                                unlocked.extend(check_exploration(
                                    &mut player.achievement_data,
                                    area_id,
                                    &name,
                                    now,
                                ));
                                unlocked.extend(check_login_streak(
                                    &mut player.achievement_data,
                                    &mut player.achievement_stats,
                                    &name,
                                    now,
                                ));
                                if !unlocked.is_empty() {
                                    record_achievement_firsts_and_announce(
                                        &mut world,
                                        &achievement_repository,
                                        character_id,
                                        &name,
                                        &unlocked,
                                    )
                                    .await;
                                }
                                for ty in unlocked {
                                    payloads.push(achievement_unlock_payload(ty, now));
                                }
                            }
                        }
                        for payload in payloads {
                            for (session_id, _) in runtime.sessions_for_character(character_id) {
                                runtime.send_to_session(session_id, payload.clone());
                            }
                        }
                    }
                }
                if let Some(repository) = &auction_repository {
                    let due_auction_notices: Vec<CharacterId> = runtime
                        .players
                        .values()
                        .filter(|player| {
                            player.deferred_init & DEFERRED_AUCTION != 0
                                && world.tick.0.saturating_sub(player.login_tick) >= 6
                        })
                        .filter_map(|player| player.character_id)
                        .collect();
                    for character_id in due_auction_notices {
                        if let Some(player) = runtime.player_for_character_mut(character_id) {
                            player.deferred_init &= !DEFERRED_AUCTION;
                        }
                        match auction::auction_login_notice(repository, character_id).await {
                            Ok(Some(message)) => {
                                let payload = ugaris_protocol::packet::system_text_bytes(&message);
                                for (session_id, _) in runtime.sessions_for_character(character_id)
                                {
                                    runtime.send_to_session(session_id, payload.clone());
                                }
                            }
                            Ok(None) => {}
                            Err(err) => {
                                warn!(character_id = character_id.0, error = %err, "failed to check pending auction deliveries");
                            }
                        }
                    }
                }
                {
                    let mut merchant_view_updates: Vec<(CharacterId, Option<bytes::BytesMut>)> =
                        Vec::new();
                    let session_characters: Vec<CharacterId> = runtime
                        .players
                        .values()
                        .filter_map(|player| player.character_id)
                        .collect();
                    for character_id in session_characters {
                        world.check_merchant(character_id);
                        let current = world
                            .characters
                            .get(&character_id)
                            .and_then(|character| character.merchant);
                        let cached = runtime.merchant_views.get(&character_id).copied();
                        match (current, cached) {
                            (Some(merchant_id), cached) if cached != Some(merchant_id) => {
                                runtime.merchant_views.insert(character_id, merchant_id);
                                merchant_view_updates.push((
                                    character_id,
                                    merchant_store_payload(&mut world, character_id),
                                ));
                            }
                            (None, Some(_)) => {
                                runtime.merchant_views.remove(&character_id);
                                merchant_view_updates
                                    .push((character_id, Some(container_close_payload())));
                            }
                            _ => {}
                        }
                    }
                    for (character_id, payload) in merchant_view_updates {
                        let Some(payload) = payload else { continue };
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            runtime.send_to_session(session_id, payload.clone());
                        }
                    }
                }

                let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                let pk_hate_updates = apply_pk_hate_from_hurt_events(
                    &mut runtime,
                    &mut world,
                    realtime_seconds,
                    &zone_loader,
                );
                if pk_hate_updates != 0 {
                    info!(pk_hate_updates, tick = world.tick.0, "applied PK hate updates from hurt events");
                }

                let area_text_sessions = send_pending_world_area_texts(&mut runtime, &mut world);
                if area_text_sessions != 0 {
                    info!(area_text_sessions, tick = world.tick.0, "queued world area text feedback");
                }

                let world_text_sessions = send_pending_world_system_texts(&mut runtime, &mut world);
                if world_text_sessions != 0 {
                    info!(world_text_sessions, tick = world.tick.0, "queued world system text feedback");
                }

                let world_text_bytes_sessions =
                    send_pending_world_system_text_bytes(&mut runtime, &mut world);
                if world_text_bytes_sessions != 0 {
                    info!(world_text_bytes_sessions, tick = world.tick.0, "queued world system text byte feedback");
                }

                let channel_broadcast_sessions =
                    send_pending_world_channel_broadcasts(&mut runtime, &mut world);
                if channel_broadcast_sessions != 0 {
                    info!(channel_broadcast_sessions, tick = world.tick.0, "queued world channel broadcast feedback");
                }

                let resource_sync_sessions = queue_resource_sync_frames(&mut runtime, &mut world);
                if resource_sync_sessions != 0 {
                    info!(resource_sync_sessions, tick = world.tick.0, "queued resource/value sync frames");
                }

                let (periodic_diff_sessions, periodic_empty_frames) =
                    queue_periodic_player_frames(&mut runtime, &world);
                if periodic_diff_sessions != 0 {
                    info!(periodic_diff_sessions, tick = world.tick.0, "queued periodic map/action diffs");
                }
                if periodic_empty_frames != 0 {
                    tracing::trace!(periodic_empty_frames, tick = world.tick.0, "queued empty legacy tick frames");
                }
                // Exactly one legacy tick frame per session per tick: the
                // lockstep client advances its clock per received frame.
                runtime.flush_tick_frames(true);
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
                                no_login: false,
                            };
                            match repository.begin_login(request).await {
                                Ok(LoginOutcome::Ready { character_id: db_character_id, character_number, mirror, .. }) => {
                                    character_id = db_character_id;
                                    if let Some(player) = runtime.players.get_mut(&id.0) {
                                        player.character_id = Some(db_character_id);
                                        player.character_number = if character_number == 0 { db_character_id.0 } else { character_number };
                                        player.set_current_mirror(mirror.max(0) as u32);
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

    Ok(())
}
