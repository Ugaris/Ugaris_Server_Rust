use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use tokio::{sync::mpsc, time};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use ugaris_core::{
    area_section::{section_at, section_look_text, section_name_by_id},
    area_sound::area_sound_special,
    character_driver::{CharacterDriverState, CDR_SIMPLEBADDY},
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode,
        CHARACTER_VALUE_NAMES, POWERSCALE,
    },
    ids::{CharacterId, ItemId},
    item_driver::{
        IDR_ACCOUNT_DEPOT, IDR_DECAYITEM, IDR_DEMONSHRINE, IDR_FOOD, IDR_KEY_RING,
        IDR_SPECIAL_POTION, IDR_TORCH,
    },
    item_ops::{consume_item, give_item_to_character, GiveItemFlags, GiveItemResult},
    key_registry::{is_registered_key, REGISTERED_KEY_IDS},
    legacy::INVENTORY_START_INVENTORY,
    map::{MapFlags, MapTile},
    player::{
        DemonShrineResult, KeyringAddResult, PlayerActionCode, PlayerConnectionState,
        PlayerRuntime, QueuedAction, XmasTreeResult,
    },
    spell::{
        EF_BALL, EF_BLESS, EF_BUBBLE, EF_BURN, EF_CAP, EF_CURSE, EF_EARTHMUD, EF_EARTHRAIN,
        EF_EDEMONBALL, EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE, EF_HEAL, EF_LAG,
        EF_MAGICSHIELD, EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE, EF_WARCRY,
    },
    text::COL_DARK_GRAY,
    tick::TICKS_PER_SECOND,
    world::LookMapRequest,
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
};
use ugaris_db::{
    CharacterRepository, CharacterSaveMode, CharacterSaveRequest, CharacterSnapshot, LoginOutcome,
    LoginRequest,
};
use ugaris_net::{NetServer, SessionCommand, SessionEvent};
use ugaris_protocol::{
    packet::{
        CharacterMapAction, CharacterMapStatus, MapLayer, MapPosition, PacketBuilder, CMF_LIGHT,
        CMF_SINK_ANKLE, CMF_SINK_BELLY, CMF_SINK_CHEST, CMF_SINK_KNEE, CMF_TAKE, CMF_UNDERWATER,
        CMF_USE, CMF_VISIBLE, MAP_CHARACTER_CLEAR, SV_SCROLL_DOWN, SV_SCROLL_LEFT,
        SV_SCROLL_LEFTDOWN, SV_SCROLL_LEFTUP, SV_SCROLL_RIGHT, SV_SCROLL_RIGHTDOWN,
        SV_SCROLL_RIGHTUP, SV_SCROLL_UP,
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

#[derive(Debug, Default)]
struct ServerRuntime {
    players: HashMap<u64, PlayerRuntime>,
    sessions: HashMap<u64, mpsc::Sender<SessionCommand>>,
    map_caches: HashMap<u64, VisibleMapCache>,
    effect_caches: HashMap<u64, ClientEffectCache>,
    account_depots: HashMap<CharacterId, AccountDepotState>,
    action_queue: VecDeque<(u64, ClientAction)>,
    next_character_id: u32,
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
        new_character_id
    }

    fn disconnect(&mut self, session_id: u64) -> Option<PlayerRuntime> {
        let player = self.players.remove(&session_id);
        self.sessions.remove(&session_id);
        self.map_caches.remove(&session_id);
        self.effect_caches.remove(&session_id);
        self.action_queue.retain(|(id, _)| *id != session_id);
        if let Some(player) = &player {
            if let Some(character_id) = player.character_id {
                self.account_depots.remove(&character_id);
            }
        }
        player
    }

    fn send_to_session(&self, session_id: u64, payload: bytes::BytesMut) -> bool {
        self.sessions
            .get(&session_id)
            .is_some_and(|commands| commands.try_send(SessionCommand::Send(payload)).is_ok())
    }

    fn send_many_to_session(&self, session_id: u64, payloads: Vec<bytes::BytesMut>) -> bool {
        payloads
            .into_iter()
            .all(|payload| self.send_to_session(session_id, payload))
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

    fn queue_action(&mut self, session_id: u64, action: ClientAction, current_tick: u64) {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.last_command_tick = current_tick;
            apply_player_action(player, &action, current_tick);
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

#[derive(Debug, Clone)]
struct AccountDepotState {
    slots: Vec<Option<Item>>,
}

impl Default for AccountDepotState {
    fn default() -> Self {
        Self {
            slots: vec![None; ugaris_core::entity::INVENTORY_SIZE],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AccountDepotCommandResult {
    Ignored,
    Changed,
    Look(String),
    Blocked(String),
}

const LEGACY_CONTAINER_SIZE: usize = ugaris_core::entity::INVENTORY_SIZE - 2;
const IID_AREA19_WOLFSSKIN: u32 = 0x0100008A;
const IID_AREA19_SALT: u32 = 0x0100008B;
const IID_AREA19_WOLFSSKIN2: u32 = 0x0100008C;
const IID_BRONZECHIP: u32 = 0x010000AC;
const IID_SILVERCHIP: u32 = 0x010000AD;
const IID_GOLDCHIP: u32 = 0x010000AE;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NomadStackApplyResult {
    Split {
        left: u32,
        right: u32,
        unit: &'static str,
    },
    Merged {
        count: u32,
        unit: &'static str,
    },
    CannotSplitOne {
        unit: &'static str,
    },
    CannotMix,
    Bug(&'static str),
    MissingPlayer,
    MissingItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleMapCell {
    effect_packet: Vec<u8>,
    tile_packet: Vec<u8>,
    character_id: Option<u16>,
    character_packet: Option<Vec<u8>>,
    character_name_packet: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleMapCache {
    center_x: u16,
    center_y: u16,
    view_distance: usize,
    cells: HashMap<u16, VisibleMapCell>,
    known_character_names: HashMap<u16, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientEffectSlot {
    effect_id: u32,
    serial: i32,
    body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientEffectCache {
    slots: Vec<Option<ClientEffectSlot>>,
    used_mask: u64,
}

impl Default for ClientEffectCache {
    fn default() -> Self {
        Self {
            slots: vec![None; MAX_CLIENT_EFFECTS],
            used_mask: 0,
        }
    }
}

const LOGIN_SPAWN_X: usize = 128;
const LOGIN_SPAWN_Y: usize = 128;
const LOGIN_ACCEPTED_MESSAGE: &str = "Rust Ugaris compatibility login accepted.";
const CHEST_EMPTY_MESSAGE: &str = "The chest is empty.";
const CHEST_CURSOR_OCCUPIED_MESSAGE: &str = "Please empty your 'hand' (mouse cursor) first.";
const CHEST_KEY_REQUIRED_MESSAGE: &str = "You need a key to open this chest.";
const RANDCHEST_CURSOR_OCCUPIED_MESSAGE: &str = "Please empty your hand (mouse cursor) first.";
const RANDCHEST_EMPTY_MESSAGE: &str = "You didn't find anything.";
const TORCH_UNDERWATER_MESSAGE: &str = "Obviously, thou canst not light thy torch under water.";
const TORCH_HISS_MESSAGE: &str = "Your hear your torch hiss.";
const MAP_BOOTSTRAP_CHUNK_TARGET: usize = MAX_LEGACY_TICK_PAYLOAD - 512;
const MAX_CLIENT_EFFECTS: usize = 64;
const DEFAULT_PLAYER_TEMPLATE: &str = "new_warrior_m";
const IID_KEY_RING: u32 = (59 << 24) | 0x000002;
const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;
const IID_PLACEHOLDER_KEY: u32 = (59 << 24) | 0x000004;
#[cfg(test)]
const IID_AREA1_SKELKEY1: u32 = (1 << 24) | 0x000002;
const INVENTORY_KEY_START_SLOT: usize = 30;
const RANDCHEST_COOLDOWN_SECONDS: u64 = 60 * 60 * 24;
const ORBSPAWN_RESPAWN_SECONDS: u64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChestTreasureApplyResult {
    Granted {
        item_name: String,
        key_name: Option<String>,
    },
    Empty,
    KeyRequired,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AssembleApplyResult {
    Assembled,
    MissingPlayer,
    MissingItem,
    TemplateUnavailable,
}

fn apply_character_snapshot(
    world: &mut World,
    player: &mut PlayerRuntime,
    snapshot: CharacterSnapshot,
    fallback_x: usize,
    fallback_y: usize,
) -> bool {
    let CharacterSnapshot {
        mut character,
        items,
        ppd_blob,
        subscriber_blob,
        ..
    } = snapshot;

    player.ppd_blob = ppd_blob;
    player.subscriber_blob = subscriber_blob;
    let ppd_blob = player.ppd_blob.clone();
    if !ppd_blob.is_empty() && !player.decode_legacy_ppd_blob(&ppd_blob) {
        warn!(
            character_id = character.id.0,
            "failed to decode legacy PPD blob for DB character"
        );
    }

    let spawn_x = usize::from(character.x).max(1);
    let spawn_y = usize::from(character.y).max(1);
    if !world.spawn_character(character.clone(), spawn_x, spawn_y) {
        character.x = 0;
        character.y = 0;
        if !world.spawn_character(character, fallback_x, fallback_y) {
            return false;
        }
    }

    for item in items {
        world.add_item(item);
    }
    true
}

fn character_snapshot_items(world: &World, character: &Character) -> Vec<Item> {
    world
        .items
        .values()
        .filter(|item| {
            item.carried_by == Some(character.id)
                || character.cursor_item == Some(item.id)
                || character
                    .inventory
                    .iter()
                    .any(|slot| *slot == Some(item.id))
        })
        .cloned()
        .collect()
}

fn character_save_request(
    world: &World,
    player: &PlayerRuntime,
    character: &Character,
    area_id: u16,
    mirror_id: u16,
) -> CharacterSaveRequest {
    CharacterSaveRequest {
        character: character.clone(),
        items: character_snapshot_items(world, character),
        ppd_blob: player.encode_legacy_ppd_blob(&player.ppd_blob),
        subscriber_blob: player.subscriber_blob.clone(),
        mode: CharacterSaveMode::Logout {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            allowed_area: i32::from(area_id),
            mirror: i32::from(mirror_id),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RandomChestApplyResult {
    Money { amount: u32 },
    Item { item_name: String },
    Empty,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyringAddApplyResult {
    Added { key_name: String },
    Duplicate,
    Full,
    NotAKey,
    MissingPlayer,
    MissingCursorItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OrbSpawnApplyResult {
    Granted { item_name: String, special: bool },
    Cooldown { days_left: String },
    Nothing,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyringAutoAddPickupResult {
    Added { key_name: String },
    Duplicate { key_name: String },
    Full { key_name: String },
    MissingPlayer,
    MissingCursorItem,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct KeyringCommandResult {
    messages: Vec<String>,
    inventory_changed: bool,
}

#[derive(Debug, Clone)]
struct ZoneLoadSummary {
    root: PathBuf,
    map_file: PathBuf,
    item_templates: usize,
    character_templates: usize,
    skipped_template_files: usize,
    placed_items: usize,
    placed_characters: usize,
    ground_tiles: usize,
    blocked_tiles: usize,
    scheduled_light_timers: usize,
}

fn login_character(
    character_id: CharacterId,
    login: &LoginBlock,
    area_id: u16,
    spawn_x: usize,
    spawn_y: usize,
) -> Character {
    let mut values = Character::empty_values();
    set_character_value(&mut values, CharacterValue::Hp, 50);
    set_character_value(&mut values, CharacterValue::Endurance, 50);
    set_character_value(&mut values, CharacterValue::Mana, 50);
    set_character_value(&mut values, CharacterValue::Speed, 50);

    Character {
        id: character_id,
        name: login.name.clone(),
        description: String::new(),
        flags: CharacterFlags::USED | CharacterFlags::PLAYER | CharacterFlags::ALIVE,
        sprite: 1,
        driver: 0,
        group: 0,
        speed_mode: SpeedMode::Normal,
        x: 0,
        y: 0,
        rest_area: area_id,
        rest_x: spawn_x as u16,
        rest_y: spawn_y as u16,
        tox: 0,
        toy: 0,
        dir: 0,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 50 * POWERSCALE,
        mana: 50 * POWERSCALE,
        endurance: 50 * POWERSCALE,
        lifeshield: 0,
        level: 1,
        exp: 0,
        exp_used: 0,
        gold: 0,
        creation_time: 0,
        saves: 0,
        deaths: 0,
        regen_ticker: 0,
        cursor_item: None,
        current_container: None,
        values,
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
    }
}

fn login_character_from_template(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    login: &LoginBlock,
    area_id: u16,
    spawn_x: usize,
    spawn_y: usize,
) -> Result<(Character, Vec<ugaris_core::entity::Item>), ugaris_core::zone::ZoneError> {
    let (mut character, items) =
        loader.instantiate_character_template(DEFAULT_PLAYER_TEMPLATE, character_id)?;
    character.name = login.name.clone();
    character.description.clear();
    character
        .flags
        .insert(CharacterFlags::USED | CharacterFlags::PLAYER | CharacterFlags::ALIVE);
    character.rest_area = area_id;
    character.rest_x = spawn_x as u16;
    character.rest_y = spawn_y as u16;
    character.level = character.level.max(1);
    Ok((character, items))
}

fn grant_chest_treasure(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    treasure_index: u8,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }

    let key = format!("treasure_{treasure_index}");
    let Ok(item) = loader.instantiate_item_template(&key, Some(character_id)) else {
        return None;
    };
    let item_id = item.id;
    let item_name = item.name.clone();

    let Some(character) = world.characters.get_mut(&character_id) else {
        return None;
    };
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

fn legacy_orb_value_from_seed(seed: u64) -> CharacterValue {
    const VALUES: [CharacterValue; 32] = [
        CharacterValue::Endurance,
        CharacterValue::Hp,
        CharacterValue::Mana,
        CharacterValue::Wisdom,
        CharacterValue::Intelligence,
        CharacterValue::Agility,
        CharacterValue::Strength,
        CharacterValue::Barter,
        CharacterValue::Percept,
        CharacterValue::Stealth,
        CharacterValue::Hand,
        CharacterValue::Warcry,
        CharacterValue::Surround,
        CharacterValue::BodyControl,
        CharacterValue::SpeedSkill,
        CharacterValue::Heal,
        CharacterValue::Fireball,
        CharacterValue::Tactics,
        CharacterValue::Duration,
        CharacterValue::Rage,
        CharacterValue::Bless,
        CharacterValue::Freeze,
        CharacterValue::MagicShield,
        CharacterValue::Flash,
        CharacterValue::Pulse,
        CharacterValue::Dagger,
        CharacterValue::Staff,
        CharacterValue::Sword,
        CharacterValue::TwoHand,
        CharacterValue::Attack,
        CharacterValue::Parry,
        CharacterValue::Immunity,
    ];
    VALUES[(seed as usize) % VALUES.len()]
}

fn grant_orb_spawn_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    anti: bool,
    special: bool,
    seed: u64,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }

    let template = if anti { "empty_anti_orb" } else { "empty_orb" };
    let Ok(mut item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return None;
    };
    let value = legacy_orb_value_from_seed(seed) as u8;
    let value_name = CHARACTER_VALUE_NAMES[usize::from(value)];
    if anti {
        if special {
            item.name = format!("Extracting Anti-Orb of {value_name}");
            item.description =
                format!("A dark orb that extracts {value_name} from items and crystallizes it.");
            ensure_drdata_len(&mut item, 3);
            item.driver_data[2] = 1;
        } else {
            item.name = format!("Anti-Orb of {value_name}");
            item.description = format!("A dark orb that removes {value_name} from items.");
            ensure_drdata_len(&mut item, 3);
            item.driver_data[2] = 0;
        }
    } else {
        item.name = format!("Orb of {value_name}");
        ensure_drdata_len(&mut item, 2);
    }
    item.driver_data[0] = value;
    item.driver_data[1] = 1;

    let item_id = item.id;
    let item_name = item.name.clone();
    let Some(character) = world.characters.get_mut(&character_id) else {
        return None;
    };
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

fn instantiate_orb_with_modifier(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    modifier: i16,
) -> Option<Item> {
    let value = u8::try_from(modifier).ok()?;
    let value_name = CHARACTER_VALUE_NAMES.get(usize::from(value))?;
    let Ok(mut item) = loader.instantiate_item_template("empty_orb", Some(character_id)) else {
        return None;
    };
    item.name = format!("Orb of {value_name}");
    ensure_drdata_len(&mut item, 2);
    item.driver_data[0] = value;
    item.driver_data[1] = 1;
    Some(item)
}

fn ensure_drdata_len(item: &mut Item, len: usize) {
    if item.driver_data.len() < len {
        item.driver_data.resize(len, 0);
    }
}

fn apply_orb_spawn(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    spawn_item_id: ItemId,
    character_id: CharacterId,
    area_id: u16,
    realtime_seconds: u64,
    anti: bool,
    special: bool,
    random_seed: u64,
) -> OrbSpawnApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return OrbSpawnApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return OrbSpawnApplyResult::CursorOccupied;
    }
    let Some(player) = player else {
        return OrbSpawnApplyResult::MissingPlayer;
    };
    let Some(spawner) = world.items.get(&spawn_item_id) else {
        return OrbSpawnApplyResult::Nothing;
    };
    let location_id =
        u32::from(spawner.x) + (u32::from(spawner.y) << 8) + (u32::from(area_id) << 16);
    if let Some(last_used) = player.orb_spawn_last_used_seconds(location_id) {
        if last_used.saturating_add(ORBSPAWN_RESPAWN_SECONDS) > realtime_seconds {
            let remaining = last_used
                .saturating_add(ORBSPAWN_RESPAWN_SECONDS)
                .saturating_sub(realtime_seconds);
            return OrbSpawnApplyResult::Cooldown {
                days_left: format!("{:.2}", remaining as f64 / 60.0 / 60.0 / 24.0),
            };
        }
    }

    player.mark_orb_spawn_used(location_id, realtime_seconds);
    match grant_orb_spawn_item(world, loader, character_id, anti, special, random_seed) {
        Some(item_name) => OrbSpawnApplyResult::Granted { item_name, special },
        None => OrbSpawnApplyResult::Nothing,
    }
}

fn keyring_show_messages(player: Option<&PlayerRuntime>) -> Vec<String> {
    player
        .map(PlayerRuntime::keyring_display_lines)
        .unwrap_or_else(|| vec!["Your keyring is empty.".to_string()])
}

fn cursor_holds_keyring(world: &World, character: &Character) -> bool {
    character
        .cursor_item
        .and_then(|item_id| world.items.get(&item_id))
        .is_some_and(|item| item.template_id == IID_KEY_RING || item.driver == IDR_KEY_RING)
}

fn is_runtime_keyring_candidate(item: &Item) -> bool {
    is_registered_key(item.template_id)
}

fn normalize_text_command(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes)
        .ok()?
        .trim_matches(char::from(0))
        .trim();
    if text.is_empty() {
        return None;
    }
    Some(text.to_ascii_lowercase())
}

fn keyring_entry_to_item(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    entry: &ugaris_core::player::KeyringEntry,
) -> Item {
    if let Some(item) =
        loader.instantiate_item_template_by_id(entry.template_id, Some(character_id))
    {
        return item;
    }

    if let Some(mut item) =
        loader.instantiate_item_template_by_id(IID_PLACEHOLDER_KEY, Some(character_id))
    {
        item.template_id = entry.template_id;
        item.name = entry.name.clone();
        item.description = entry.description.clone();
        item.sprite = entry.sprite;
        item.flags = ItemFlags::from_bits_retain(entry.flags) | ItemFlags::USED;
        item.value = entry.value;
        item.driver = entry.driver;
        item.driver_data = entry.driver_data.clone();
        item.serial = entry.expire_serial;
        return item;
    }

    Item {
        id: loader.allocate_item_id(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        flags: ItemFlags::from_bits_retain(entry.flags) | ItemFlags::USED,
        sprite: entry.sprite,
        value: entry.value,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: entry.template_id,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(character_id),
        contained_in: None,
        content_id: 0,
        driver: entry.driver,
        driver_data: entry.driver_data.clone(),
        serial: entry.expire_serial,
    }
}

fn give_removed_keyring_entry(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    entry: &ugaris_core::player::KeyringEntry,
) -> Result<(), &'static str> {
    let mut item = keyring_entry_to_item(loader, character_id, entry);
    let item_id = item.id;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return Err("Failed to access keyring data.");
    };
    match give_item_to_character(character, &mut item, GiveItemFlags::NONE) {
        GiveItemResult::Ok => {
            world.add_item(item);
            Ok(())
        }
        GiveItemResult::Full => Err("Your inventory is full."),
        _ => {
            world.items.remove(&item_id);
            Err("Cannot remove this key here. Return to where you found it.")
        }
    }
}

fn apply_keyring_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let rest = command
        .strip_prefix("#keyring")
        .or_else(|| command.strip_prefix("/keyring"))?;
    let rest = rest.trim();

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if !cursor_holds_keyring(world, character) {
        return Some(KeyringCommandResult {
            messages: vec![
                "You need to hold a keyring on your cursor to use this command.".to_string(),
            ],
            inventory_changed: false,
        });
    }

    if rest.is_empty() {
        let mut messages = player.keyring_display_lines();
        if player.keyring.is_empty() {
            messages.push("Use '#keyring addall' to add all keys from inventory.".to_string());
        }
        return Some(KeyringCommandResult {
            messages,
            inventory_changed: false,
        });
    }

    let mut words = rest.split_whitespace();
    match words.next().unwrap_or_default() {
        "remove" => {
            let Some(number) = words.next().and_then(|word| word.parse::<usize>().ok()) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Usage: #keyring remove <number>".to_string()],
                    inventory_changed: false,
                });
            };
            let Some(index) = number.checked_sub(1) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                });
            };
            let Some(entry) = player.keyring.get(index).cloned() else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                });
            };
            if let Err(message) = give_removed_keyring_entry(world, loader, character_id, &entry) {
                return Some(KeyringCommandResult {
                    messages: vec![message.to_string()],
                    inventory_changed: false,
                });
            }
            player.remove_keyring_key_at(index);
            Some(KeyringCommandResult {
                messages: vec![format!("Removed {} from your keyring.", entry.name)],
                inventory_changed: true,
            })
        }
        "addall" => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            let mut added_count = 0;
            for slot in INVENTORY_KEY_START_SLOT..character.inventory.len() {
                let Some(item_id) = character.inventory[slot] else {
                    continue;
                };
                let Some(item) = world.items.get(&item_id).cloned() else {
                    continue;
                };
                if !is_runtime_keyring_candidate(&item) {
                    continue;
                }
                if player.add_keyring_item(&item) == KeyringAddResult::Added {
                    character.inventory[slot] = None;
                    character.flags.insert(CharacterFlags::ITEMS);
                    world.items.remove(&item_id);
                    added_count += 1;
                }
            }
            let messages = if added_count > 0 {
                vec![format!("Added {added_count} keys to your keyring.")]
            } else {
                vec!["No keys found to add.".to_string()]
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: added_count > 0,
            })
        }
        "addallkeys" => {
            let Some(character) = world.characters.get(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            if !character
                .flags
                .intersects(CharacterFlags::GOD | CharacterFlags::STAFF)
            {
                return Some(KeyringCommandResult {
                    messages: vec!["This command requires staff privileges.".to_string()],
                    inventory_changed: false,
                });
            }

            let mut added_count = 0;
            for template_id in REGISTERED_KEY_IDS {
                let Some(item) = loader.instantiate_item_template_by_id(*template_id, Some(character_id))
                else {
                    continue;
                };
                if player.add_keyring_item(&item) == KeyringAddResult::Added {
                    added_count += 1;
                }
                if player.keyring.len() >= ugaris_core::player::KEYRING_MAX_KEYS {
                    break;
                }
            }

            Some(KeyringCommandResult {
                messages: vec![
                    "Adding all registered keys to keyring...".to_string(),
                    format!(
                        "Added {added_count} keys to your keyring (total: {}/{}).",
                        player.keyring.len(),
                        ugaris_core::player::KEYRING_MAX_KEYS
                    ),
                ],
                inventory_changed: false,
            })
        }
        "auto" => {
            let enabled = !player.keyring_auto_add();
            player.set_keyring_auto_add(enabled);
            let message = if enabled {
                "Auto-add keys enabled. Keys will be automatically added to your keyring when picked up."
            } else {
                "Auto-add keys disabled. Keys will go to your inventory as normal."
            };
            Some(KeyringCommandResult {
                messages: vec![message.to_string()],
                inventory_changed: false,
            })
        }
        _ => Some(KeyringCommandResult {
            messages: vec!["Unknown keyring subcommand. Use: #keyring, #keyring remove <n>, #keyring addall, #keyring auto".to_string()],
            inventory_changed: false,
        }),
    }
}

fn apply_keyring_add_cursor_item(
    world: &mut World,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    key_item_id: ItemId,
) -> KeyringAddApplyResult {
    let Some(player) = player else {
        return KeyringAddApplyResult::MissingPlayer;
    };
    let Some(key_item) = world.items.get(&key_item_id) else {
        return KeyringAddApplyResult::MissingCursorItem;
    };
    if !is_runtime_keyring_candidate(key_item) {
        return KeyringAddApplyResult::NotAKey;
    }
    let key_snapshot: Item = key_item.clone();

    match player.add_keyring_item(&key_snapshot) {
        KeyringAddResult::Added => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return KeyringAddApplyResult::MissingPlayer;
            };
            let Some(key_item) = world.items.get_mut(&key_item_id) else {
                return KeyringAddApplyResult::MissingCursorItem;
            };
            if character.cursor_item != Some(key_item_id) || !consume_item(character, key_item) {
                return KeyringAddApplyResult::MissingCursorItem;
            }
            KeyringAddApplyResult::Added {
                key_name: key_snapshot.name,
            }
        }
        KeyringAddResult::Duplicate => KeyringAddApplyResult::Duplicate,
        KeyringAddResult::Full => KeyringAddApplyResult::Full,
    }
}

fn apply_keyring_auto_add_pickup(
    world: &mut World,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    key_item_id: ItemId,
) -> Option<KeyringAutoAddPickupResult> {
    let player = player?;
    if !player.keyring_auto_add() {
        return None;
    }
    let key_item = world.items.get(&key_item_id)?;
    if !is_runtime_keyring_candidate(key_item) {
        return None;
    }
    let key_snapshot: Item = key_item.clone();

    Some(match player.add_keyring_item(&key_snapshot) {
        KeyringAddResult::Added => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringAutoAddPickupResult::MissingPlayer);
            };
            let Some(key_item) = world.items.get_mut(&key_item_id) else {
                return Some(KeyringAutoAddPickupResult::MissingCursorItem);
            };
            if character.cursor_item != Some(key_item_id) || !consume_item(character, key_item) {
                return Some(KeyringAutoAddPickupResult::MissingCursorItem);
            }
            KeyringAutoAddPickupResult::Added {
                key_name: key_snapshot.name,
            }
        }
        KeyringAddResult::Duplicate => KeyringAutoAddPickupResult::Duplicate {
            key_name: key_snapshot.name,
        },
        KeyringAddResult::Full => KeyringAutoAddPickupResult::Full {
            key_name: key_snapshot.name,
        },
    })
}

fn apply_chest_treasure(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    chest_item_id: ItemId,
    character_id: CharacterId,
    treasure_index: u8,
    realtime_seconds: u64,
) -> ChestTreasureApplyResult {
    let required_key_id = world
        .items
        .get(&chest_item_id)
        .map(chest_required_key_id)
        .unwrap_or_default();
    let key_name = if required_key_id == 0 {
        None
    } else {
        match chest_key_name(world, character_id, required_key_id).or_else(|| {
            player
                .as_deref()
                .and_then(|player| player.keyring_key_name(required_key_id))
                .map(str::to_string)
        }) {
            Some(name) => Some(name),
            None => return ChestTreasureApplyResult::KeyRequired,
        }
    };

    let Some(character) = world.characters.get(&character_id) else {
        return ChestTreasureApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return ChestTreasureApplyResult::CursorOccupied;
    }

    let Some(player) = player else {
        return ChestTreasureApplyResult::MissingPlayer;
    };

    let timeout_seconds = world
        .items
        .get(&chest_item_id)
        .map(chest_timeout_seconds)
        .unwrap_or_default();
    let last_access = player.chest_last_access_seconds(treasure_index);
    if last_access != 0
        && timeout_seconds != 0
        && last_access.saturating_add(timeout_seconds) > realtime_seconds
    {
        return ChestTreasureApplyResult::Empty;
    }

    let required_deaths = world
        .items
        .get(&chest_item_id)
        .map(chest_required_deaths)
        .unwrap_or_default();
    if required_deaths != 0
        && world
            .characters
            .get(&character_id)
            .is_none_or(|character| character.deaths < u32::from(required_deaths))
    {
        return ChestTreasureApplyResult::Empty;
    }

    match grant_chest_treasure(world, loader, character_id, treasure_index) {
        Some(item_name) => {
            player.mark_chest_access(treasure_index, realtime_seconds);
            player.record_chest_opened(treasure_index);
            ChestTreasureApplyResult::Granted {
                item_name,
                key_name,
            }
        }
        None => ChestTreasureApplyResult::Empty,
    }
}

fn chest_timeout_seconds(item: &ugaris_core::entity::Item) -> u64 {
    let low = item.driver_data.get(5).copied().unwrap_or_default();
    let high = item.driver_data.get(6).copied().unwrap_or_default();
    u64::from(u16::from_le_bytes([low, high])) * 60 * 60
}

fn chest_required_deaths(item: &ugaris_core::entity::Item) -> u8 {
    item.driver_data.get(7).copied().unwrap_or_default()
}

fn chest_blocked_message(
    world: &World,
    item_id: ItemId,
    character_id: CharacterId,
) -> &'static str {
    if world
        .items
        .get(&item_id)
        .is_some_and(|item| chest_required_key_id(item) != 0)
    {
        return CHEST_KEY_REQUIRED_MESSAGE;
    }
    if world
        .characters
        .get(&character_id)
        .is_some_and(|character| character.cursor_item.is_some())
    {
        return CHEST_CURSOR_OCCUPIED_MESSAGE;
    }
    CHEST_EMPTY_MESSAGE
}

fn special_potion_fun_message(
    world: &World,
    character_id: CharacterId,
    kind: u8,
) -> Option<String> {
    let name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");

    match kind {
        8 => Some(format!(
            "You see {name} hit himself on the head with a mug."
        )),
        9 => Some(format!(
            "{name} suddenly starts singing in a slurred tongue... Dogs start howling..."
        )),
        10 => Some(format!(
            "{name}'s hair suddenly shoots up as if hit by electricity."
        )),
        11 => Some(format!("{name} seems to be enjoying a fine ale.")),
        12 => Some(format!("{name} drinks a delicious apple juice.")),
        13 => Some(format!("{name} feels refreshed.")),
        14 => Some(format!("{name} cracks his strong knuckles.")),
        15 => Some(format!("{name} starts frothing at the mouth.")),
        _ => None,
    }
}

fn is_torch_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_TORCH)
}

fn is_beyond_potion_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_BEYONDPOTION)
}

fn is_no_potion_area_blocked_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| matches!(item.driver, IDR_BEYONDPOTION | IDR_SPECIAL_POTION))
}

fn is_demonshrine_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_DEMONSHRINE)
}

fn character_has_active_beyond_potion(world: &World, character_id: CharacterId) -> bool {
    world
        .characters
        .get(&character_id)
        .is_some_and(|character| {
            character.inventory[12..30].iter().any(|item_id| {
                item_id
                    .and_then(|item_id| world.items.get(&item_id))
                    .is_some_and(|item| item.driver == ugaris_core::spell::IDR_POTION_SP)
            })
        })
}

fn timer_outcome_feedback(
    outcomes: &[ugaris_core::item_driver::ItemDriverOutcome],
) -> Vec<(CharacterId, String)> {
    outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater {
                character_id,
                ..
            } => Some((*character_id, TORCH_HISS_MESSAGE.to_string())),
            ugaris_core::item_driver::ItemDriverOutcome::TorchExpired {
                character_id,
                item_name,
                ..
            } => Some((
                *character_id,
                format!("Your {} expired.", outcome_item_name_text(item_name)),
            )),
            _ => None,
        })
        .collect()
}

fn outcome_item_name_text(bytes: &[u8]) -> String {
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

fn chest_required_key_id(item: &ugaris_core::entity::Item) -> u32 {
    let b1 = item.driver_data.get(1).copied().unwrap_or_default();
    let b2 = item.driver_data.get(2).copied().unwrap_or_default();
    let b3 = item.driver_data.get(3).copied().unwrap_or_default();
    let b4 = item.driver_data.get(4).copied().unwrap_or_default();
    u32::from_le_bytes([b1, b2, b3, b4])
}

fn chest_key_name(
    world: &World,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<String> {
    let character = world.characters.get(&character_id)?;
    if let Some(item_id) = character.cursor_item {
        if let Some(name) = chest_key_item_name(world, item_id, required_key_id) {
            return Some(name);
        }
    }
    for item_id in character.inventory.iter().skip(30).flatten() {
        if let Some(name) = chest_key_item_name(world, *item_id, required_key_id) {
            return Some(name);
        }
    }
    None
}

fn chest_key_item_name(world: &World, item_id: ItemId, required_key_id: u32) -> Option<String> {
    let item = world.items.get(&item_id)?;
    (item.template_id == required_key_id || item.template_id == IID_SKELETON_KEY)
        .then(|| item.name.clone())
}

fn item_driver_context_for_request(
    world: &World,
    player: Option<&PlayerRuntime>,
    request: &ugaris_core::item_driver::ItemDriverRequest,
) -> ugaris_core::item_driver::ItemDriverContext {
    let ugaris_core::item_driver::ItemDriverRequest::Driver {
        driver,
        item_id,
        character_id,
        ..
    } = request
    else {
        return ugaris_core::item_driver::ItemDriverContext::default();
    };
    if *driver == ugaris_core::item_driver::IDR_ASSEMBLE
        || *driver == ugaris_core::item_driver::IDR_PALACEKEY
    {
        let cursor_item = world
            .characters
            .get(character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| world.items.get(&cursor_item_id));
        return ugaris_core::item_driver::ItemDriverContext {
            door_key: None,
            cursor_template_id: cursor_item.map(|item| item.template_id),
            cursor_sprite: cursor_item.map(|item| item.sprite),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver != ugaris_core::item_driver::IDR_DOOR
        && *driver != ugaris_core::item_driver::IDR_INFINITE_CHEST
    {
        return ugaris_core::item_driver::ItemDriverContext::default();
    }
    let required_key_id = world
        .items
        .get(item_id)
        .map(chest_required_key_id)
        .unwrap_or_default();
    if required_key_id == 0 {
        return ugaris_core::item_driver::ItemDriverContext::default();
    }

    let door_key = if *driver == ugaris_core::item_driver::IDR_INFINITE_CHEST {
        infinite_chest_key_access(world, *character_id, required_key_id)
    } else {
        door_key_access(world, player, *character_id, required_key_id)
    };

    ugaris_core::item_driver::ItemDriverContext {
        door_key,
        cursor_template_id: None,
        ..ugaris_core::item_driver::ItemDriverContext::default()
    }
}

fn infinite_chest_key_access(
    world: &World,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let character = world.characters.get(&character_id)?;
    let inventory_items = character.inventory.iter().skip(30).flatten().copied();
    for item_id in inventory_items {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    None
}

fn door_key_access(
    world: &World,
    player: Option<&PlayerRuntime>,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let character = world.characters.get(&character_id)?;
    let inventory_items = character.inventory.iter().skip(30).flatten().copied();

    for item_id in inventory_items.clone() {
        if let Some(access) = carried_door_key_access(world, item_id, IID_SKELETON_KEY) {
            return Some(access);
        }
    }
    if let Some(item_id) = character.cursor_item {
        if let Some(access) = carried_door_key_access(world, item_id, IID_SKELETON_KEY) {
            return Some(access);
        }
    }
    for item_id in inventory_items {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    if let Some(item_id) = character.cursor_item {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    player
        .and_then(|player| player.keyring_key_name(required_key_id))
        .map(|name| ugaris_core::item_driver::DoorKeyAccess {
            key_id: required_key_id,
            name: name.to_string(),
            source: ugaris_core::item_driver::DoorKeySource::Keyring,
        })
}

fn carried_door_key_access(
    world: &World,
    item_id: ItemId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let item = world.items.get(&item_id)?;
    (item.template_id == required_key_id).then(|| ugaris_core::item_driver::DoorKeyAccess {
        key_id: item.template_id,
        name: item.name.clone(),
        source: ugaris_core::item_driver::DoorKeySource::Carried,
    })
}

fn apply_random_chest(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    item_id: ItemId,
    character_id: CharacterId,
    area_id: u16,
    realtime_seconds: u64,
    random_seed: u64,
) -> RandomChestApplyResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return RandomChestApplyResult::CursorOccupied;
    }

    let Some(player) = player else {
        return RandomChestApplyResult::MissingPlayer;
    };
    let Some(chest) = world.items.get(&item_id) else {
        return RandomChestApplyResult::MissingPlayer;
    };
    let location_id = random_chest_location_id(chest.x, chest.y, area_id);
    if player
        .random_chest_last_used_seconds(location_id)
        .is_some_and(|last_used| {
            last_used.saturating_add(RANDCHEST_COOLDOWN_SECONDS) > realtime_seconds
        })
    {
        return RandomChestApplyResult::Empty;
    }

    let money_level = chest.driver_data.first().copied().unwrap_or_default();
    let loot_tier = chest.driver_data.get(1).copied().unwrap_or_default();
    player.mark_random_chest_used(location_id, realtime_seconds);

    if loot_tier == 0 && legacy_random(random_seed, 4) != 0 {
        return RandomChestApplyResult::Empty;
    }

    if let Some(template) = random_chest_loot_template(loot_tier, legacy_random(random_seed, 28)) {
        if let Some(item_name) =
            grant_template_item_to_cursor(world, loader, character_id, template)
        {
            player.record_chest_opened(0);
            return RandomChestApplyResult::Item { item_name };
        }
    }

    let amount = random_chest_money_amount(money_level, random_seed);
    if amount == 0 || !grant_money_to_cursor(world, loader, character_id, amount) {
        return RandomChestApplyResult::Empty;
    }
    player.record_chest_opened(0);
    RandomChestApplyResult::Money { amount }
}

fn random_chest_location_id(x: u16, y: u16, area_id: u16) -> u32 {
    u32::from(x) + (u32::from(y) << 8) + (u32::from(area_id) << 16)
}

fn legacy_random(seed: u64, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let mut value = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((value ^ (value >> 31)) % u64::from(max)) as u32
}

fn random_chest_money_amount(level: u8, seed: u64) -> u32 {
    let level = u32::from(level);
    if level == 0 {
        return 0;
    }
    let first = legacy_random(seed.wrapping_add(1), level) + 1;
    let second = legacy_random(seed.wrapping_add(2), level) + 1;
    first.saturating_mul(second)
}

fn random_chest_loot_template(tier: u8, roll: u32) -> Option<&'static str> {
    if !(1..=10).contains(&tier) || !(21..=27).contains(&roll) {
        return None;
    }
    let potion_level = match tier {
        1 => match roll {
            21 => return Some("healing_potion1"),
            22 => return Some("mana_potion1"),
            23 => return Some("combo_potion1"),
            _ => 4,
        },
        2 | 3 => match roll {
            21 => return Some("healing_potion2"),
            22 => return Some("mana_potion2"),
            23 => return Some("combo_potion2"),
            _ => tier * 4,
        },
        4..=10 => match roll {
            21 => return Some("healing_potion3"),
            22 => return Some("mana_potion3"),
            23 => return Some("combo_potion3"),
            _ => tier * 4,
        },
        _ => return None,
    };

    match roll {
        24 => Some(match potion_level {
            4 => "sword4_potion",
            8 => "sword8_potion",
            12 => "sword12_potion",
            16 => "sword16_potion",
            20 => "sword20_potion",
            24 => "sword24_potion",
            28 => "sword28_potion",
            32 => "sword32_potion",
            36 => "sword36_potion",
            _ => "sword40_potion",
        }),
        25 => Some(match potion_level {
            4 => "twohand4_potion",
            8 => "twohand8_potion",
            12 => "twohand12_potion",
            16 => "twohand16_potion",
            20 => "twohand20_potion",
            24 => "twohand24_potion",
            28 => "twohand28_potion",
            32 => "twohand32_potion",
            36 => "twohand36_potion",
            _ => "twohand40_potion",
        }),
        26 => Some(match potion_level {
            4 => "flash4_potion",
            8 => "flash8_potion",
            12 => "flash12_potion",
            16 => "flash16_potion",
            20 => "flash20_potion",
            24 => "flash24_potion",
            28 => "flash28_potion",
            32 => "flash32_potion",
            36 => "flash36_potion",
            _ => "flash40_potion",
        }),
        27 => Some(match potion_level {
            4 => "immunity4_potion",
            8 => "immunity8_potion",
            12 => "immunity12_potion",
            16 => "immunity16_potion",
            20 => "immunity20_potion",
            24 => "immunity24_potion",
            28 => "immunity28_potion",
            32 => "immunity32_potion",
            36 => "immunity36_potion",
            _ => "immunity40_potion",
        }),
        _ => None,
    }
}

fn grant_template_item_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    template: &str,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }
    let item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    let item_id = item.id;
    let item_name = item.name.clone();
    let character = world.characters.get_mut(&character_id)?;
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

fn grant_template_item_smart(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    template: &str,
) -> Option<String> {
    let mut item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    let item_name = item.name.clone();
    let (give_result, drop_x, drop_y) = {
        let character = world.characters.get_mut(&character_id)?;
        let result = give_item_to_character(character, &mut item, GiveItemFlags::ALLOW_DROP);
        (result, usize::from(character.x), usize::from(character.y))
    };
    match give_result {
        GiveItemResult::Ok => {}
        GiveItemResult::Dropped => {
            if !world.map.drop_item_extended(&mut item, drop_x, drop_y, 1) {
                return None;
            }
        }
        GiveItemResult::Money => {}
        GiveItemResult::Full | GiveItemResult::Failed => return None,
    }
    world.add_item(item);
    Some(item_name)
}

fn apply_xmasmaker(world: &mut World, loader: &mut ZoneLoader, character_id: CharacterId) -> bool {
    grant_template_item_smart(world, loader, character_id, "xmaspop").is_some()
}

const XMAS_TREE_GIFT_TEMPLATES: [&str; 17] = [
    "ad_bracelet1",
    "ad_bracelet2",
    "ad_ring1",
    "ad_ring2",
    "ad_ring3",
    "ad_ring4",
    "ad_ring5",
    "ad_necklace1",
    "ad_necklace2",
    "ad_cape1",
    "ad_cape2",
    "ad_cape3",
    "ad_boots1",
    "ad_boots2",
    "ad_boots3",
    "ad_belt1",
    "ad_belt2",
];

fn xmas_event_from_ymd(year: i32, month: u32, day: u32) -> (bool, i32) {
    if month == 12 && day >= 20 {
        (true, year)
    } else if month == 1 && day <= 7 {
        (true, year - 1)
    } else {
        (false, year)
    }
}

fn civil_from_unix_seconds(seconds: u64) -> (i32, u32, u32) {
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn current_xmas_event() -> (bool, i32) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let (year, month, day) = civil_from_unix_seconds(seconds);
    xmas_event_from_ymd(year, month, day)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum XmasTreeApplyResult {
    Dormant,
    AlreadyGranted,
    NeedsHolidayTreat,
    GiftGranted(String),
    NoSpace,
    MissingPlayer,
}

fn apply_xmastree(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    area_id: u16,
    is_xmas: bool,
    event_year: i32,
    gift_seed: u64,
) -> XmasTreeApplyResult {
    let has_holiday_treat = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item)
        .and_then(|item_id| world.items.get(&item_id))
        .is_some_and(|item| item.driver == IDR_FOOD && item.driver_data.first() == Some(&3));

    match player.touch_xmas_tree(area_id, event_year, is_xmas, has_holiday_treat) {
        XmasTreeResult::Dormant => XmasTreeApplyResult::Dormant,
        XmasTreeResult::AlreadyGranted => XmasTreeApplyResult::AlreadyGranted,
        XmasTreeResult::NeedsHolidayTreat => XmasTreeApplyResult::NeedsHolidayTreat,
        XmasTreeResult::GiftGranted => {
            let template =
                XMAS_TREE_GIFT_TEMPLATES[(gift_seed as usize) % XMAS_TREE_GIFT_TEMPLATES.len()];
            let Some(item_name) = grant_template_item_smart(world, loader, character_id, template)
            else {
                player.unmark_xmas_tree(area_id);
                return XmasTreeApplyResult::NoSpace;
            };
            let Some(character) = world.characters.get_mut(&character_id) else {
                player.unmark_xmas_tree(area_id);
                return XmasTreeApplyResult::MissingPlayer;
            };
            if let Some(cursor_item_id) = character.cursor_item.take() {
                world.items.remove(&cursor_item_id);
                character.flags.insert(CharacterFlags::ITEMS);
            }
            XmasTreeApplyResult::GiftGranted(item_name)
        }
    }
}

fn apply_assemble_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_item_id: ItemId,
    template: &str,
) -> AssembleApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    if character.cursor_item != Some(cursor_item_id) {
        return AssembleApplyResult::MissingItem;
    }
    let Some(slot) = character
        .inventory
        .iter()
        .position(|slot_item| *slot_item == Some(item_id))
    else {
        return AssembleApplyResult::MissingItem;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
        || !world.items.contains_key(&cursor_item_id)
    {
        return AssembleApplyResult::MissingItem;
    }

    let Ok(new_item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    let new_item_id = new_item.id;

    world.items.remove(&cursor_item_id);
    world.items.remove(&item_id);
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.inventory[slot] = Some(new_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(new_item);
    AssembleApplyResult::Assembled
}

fn apply_palace_key_split(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_part_sprite: i32,
    carried_part_sprite: i32,
) -> AssembleApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return AssembleApplyResult::MissingItem;
    }
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
    {
        return AssembleApplyResult::MissingItem;
    }

    let Ok(mut cursor_item) =
        loader.instantiate_item_template("palace_key_part1", Some(character_id))
    else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    cursor_item.sprite = cursor_part_sprite;
    let cursor_item_id = cursor_item.id;

    let Some(item) = world.items.get_mut(&item_id) else {
        return AssembleApplyResult::MissingItem;
    };
    item.sprite = carried_part_sprite;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = Some(cursor_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(cursor_item);
    AssembleApplyResult::Assembled
}

fn apply_nomad_stack(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
) -> NomadStackApplyResult {
    let Some((kind, unit, template)) = world.items.get(&item_id).and_then(|item| {
        stack_kind(item.template_id).map(|kind| (kind, stack_unit(kind), stack_template(kind)))
    }) else {
        return NomadStackApplyResult::Bug("Bug #1442y");
    };
    let Some(character) = world.characters.get(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
    {
        return NomadStackApplyResult::MissingItem;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return split_nomad_stack(world, loader, item_id, character_id, kind, unit, template);
    };
    if cursor_item_id == item_id {
        return NomadStackApplyResult::MissingItem;
    }
    let Some(cursor_kind) = world
        .items
        .get(&cursor_item_id)
        .and_then(|item| stack_kind(item.template_id))
    else {
        return NomadStackApplyResult::CannotMix;
    };
    if cursor_kind != kind {
        return NomadStackApplyResult::CannotMix;
    }
    let cursor_value = world
        .items
        .get(&cursor_item_id)
        .map(|item| item.value)
        .unwrap_or_default();
    let cursor_count = world
        .items
        .get(&cursor_item_id)
        .map(stack_count)
        .unwrap_or_default();
    let Some(item) = world.items.get_mut(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    item.value = item.value.saturating_add(cursor_value);
    let count = stack_count(item).saturating_add(cursor_count);
    set_stack_count(item, count, kind);
    world.items.remove(&cursor_item_id);

    let Some(character) = world.characters.get_mut(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    NomadStackApplyResult::Merged { count, unit }
}

fn split_nomad_stack(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    stack_kind: StackKind,
    unit: &'static str,
    template: &'static str,
) -> NomadStackApplyResult {
    let Some(item) = world.items.get(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    let old_count = stack_count(item);
    if old_count < 2 {
        return NomadStackApplyResult::CannotSplitOne { unit };
    }
    let right = stack_split_amount(old_count / 2);
    let left = old_count - right;
    let old_value = item.value;

    let Ok(mut split_item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return NomadStackApplyResult::Bug("Bug #3199i");
    };
    split_item.value = old_value.saturating_mul(right) / old_count;
    set_stack_count(&mut split_item, right, stack_kind);
    let split_item_id = split_item.id;

    let Some(item) = world.items.get_mut(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    item.value = old_value.saturating_mul(left) / old_count;
    set_stack_count(item, left, stack_kind);

    let Some(character) = world.characters.get_mut(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return NomadStackApplyResult::MissingItem;
    }
    character.cursor_item = Some(split_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(split_item);
    NomadStackApplyResult::Split { left, right, unit }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackKind {
    Salt,
    Skin1,
    Skin2,
    BronzeChip,
    SilverChip,
    GoldChip,
}

fn stack_kind(template_id: u32) -> Option<StackKind> {
    match template_id {
        IID_AREA19_SALT => Some(StackKind::Salt),
        IID_AREA19_WOLFSSKIN => Some(StackKind::Skin1),
        IID_AREA19_WOLFSSKIN2 => Some(StackKind::Skin2),
        IID_BRONZECHIP => Some(StackKind::BronzeChip),
        IID_SILVERCHIP => Some(StackKind::SilverChip),
        IID_GOLDCHIP => Some(StackKind::GoldChip),
        _ => None,
    }
}

fn stack_template(kind: StackKind) -> &'static str {
    match kind {
        StackKind::Salt => "salt",
        StackKind::Skin1 => "skin1",
        StackKind::Skin2 => "skin2",
        StackKind::BronzeChip => "bronzechip",
        StackKind::SilverChip => "silverchip",
        StackKind::GoldChip => "goldchip",
    }
}

fn stack_unit(kind: StackKind) -> &'static str {
    match kind {
        StackKind::Salt => "ounce",
        StackKind::Skin1 | StackKind::Skin2 => "skin",
        StackKind::BronzeChip | StackKind::SilverChip | StackKind::GoldChip => "chip",
    }
}

fn stack_split_amount(mut amount: u32) -> u32 {
    for step in [10000, 5000, 2500, 1000, 500, 250, 100, 50, 25, 10] {
        if amount >= step {
            amount = step;
            break;
        }
    }
    amount
}

fn stack_count(item: &Item) -> u32 {
    let mut bytes = [0_u8; 4];
    for (idx, byte) in item.driver_data.iter().take(4).enumerate() {
        bytes[idx] = *byte;
    }
    u32::from_le_bytes(bytes)
}

fn set_stack_count(item: &mut Item, count: u32, kind: StackKind) {
    if item.driver_data.len() < 4 {
        item.driver_data.resize(4, 0);
    }
    item.driver_data[..4].copy_from_slice(&count.to_le_bytes());
    match kind {
        StackKind::Salt => {
            item.sprite = if count >= 10000 {
                13212
            } else if count >= 1000 {
                13211
            } else if count >= 100 {
                13210
            } else if count >= 10 {
                13209
            } else {
                13208
            };
            item.description = format!("{count} ounces of {}.", item.name);
        }
        StackKind::Skin1 => {
            item.sprite = skin_stack_sprite(count, 59655);
            item.description = format!("{count} {}s.", item.name);
        }
        StackKind::Skin2 => {
            item.sprite = skin_stack_sprite(count, 59660);
            item.description = format!("{count} {}s.", item.name);
        }
        StackKind::BronzeChip => set_chip_stack_data(item, count, 0),
        StackKind::SilverChip => set_chip_stack_data(item, count, 12),
        StackKind::GoldChip => set_chip_stack_data(item, count, 6),
    }
}

fn set_chip_stack_data(item: &mut Item, count: u32, sprite_offset: i32) {
    item.sprite = if count > 5 {
        53012 + sprite_offset
    } else if count == 5 {
        53011 + sprite_offset
    } else if count == 4 {
        53010 + sprite_offset
    } else if count == 3 {
        53009 + sprite_offset
    } else if count == 2 {
        53008 + sprite_offset
    } else {
        53007 + sprite_offset
    };
    item.description = if count > 1 {
        format!("{count} {}s.", item.name)
    } else {
        format!("{count} {}.", item.name)
    };
}

fn skin_stack_sprite(count: u32, base: i32) -> i32 {
    if count >= 5 {
        base + 4
    } else if count >= 4 {
        base + 3
    } else if count >= 3 {
        base + 2
    } else if count >= 2 {
        base + 1
    } else {
        base
    }
}

fn grant_money_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    amount: u32,
) -> bool {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return false;
    }
    let item_id = loader.allocate_item_id();
    let item = ugaris_core::entity::Item {
        id: item_id,
        name: "Money".to_string(),
        description: String::new(),
        flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::MONEY,
        sprite: 0,
        value: amount,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(character_id),
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: Vec::new(),
        serial: amount,
    };
    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    if character.cursor_item.is_some() {
        return false;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    true
}

fn resolve_zone_root(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        return path.exists().then(|| path.to_path_buf());
    }

    [
        PathBuf::from("ugaris_data/zones"),
        PathBuf::from("../ugaris_data/zones"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn load_area_zone(
    world: &mut World,
    loader: &mut ZoneLoader,
    zone_root: &Path,
    area_id: u16,
) -> anyhow::Result<ZoneLoadSummary> {
    let area_dir = zone_root.join(area_id.to_string());
    let map_file = first_file_with_extension(&area_dir, "map")?
        .ok_or_else(|| anyhow::anyhow!("no .map file found in {}", area_dir.display()))?;
    let map_text = std::fs::read_to_string(&map_file)?;
    let skipped_template_files = load_zone_templates(loader, zone_root, &area_dir)?;
    loader.apply_map_str(world, &map_text)?;
    let scheduled_light_timers = world.schedule_existing_light_timers();

    let (ground_tiles, blocked_tiles) = map_tile_counts(world);
    Ok(ZoneLoadSummary {
        root: zone_root.to_path_buf(),
        map_file,
        item_templates: loader.item_templates.len(),
        character_templates: loader.character_templates.len(),
        skipped_template_files,
        placed_items: world.items.len(),
        placed_characters: world.characters.len(),
        ground_tiles,
        blocked_tiles,
        scheduled_light_timers,
    })
}

fn next_available_character_id(world: &World) -> u32 {
    world
        .characters
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1)
}

fn load_zone_templates(
    loader: &mut ZoneLoader,
    zone_root: &Path,
    area_dir: &Path,
) -> anyhow::Result<usize> {
    let mut skipped = 0;
    for dir in [zone_root.join("generic"), area_dir.to_path_buf()] {
        skipped += load_zone_template_dir(loader, &dir, "itm")?;
        skipped += load_zone_template_dir(loader, &dir, "chr")?;
    }
    Ok(skipped)
}

fn load_zone_template_dir(
    loader: &mut ZoneLoader,
    dir: &Path,
    extension: &str,
) -> anyhow::Result<usize> {
    let mut skipped = 0;
    for file in files_with_extension(dir, extension)? {
        let text = std::fs::read_to_string(&file)?;
        let result = if extension.eq_ignore_ascii_case("itm") {
            loader.load_item_templates_str(&text)
        } else {
            loader.load_character_templates_str(&text)
        };
        if result.is_err() {
            warn!(file = %file.display(), error = %result.unwrap_err(), "skipping unsupported zone template file");
            skipped += 1;
        }
    }
    Ok(skipped)
}

fn first_file_with_extension(dir: &Path, extension: &str) -> anyhow::Result<Option<PathBuf>> {
    Ok(files_with_extension(dir, extension)?.into_iter().next())
}

fn files_with_extension(dir: &Path, extension: &str) -> anyhow::Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn map_tile_counts(world: &World) -> (usize, usize) {
    let mut ground_tiles = 0;
    let mut blocked_tiles = 0;
    for y in 0..world.map.height() {
        for x in 0..world.map.width() {
            let Some(tile) = world.map.tile(x, y) else {
                continue;
            };
            if tile.ground_sprite != 0 || tile.foreground_sprite != 0 {
                ground_tiles += 1;
            }
            if tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                blocked_tiles += 1;
            }
        }
    }
    (ground_tiles, blocked_tiles)
}

fn choose_spawn_tile(world: &World) -> (usize, usize) {
    if is_spawn_tile_open(world, LOGIN_SPAWN_X, LOGIN_SPAWN_Y) {
        return (LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    }

    for radius in 1..80 {
        let min_x = LOGIN_SPAWN_X.saturating_sub(radius);
        let max_x = (LOGIN_SPAWN_X + radius).min(world.map.width().saturating_sub(2));
        let min_y = LOGIN_SPAWN_Y.saturating_sub(radius);
        let max_y = (LOGIN_SPAWN_Y + radius).min(world.map.height().saturating_sub(2));
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if is_spawn_tile_open(world, x, y) {
                    return (x, y);
                }
            }
        }
    }

    for y in 1..world.map.height().saturating_sub(1) {
        for x in 1..world.map.width().saturating_sub(1) {
            if is_spawn_tile_open(world, x, y) {
                return (x, y);
            }
        }
    }

    (LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
}

fn is_spawn_tile_open(world: &World, x: usize, y: usize) -> bool {
    world.map.legacy_inner_bounds(x, y)
        && world.map.tile(x, y).is_some_and(|tile| {
            tile.character == 0
                && !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        })
}

fn set_character_value(values: &mut [Vec<i16>], value: CharacterValue, amount: i16) {
    let index = value as usize;
    values[0][index] = amount;
    values[1][index] = amount;
}

fn login_payload(
    world: &World,
    character: &Character,
    mirror_id: u16,
    tick: u64,
) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder
        .login_done()
        .ticker(tick.saturating_sub(1) as u32)
        .mirror(u32::from(mirror_id))
        .protocol(ugaris_protocol::packet::SERVER_PROTOCOL_VERSION)
        .origin(character.x, character.y);

    for value in 0..ugaris_core::entity::CHARACTER_VALUE_COUNT {
        builder.set_value0(value as u8, character.values[0][value]);
        builder.set_value1(value as u8, character.values[1][value]);
    }

    builder
        .set_hp((character.hp / POWERSCALE) as u16)
        .set_endurance((character.endurance / POWERSCALE) as u16)
        .set_mana((character.mana / POWERSCALE) as u16)
        .set_lifeshield((character.lifeshield / POWERSCALE) as u16)
        .exp(character.exp)
        .exp_used(character.exp_used)
        .gold(character.gold);

    let (cursor_sprite, cursor_flags) = character
        .cursor_item
        .and_then(|item_id| item_packet_fields(world, item_id))
        .unwrap_or((0, 0));
    builder.set_cursor_item(cursor_sprite, cursor_flags);

    for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
        let (sprite, flags) = character.inventory[slot]
            .and_then(|item_id| item_packet_fields(world, item_id))
            .unwrap_or((0, 0));
        builder.set_item(slot as u8, sprite, flags);
    }

    builder.system_text(LOGIN_ACCEPTED_MESSAGE);
    builder.into_payload()
}

fn item_packet_fields(world: &World, item_id: ugaris_core::ids::ItemId) -> Option<(u32, u32)> {
    world.items.get(&item_id).map(|item| {
        let sprite = item.sprite.max(0) as u32;
        let flags = item.flags.bits() as u32;
        (sprite, flags)
    })
}

fn inventory_snapshot_payload(world: &World, character: &Character) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    let (cursor_sprite, cursor_flags) = character
        .cursor_item
        .and_then(|item_id| item_packet_fields(world, item_id))
        .unwrap_or((0, 0));
    builder.set_cursor_item(cursor_sprite, cursor_flags);

    for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
        let (sprite, flags) = character.inventory[slot]
            .and_then(|item_id| item_packet_fields(world, item_id))
            .unwrap_or((0, 0));
        builder.set_item(slot as u8, sprite, flags);
    }

    builder.into_payload()
}

fn account_depot_payload(depot: &AccountDepotState) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder
        .container_type(1)
        .container_name("Your Account Depot")
        .container_count(depot.slots.len().min(u8::MAX as usize) as u8);
    for (slot, item) in depot.slots.iter().enumerate().take(u8::MAX as usize + 1) {
        builder.container_item(
            slot as u8,
            item.as_ref()
                .map(|item| item.sprite.max(0) as u32)
                .unwrap_or(0),
        );
    }
    builder.into_payload()
}

fn generic_container_item_ids(world: &World, container_id: ItemId) -> Vec<ItemId> {
    let mut items = world
        .items
        .values()
        .filter(|item| {
            item.contained_in == Some(container_id) && item.flags.contains(ItemFlags::USED)
        })
        .map(|item| item.id)
        .collect::<Vec<_>>();
    items.sort_unstable_by_key(|id| id.0);
    items.truncate(LEGACY_CONTAINER_SIZE);
    items
}

fn generic_container_payload(world: &World, container_id: ItemId) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    let name = world
        .items
        .get(&container_id)
        .map(|item| {
            if item.description.is_empty() {
                item.name.as_str()
            } else {
                item.description.as_str()
            }
        })
        .unwrap_or("Container");
    builder
        .container_type(1)
        .container_name(name)
        .container_count(LEGACY_CONTAINER_SIZE as u8);

    let item_ids = generic_container_item_ids(world, container_id);
    for slot in 0..LEGACY_CONTAINER_SIZE {
        let sprite = item_ids
            .get(slot)
            .and_then(|item_id| world.items.get(item_id))
            .map(|item| item.sprite.max(0) as u32)
            .unwrap_or(0);
        builder.container_item(slot as u8, sprite);
    }
    builder.into_payload()
}

fn current_container_payload(
    world: &World,
    depot: Option<&AccountDepotState>,
    character_id: CharacterId,
) -> Option<bytes::BytesMut> {
    let container_id = world.characters.get(&character_id)?.current_container?;
    let container = world.items.get(&container_id)?;
    if container.driver == IDR_ACCOUNT_DEPOT {
        depot.map(account_depot_payload)
    } else if container.content_id != 0 {
        Some(generic_container_payload(world, container_id))
    } else {
        None
    }
}

fn apply_account_depot_command(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
    action: &ClientAction,
) -> AccountDepotCommandResult {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AccountDepotCommandResult::Ignored;
    };
    let Some(container_id) = character.current_container else {
        return AccountDepotCommandResult::Ignored;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| item.driver != IDR_ACCOUNT_DEPOT)
    {
        return AccountDepotCommandResult::Ignored;
    }

    match *action {
        ClientAction::Container { slot, fast } => {
            let slot = usize::from(slot);
            if slot >= depot.slots.len() {
                return AccountDepotCommandResult::Ignored;
            }
            if fast && character.cursor_item.is_some() {
                return account_depot_store_cursor(world, depot, character_id);
            }
            account_depot_swap_slot(world, depot, character_id, slot)
        }
        ClientAction::LookContainer { slot } => {
            let Some(character) = world.characters.get(&character_id) else {
                return AccountDepotCommandResult::Ignored;
            };
            depot
                .slots
                .get(usize::from(slot))
                .and_then(Option::as_ref)
                .map(|item| legacy_item_look_text(item, character))
                .filter(|text| !text.is_empty())
                .map(AccountDepotCommandResult::Look)
                .unwrap_or(AccountDepotCommandResult::Ignored)
        }
        _ => AccountDepotCommandResult::Ignored,
    }
}

fn apply_item_container_command(
    world: &mut World,
    character_id: CharacterId,
    action: &ClientAction,
) -> AccountDepotCommandResult {
    let Some(container_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.current_container)
    else {
        return AccountDepotCommandResult::Ignored;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| item.content_id == 0 || item.driver == IDR_ACCOUNT_DEPOT)
    {
        return AccountDepotCommandResult::Ignored;
    }

    match *action {
        ClientAction::Container { slot, fast } => {
            apply_item_container_swap(world, character_id, container_id, usize::from(slot), fast)
        }
        ClientAction::LookContainer { slot } => {
            let Some(character) = world.characters.get(&character_id) else {
                return AccountDepotCommandResult::Ignored;
            };
            generic_container_item_ids(world, container_id)
                .get(usize::from(slot))
                .and_then(|item_id| world.items.get(item_id))
                .map(|item| legacy_item_look_text(item, character))
                .filter(|text| !text.is_empty())
                .map(AccountDepotCommandResult::Look)
                .unwrap_or(AccountDepotCommandResult::Ignored)
        }
        _ => AccountDepotCommandResult::Ignored,
    }
}

fn apply_item_container_swap(
    world: &mut World,
    character_id: CharacterId,
    container_id: ItemId,
    slot: usize,
    fast: bool,
) -> AccountDepotCommandResult {
    if slot >= LEGACY_CONTAINER_SIZE {
        return AccountDepotCommandResult::Ignored;
    }

    let cursor_id = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    if cursor_id.is_some_and(|item_id| {
        world
            .items
            .get(&item_id)
            .is_some_and(|item| item.flags.contains(ItemFlags::QUEST))
    }) {
        return AccountDepotCommandResult::Blocked(
            "You cannot store quest items in a container.".to_string(),
        );
    }

    let withdrawn_id = generic_container_item_ids(world, container_id)
        .get(slot)
        .copied();
    if cursor_id.is_none() && withdrawn_id.is_none() {
        return AccountDepotCommandResult::Ignored;
    }

    if let Some(item_id) = cursor_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = None;
            item.contained_in = Some(container_id);
            item.x = 0;
            item.y = 0;
        }
    }
    if let Some(item_id) = withdrawn_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = Some(character_id);
            item.contained_in = None;
            item.x = 0;
            item.y = 0;
        }
    }

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = withdrawn_id;
        if fast {
            if let Some(item_id) = withdrawn_id {
                if let Some(slot) = character
                    .inventory
                    .iter_mut()
                    .skip(INVENTORY_START_INVENTORY)
                    .find(|slot| slot.is_none())
                {
                    *slot = Some(item_id);
                    character.cursor_item = None;
                }
            }
        }
        character.flags.insert(CharacterFlags::ITEMS);
    }

    AccountDepotCommandResult::Changed
}

fn account_depot_swap_slot(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
    slot: usize,
) -> AccountDepotCommandResult {
    let cursor_id = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    if let Some(cursor_id) = cursor_id {
        if world
            .items
            .get(&cursor_id)
            .is_some_and(|item| item.flags.intersects(ItemFlags::QUEST | ItemFlags::NODEPOT))
        {
            return AccountDepotCommandResult::Blocked(
                "You cannot store this item in the depot.".to_string(),
            );
        }
    }

    let withdrawn = depot.slots[slot].take();
    let stored = cursor_id.and_then(|item_id| world.items.remove(&item_id));

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = None;
    } else {
        return AccountDepotCommandResult::Ignored;
    }
    if let Some(mut item) = stored {
        item.carried_by = None;
        item.contained_in = world
            .characters
            .get(&character_id)
            .and_then(|character| character.current_container);
        item.x = 0;
        item.y = 0;
        depot.slots[slot] = Some(item);
    }
    if let Some(mut item) = withdrawn {
        item.id = next_runtime_item_id(world);
        item.carried_by = Some(character_id);
        item.contained_in = None;
        item.x = 0;
        item.y = 0;
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.cursor_item = Some(item.id);
        }
        world.items.insert(item.id, item);
    }
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.flags.insert(CharacterFlags::ITEMS);
    }
    AccountDepotCommandResult::Changed
}

fn account_depot_store_cursor(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
) -> AccountDepotCommandResult {
    let Some(empty_slot) = depot.slots.iter().position(Option::is_none) else {
        return AccountDepotCommandResult::Ignored;
    };
    account_depot_swap_slot(world, depot, character_id, empty_slot)
}

fn account_depot_sort(depot: &mut AccountDepotState) {
    depot.slots.sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => right
            .sprite
            .cmp(&left.sprite)
            .then_with(|| right.value.cmp(&left.value))
            .then_with(|| {
                left.name[..left.name.len().min(35)].cmp(&right.name[..right.name.len().min(35)])
            }),
    });
}

const IDR_FLASK: u16 = 32;
const IDR_BEYONDPOTION: u16 = 133;
const IID_HARDKILL: u32 = (1 << 24) | 0x00005D;

fn legacy_item_look_text(item: &Item, character: &Character) -> String {
    if item.name.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("{}:", item.name)];
    if !item.description.is_empty() {
        lines.push(item.description.clone());
    }
    if item.template_id == IID_HARDKILL {
        lines.push(format!(
            "This is a level {} holy weapon.",
            item.driver_data.get(37).copied().unwrap_or_default()
        ));
    }

    let mut showed_modifiers = false;
    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if value == 0 || index < 0 {
            continue;
        }
        if !showed_modifiers {
            lines.push("Modifiers:".to_string());
            showed_modifiers = true;
        }
        let name = value_name(index);
        if item.driver == IDR_DECAYITEM {
            lines.push(format!(
                "{} +{} (active: {:+})",
                name,
                value,
                item.driver_data.get(2).copied().unwrap_or_default() as i8
            ));
        } else if index == CharacterValue::Armor as i16 {
            lines.push(format!("{} {:+.2}", name, f32::from(value) / 20.0));
        } else {
            lines.push(format!("{} {:+}", name, value));
        }
    }

    let mut showed_requirements = false;
    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if value == 0 || index >= 0 {
            continue;
        }
        if !showed_requirements {
            lines.push("Requirements:".to_string());
            showed_requirements = true;
        }
        let required_index = -index;
        let current = character
            .values
            .get(1)
            .and_then(|values| values.get(required_index as usize))
            .copied()
            .unwrap_or_default();
        lines.push(format!(
            "{} {} (you have {})",
            value_name(required_index),
            value,
            current
        ));
    }
    if !showed_requirements && (item.min_level != 0 || item.max_level != 0 || item.needs_class != 0)
    {
        lines.push("Requirements:".to_string());
    }
    if item.min_level != 0 {
        lines.push(format!("Minimum Level: {}", item.min_level));
    }
    if item.max_level != 0 {
        lines.push(format!("Maximum Level: {}", item.max_level));
    }
    if item.needs_class & 1 != 0 {
        lines.push("Only usable by a Warrior.".to_string());
    }
    if item.needs_class & 2 != 0 {
        lines.push("Only usable by a Mage.".to_string());
    }
    if item.needs_class & 4 != 0 {
        lines.push("Only usable by a Seyan'Du.".to_string());
    }
    if item.needs_class & 8 != 0 {
        lines.push("Only usable by an Arch.".to_string());
    }

    if item.flags.contains(ItemFlags::BONDTAKE) {
        let target = if item.owner_id == character.id.0 as i32 {
            ("you", "you")
        } else {
            ("somebody else", "he")
        };
        lines.push(format!(
            "This item is bonded to {}. Only {} can take it.",
            target.0, target.1
        ));
    }
    if item.flags.contains(ItemFlags::BONDWEAR) {
        let target = if item.owner_id == character.id.0 as i32 {
            ("you", "you")
        } else {
            ("somebody else", "he")
        };
        lines.push(format!(
            "This item is bonded to {}. Only {} can wear it.",
            target.0, target.1
        ));
    }
    if item.flags.contains(ItemFlags::QUEST) {
        lines.push("This is a quest item. You cannot drop it or give it away.".to_string());
    }
    if item.flags.contains(ItemFlags::NOENHANCE) {
        lines.push(
            "This item resists magic, so you cannot enhance it using orbs, metals or shrines."
                .to_string(),
        );
    }
    if item.flags.contains(ItemFlags::BEYONDMAXMOD) {
        lines.push("This item goes beyond maximum modifier limits.".to_string());
    }

    if item.driver == IDR_FLASK && item.driver_data.get(2).copied().unwrap_or_default() != 0 {
        lines.push(format!(
            "Duration: {} minutes.",
            item.driver_data.get(3).copied().unwrap_or_default()
        ));
    }
    if item.driver == IDR_BEYONDPOTION {
        lines.push(format!(
            "Duration: {} minutes.",
            item.driver_data.first().copied().unwrap_or_default()
        ));
    }
    if item.driver == IDR_DECAYITEM {
        let used = u16::from_le_bytes([
            item.driver_data.get(3).copied().unwrap_or_default(),
            item.driver_data.get(4).copied().unwrap_or_default(),
        ]) as u32
            * 2;
        let max = u16::from_le_bytes([
            item.driver_data.get(5).copied().unwrap_or_default(),
            item.driver_data.get(6).copied().unwrap_or_default(),
        ]) as u32
            * 2;
        lines.push(format!(
            "Duration: {}:{:02}:{:02} of {}:{:02}:{:02} active time used up.",
            used / 3600,
            (used / 60) % 60,
            used % 60,
            max / 3600,
            (max / 60) % 60,
            max % 60
        ));
    }

    if (59200..59299).contains(&item.sprite) || item.sprite == 59474 {
        lines.push("The item has been gilded.".to_string());
    }
    if (59299..=59390).contains(&item.sprite) || item.sprite == 59473 {
        lines.push("The item has been silvered.".to_string());
    }
    if (53000..=53006).contains(&item.sprite) {
        lines.push("This is part of an earth demon suit.".to_string());
    }
    if (53025..=53030).contains(&item.sprite) {
        lines.push("This is part of an ice demon suit.".to_string());
    }
    if (53031..=53036).contains(&item.sprite) {
        lines.push("This is part of an fire demon suit.".to_string());
    }

    lines.join("\n")
}

fn value_name(index: i16) -> &'static str {
    CHARACTER_VALUE_NAMES
        .get(index as usize)
        .copied()
        .unwrap_or("Unknown")
}

fn next_runtime_item_id(world: &World) -> ItemId {
    let next = world
        .items
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or_default()
        .saturating_add(1)
        .max(1);
    ItemId(next)
}

fn login_bootstrap_payloads(
    world: &World,
    character: &Character,
    mirror_id: u16,
    tick: u64,
    view_distance: usize,
    effect_cache: &mut ClientEffectCache,
) -> Vec<bytes::BytesMut> {
    let mut payloads = vec![login_payload(world, character, mirror_id, tick)];
    payloads.extend(initial_map_payloads(world, character, view_distance));
    payloads.extend(client_effect_payloads(
        world,
        character,
        view_distance,
        effect_cache,
    ));
    payloads
}

fn map_refresh_payloads(
    world: &World,
    character: &Character,
    view_distance: usize,
) -> Vec<bytes::BytesMut> {
    let mut builder = PacketBuilder::new();
    builder.origin(character.x, character.y);

    let mut payloads = vec![builder.into_payload()];
    payloads.extend(initial_map_payloads(world, character, view_distance));
    payloads
}

fn visible_map_cache(
    world: &World,
    character: &Character,
    view_distance: usize,
) -> VisibleMapCache {
    let mut known_character_names = HashMap::new();
    let cells = legacy_diamond_positions(character, view_distance)
        .filter_map(|(client_pos, map_x, map_y)| {
            let tile = world.map.tile(map_x, map_y)?;
            let visible_character = tile_character(world, tile);
            let character_id = visible_character.map(client_character_id);
            let character_packet = visible_character
                .map(|character| map_character_packet(character, client_pos).to_vec());
            let character_name_packet = visible_character.map(|character| {
                let packet = character_name_packet(character).to_vec();
                known_character_names.insert(client_character_id(character), packet.clone());
                packet
            });
            Some((
                client_pos,
                VisibleMapCell {
                    effect_packet: map_effect_packet(tile, client_pos).to_vec(),
                    tile_packet: map_tile_packet(world, tile, client_pos).to_vec(),
                    character_id,
                    character_packet,
                    character_name_packet,
                },
            ))
        })
        .collect();

    VisibleMapCache {
        center_x: character.x,
        center_y: character.y,
        view_distance,
        cells,
        known_character_names,
    }
}

fn map_diff_payloads(
    world: &World,
    character: &Character,
    view_distance: usize,
    cache: &mut VisibleMapCache,
) -> Vec<bytes::BytesMut> {
    if cache.center_x != character.x
        || cache.center_y != character.y
        || cache.view_distance != view_distance
    {
        *cache = visible_map_cache(world, character, view_distance);
        return map_refresh_payloads(world, character, view_distance);
    }

    let next_cache = visible_map_cache(world, character, view_distance);
    let mut payloads = Vec::new();
    let mut current = bytes::BytesMut::new();

    for (client_pos, next_cell) in &next_cache.cells {
        match cache.cells.get(client_pos) {
            Some(previous) if previous.effect_packet == next_cell.effect_packet => {}
            _ => append_map_packet(
                &mut payloads,
                &mut current,
                bytes::BytesMut::from(&next_cell.effect_packet[..]),
            ),
        }

        match cache.cells.get(client_pos) {
            Some(previous) if previous.tile_packet == next_cell.tile_packet => {}
            _ => append_map_packet(
                &mut payloads,
                &mut current,
                bytes::BytesMut::from(&next_cell.tile_packet[..]),
            ),
        }

        let previous_character = cache
            .cells
            .get(client_pos)
            .and_then(|cell| cell.character_packet.as_ref());
        if let (Some(character_id), Some(name_packet)) = (
            next_cell.character_id,
            next_cell.character_name_packet.as_ref(),
        ) {
            if cache.known_character_names.get(&character_id) != Some(name_packet) {
                append_map_packet(
                    &mut payloads,
                    &mut current,
                    bytes::BytesMut::from(&name_packet[..]),
                );
            }
        }
        match (previous_character, next_cell.character_packet.as_ref()) {
            (Some(previous), Some(next)) if previous == next => {}
            (_, Some(next)) => {
                append_map_packet(
                    &mut payloads,
                    &mut current,
                    bytes::BytesMut::from(&next[..]),
                );
            }
            (Some(_), None) => append_map_packet(
                &mut payloads,
                &mut current,
                map_character_clear_packet(*client_pos),
            ),
            (None, None) => {}
        }
    }

    for client_pos in cache.cells.keys() {
        if !next_cache.cells.contains_key(client_pos) {
            append_map_packet(
                &mut payloads,
                &mut current,
                map_character_clear_packet(*client_pos),
            );
        }
    }

    if !current.is_empty() {
        payloads.push(current);
    }
    *cache = next_cache;
    payloads
}

fn client_effect_payloads(
    world: &World,
    viewer: &Character,
    view_distance: usize,
    cache: &mut ClientEffectCache,
) -> Vec<bytes::BytesMut> {
    let mut visible_effects: Vec<_> = world
        .effects
        .iter()
        .filter_map(|(&effect_id, effect)| {
            visible_client_effect_body(effect_id, effect, world, viewer, view_distance).map(
                |body| {
                    (
                        effect_id,
                        effect.serial,
                        body.into_iter().collect::<Vec<u8>>(),
                    )
                },
            )
        })
        .collect();
    visible_effects.sort_by_key(|(effect_id, _, _)| *effect_id);
    visible_effects.truncate(MAX_CLIENT_EFFECTS);

    let mut payloads = Vec::new();
    let mut used = vec![false; cache.slots.len()];
    let mut pending = Vec::new();

    for (effect_id, serial, body) in visible_effects {
        if let Some(slot_index) = cache.slots.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|slot| slot.effect_id == effect_id)
        }) {
            used[slot_index] = true;
            let slot = cache.slots[slot_index].as_mut().expect("slot exists");
            if slot.serial != serial || slot.body != body {
                payloads.push(ugaris_protocol::packet::client_effect(
                    slot_index as u8,
                    &body,
                ));
                slot.serial = serial;
                slot.body = body;
            }
        } else {
            pending.push((effect_id, serial, body));
        }
    }

    for (effect_id, serial, body) in pending {
        let Some(slot_index) = used.iter().position(|used| !*used) else {
            break;
        };
        used[slot_index] = true;
        cache.slots[slot_index] = Some(ClientEffectSlot {
            effect_id,
            serial,
            body: body.clone(),
        });
        payloads.push(ugaris_protocol::packet::client_effect(
            slot_index as u8,
            &body,
        ));
    }

    let used_mask =
        used.iter().enumerate().fold(
            0_u64,
            |mask, (index, used)| {
                if *used {
                    mask | (1_u64 << index)
                } else {
                    mask
                }
            },
        );
    if used_mask != cache.used_mask {
        cache.used_mask = used_mask;
        payloads.push(bytes::BytesMut::from(
            &ugaris_protocol::packet::used_effects(used_mask)[..],
        ));
    }

    payloads
}

fn visible_client_effect_body(
    effect_id: u32,
    effect: &Effect,
    world: &World,
    viewer: &Character,
    view_distance: usize,
) -> Option<bytes::BytesMut> {
    if !effect_visible_to_viewer(effect, world, viewer, view_distance) {
        return None;
    }

    match effect.effect_type {
        EF_MAGICSHIELD => Some(ugaris_protocol::packet::ceffect_shield(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_BALL => Some(ugaris_protocol::packet::ceffect_ball(
            effect_id as i32,
            effect.start_tick,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_FIREBALL => Some(ugaris_protocol::packet::ceffect_fireball(
            effect_id as i32,
            effect.start_tick,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_FLASH => Some(ugaris_protocol::packet::ceffect_flash(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_WARCRY => Some(ugaris_protocol::packet::ceffect_warcry(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.stop_tick,
        )),
        EF_BLESS => Some(ugaris_protocol::packet::ceffect_bless(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_HEAL => Some(ugaris_protocol::packet::ceffect_heal(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_FREEZE => Some(ugaris_protocol::packet::ceffect_freeze(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
        )),
        EF_STRIKE => Some(ugaris_protocol::packet::ceffect_strike(
            effect_id as i32,
            effect
                .target_character
                .map(|character_id| character_id.0 as i32)
                .unwrap_or_default(),
            effect.x,
            effect.y,
        )),
        EF_BURN => Some(ugaris_protocol::packet::ceffect_burn(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.stop_tick,
        )),
        EF_POTION => Some(ugaris_protocol::packet::ceffect_potion(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_PULSE => Some(ugaris_protocol::packet::ceffect_pulse(
            effect_id as i32,
            effect.start_tick,
        )),
        EF_PULSEBACK => Some(ugaris_protocol::packet::ceffect_pulseback(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.x,
            effect.y,
        )),
        EF_FIRERING => Some(ugaris_protocol::packet::ceffect_firering(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
        )),
        EF_EXPLODE => Some(ugaris_protocol::packet::ceffect_explode(
            effect_id as i32,
            effect.start_tick,
            effect.base_sprite,
        )),
        EF_MIST => Some(ugaris_protocol::packet::ceffect_mist(
            effect_id as i32,
            effect.start_tick,
        )),
        EF_EARTHRAIN => Some(ugaris_protocol::packet::ceffect_earthrain(
            effect_id as i32,
            effect.strength,
        )),
        EF_EARTHMUD => Some(ugaris_protocol::packet::ceffect_earthmud(effect_id as i32)),
        EF_EDEMONBALL => Some(ugaris_protocol::packet::ceffect_edemonball(
            effect_id as i32,
            effect.start_tick,
            effect.base_sprite,
            effect.from_x,
            effect.from_y,
            effect.to_x,
            effect.to_y,
        )),
        EF_CURSE => Some(ugaris_protocol::packet::ceffect_curse(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
            effect.start_tick,
            effect.stop_tick,
            effect.strength,
        )),
        EF_CAP => Some(ugaris_protocol::packet::ceffect_cap(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_LAG => Some(ugaris_protocol::packet::ceffect_lag(
            effect_id as i32,
            effect_character_id(effect)?.0 as i32,
        )),
        EF_BUBBLE => Some(ugaris_protocol::packet::ceffect_bubble(
            effect_id as i32,
            effect.strength,
        )),
        _ => None,
    }
}

fn effect_character_id(effect: &Effect) -> Option<CharacterId> {
    effect.target_character.or(effect.caster)
}

fn effect_visible_to_viewer(
    effect: &Effect,
    world: &World,
    viewer: &Character,
    view_distance: usize,
) -> bool {
    let (x, y) = match effect.effect_type {
        EF_BALL | EF_FIREBALL | EF_EDEMONBALL => (effect.x / 1024, effect.y / 1024),
        EF_STRIKE | EF_PULSE | EF_EXPLODE | EF_MIST | EF_EARTHRAIN | EF_EARTHMUD | EF_BUBBLE => {
            (effect.x, effect.y)
        }
        EF_MAGICSHIELD | EF_FLASH | EF_WARCRY | EF_BLESS | EF_HEAL | EF_FREEZE | EF_BURN
        | EF_POTION | EF_CURSE | EF_CAP | EF_LAG | EF_PULSEBACK | EF_FIRERING => {
            let Some(character_id) = effect_character_id(effect) else {
                return false;
            };
            let Some(character) = world.characters.get(&character_id) else {
                return false;
            };
            (i32::from(character.x), i32::from(character.y))
        }
        _ => return false,
    };
    let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
        return false;
    };
    map_position_in_diamond(x, y, viewer.x, viewer.y, view_distance)
}

fn queue_periodic_player_frames(runtime: &mut ServerRuntime, world: &World) -> (usize, usize) {
    let sessions: Vec<_> = runtime
        .players
        .iter()
        .filter_map(|(&session_id, player)| {
            if player.state != PlayerConnectionState::Normal {
                return None;
            }
            Some((session_id, player.character_id?, player.view_distance))
        })
        .collect();

    let mut diff_sessions = 0;
    let mut empty_frames = 0;
    for (session_id, character_id, view_distance) in sessions {
        let Some(character) = world.characters.get(&character_id) else {
            continue;
        };
        let mut payloads = match runtime.map_caches.get_mut(&session_id) {
            Some(cache) => map_diff_payloads(world, character, view_distance, cache),
            None => {
                let payloads = map_refresh_payloads(world, character, view_distance);
                runtime.map_caches.insert(
                    session_id,
                    visible_map_cache(world, character, view_distance),
                );
                payloads
            }
        };
        payloads.extend(client_effect_payloads(
            world,
            character,
            view_distance,
            runtime.effect_caches.entry(session_id).or_default(),
        ));

        if payloads.is_empty() {
            if runtime.send_to_session(session_id, bytes::BytesMut::new()) {
                empty_frames += 1;
            }
        } else if runtime.send_many_to_session(session_id, payloads) {
            diff_sessions += 1;
        }
    }

    (diff_sessions, empty_frames)
}

fn look_map_payloads(world: &World, area_id: u16, request: LookMapRequest) -> Vec<bytes::BytesMut> {
    if !request.visible {
        return vec![ugaris_protocol::packet::system_text(
            "Too far away or hidden.",
        )];
    }

    let mut messages = Vec::new();
    messages.push(section_look_text(
        area_id,
        request.x,
        request.y,
        request.character_level,
    ));

    if let Some(tile) = world.map.tile(request.x, request.y) {
        if tile.flags.contains(MapFlags::RESTAREA) {
            messages.push("This place is a rest area.".to_string());
        }
        if tile.flags.contains(MapFlags::CLAN) {
            messages.push("This is a clan area.".to_string());
        }
        if tile.flags.contains(MapFlags::ARENA) {
            messages.push("This place is an arena.".to_string());
        }
        if tile.flags.contains(MapFlags::PEACE) {
            messages.push("This place is a peaceful zone.".to_string());
        }
    }

    messages
        .into_iter()
        .map(|message| ugaris_protocol::packet::system_text(&message))
        .collect()
}

fn walk_section_payload(
    area_id: u16,
    player: &mut PlayerRuntime,
    character: &Character,
) -> Option<bytes::BytesMut> {
    let next_section = section_at(area_id, usize::from(character.x), usize::from(character.y));
    let next_section_id = next_section.map_or(0, |section| section.id);
    if next_section_id == player.current_section_id {
        return None;
    }

    let message = if let Some(section) = next_section {
        format!("Now entering {}.", section.name)
    } else if let Some(name) = section_name_by_id(player.current_section_id) {
        format!("Now leaving {name}.")
    } else {
        player.current_section_id = next_section_id;
        return None;
    };

    player.current_section_id = next_section_id;
    let mut bytes = Vec::with_capacity(COL_DARK_GRAY.len() + message.len());
    bytes.extend_from_slice(COL_DARK_GRAY);
    bytes.extend_from_slice(message.as_bytes());
    let mut payload = ugaris_protocol::packet::system_text_bytes(&bytes);
    if let Some(section) = next_section {
        if let Some(music) = section_music_special(section.id) {
            payload.extend_from_slice(&ugaris_protocol::packet::special(music, u32::MAX, 0));
        }
    }
    Some(payload)
}

fn section_music_special(section_id: u16) -> Option<u32> {
    match section_id {
        4 | 17 | 18 | 19 | 29..=44 | 46..=48 | 50 => Some(1003),
        57 | 59 => Some(1010),
        58 | 68..=70 => Some(1004),
        60..=66 => Some(1002),
        _ => None,
    }
}

fn area_sound_payload(
    area_id: u16,
    character: &Character,
    hour: i64,
    random_seed: u64,
) -> Option<[u8; 13]> {
    let section = section_at(area_id, usize::from(character.x), usize::from(character.y))?;
    let sound = area_sound_special(
        section.id,
        hour,
        legacy_random(random_seed, 100),
        legacy_random(random_seed.wrapping_add(1), 1000),
        legacy_random(random_seed.wrapping_add(2), 10000),
    )?;
    Some(ugaris_protocol::packet::special(
        sound.special_type,
        sound.opt1 as u32,
        sound.opt2 as u32,
    ))
}

fn movement_scroll_payload(
    world: &World,
    character: &Character,
    old_x: u16,
    old_y: u16,
    view_distance: usize,
) -> Option<bytes::BytesMut> {
    let scroll = scroll_command(old_x, old_y, character.x, character.y)?;
    let mut builder = PacketBuilder::new();
    builder.scroll(scroll).origin(character.x, character.y);

    if let Some(old_pos) =
        old_relative_client_position(old_x, old_y, character.x, character.y, view_distance)
    {
        builder.raw(&map_character_clear_packet(old_pos));
    }
    builder.raw(&map_character_packet(
        character,
        client_center_map_position(view_distance),
    ));
    let mut payload = builder.into_payload();

    for (client_pos, map_x, map_y) in
        movement_fringe_positions(character, old_x, old_y, view_distance)
    {
        let Some(tile) = world.map.tile(map_x, map_y) else {
            continue;
        };
        payload.extend_from_slice(&map_tile_packet(world, tile, client_pos));
        if let Some(character) = tile_character(world, tile) {
            payload.extend_from_slice(&character_name_packet(character));
            payload.extend_from_slice(&map_character_packet(character, client_pos));
        }
    }

    Some(payload)
}

fn movement_fringe_positions(
    viewer: &Character,
    old_x: u16,
    old_y: u16,
    view_distance: usize,
) -> Vec<(u16, usize, usize)> {
    legacy_diamond_positions(viewer, view_distance)
        .filter(|(_, map_x, map_y)| {
            !map_position_in_diamond(*map_x, *map_y, old_x, old_y, view_distance)
        })
        .collect()
}

fn map_position_in_diamond(
    map_x: usize,
    map_y: usize,
    center_x: u16,
    center_y: u16,
    view_distance: usize,
) -> bool {
    let dx = map_x as i32 - i32::from(center_x);
    let dy = map_y as i32 - i32::from(center_y);
    dx.abs() + dy.abs() <= view_distance as i32
}

fn scroll_command(old_x: u16, old_y: u16, new_x: u16, new_y: u16) -> Option<u8> {
    match (
        i32::from(new_x) - i32::from(old_x),
        i32::from(new_y) - i32::from(old_y),
    ) {
        (1, 0) => Some(SV_SCROLL_RIGHT),
        (-1, 0) => Some(SV_SCROLL_LEFT),
        (0, 1) => Some(SV_SCROLL_DOWN),
        (0, -1) => Some(SV_SCROLL_UP),
        (-1, -1) => Some(SV_SCROLL_LEFTUP),
        (-1, 1) => Some(SV_SCROLL_LEFTDOWN),
        (1, -1) => Some(SV_SCROLL_RIGHTUP),
        (1, 1) => Some(SV_SCROLL_RIGHTDOWN),
        _ => None,
    }
}

fn old_relative_client_position(
    old_x: u16,
    old_y: u16,
    new_x: u16,
    new_y: u16,
    view_distance: usize,
) -> Option<u16> {
    let side = view_distance.checked_mul(2)?.checked_add(1)?;
    let client_x = i32::from(old_x) - i32::from(new_x) + view_distance as i32;
    let client_y = i32::from(old_y) - i32::from(new_y) + view_distance as i32;
    if client_x < 0 || client_y < 0 || client_x >= side as i32 || client_y >= side as i32 {
        return None;
    }
    Some((client_x as usize + client_y as usize * side).min(u16::MAX as usize) as u16)
}

fn map_character_clear_packet(client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_delta(
        MapLayer::Character,
        MapPosition::Absolute(client_pos),
        MAP_CHARACTER_CLEAR,
        &[],
    )
    .expect("fixed character clear map field mask is valid")
}

fn initial_map_payloads(
    world: &World,
    viewer: &Character,
    view_distance: usize,
) -> Vec<bytes::BytesMut> {
    let mut payloads = Vec::new();
    let mut current = bytes::BytesMut::new();

    for (client_pos, map_x, map_y) in legacy_diamond_positions(viewer, view_distance) {
        let Some(tile) = world.map.tile(map_x, map_y) else {
            continue;
        };

        if tile.effects.iter().any(|effect| *effect != 0) {
            append_map_packet(
                &mut payloads,
                &mut current,
                map_effect_packet(tile, client_pos),
            );
        }

        append_map_packet(
            &mut payloads,
            &mut current,
            map_tile_packet(world, tile, client_pos),
        );

        if let Some(character) = tile_character(world, tile) {
            append_map_packet(
                &mut payloads,
                &mut current,
                character_name_packet(character),
            );
            append_map_packet(
                &mut payloads,
                &mut current,
                map_character_packet(character, client_pos),
            );
        }
    }

    if !current.is_empty() {
        payloads.push(current);
    }
    payloads
}

fn append_map_packet(
    payloads: &mut Vec<bytes::BytesMut>,
    current: &mut bytes::BytesMut,
    packet: bytes::BytesMut,
) {
    if !current.is_empty() && current.len() + packet.len() > MAP_BOOTSTRAP_CHUNK_TARGET {
        payloads.push(std::mem::take(current));
    }
    current.extend_from_slice(&packet);
}

fn map_effect_packet(tile: &MapTile, client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_effects_basic(MapPosition::Absolute(client_pos), tile.effects)
        .expect("fixed effect map field mask is valid")
}

fn map_tile_packet(world: &World, tile: &MapTile, client_pos: u16) -> bytes::BytesMut {
    let (item_sprite, item_flags) = (tile.item != 0)
        .then_some(tile.item)
        .and_then(|id| world.items.get(&ugaris_core::ids::ItemId(id)))
        .map(|item| (item.sprite.max(0) as u32, item.flags))
        .unwrap_or((0, ItemFlags::empty()));

    ugaris_protocol::packet::map_tile_basic(
        MapPosition::Absolute(client_pos),
        tile.ground_sprite,
        tile.foreground_sprite,
        item_sprite,
        client_map_flags(tile, item_flags),
    )
    .expect("fixed tile map field mask is valid")
}

fn client_map_flags(tile: &MapTile, item_flags: ItemFlags) -> u16 {
    let mut flags = CMF_VISIBLE | 1;
    if tile
        .flags
        .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
    {
        flags |= CMF_LIGHT;
    }
    if tile.flags.contains(MapFlags::UNDERWATER) {
        flags |= CMF_UNDERWATER;
    }
    if tile.flags.contains(MapFlags::SINK_ANKLE) {
        flags |= CMF_SINK_ANKLE;
    }
    if tile.flags.contains(MapFlags::SINK_KNEE) {
        flags |= CMF_SINK_KNEE;
    }
    if tile.flags.contains(MapFlags::SINK_BELLY) {
        flags |= CMF_SINK_BELLY;
    }
    if tile.flags.contains(MapFlags::SINK_CHEST) {
        flags |= CMF_SINK_CHEST;
    }
    if item_flags.contains(ItemFlags::TAKE) {
        flags |= CMF_TAKE;
    }
    if item_flags.contains(ItemFlags::USE) {
        flags |= CMF_USE;
    }
    flags
}

fn tile_character<'a>(world: &'a World, tile: &MapTile) -> Option<&'a Character> {
    (tile.character != 0)
        .then_some(CharacterId(u32::from(tile.character)))
        .and_then(|id| world.characters.get(&id))
}

fn map_character_packet(character: &Character, client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_character_basic(
        MapPosition::Absolute(client_pos),
        character.sprite.max(0) as u32,
        character.id.0 as u16,
        CharacterMapAction {
            action: character.action.min(u16::from(u8::MAX)) as u8,
            duration: character.duration.clamp(0, i32::from(u8::MAX)) as u8,
            step: character.step.clamp(0, i32::from(u8::MAX)) as u8,
        },
        CharacterMapStatus {
            dir: character.dir,
            health: resource_percent(
                character.hp,
                character.values[0][CharacterValue::Hp as usize],
            ),
            mana: resource_percent(
                character.mana,
                character.values[0][CharacterValue::Mana as usize],
            ),
            shield: resource_percent(
                character.lifeshield,
                character.values[0][CharacterValue::MagicShield as usize]
                    .max(character.values[0][CharacterValue::Rage as usize]),
            ),
        },
    )
    .expect("fixed character map field mask is valid")
}

fn client_character_id(character: &Character) -> u16 {
    character.id.0 as u16
}

fn character_name_packet(character: &Character) -> bytes::BytesMut {
    let name = if character.flags.contains(CharacterFlags::WON) {
        if character.flags.contains(CharacterFlags::FEMALE) {
            format!("Lady {}", character.name)
        } else {
            format!("Sir {}", character.name)
        }
    } else {
        character.name.clone()
    };

    ugaris_protocol::packet::character_name(
        client_character_id(character),
        character.level.min(u32::from(u8::MAX)) as u8,
        [0, 0, 0],
        0,
        0,
        &name,
    )
}

fn legacy_diamond_positions(
    viewer: &Character,
    view_distance: usize,
) -> impl Iterator<Item = (u16, usize, usize)> {
    let xoff = i32::from(viewer.x) - view_distance as i32;
    let yoff = i32::from(viewer.y) - view_distance as i32;
    let side = view_distance * 2 + 1;

    (0..=view_distance * 2).flat_map(move |y| {
        let (xs, xe) = if y < view_distance {
            (view_distance - y, view_distance + y)
        } else {
            (y - view_distance, view_distance * 3 - y)
        };

        (xs..=xe).filter_map(move |x| {
            let map_x = xoff + x as i32;
            let map_y = yoff + y as i32;
            if map_x < 1 || map_y < 1 {
                return None;
            }
            Some(((x + y * side) as u16, map_x as usize, map_y as usize))
        })
    })
}

fn client_center_map_position(view_distance: usize) -> u16 {
    let side = view_distance.saturating_mul(2).saturating_add(1);
    view_distance
        .saturating_add(view_distance.saturating_mul(side))
        .min(u16::MAX as usize) as u16
}

fn resource_percent(current: i32, max_value: i16) -> u8 {
    let max_value = i32::from(max_value).max(1);
    let percent = (current / (POWERSCALE / 100)) / max_value;
    percent.clamp(0, i32::from(u8::MAX)) as u8
}

fn apply_player_action(player: &mut PlayerRuntime, action: &ClientAction, current_tick: u64) {
    match action {
        ClientAction::Move { x, y } => player.driver_move(*x as i32, *y as i32),
        ClientAction::Drop { x, y } => player.driver_drop(*x as i32, *y as i32),
        ClientAction::Teleport { teleport, mirror } => {
            player.driver_teleport((*teleport as i32) + (*mirror as i32 * 256));
        }
        ClientAction::WalkDir { direction } if *direction == 0 => {
            player.driver_stop(current_tick, false);
        }
        ClientAction::WalkDir { direction } => player.set_pending_action(QueuedAction {
            action: PlayerActionCode::WalkDir,
            arg1: *direction as i32,
            arg2: 0,
        }),
        ClientAction::MapSpell { spell, x, y } => {
            if *x == 0 {
                player.driver_charspell(
                    spell_to_player_action(*spell, true),
                    ugaris_core::ids::CharacterId(*y as u32),
                    0,
                );
            } else {
                player.driver_mapspell(spell_to_player_action(*spell, false), *x as i32, *y as i32);
            }
        }
        ClientAction::SelfSpell { spell } => {
            player.driver_selfspell(spell_to_player_action(*spell, false));
        }
        ClientAction::CharacterSpell { spell, character } => player.driver_charspell(
            spell_to_player_action(*spell, false),
            ugaris_core::ids::CharacterId(*character as u32),
            0,
        ),
        ClientAction::Text(bytes) => player.command = bytes.clone(),
        ClientAction::Ticker { tick } => player.client_ticker = *tick,
        ClientAction::Stop => player.driver_stop(current_tick, false),
        _ => {
            if let Some(queued) = action_to_queued(action) {
                player.set_pending_action(queued);
            }
        }
    }
}

fn action_to_queued(action: &ClientAction) -> Option<QueuedAction> {
    let queued = match action {
        ClientAction::Move { x, y } => queued(PlayerActionCode::Move, *x, *y),
        ClientAction::Take { x, y } => queued(PlayerActionCode::Take, *x, *y),
        ClientAction::Drop { x, y } => queued(PlayerActionCode::Drop, *x, *y),
        ClientAction::Kill { character } => queued1(PlayerActionCode::Kill, *character),
        ClientAction::UseMap { x, y } => queued(PlayerActionCode::Use, *x, *y),
        ClientAction::CharacterSpell { spell, character } => {
            queued1(spell_to_player_action(*spell, true), *character)
        }
        ClientAction::MapSpell { spell, x, y } => {
            if *x == 0 {
                queued1(spell_to_player_action(*spell, true), *y)
            } else {
                queued(spell_to_player_action(*spell, false), *x, *y)
            }
        }
        ClientAction::SelfSpell { spell } => queued0(spell_to_player_action(*spell, false)),
        ClientAction::LookMap { x, y } => queued(PlayerActionCode::LookMap, *x, *y),
        ClientAction::Give { character } => queued1(PlayerActionCode::Give, *character),
        ClientAction::Teleport { teleport, mirror } => QueuedAction {
            action: PlayerActionCode::Teleport,
            arg1: (*teleport as i32) + (*mirror as i32 * 256),
            arg2: 0,
        },
        ClientAction::WalkDir { direction } if *direction != 0 => {
            queued1(PlayerActionCode::WalkDir, *direction as u16)
        }
        _ => return None,
    };
    Some(queued)
}

fn spell_to_player_action(spell: SpellAction, character_target: bool) -> PlayerActionCode {
    match (spell, character_target) {
        (SpellAction::Bless, _) => PlayerActionCode::Bless,
        (SpellAction::Heal, _) => PlayerActionCode::Heal,
        (SpellAction::Freeze, _) => PlayerActionCode::Freeze,
        (SpellAction::Fireball, true) => PlayerActionCode::FireballCharacter,
        (SpellAction::Fireball, false) => PlayerActionCode::Fireball,
        (SpellAction::Ball, true) => PlayerActionCode::BallCharacter,
        (SpellAction::Ball, false) => PlayerActionCode::Ball,
        (SpellAction::MagicShield, _) => PlayerActionCode::MagicShield,
        (SpellAction::Flash, _) => PlayerActionCode::Flash,
        (SpellAction::Warcry, _) => PlayerActionCode::Warcry,
        (SpellAction::Pulse, _) => PlayerActionCode::Pulse,
    }
}

fn queued(action: PlayerActionCode, x: u16, y: u16) -> QueuedAction {
    QueuedAction {
        action,
        arg1: x as i32,
        arg2: y as i32,
    }
}

fn queued1(action: PlayerActionCode, arg: u16) -> QueuedAction {
    QueuedAction {
        action,
        arg1: arg as i32,
        arg2: 0,
    }
}

fn queued0(action: PlayerActionCode) -> QueuedAction {
    QueuedAction {
        action,
        arg1: 0,
        arg2: 0,
    }
}

#[cfg(test)]
mod tests {
    use ugaris_protocol::packet::{
        MAP_CHARACTER_ACTION, MAP_CHARACTER_SPRITE, MAP_CHARACTER_STATUS, MAP_EFFECT_0,
        MAP_EFFECT_1, MAP_EFFECT_2, MAP_EFFECT_3, MAP_TILE_FLAGS, MAP_TILE_FSPRITE,
        MAP_TILE_GSPRITE, MAP_TILE_ISPRITE, SV_CONCNT, SV_CONNAME, SV_CONTAINER, SV_CONTYPE,
        SV_LOGINDONE, SV_MAP01, SV_MAP10, SV_MAP11, SV_MAPPOS, SV_MIRROR, SV_ORIGIN, SV_PROTOCOL,
        SV_SETCITEM, SV_SETHP, SV_SETITEM, SV_SETVAL0, SV_SETVAL1, SV_SPECIAL, SV_TEXT, SV_TICKER,
    };

    use super::*;

    #[test]
    fn character_fireball_command_queues_character_target_action() {
        let queued = action_to_queued(&ClientAction::CharacterSpell {
            spell: SpellAction::Fireball,
            character: 42,
        })
        .unwrap();

        assert_eq!(queued.action, PlayerActionCode::FireballCharacter);
        assert_eq!((queued.arg1, queued.arg2), (42, 0));
    }

    #[test]
    fn timer_outcome_feedback_matches_legacy_torch_messages() {
        let feedback = timer_outcome_feedback(&[
            ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: 30 * ugaris_core::tick::TICKS_PER_SECOND,
            },
            ugaris_core::item_driver::ItemDriverOutcome::TorchExpired {
                item_id: ItemId(8),
                character_id: CharacterId(2),
                item_name: ugaris_core::item_driver::outcome_item_name("torch"),
            },
        ]);

        assert_eq!(
            feedback,
            vec![
                (CharacterId(1), TORCH_HISS_MESSAGE.to_string()),
                (CharacterId(2), "Your torch expired.".to_string()),
            ]
        );
    }

    #[test]
    fn special_potion_fun_message_matches_legacy_text() {
        let mut world = World::default();
        let login = login_block("Ralph");
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));

        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 8).as_deref(),
            Some("You see Ralph hit himself on the head with a mug.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 9).as_deref(),
            Some("Ralph suddenly starts singing in a slurred tongue... Dogs start howling...")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 10).as_deref(),
            Some("Ralph's hair suddenly shoots up as if hit by electricity.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 11).as_deref(),
            Some("Ralph seems to be enjoying a fine ale.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 12).as_deref(),
            Some("Ralph drinks a delicious apple juice.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 13).as_deref(),
            Some("Ralph feels refreshed.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 14).as_deref(),
            Some("Ralph cracks his strong knuckles.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 15).as_deref(),
            Some("Ralph starts frothing at the mouth.")
        );
        assert_eq!(special_potion_fun_message(&world, CharacterId(1), 7), None);
    }

    #[test]
    fn no_potion_area_feedback_applies_to_special_and_beyond_potions() {
        let mut world = World::default();
        world.add_item(test_item_with_driver(ItemId(1), IDR_SPECIAL_POTION));
        world.add_item(test_item_with_driver(ItemId(2), IDR_BEYONDPOTION));
        world.add_item(test_item_with_driver(ItemId(3), IDR_TORCH));

        assert!(is_no_potion_area_blocked_item(&world, ItemId(1)));
        assert!(is_no_potion_area_blocked_item(&world, ItemId(2)));
        assert!(!is_no_potion_area_blocked_item(&world, ItemId(3)));
        assert!(!is_no_potion_area_blocked_item(&world, ItemId(99)));
    }

    #[test]
    fn area_message_sessions_match_legacy_square_distance() {
        let mut world = World::default();
        let mut origin = login_character(CharacterId(1), &login_block("Ralph"), 1, 10, 10);
        origin.x = 10;
        origin.y = 10;
        let mut edge = login_character(CharacterId(2), &login_block("Lisa"), 1, 26, 26);
        edge.x = 26;
        edge.y = 26;
        let mut outside = login_character(CharacterId(3), &login_block("Milhouse"), 1, 27, 10);
        outside.x = 27;
        outside.y = 10;
        world.add_character(origin);
        world.add_character(edge);
        world.add_character(outside);

        let mut runtime = ServerRuntime::default();
        let mut origin_player = PlayerRuntime::connected(10, 0);
        origin_player.character_id = Some(CharacterId(1));
        let mut edge_player = PlayerRuntime::connected(20, 0);
        edge_player.character_id = Some(CharacterId(2));
        let mut outside_player = PlayerRuntime::connected(30, 0);
        outside_player.character_id = Some(CharacterId(3));
        runtime.players.insert(10, origin_player);
        runtime.players.insert(20, edge_player);
        runtime.players.insert(30, outside_player);

        let mut sessions = runtime.sessions_for_area_message(&world, CharacterId(1), 16);
        sessions.sort_unstable_by_key(|(session_id, _)| *session_id);

        assert_eq!(sessions, vec![(10, CharacterId(1)), (20, CharacterId(2))]);
    }

    #[test]
    fn login_payload_sends_legacy_session_start_packets() {
        let login = login_block("Tester");
        let mut character =
            login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
        character.x = LOGIN_SPAWN_X as u16;
        character.y = LOGIN_SPAWN_Y as u16;
        let world = World::default();
        let payload = login_payload(&world, &character, 2, 0x0102_0304);

        assert_eq!(payload[0], SV_LOGINDONE);
        assert_eq!(payload[1], SV_TICKER);
        assert_eq!(&payload[2..6], &[3, 3, 2, 1]);
        assert_eq!(payload[6], SV_MIRROR);
        assert_eq!(&payload[7..11], &[2, 0, 0, 0]);
        assert_eq!(payload[11], SV_PROTOCOL);
        assert_eq!(payload[13], SV_ORIGIN);
        assert_eq!(&payload[14..18], &[128, 0, 128, 0]);
        assert_eq!(payload[18], SV_SETVAL0);
        assert_eq!(payload[22], SV_SETVAL1);
        let first_resource_offset = 18 + ugaris_core::entity::CHARACTER_VALUE_COUNT * 8;
        assert_eq!(payload[first_resource_offset], SV_SETHP);
        assert_eq!(
            payload[payload.len() - LOGIN_ACCEPTED_MESSAGE.len() - 3],
            SV_TEXT
        );
    }

    #[test]
    fn login_payload_sends_inventory_item_sprites() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        let item_id = ugaris_core::ids::ItemId(99);
        character.inventory[30] = Some(item_id);

        let mut world = World::default();
        world.add_item(ugaris_core::entity::Item {
            id: item_id,
            name: "Torch".into(),
            description: String::new(),
            flags: ItemFlags::TAKE | ItemFlags::USE,
            sprite: 1234,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: Some(character.id),
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 1,
        });

        let payload = login_payload(&world, &character, 1, 0);
        let expected = [
            SV_SETITEM,
            30,
            0xd2,
            0x04,
            0,
            0,
            (ItemFlags::TAKE | ItemFlags::USE).bits() as u8,
            0,
            0,
            0,
        ];

        assert!(payload
            .windows(expected.len())
            .any(|window| window == expected));
    }

    #[test]
    fn inventory_snapshot_payload_sends_cursor_and_inventory() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        let cursor_id = ugaris_core::ids::ItemId(98);
        let slot_id = ugaris_core::ids::ItemId(99);
        character.cursor_item = Some(cursor_id);
        character.inventory[30] = Some(slot_id);

        let mut world = World::default();
        world.add_item(test_item(cursor_id, 5000, ItemFlags::TAKE));
        world.add_item(test_item(slot_id, 1234, ItemFlags::TAKE | ItemFlags::USE));

        let payload = inventory_snapshot_payload(&world, &character);

        assert_eq!(&payload[..9], &[SV_SETCITEM, 0x88, 0x13, 0, 0, 8, 0, 0, 0]);
        assert!(payload.windows(10).any(|window| {
            window
                == [
                    SV_SETITEM,
                    30,
                    0xd2,
                    0x04,
                    0,
                    0,
                    (ItemFlags::TAKE | ItemFlags::USE).bits() as u8,
                    0,
                    0,
                    0,
                ]
        }));
    }

    #[test]
    fn legacy_item_look_text_includes_c_shaped_modifiers_requirements_and_flags() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.values[1][CharacterValue::Strength as usize] = 12;

        let mut item = test_item(ItemId(99), 59210, ItemFlags::QUEST | ItemFlags::NOENHANCE);
        item.name = "Fine Sword".to_string();
        item.description = "A carefully balanced blade.".to_string();
        item.modifier_index = [
            CharacterValue::Armor as i16,
            CharacterValue::Sword as i16,
            -(CharacterValue::Strength as i16),
            0,
            0,
        ];
        item.modifier_value = [15, 3, 20, 0, 0];
        item.min_level = 4;
        item.needs_class = 1 | 4;

        let text = legacy_item_look_text(&item, &character);

        assert_eq!(
            text,
            "Fine Sword:\nA carefully balanced blade.\nModifiers:\nArmor Value +0.75\nSword +3\nRequirements:\nStrength 20 (you have 12)\nMinimum Level: 4\nOnly usable by a Warrior.\nOnly usable by a Seyan'Du.\nThis is a quest item. You cannot drop it or give it away.\nThis item resists magic, so you cannot enhance it using orbs, metals or shrines.\nThe item has been gilded."
        );
    }

    #[test]
    fn legacy_item_look_text_includes_bonding_duration_and_sprite_notes() {
        let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        let mut item = test_item(
            ItemId(99),
            53026,
            ItemFlags::BONDTAKE | ItemFlags::BONDWEAR | ItemFlags::BEYONDMAXMOD,
        );
        item.name = "Frozen Charm".to_string();
        item.owner_id = 12345;
        item.driver = IDR_DECAYITEM;
        item.modifier_index[0] = CharacterValue::Speed as i16;
        item.modifier_value[0] = 5;
        item.driver_data = vec![0; 7];
        item.driver_data[2] = 253_u8;
        item.driver_data[3..5].copy_from_slice(&30_u16.to_le_bytes());
        item.driver_data[5..7].copy_from_slice(&1800_u16.to_le_bytes());

        let text = legacy_item_look_text(&item, &character);

        assert_eq!(
            text,
            "Frozen Charm:\nModifiers:\nSpeed +5 (active: -3)\nThis item is bonded to somebody else. Only he can take it.\nThis item is bonded to somebody else. Only he can wear it.\nThis item goes beyond maximum modifier limits.\nDuration: 0:01:00 of 1:00:00 active time used up.\nThis is part of an ice demon suit."
        );
    }

    #[test]
    fn container_look_uses_legacy_item_text() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        world.add_character(character);

        let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        container.content_id = 1;
        world.add_item(container);
        let mut stored = test_item(ItemId(20), 1234, ItemFlags::USED | ItemFlags::TAKE);
        stored.name = "Stored Gem".to_string();
        stored.description = "It sparkles.".to_string();
        stored.contained_in = Some(ItemId(10));
        world.add_item(stored);

        let result = apply_item_container_command(
            &mut world,
            character_id,
            &ClientAction::LookContainer { slot: 0 },
        );

        assert_eq!(
            result,
            AccountDepotCommandResult::Look("Stored Gem:\nIt sparkles.".to_string())
        );
    }

    fn test_item(
        id: ugaris_core::ids::ItemId,
        sprite: i32,
        flags: ItemFlags,
    ) -> ugaris_core::entity::Item {
        ugaris_core::entity::Item {
            id,
            name: "Item".into(),
            description: String::new(),
            flags,
            sprite,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 1,
        }
    }

    fn test_item_with_driver(
        id: ugaris_core::ids::ItemId,
        driver: u16,
    ) -> ugaris_core::entity::Item {
        let mut item = test_item(id, 0, ItemFlags::USED | ItemFlags::USE);
        item.driver = driver;
        item
    }

    #[test]
    fn grant_template_item_smart_places_xmaspop_in_inventory_first() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::STAFF);
        let mut world = World::default();
        world.add_character(character);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
            .unwrap();

        assert_eq!(
            grant_template_item_smart(&mut world, &mut loader, character_id, "xmaspop"),
            Some("Christmas Pop".to_string())
        );
        let character = world.characters.get(&character_id).unwrap();
        let item_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
        let item = world.items.get(&item_id).unwrap();
        assert_eq!(item.name, "Christmas Pop");
        assert_eq!(item.carried_by, Some(character_id));
    }

    #[test]
    fn grant_template_item_smart_uses_cursor_when_inventory_full() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        for slot in character
            .inventory
            .iter_mut()
            .skip(INVENTORY_START_INVENTORY)
        {
            *slot = Some(ItemId(99));
        }
        let mut world = World::default();
        world.add_character(character);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
            .unwrap();

        assert_eq!(
            grant_template_item_smart(&mut world, &mut loader, character_id, "xmaspop"),
            Some("Christmas Pop".to_string())
        );
        let character = world.characters.get(&character_id).unwrap();
        let item_id = character.cursor_item.unwrap();
        assert_eq!(
            world.items.get(&item_id).unwrap().carried_by,
            Some(character_id)
        );
    }

    #[test]
    fn apply_xmasmaker_silently_grants_xmaspop_like_c() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::STAFF);
        let mut world = World::default();
        world.add_character(character);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
            .unwrap();

        assert!(apply_xmasmaker(&mut world, &mut loader, character_id));

        let character = world.characters.get(&character_id).unwrap();
        let item_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
        let item = world.items.get(&item_id).unwrap();
        assert_eq!(item.name, "Christmas Pop");
        assert_eq!(item.carried_by, Some(character_id));
    }

    #[test]
    fn apply_xmastree_consumes_holiday_treat_and_marks_area() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut treat = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        treat.driver = IDR_FOOD;
        treat.driver_data = vec![3];
        treat.carried_by = Some(character_id);
        world.add_item(treat);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"ad_bracelet1: name="Holiday Bracelet" flag=IF_TAKE ;"#)
            .unwrap();
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            apply_xmastree(
                &mut world,
                &mut loader,
                &mut player,
                character_id,
                1,
                true,
                2025,
                0
            ),
            XmasTreeApplyResult::GiftGranted("Holiday Bracelet".to_string())
        );

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(!world.items.contains_key(&ItemId(20)));
        let gift_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
        assert_eq!(world.items.get(&gift_id).unwrap().name, "Holiday Bracelet");
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, true),
            XmasTreeResult::AlreadyGranted
        );
    }

    #[test]
    fn apply_xmastree_rolls_back_area_mark_when_gift_cannot_be_created() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut treat = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        treat.driver = IDR_FOOD;
        treat.driver_data = vec![3];
        treat.carried_by = Some(character_id);
        world.add_item(treat);
        let mut loader = ZoneLoader::new();
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            apply_xmastree(
                &mut world,
                &mut loader,
                &mut player,
                character_id,
                1,
                true,
                2025,
                0
            ),
            XmasTreeApplyResult::NoSpace
        );
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, false),
            XmasTreeResult::NeedsHolidayTreat
        );
        assert!(world.items.contains_key(&ItemId(20)));
    }

    #[test]
    fn xmas_event_window_matches_legacy_december_to_january_span() {
        assert_eq!(xmas_event_from_ymd(2025, 12, 20), (true, 2025));
        assert_eq!(xmas_event_from_ymd(2026, 1, 7), (true, 2025));
        assert_eq!(xmas_event_from_ymd(2026, 1, 8), (false, 2026));
    }

    #[test]
    fn nomad_stack_split_creates_cursor_stack_with_legacy_counts() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut stack = test_item(ItemId(20), 13208, ItemFlags::USED | ItemFlags::USE);
        stack.name = "salt".to_string();
        stack.template_id = IID_AREA19_SALT;
        stack.driver = ugaris_core::item_driver::IDR_NOMADSTACK;
        stack.value = 1_000;
        stack.carried_by = Some(character_id);
        set_stack_count(&mut stack, 123, StackKind::Salt);
        world.add_item(stack);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"salt: name="salt" ID=0100008B flag=IF_TAKE driver=96 ;"#)
            .unwrap();

        assert_eq!(
            apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
            NomadStackApplyResult::Split {
                left: 73,
                right: 50,
                unit: "ounce",
            }
        );
        let character = world.characters.get(&character_id).unwrap();
        let cursor_id = character
            .cursor_item
            .expect("split stack should be on cursor");
        let carried = world.items.get(&ItemId(20)).unwrap();
        let cursor = world.items.get(&cursor_id).unwrap();
        assert_eq!(stack_count(carried), 73);
        assert_eq!(stack_count(cursor), 50);
        assert_eq!(carried.sprite, 13209);
        assert_eq!(cursor.description, "50 ounces of salt.");
    }

    #[test]
    fn nomad_stack_merge_consumes_matching_cursor_stack() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        character.cursor_item = Some(ItemId(21));
        let mut world = World::default();
        world.add_character(character);
        let mut carried = test_item(ItemId(20), 59655, ItemFlags::USED | ItemFlags::USE);
        carried.name = "skin".to_string();
        carried.template_id = IID_AREA19_WOLFSSKIN;
        carried.driver = ugaris_core::item_driver::IDR_NOMADSTACK;
        carried.value = 30;
        carried.carried_by = Some(character_id);
        set_stack_count(&mut carried, 3, StackKind::Skin1);
        world.add_item(carried);
        let mut cursor = test_item(ItemId(21), 59655, ItemFlags::USED | ItemFlags::USE);
        cursor.name = "skin".to_string();
        cursor.template_id = IID_AREA19_WOLFSSKIN;
        cursor.value = 20;
        cursor.carried_by = Some(character_id);
        set_stack_count(&mut cursor, 2, StackKind::Skin1);
        world.add_item(cursor);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
            NomadStackApplyResult::Merged {
                count: 5,
                unit: "skin",
            }
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            None
        );
        assert!(!world.items.contains_key(&ItemId(21)));
        let stack = world.items.get(&ItemId(20)).unwrap();
        assert_eq!(stack_count(stack), 5);
        assert_eq!(stack.value, 50);
        assert_eq!(stack.sprite, 59659);
    }

    #[test]
    fn demon_chip_stack_split_uses_legacy_sprite_offsets() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut stack = test_item(ItemId(20), 53019, ItemFlags::USED | ItemFlags::USE);
        stack.name = "Silver Chip".to_string();
        stack.template_id = IID_SILVERCHIP;
        stack.driver = ugaris_core::item_driver::IDR_DEMONCHIP;
        stack.value = 123_000;
        stack.carried_by = Some(character_id);
        set_stack_count(&mut stack, 123, StackKind::SilverChip);
        world.add_item(stack);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"silverchip: name="Silver Chip" ID=010000AD flag=IF_TAKE driver=136 ;"#,
            )
            .unwrap();

        assert_eq!(
            apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
            NomadStackApplyResult::Split {
                left: 73,
                right: 50,
                unit: "chip",
            }
        );
        let cursor_id = world
            .characters
            .get(&character_id)
            .unwrap()
            .cursor_item
            .unwrap();
        let carried = world.items.get(&ItemId(20)).unwrap();
        let cursor = world.items.get(&cursor_id).unwrap();
        assert_eq!(stack_count(carried), 73);
        assert_eq!(stack_count(cursor), 50);
        assert_eq!(carried.sprite, 53024);
        assert_eq!(cursor.description, "50 Silver Chips.");
    }

    #[test]
    fn account_depot_swap_moves_cursor_item_into_snapshot_slot() {
        let character_id = CharacterId(7);
        let cursor_id = ItemId(20);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        character.cursor_item = Some(cursor_id);

        let mut world = World::default();
        world.add_character(character);
        let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        depot_item.driver = IDR_ACCOUNT_DEPOT;
        world.add_item(depot_item);
        let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::TAKE);
        cursor.carried_by = Some(character_id);
        world.add_item(cursor);
        let mut depot = AccountDepotState::default();

        assert_eq!(
            apply_account_depot_command(
                &mut world,
                &mut depot,
                character_id,
                &ClientAction::Container {
                    slot: 3,
                    fast: false,
                },
            ),
            AccountDepotCommandResult::Changed
        );

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(!world.items.contains_key(&cursor_id));
        assert_eq!(depot.slots[3].as_ref().unwrap().sprite, 1234);
        assert_eq!(
            depot.slots[3].as_ref().unwrap().contained_in,
            Some(ItemId(10))
        );
    }

    #[test]
    fn account_depot_swap_withdraws_snapshot_to_cursor_with_new_live_id() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));

        let mut world = World::default();
        world.add_character(character);
        let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        depot_item.driver = IDR_ACCOUNT_DEPOT;
        world.add_item(depot_item);
        let mut stored = test_item(ItemId(99), 2222, ItemFlags::USED | ItemFlags::TAKE);
        stored.name = "Stored".to_string();
        let mut depot = AccountDepotState::default();
        depot.slots[4] = Some(stored);

        assert_eq!(
            apply_account_depot_command(
                &mut world,
                &mut depot,
                character_id,
                &ClientAction::Container {
                    slot: 4,
                    fast: false,
                },
            ),
            AccountDepotCommandResult::Changed
        );

        let cursor_id = world
            .characters
            .get(&character_id)
            .unwrap()
            .cursor_item
            .unwrap();
        assert_ne!(cursor_id, ItemId(99));
        let cursor = world.items.get(&cursor_id).unwrap();
        assert_eq!(cursor.name, "Stored");
        assert_eq!(cursor.carried_by, Some(character_id));
        assert!(depot.slots[4].is_none());
    }

    #[test]
    fn account_depot_blocks_quest_and_nodepot_items() {
        let character_id = CharacterId(7);
        let cursor_id = ItemId(20);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        character.cursor_item = Some(cursor_id);

        let mut world = World::default();
        world.add_character(character);
        let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        depot_item.driver = IDR_ACCOUNT_DEPOT;
        world.add_item(depot_item);
        let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::QUEST);
        cursor.carried_by = Some(character_id);
        world.add_item(cursor);
        let mut depot = AccountDepotState::default();

        assert_eq!(
            apply_account_depot_command(
                &mut world,
                &mut depot,
                character_id,
                &ClientAction::Container {
                    slot: 0,
                    fast: false,
                },
            ),
            AccountDepotCommandResult::Blocked(
                "You cannot store this item in the depot.".to_string()
            )
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(cursor_id)
        );
        assert!(depot.slots[0].is_none());
    }

    #[test]
    fn account_depot_payload_matches_legacy_container_header_and_slots() {
        let mut depot = AccountDepotState::default();
        depot.slots[2] = Some(test_item(ItemId(99), 0x11223344, ItemFlags::USED));

        let payload = account_depot_payload(&depot);

        assert_eq!(&payload[..2], &[SV_CONTYPE, 1]);
        assert_eq!(payload[2], SV_CONNAME);
        assert!(payload.windows(2).any(|window| window == [SV_CONCNT, 110]));
        assert!(payload
            .windows(6)
            .any(|window| { window == [SV_CONTAINER, 2, 0x44, 0x33, 0x22, 0x11] }));
    }

    #[test]
    fn generic_container_payload_uses_open_item_description_and_clears_empty_slots() {
        let mut world = World::default();
        let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        container.description = "Opened Chest".to_string();
        container.content_id = 22;
        world.add_item(container);
        let mut stored = test_item(ItemId(20), 0x11223344, ItemFlags::USED | ItemFlags::TAKE);
        stored.contained_in = Some(ItemId(10));
        world.add_item(stored);

        let payload = generic_container_payload(&world, ItemId(10));

        assert_eq!(&payload[..2], &[SV_CONTYPE, 1]);
        assert!(payload.windows(14).any(|window| {
            window
                == [
                    SV_CONNAME, 12, b'O', b'p', b'e', b'n', b'e', b'd', b' ', b'C', b'h', b'e',
                    b's', b't',
                ]
        }));
        assert!(payload.windows(2).any(|window| window == [SV_CONCNT, 108]));
        assert!(payload
            .windows(6)
            .any(|window| window == [SV_CONTAINER, 0, 0x44, 0x33, 0x22, 0x11]));
        assert!(payload
            .windows(6)
            .any(|window| window == [SV_CONTAINER, 1, 0, 0, 0, 0]));
    }

    #[test]
    fn generic_container_swap_exchanges_cursor_and_container_item() {
        let character_id = CharacterId(7);
        let cursor_id = ItemId(30);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        character.cursor_item = Some(cursor_id);

        let mut world = World::default();
        world.add_character(character);
        let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        container.content_id = 22;
        world.add_item(container);
        let mut stored = test_item(ItemId(20), 2222, ItemFlags::USED | ItemFlags::TAKE);
        stored.contained_in = Some(ItemId(10));
        world.add_item(stored);
        let mut cursor = test_item(cursor_id, 3333, ItemFlags::USED | ItemFlags::TAKE);
        cursor.carried_by = Some(character_id);
        world.add_item(cursor);

        assert_eq!(
            apply_item_container_command(
                &mut world,
                character_id,
                &ClientAction::Container {
                    slot: 0,
                    fast: false,
                },
            ),
            AccountDepotCommandResult::Changed
        );

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, Some(ItemId(20)));
        assert_eq!(
            world.items.get(&ItemId(20)).unwrap().carried_by,
            Some(character_id)
        );
        assert_eq!(
            world.items.get(&cursor_id).unwrap().contained_in,
            Some(ItemId(10))
        );
    }

    #[test]
    fn generic_container_fast_swap_stores_withdrawn_item_in_inventory() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        let mut world = World::default();
        world.add_character(character);
        let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        container.content_id = 22;
        world.add_item(container);
        let mut stored = test_item(ItemId(20), 2222, ItemFlags::USED | ItemFlags::TAKE);
        stored.contained_in = Some(ItemId(10));
        world.add_item(stored);

        assert_eq!(
            apply_item_container_command(
                &mut world,
                character_id,
                &ClientAction::Container {
                    slot: 0,
                    fast: true,
                },
            ),
            AccountDepotCommandResult::Changed
        );

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert_eq!(
            character.inventory[INVENTORY_START_INVENTORY],
            Some(ItemId(20))
        );
        assert_eq!(
            world.items.get(&ItemId(20)).unwrap().carried_by,
            Some(character_id)
        );
    }

    #[test]
    fn generic_container_blocks_quest_cursor_storage() {
        let character_id = CharacterId(7);
        let cursor_id = ItemId(30);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.current_container = Some(ItemId(10));
        character.cursor_item = Some(cursor_id);
        let mut world = World::default();
        world.add_character(character);
        let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
        container.content_id = 22;
        world.add_item(container);
        let mut cursor = test_item(cursor_id, 3333, ItemFlags::USED | ItemFlags::QUEST);
        cursor.carried_by = Some(character_id);
        world.add_item(cursor);

        assert_eq!(
            apply_item_container_command(
                &mut world,
                character_id,
                &ClientAction::Container {
                    slot: 0,
                    fast: false,
                },
            ),
            AccountDepotCommandResult::Blocked(
                "You cannot store quest items in a container.".to_string()
            )
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(cursor_id)
        );
        assert_eq!(world.items.get(&cursor_id).unwrap().contained_in, None);
    }

    #[test]
    fn apply_orb_spawn_grants_orb_and_records_cooldown() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::PAID);
        let mut world = World::default();
        world.add_character(character);
        let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
        spawner.x = 5;
        spawner.y = 6;
        world.add_item(spawner);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"empty_orb: name="Empty Orb" ;"#)
            .unwrap();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);

        assert_eq!(
            apply_orb_spawn(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(77),
                character_id,
                1,
                10_000,
                false,
                false,
                0,
            ),
            OrbSpawnApplyResult::Granted {
                item_name: "Orb of Endurance".to_string(),
                special: false,
            }
        );
        let character = world.characters.get(&character_id).unwrap();
        let orb_id = character.cursor_item.expect("orb should be on cursor");
        let orb = world.items.get(&orb_id).unwrap();
        assert_eq!(orb.name, "Orb of Endurance");
        assert_eq!(orb.driver_data[0], CharacterValue::Endurance as u8);
        assert_eq!(orb.driver_data[1], 1);
        assert_eq!(
            player.orb_spawn_last_used_seconds(0x0001_0605),
            Some(10_000)
        );
    }

    #[test]
    fn apply_orb_spawn_enforces_legacy_respawn_cooldown() {
        let character_id = CharacterId(7);
        let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        let mut world = World::default();
        world.add_character(character);
        let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
        spawner.x = 5;
        spawner.y = 6;
        world.add_item(spawner);
        let mut loader = ZoneLoader::new();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        player.mark_orb_spawn_used(0x0001_0605, 10_000);

        assert_eq!(
            apply_orb_spawn(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(77),
                character_id,
                1,
                10_000 + 60 * 60 * 24,
                false,
                false,
                1,
            ),
            OrbSpawnApplyResult::Cooldown {
                days_left: "29.00".to_string(),
            }
        );
    }

    #[test]
    fn apply_anti_orb_spawn_marks_extracting_anti_orb() {
        let character_id = CharacterId(7);
        let mut world = World::default();
        world.add_character(login_character(
            character_id,
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
        spawner.x = 5;
        spawner.y = 6;
        world.add_item(spawner);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"empty_anti_orb: name="Empty Anti-Orb" ;"#)
            .unwrap();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);

        assert_eq!(
            apply_orb_spawn(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(77),
                character_id,
                1,
                10_000,
                true,
                true,
                2,
            ),
            OrbSpawnApplyResult::Granted {
                item_name: "Extracting Anti-Orb of Mana".to_string(),
                special: true,
            }
        );
        let orb_id = world
            .characters
            .get(&character_id)
            .unwrap()
            .cursor_item
            .unwrap();
        let orb = world.items.get(&orb_id).unwrap();
        assert_eq!(orb.driver_data[0], CharacterValue::Mana as u8);
        assert_eq!(orb.driver_data[1], 1);
        assert_eq!(orb.driver_data[2], 1);
        assert_eq!(
            orb.description,
            "A dark orb that extracts Mana from items and crystallizes it."
        );
    }

    #[test]
    fn keyring_command_requires_keyring_on_cursor() {
        let login = login_block("Tester");
        let character_id = CharacterId(7);
        let character = login_character(character_id, &login, 1, 10, 10);
        let mut world = World::default();
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        let mut loader = ZoneLoader::new();

        let result = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring",
        )
        .expect("keyring command should be recognized");

        assert_eq!(
            result.messages,
            vec!["You need to hold a keyring on your cursor to use this command."]
        );
        assert!(!result.inventory_changed);
    }

    #[test]
    fn keyring_command_addall_consumes_inventory_keys() {
        let login = login_block("Tester");
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login, 1, 10, 10);
        let keyring_id = ItemId(90);
        let key_id = ItemId(91);
        let potion_id = ItemId(92);
        character.cursor_item = Some(keyring_id);
        character.inventory[30] = Some(key_id);
        character.inventory[31] = Some(potion_id);
        let mut world = World::default();
        world.add_character(character);
        let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
        keyring.template_id = IID_KEY_RING;
        keyring.driver = IDR_KEY_RING;
        let mut key = test_item(key_id, 501, ItemFlags::TAKE);
        key.template_id = IID_AREA1_SKELKEY1;
        key.name = "Copper Key".to_string();
        let mut potion = test_item(potion_id, 502, ItemFlags::TAKE);
        potion.template_id = 0x5566_7788;
        potion.name = "Potion".to_string();
        world.add_item(keyring);
        world.add_item(key);
        world.add_item(potion);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        let mut loader = ZoneLoader::new();

        let result = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring addall",
        )
        .expect("keyring command should be recognized");

        assert_eq!(result.messages, vec!["Added 1 keys to your keyring."]);
        assert!(result.inventory_changed);
        assert_eq!(
            player.keyring_key_name(IID_AREA1_SKELKEY1),
            Some("Copper Key")
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().inventory[30],
            None
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().inventory[31],
            Some(potion_id)
        );
        assert!(!world.items.contains_key(&key_id));
        assert!(world.items.contains_key(&potion_id));
    }

    #[test]
    fn keyring_command_addallkeys_requires_staff_and_uses_registered_templates() {
        let login = login_block("Tester");
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login, 1, 10, 10);
        let keyring_id = ItemId(90);
        character.cursor_item = Some(keyring_id);
        let mut world = World::default();
        world.add_character(character);
        let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
        keyring.template_id = IID_KEY_RING;
        keyring.driver = IDR_KEY_RING;
        world.add_item(keyring);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                CopperKey:
                  name="Copper Key"
                  ID=1000002
                  flag=IF_TAKE
                ;
                UnregisteredKey:
                  name="Unregistered Key"
                  ID=55667788
                  flag=IF_TAKE
                ;
                "#,
            )
            .unwrap();

        let denied = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring addallkeys",
        )
        .expect("keyring command should be recognized");
        assert_eq!(
            denied.messages,
            vec!["This command requires staff privileges."]
        );
        assert_eq!(player.keyring.len(), 0);

        world
            .characters
            .get_mut(&character_id)
            .unwrap()
            .flags
            .insert(CharacterFlags::STAFF);
        let added = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring addallkeys",
        )
        .expect("keyring command should be recognized");

        assert_eq!(
            added.messages,
            vec![
                "Adding all registered keys to keyring...",
                "Added 1 keys to your keyring (total: 1/100).",
            ]
        );
        assert_eq!(
            player.keyring_key_name(IID_AREA1_SKELKEY1),
            Some("Copper Key")
        );
        assert_eq!(player.keyring.len(), 1);
    }

    #[test]
    fn keyring_command_remove_and_auto_match_legacy_feedback() {
        let login = login_block("Tester");
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login, 1, 10, 10);
        let keyring_id = ItemId(90);
        character.cursor_item = Some(keyring_id);
        let mut world = World::default();
        world.add_character(character);
        let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
        keyring.template_id = IID_KEY_RING;
        keyring.driver = IDR_KEY_RING;
        world.add_item(keyring);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        let mut loader = ZoneLoader::new();
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );

        let removed = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring remove 1",
        )
        .expect("keyring command should be recognized");
        let auto = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring auto",
        )
        .expect("keyring command should be recognized");

        assert_eq!(
            removed.messages,
            vec!["Removed Copper Key from your keyring."]
        );
        assert!(removed.inventory_changed);
        assert_eq!(player.keyring_key_name(0x1122_3344), None);
        let character = world.characters.get(&character_id).unwrap();
        let restored_key_id = character.inventory[30].expect("removed key should be restored");
        let restored_key = world.items.get(&restored_key_id).unwrap();
        assert_eq!(restored_key.template_id, 0x1122_3344);
        assert_eq!(restored_key.name, "Copper Key");
        assert_eq!(restored_key.carried_by, Some(character_id));
        assert_eq!(
            auto.messages,
            vec!["Auto-add keys enabled. Keys will be automatically added to your keyring when picked up."]
        );
        assert!(player.keyring_auto_add());
    }

    #[test]
    fn keyring_command_remove_keeps_entry_when_inventory_is_full() {
        let login = login_block("Tester");
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login, 1, 10, 10);
        let keyring_id = ItemId(90);
        character.cursor_item = Some(keyring_id);
        for slot in 30..character.inventory.len() {
            character.inventory[slot] = Some(ItemId(1_000 + slot as u32));
        }
        let mut world = World::default();
        world.add_character(character);
        let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
        keyring.template_id = IID_KEY_RING;
        keyring.driver = IDR_KEY_RING;
        world.add_item(keyring);
        for slot in 30..ugaris_core::entity::INVENTORY_SIZE {
            world.add_item(test_item(
                ItemId(1_000 + slot as u32),
                10,
                ItemFlags::USED | ItemFlags::TAKE,
            ));
        }
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        let mut loader = ZoneLoader::new();
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );

        let result = apply_keyring_command(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            "#keyring remove 1",
        )
        .expect("keyring command should be recognized");

        assert_eq!(result.messages, vec!["Your inventory is full."]);
        assert!(!result.inventory_changed);
        assert_eq!(player.keyring_key_name(0x1122_3344), Some("Copper Key"));
    }

    #[test]
    fn initial_map_payloads_send_visible_diamond_and_center_character() {
        let login = login_block("Tester");
        let mut character =
            login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
        character.x = LOGIN_SPAWN_X as u16;
        character.y = LOGIN_SPAWN_Y as u16;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

        let payloads = initial_map_payloads(&world, &character, 1);
        assert_eq!(payloads.len(), 1);
        let payload = &payloads[0];

        assert_eq!(
            payload[0],
            SV_MAP11
                | SV_MAPPOS
                | MAP_TILE_GSPRITE
                | MAP_TILE_FSPRITE
                | MAP_TILE_ISPRITE
                | MAP_TILE_FLAGS
        );
        assert!(payload.windows(16).any(|window| {
            window
                == [
                    SV_MAP10
                        | SV_MAPPOS
                        | MAP_CHARACTER_SPRITE
                        | MAP_CHARACTER_ACTION
                        | MAP_CHARACTER_STATUS,
                    4,
                    0,
                    1,
                    0,
                    0,
                    0,
                    7,
                    0,
                    0,
                    0,
                    0,
                    0,
                    100,
                    100,
                    0,
                ]
        }));
        assert!(payload_contains_character_name(payload, 7, "Tester"));
    }

    #[test]
    fn initial_map_payloads_send_visible_map_effect_slots() {
        let login = login_block("Tester");
        let mut character =
            login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
        character.x = LOGIN_SPAWN_X as u16;
        character.y = LOGIN_SPAWN_Y as u16;
        let mut world = World::default();
        world
            .map
            .tile_mut(LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
            .unwrap()
            .effects = [42, 0, 77, 0];
        assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

        let payloads = initial_map_payloads(&world, &character, 1);
        let payload = &payloads[0];

        assert!(payload.windows(19).any(|window| {
            window
                == [
                    SV_MAP01
                        | SV_MAPPOS
                        | MAP_EFFECT_0
                        | MAP_EFFECT_1
                        | MAP_EFFECT_2
                        | MAP_EFFECT_3,
                    4,
                    0,
                    42,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    77,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ]
        }));
    }

    #[test]
    fn map_diff_payloads_clear_removed_map_effect_slots() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        world.map.tile_mut(10, 10).unwrap().effects = [42, 0, 77, 0];
        assert!(world.spawn_character(character.clone(), 10, 10));
        let mut cache = visible_map_cache(&world, &character, 1);
        world.map.tile_mut(10, 10).unwrap().effects = [0; 4];

        let payloads = map_diff_payloads(&world, &character, 1, &mut cache);
        let payload = payloads.concat();

        assert!(payload.windows(19).any(|window| {
            window
                == [
                    SV_MAP01
                        | SV_MAPPOS
                        | MAP_EFFECT_0
                        | MAP_EFFECT_1
                        | MAP_EFFECT_2
                        | MAP_EFFECT_3,
                    4,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ]
        }));
    }

    #[test]
    fn initial_map_payloads_chunk_modern_view_distance_under_frame_limit() {
        let login = login_block("Tester");
        let mut character =
            login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
        character.x = LOGIN_SPAWN_X as u16;
        character.y = LOGIN_SPAWN_Y as u16;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

        let payloads = initial_map_payloads(&world, &character, 40);

        assert!(payloads.len() > 1);
        assert!(payloads
            .iter()
            .all(|payload| payload.len() <= MAP_BOOTSTRAP_CHUNK_TARGET));
    }

    #[test]
    fn map_refresh_payloads_start_with_origin_then_map_chunks() {
        let login = login_block("Tester");
        let mut character =
            login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
        character.x = LOGIN_SPAWN_X as u16;
        character.y = LOGIN_SPAWN_Y as u16;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

        let payloads = map_refresh_payloads(&world, &character, 1);

        assert_eq!(&payloads[0][..], &[SV_ORIGIN, 128, 0, 128, 0]);
        assert_eq!(
            payloads[1][0],
            SV_MAP11
                | SV_MAPPOS
                | MAP_TILE_GSPRITE
                | MAP_TILE_FSPRITE
                | MAP_TILE_ISPRITE
                | MAP_TILE_FLAGS
        );
    }

    #[test]
    fn login_bootstrap_payloads_include_visible_client_effect_slots() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 10, 10));
        let mut effect = Effect::new(EF_FIREBALL, 123, 55, 65);
        effect.from_x = 10;
        effect.from_y = 10;
        effect.to_x = 12;
        effect.to_y = 10;
        effect.x = 11 * 1024 + 512;
        effect.y = 10 * 1024 + 512;
        world.effects.insert(123, effect);
        let mut effect_cache = ClientEffectCache::default();

        let payloads = login_bootstrap_payloads(&world, &character, 1, 10, 2, &mut effect_cache);

        assert!(payloads.iter().any(|payload| {
            payload.first().copied() == Some(ugaris_protocol::packet::SV_CEFFECT)
                && payload.get(1).copied() == Some(0)
                && payload[2..].starts_with(&ugaris_protocol::packet::ceffect_fireball(
                    123, 55, 10, 10, 12, 10,
                ))
        }));
        assert!(payloads
            .iter()
            .any(|payload| &payload[..] == &ugaris_protocol::packet::used_effects(1)[..]));
        assert!(client_effect_payloads(&world, &character, 2, &mut effect_cache).is_empty());
    }

    #[test]
    fn map_diff_payloads_send_only_changed_same_origin_cells() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 10, 10));
        let mut cache = visible_map_cache(&world, &character, 1);

        world.map.tile_mut(11, 10).unwrap().ground_sprite = 777;
        let payloads = map_diff_payloads(&world, &character, 1, &mut cache);

        assert_eq!(payloads.len(), 1);
        let payload = &payloads[0];
        assert_ne!(payload.first().copied(), Some(SV_ORIGIN));
        assert!(payload.windows(17).any(|window| {
            window
                == [
                    SV_MAP11
                        | SV_MAPPOS
                        | MAP_TILE_GSPRITE
                        | MAP_TILE_FSPRITE
                        | MAP_TILE_ISPRITE
                        | MAP_TILE_FLAGS,
                    5,
                    0,
                    9,
                    3,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    17,
                    0,
                ]
        }));
        assert!(map_diff_payloads(&world, &character, 1, &mut cache).is_empty());
    }

    #[test]
    fn map_diff_payloads_clear_removed_visible_character() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut other = login_character(CharacterId(8), &login, 1, 11, 10);
        other.x = 11;
        other.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 10, 10));
        assert!(world.spawn_character(other, 11, 10));
        let mut cache = visible_map_cache(&world, &character, 1);

        world.remove_character(CharacterId(8));
        let payloads = map_diff_payloads(&world, &character, 1, &mut cache);

        assert_eq!(payloads.len(), 1);
        assert!(payloads[0]
            .windows(3)
            .any(|window| { window == [SV_MAP10 | SV_MAPPOS | MAP_CHARACTER_CLEAR, 5, 0] }));
    }

    #[test]
    fn map_diff_payloads_send_name_for_new_visible_character() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut other = login_character(CharacterId(8), &login_block("Guard"), 1, 11, 10);
        other.x = 11;
        other.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 10, 10));
        let mut cache = visible_map_cache(&world, &character, 1);

        assert!(world.spawn_character(other, 11, 10));
        let payloads = map_diff_payloads(&world, &character, 1, &mut cache);

        assert_eq!(payloads.len(), 1);
        assert!(payload_contains_character_name(&payloads[0], 8, "Guard"));
        assert!(payloads[0].windows(16).any(|window| {
            window[0]
                == SV_MAP10
                    | SV_MAPPOS
                    | MAP_CHARACTER_SPRITE
                    | MAP_CHARACTER_ACTION
                    | MAP_CHARACTER_STATUS
                && window[7] == 8
                && window[8] == 0
        }));
        assert!(map_diff_payloads(&world, &character, 1, &mut cache).is_empty());
    }

    #[test]
    fn client_effect_payloads_send_visible_effect_records_and_used_mask() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        let mut effect = Effect::new(EF_FIREBALL, 123, 55, 65);
        effect.from_x = 10;
        effect.from_y = 10;
        effect.to_x = 12;
        effect.to_y = 10;
        effect.x = 11 * 1024 + 512;
        effect.y = 10 * 1024 + 512;
        world.effects.insert(123, effect);
        let mut cache = ClientEffectCache::default();

        let payloads = client_effect_payloads(&world, &character, 2, &mut cache);

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
        assert_eq!(payloads[0][1], 0);
        assert_eq!(&payloads[0][2..10], &[123, 0, 0, 0, 4, 0, 0, 0]);
        assert_eq!(
            &payloads[1][..],
            &ugaris_protocol::packet::used_effects(1)[..]
        );
        assert!(client_effect_payloads(&world, &character, 2, &mut cache).is_empty());

        world.effects.clear();
        let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            &payloads[0][..],
            &ugaris_protocol::packet::used_effects(0)[..]
        );
    }

    #[test]
    fn client_effect_payloads_send_visible_edemonball_records() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        let mut effect = Effect::new(EF_EDEMONBALL, 125, 55, 65);
        effect.base_sprite = 50050;
        effect.from_x = 10;
        effect.from_y = 10;
        effect.to_x = 12;
        effect.to_y = 10;
        effect.x = 11 * 1024 + 512;
        effect.y = 10 * 1024 + 512;
        world.effects.insert(125, effect);

        let payloads =
            client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default());

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
        assert_eq!(payloads[0][1], 0);
        assert_eq!(
            &payloads[0][2..],
            &ugaris_protocol::packet::ceffect_edemonball(125, 55, 50050, 10, 10, 12, 10)[..]
        );
        assert_eq!(
            &payloads[1][..],
            &ugaris_protocol::packet::used_effects(1)[..]
        );
    }

    #[test]
    fn client_effect_payloads_send_visible_map_anchored_effect_records() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        let mut effect = Effect::new(EF_EXPLODE, 90, 55, 63);
        effect.x = 11;
        effect.y = 10;
        effect.base_sprite = 50050;
        world.effects.insert(90, effect);

        let payloads =
            client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default());

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
        assert_eq!(payloads[0][1], 0);
        assert_eq!(
            &payloads[0][2..],
            &ugaris_protocol::packet::ceffect_explode(90, 55, 50050)[..]
        );
        assert_eq!(
            &payloads[1][..],
            &ugaris_protocol::packet::used_effects(1)[..]
        );
    }

    #[test]
    fn client_effect_payloads_skip_effects_outside_visible_diamond() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        let mut effect = Effect::new(EF_BALL, 124, 55, 65);
        effect.x = 20 * 1024 + 512;
        effect.y = 20 * 1024 + 512;
        world.effects.insert(124, effect);

        assert!(
            client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default())
                .is_empty()
        );
    }

    #[test]
    fn client_effect_payloads_send_visible_character_spell_effects() {
        let login = login_block("Tester");
        let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
        viewer.x = 10;
        viewer.y = 10;
        let mut target = login_character(CharacterId(8), &login, 1, 11, 10);
        target.x = 11;
        target.y = 10;
        let mut world = World::default();
        world.characters.insert(target.id, target.clone());
        let mut effect = Effect::new(EF_BLESS, 77, 100, 200);
        effect.target_character = Some(target.id);
        effect.strength = 33;
        world.effects.insert(77, effect);

        let payloads =
            client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default());

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
        assert_eq!(payloads[0][1], 0);
        assert_eq!(
            &payloads[0][2..],
            &ugaris_protocol::packet::ceffect_bless(77, 8, 100, 200, 33)[..]
        );
        assert_eq!(
            &payloads[1][..],
            &ugaris_protocol::packet::used_effects(1)[..]
        );
    }

    #[test]
    fn client_effect_payloads_send_legacy_curse_cap_and_lag_effects() {
        let login = login_block("Tester");
        let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
        viewer.x = 10;
        viewer.y = 10;
        let mut target = login_character(CharacterId(8), &login, 1, 11, 10);
        target.x = 11;
        target.y = 10;
        let mut world = World::default();
        world.characters.insert(target.id, target.clone());

        let mut curse = Effect::new(EF_CURSE, 77, 100, 200);
        curse.target_character = Some(target.id);
        curse.strength = 33;
        world.effects.insert(77, curse);
        let mut cap = Effect::new(EF_CAP, 78, 101, 201);
        cap.target_character = Some(target.id);
        world.effects.insert(78, cap);
        let mut lag = Effect::new(EF_LAG, 79, 102, 202);
        lag.target_character = Some(target.id);
        world.effects.insert(79, lag);

        let payloads =
            client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default());

        assert_eq!(payloads.len(), 4);
        assert_eq!(
            &payloads[0][2..],
            &ugaris_protocol::packet::ceffect_curse(77, 8, 100, 200, 33)[..]
        );
        assert_eq!(
            &payloads[1][2..],
            &ugaris_protocol::packet::ceffect_cap(78, 8)[..]
        );
        assert_eq!(
            &payloads[2][2..],
            &ugaris_protocol::packet::ceffect_lag(79, 8)[..]
        );
        assert_eq!(
            &payloads[3][..],
            &ugaris_protocol::packet::used_effects(7)[..]
        );
    }

    #[test]
    fn client_effect_payloads_hide_character_spell_effects_with_hidden_target() {
        let login = login_block("Tester");
        let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
        viewer.x = 10;
        viewer.y = 10;
        let mut target = login_character(CharacterId(8), &login, 1, 20, 20);
        target.x = 20;
        target.y = 20;
        let mut world = World::default();
        world.characters.insert(target.id, target.clone());
        let mut effect = Effect::new(EF_HEAL, 77, 100, 200);
        effect.target_character = Some(target.id);
        world.effects.insert(77, effect);

        assert!(
            client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default())
                .is_empty()
        );
    }

    #[test]
    fn look_map_payload_hidden_target_matches_legacy_feedback() {
        let payloads = look_map_payloads(
            &World::default(),
            1,
            LookMapRequest {
                character_id: CharacterId(7),
                x: 12,
                y: 13,
                character_level: 0,
                visible: false,
            },
        );

        assert_eq!(text_payloads(&payloads), vec!["Too far away or hidden."]);
    }

    #[test]
    fn look_map_payload_visible_tile_reports_coords_and_zone_flags() {
        let mut world = World::default();
        world.map.set_flags(
            12,
            13,
            MapFlags::RESTAREA | MapFlags::CLAN | MapFlags::ARENA | MapFlags::PEACE,
        );

        let payloads = look_map_payloads(
            &world,
            99,
            LookMapRequest {
                character_id: CharacterId(7),
                x: 12,
                y: 13,
                character_level: 0,
                visible: true,
            },
        );

        assert_eq!(
            text_payloads(&payloads),
            vec![
                "(12,13)",
                "This place is a rest area.",
                "This is a clan area.",
                "This place is an arena.",
                "This place is a peaceful zone.",
            ]
        );
    }

    #[test]
    fn look_map_payload_visible_area1_section_reports_name_and_difficulty() {
        let payloads = look_map_payloads(
            &World::default(),
            1,
            LookMapRequest {
                character_id: CharacterId(7),
                x: 146,
                y: 115,
                character_level: 7,
                visible: true,
            },
        );

        assert_eq!(
            text_payloads(&payloads),
            vec!["Skellie I. This area is too easy for you. (146,115)"]
        );
    }

    #[test]
    fn walk_section_payload_reports_entering_once_with_legacy_color() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 146, 115);
        character.x = 146;
        character.y = 115;
        let mut player = PlayerRuntime::connected(1, 0);

        let payload = walk_section_payload(1, &mut player, &character).unwrap();

        assert_eq!(
            text_payload_bytes(&payload),
            b"\xb0c1Now entering Skellie I."
        );
        assert_eq!(special_payload(&payload), Some((1003, u32::MAX, 0)));
        assert_eq!(player.current_section_id, 46);
        assert!(walk_section_payload(1, &mut player, &character).is_none());
    }

    #[test]
    fn walk_section_payload_reports_leaving_previous_section() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 99, 12, 13);
        character.x = 12;
        character.y = 13;
        let mut player = PlayerRuntime::connected(1, 0);
        player.current_section_id = 1;

        let payload = walk_section_payload(99, &mut player, &character).unwrap();

        assert_eq!(
            text_payload_bytes(&payload),
            b"\xb0c1Now leaving Skellie I."
        );
        assert_eq!(special_payload(&payload), None);
        assert_eq!(player.current_section_id, 0);
    }

    #[test]
    fn section_music_special_matches_legacy_music_switch() {
        assert_eq!(section_music_special(4), Some(1003));
        assert_eq!(section_music_special(57), Some(1010));
        assert_eq!(section_music_special(58), Some(1004));
        assert_eq!(section_music_special(60), Some(1002));
        assert_eq!(section_music_special(114), None);
    }

    #[test]
    fn area_sound_payload_uses_section_and_legacy_special_layout() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 146, 115);
        character.x = 146;
        character.y = 115;
        let seed = seed_for_legacy_random(100, 10);

        let payload = area_sound_payload(1, &character, 12, seed).unwrap();

        assert_eq!(payload[0], SV_SPECIAL);
        assert_eq!(u32::from_le_bytes(payload[1..5].try_into().unwrap()), 14);
        assert_eq!(
            i32::from_le_bytes(payload[5..9].try_into().unwrap()),
            -(legacy_random(seed.wrapping_add(1), 1000) as i32 + 100)
        );
        assert_eq!(
            i32::from_le_bytes(payload[9..13].try_into().unwrap()),
            5000 - legacy_random(seed.wrapping_add(2), 10000) as i32
        );
    }

    #[test]
    fn area_sound_payload_is_silent_outside_ambient_sections() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 12, 13);
        character.x = 12;
        character.y = 13;
        let seed = seed_for_legacy_random(100, 10);

        assert_eq!(area_sound_payload(99, &character, 12, seed), None);
    }

    fn text_payloads(payloads: &[bytes::BytesMut]) -> Vec<String> {
        payloads
            .iter()
            .map(|payload| {
                assert_eq!(payload[0], SV_TEXT);
                let len = u16::from_le_bytes([payload[1], payload[2]]) as usize;
                String::from_utf8(payload[3..3 + len].to_vec()).unwrap()
            })
            .collect()
    }

    fn text_payload_bytes(payload: &[u8]) -> Vec<u8> {
        assert_eq!(payload[0], SV_TEXT);
        let len = u16::from_le_bytes([payload[1], payload[2]]) as usize;
        payload[3..3 + len].to_vec()
    }

    fn payload_contains_character_name(payload: &[u8], character_id: u16, name: &str) -> bool {
        let bytes = name.as_bytes();
        let packet_len = 13 + bytes.len();
        payload.windows(packet_len).any(|window| {
            window[0] == ugaris_protocol::packet::SV_NAME
                && u16::from_le_bytes([window[1], window[2]]) == character_id
                && window[12] as usize == bytes.len()
                && &window[13..] == bytes
        })
    }

    fn special_payload(payload: &[u8]) -> Option<(u32, u32, u32)> {
        let text_len = u16::from_le_bytes([payload[1], payload[2]]) as usize;
        let start = 3 + text_len;
        if payload.len() == start {
            return None;
        }
        assert_eq!(payload.len(), start + 13);
        assert_eq!(payload[start], ugaris_protocol::packet::SV_SPECIAL);
        Some((
            u32::from_le_bytes(payload[start + 1..start + 5].try_into().unwrap()),
            u32::from_le_bytes(payload[start + 5..start + 9].try_into().unwrap()),
            u32::from_le_bytes(payload[start + 9..start + 13].try_into().unwrap()),
        ))
    }

    #[test]
    fn movement_scroll_payload_uses_scroll_origin_clear_and_center_update() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 11, 10);
        character.x = 11;
        character.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 11, 10));
        world.map.tile_mut(12, 10).unwrap().ground_sprite = 777;

        let payload = movement_scroll_payload(&world, &character, 10, 10, 1).unwrap();

        assert_eq!(payload[0], SV_SCROLL_RIGHT);
        assert_eq!(payload[1], SV_ORIGIN);
        assert_eq!(&payload[2..6], &[11, 0, 10, 0]);
        assert_eq!(payload[6], SV_MAP10 | SV_MAPPOS | MAP_CHARACTER_CLEAR);
        assert_eq!(&payload[7..9], &[3, 0]);
        assert!(payload.windows(16).any(|window| {
            window
                == [
                    SV_MAP10
                        | SV_MAPPOS
                        | MAP_CHARACTER_SPRITE
                        | MAP_CHARACTER_ACTION
                        | MAP_CHARACTER_STATUS,
                    4,
                    0,
                    1,
                    0,
                    0,
                    0,
                    7,
                    0,
                    0,
                    0,
                    0,
                    0,
                    100,
                    100,
                    0,
                ]
        }));
        assert!(payload.windows(17).any(|window| {
            window
                == [
                    SV_MAP11
                        | SV_MAPPOS
                        | MAP_TILE_GSPRITE
                        | MAP_TILE_FSPRITE
                        | MAP_TILE_ISPRITE
                        | MAP_TILE_FLAGS,
                    5,
                    0,
                    9,
                    3,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    17,
                    0,
                ]
        }));
    }

    #[test]
    fn movement_scroll_payload_sends_name_for_new_fringe_character() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 11, 10);
        character.x = 11;
        character.y = 10;
        let mut other = login_character(CharacterId(8), &login_block("Guard"), 1, 12, 10);
        other.x = 12;
        other.y = 10;
        let mut world = World::default();
        assert!(world.spawn_character(character.clone(), 11, 10));
        assert!(world.spawn_character(other, 12, 10));

        let payload = movement_scroll_payload(&world, &character, 10, 10, 1).unwrap();

        assert!(payload_contains_character_name(&payload, 8, "Guard"));
    }

    #[test]
    fn movement_fringe_positions_are_newly_visible_diamond_tiles() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 11, 10);
        character.x = 11;
        character.y = 10;

        let fringe = movement_fringe_positions(&character, 10, 10, 1);

        assert_eq!(fringe, vec![(1, 11, 9), (5, 12, 10), (7, 11, 11)]);
    }

    #[test]
    fn runtime_login_allocates_character_and_disconnect_returns_it() {
        let mut runtime = ServerRuntime::default();
        let (commands, _rx) = mpsc::channel(1);

        runtime.connect(5, commands, 10);
        let character_id = runtime.login(5, &login_block("Tester"), 11);

        let player = runtime.players.get(&5).unwrap();
        assert_eq!(character_id, CharacterId(1));
        assert_eq!(player.character_id, Some(CharacterId(1)));
        assert_eq!(player.character_number, 1);
        assert_eq!(player.state, PlayerConnectionState::Normal);
        assert_eq!(
            runtime.disconnect(5).and_then(|player| player.character_id),
            Some(CharacterId(1))
        );
    }

    #[test]
    fn character_save_request_encodes_runtime_ppd_and_carried_items() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.inventory[30] = Some(ItemId(101));
        character.cursor_item = Some(ItemId(102));

        let mut inventory_item = test_item(ItemId(101), 1, ItemFlags::TAKE);
        inventory_item.carried_by = Some(character.id);
        let mut cursor_item = test_item(ItemId(102), 2, ItemFlags::TAKE);
        cursor_item.carried_by = Some(character.id);
        let ground_item = test_item(ItemId(103), 3, ItemFlags::TAKE);

        let mut world = World::default();
        world.add_character(character.clone());
        world.add_item(inventory_item);
        world.add_item(cursor_item);
        world.add_item(ground_item);

        let mut player = PlayerRuntime::connected(5, 0);
        player.add_keyring_key(0x3b000001, "Copper Key");
        player.mark_chest_access(9, 1234);

        let request = character_save_request(&world, &player, &character, 1, 2);

        assert_eq!(request.items.len(), 2);
        assert!(request.items.iter().any(|item| item.id == ItemId(101)));
        assert!(request.items.iter().any(|item| item.id == ItemId(102)));
        let mut decoded = PlayerRuntime::connected(6, 0);
        assert!(decoded.decode_legacy_ppd_blob(&request.ppd_blob));
        assert_eq!(decoded.keyring.len(), 1);
        assert_eq!(decoded.chest_last_access_seconds(9), 1234);
    }

    #[test]
    fn runtime_finds_sessions_for_character_refresh() {
        let mut runtime = ServerRuntime::default();
        let (commands, _rx) = mpsc::channel(1);
        runtime.connect(5, commands, 10);
        let character_id = runtime.login(5, &login_block("Tester"), 11);

        assert_eq!(runtime.sessions_for_character(character_id), vec![(5, 40)]);
        assert!(runtime.sessions_for_character(CharacterId(99)).is_empty());
    }

    #[test]
    fn runtime_character_ids_can_be_seeded_after_loaded_world_characters() {
        let mut runtime = ServerRuntime::default();
        runtime.set_next_character_id(189);
        let (commands, _rx) = mpsc::channel(1);
        runtime.connect(5, commands, 10);

        assert_eq!(
            runtime.login(5, &login_block("Tester"), 11),
            CharacterId(189)
        );
    }

    #[test]
    fn login_character_uses_full_scaled_resources() {
        let character = login_character(CharacterId(3), &login_block("Tester"), 12, 42, 43);

        assert_eq!(character.name, "Tester");
        assert!(character.flags.contains(CharacterFlags::PLAYER));
        assert_eq!(character.sprite, 1);
        assert_eq!(character.rest_area, 12);
        assert_eq!((character.rest_x, character.rest_y), (42, 43));
        assert_eq!(character.hp, 50 * POWERSCALE);
        assert_eq!(character.values[0][CharacterValue::Hp as usize], 50);
        assert_eq!(character.values[1][CharacterValue::Hp as usize], 50);
    }

    #[test]
    fn load_area_zone_reads_first_area_map_file() {
        let root = unique_temp_zone_root("load_area_zone_reads_first_area_map_file");
        let area = root.join("1");
        std::fs::create_dir_all(&area).unwrap();
        std::fs::write(
            area.join("sample.map"),
            r#"
            field="10,11"
            gsprite=123
            fsprite=456
            flag=MF_MOVEBLOCK
            "#,
        )
        .unwrap();

        let mut world = World::default();
        let mut loader = ZoneLoader::new();
        let summary = load_area_zone(&mut world, &mut loader, &root, 1).unwrap();

        let tile = world.map.tile(10, 11).unwrap();
        assert_eq!(tile.ground_sprite, 123);
        assert_eq!(tile.foreground_sprite, 456);
        assert!(tile.flags.contains(MapFlags::MOVEBLOCK));
        assert_eq!(summary.ground_tiles, 1);
        assert_eq!(summary.blocked_tiles, 1);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn login_character_from_template_uses_starter_inventory() {
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                sword1q1: name="Sword" ;
                torch: name="Torch" ;
                armor1q1: name="Armor" ;
                leggings1q1: name="Leggings" ;
                sleeves1q1: name="Sleeves" ;
                helmet1q1: name="Helmet" ;
                healing_potion1: name="Potion" ;
                recall_scroll2: name="Recall" ;
                "#,
            )
            .unwrap();
        loader
            .load_character_templates_str(
                r#"
                new_warrior_m:
                    name="Newbie"
                    sprite=2
                    flag=CF_PLAYER
                    flag=CF_MALE
                    flag=CF_ALIVE
                    V_HP=10
                    V_ENDURANCE=10
                    WN_RHAND=sword1q1
                    WN_LHAND=torch
                    item=healing_potion1
                    item=recall_scroll2
                ;
                "#,
            )
            .unwrap();

        let (character, items) = login_character_from_template(
            &mut loader,
            CharacterId(77),
            &login_block("Tester"),
            12,
            42,
            43,
        )
        .unwrap();

        assert_eq!(character.id, CharacterId(77));
        assert_eq!(character.name, "Tester");
        assert_eq!(character.sprite, 2);
        assert_eq!(
            (character.rest_area, character.rest_x, character.rest_y),
            (12, 42, 43)
        );
        assert_eq!(character.values[1][CharacterValue::Hp as usize], 10);
        assert_eq!(character.inventory[6], Some(items[0].id));
        assert_eq!(character.inventory[8], Some(items[1].id));
        assert_eq!(character.inventory[30], Some(items[2].id));
        assert_eq!(character.inventory[31], Some(items[3].id));
    }

    #[test]
    fn grant_chest_treasure_instantiates_template_to_cursor() {
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                treasure_9:
                    name="Coins"
                    sprite=105
                    flag=IF_TAKE
                    flag=IF_MONEY
                    value=2500
                ;
                "#,
            )
            .unwrap();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));

        assert_eq!(
            grant_chest_treasure(&mut world, &mut loader, CharacterId(7), 9),
            Some("Coins".to_string())
        );

        let character = world.characters.get(&CharacterId(7)).unwrap();
        let item_id = character.cursor_item.unwrap();
        let item = world.items.get(&item_id).unwrap();
        assert_eq!(item.name, "Coins");
        assert_eq!(item.sprite, 105);
        assert_eq!(item.carried_by, Some(CharacterId(7)));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(
            grant_chest_treasure(&mut world, &mut loader, CharacterId(7), 9),
            None
        );
    }

    #[test]
    fn grant_template_item_to_cursor_supports_infinite_chest_runes() {
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                rune4:
                    name="Rune IV"
                    sprite=444
                    flag=IF_TAKE
                ;
                "#,
            )
            .unwrap();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));

        assert_eq!(
            grant_template_item_to_cursor(&mut world, &mut loader, CharacterId(7), "rune4"),
            Some("Rune IV".to_string())
        );

        let character = world.characters.get(&CharacterId(7)).unwrap();
        let item = world.items.get(&character.cursor_item.unwrap()).unwrap();
        assert_eq!(item.name, "Rune IV");
        assert_eq!(item.sprite, 444);
        assert_eq!(item.carried_by, Some(CharacterId(7)));
    }

    #[test]
    fn infinite_chest_context_uses_inventory_key_not_keyring() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(30));
        let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
        key.template_id = 0x1122_3344;
        key.name = "Palace Key".to_string();
        let mut chest = test_item(ItemId(70), 1, ItemFlags::USE);
        chest.driver = ugaris_core::item_driver::IDR_INFINITE_CHEST;
        chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

        let mut world = World::default();
        world.add_character(character);
        world.add_item(key);
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(5, 0);
        player.add_keyring_key(0x5566_7788, "Wrong Keyring Key");

        let context = item_driver_context_for_request(
            &world,
            Some(&player),
            &ugaris_core::item_driver::ItemDriverRequest::Driver {
                driver: ugaris_core::item_driver::IDR_INFINITE_CHEST,
                item_id: ItemId(70),
                character_id: CharacterId(7),
                spec: 0,
            },
        );

        assert_eq!(context.door_key.unwrap().name, "Palace Key");
    }

    #[test]
    fn infinite_chest_context_rejects_skeleton_key() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(30));
        let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
        key.template_id = IID_SKELETON_KEY;
        key.name = "Skeleton Key".to_string();
        let mut chest = test_item(ItemId(70), 1, ItemFlags::USE);
        chest.driver = ugaris_core::item_driver::IDR_INFINITE_CHEST;
        chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

        let mut world = World::default();
        world.add_character(character);
        world.add_item(key);
        world.add_item(chest);

        let context = item_driver_context_for_request(
            &world,
            None,
            &ugaris_core::item_driver::ItemDriverRequest::Driver {
                driver: ugaris_core::item_driver::IDR_INFINITE_CHEST,
                item_id: ItemId(70),
                character_id: CharacterId(7),
                spec: 0,
            },
        );

        assert_eq!(context.door_key, None);
    }

    #[test]
    fn apply_assemble_item_replaces_used_item_and_consumes_cursor() {
        let character_id = CharacterId(7);
        let used_id = ItemId(70);
        let cursor_id = ItemId(71);
        let mut character = login_character(character_id, &login_block("Assembler"), 1, 10, 10);
        character.inventory[30] = Some(used_id);
        character.cursor_item = Some(cursor_id);

        let mut world = World::default();
        world.add_character(character);
        let mut used = test_item(used_id, 100, ItemFlags::USED | ItemFlags::USE);
        used.carried_by = Some(character_id);
        world.add_item(used);
        let mut cursor = test_item(cursor_id, 101, ItemFlags::USED | ItemFlags::TAKE);
        cursor.carried_by = Some(character_id);
        world.add_item(cursor);

        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"sun_amulet123: name="Sun Amulet" sprite=444 ;"#)
            .unwrap();

        assert_eq!(
            apply_assemble_item(
                &mut world,
                &mut loader,
                used_id,
                character_id,
                cursor_id,
                "sun_amulet123",
            ),
            AssembleApplyResult::Assembled
        );

        let character = world.characters.get(&character_id).unwrap();
        let new_id = character.inventory[30].unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(!world.items.contains_key(&used_id));
        assert!(!world.items.contains_key(&cursor_id));
        assert_eq!(world.items.get(&new_id).unwrap().name, "Sun Amulet");
    }

    #[test]
    fn apply_keyring_add_cursor_item_stores_key_and_consumes_cursor() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let key_item_id = ItemId(44);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(key_item_id);
        world.add_character(character);
        let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = IID_AREA1_SKELKEY1;
        key.carried_by = Some(character_id);
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);

        assert_eq!(
            apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
            KeyringAddApplyResult::Added {
                key_name: "Copper Key".to_string(),
            }
        );

        assert_eq!(
            player.keyring_key_name(IID_AREA1_SKELKEY1),
            Some("Copper Key")
        );
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        let key = world.items.get(&key_item_id).unwrap();
        assert_eq!(key.carried_by, None);
        assert!(!key.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn apply_keyring_add_cursor_item_rejects_unregistered_key_like_item() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let key_item_id = ItemId(44);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(key_item_id);
        world.add_character(character);
        let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
        key.name = "Decorative Key".to_string();
        key.template_id = 0x1122_3344;
        key.carried_by = Some(character_id);
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);

        assert_eq!(
            apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
            KeyringAddApplyResult::NotAKey
        );
        assert!(player.keyring.is_empty());
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(key_item_id)
        );
        assert!(world
            .items
            .get(&key_item_id)
            .unwrap()
            .flags
            .contains(ItemFlags::USED));
    }

    #[test]
    fn apply_keyring_add_cursor_item_reports_duplicate_without_consuming() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let key_item_id = ItemId(44);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(key_item_id);
        world.add_character(character);
        let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = IID_AREA1_SKELKEY1;
        key.carried_by = Some(character_id);
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        assert_eq!(
            player.add_keyring_key(IID_AREA1_SKELKEY1, "Copper Key"),
            KeyringAddResult::Added
        );

        assert_eq!(
            apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
            KeyringAddApplyResult::Duplicate
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(key_item_id)
        );
        assert!(world
            .items
            .get(&key_item_id)
            .unwrap()
            .flags
            .contains(ItemFlags::USED));
    }

    #[test]
    fn apply_keyring_auto_add_pickup_stores_registered_key_and_consumes_cursor() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let key_item_id = ItemId(44);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(key_item_id);
        world.add_character(character);
        let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = IID_AREA1_SKELKEY1;
        key.carried_by = Some(character_id);
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        player.set_keyring_auto_add(true);

        assert_eq!(
            apply_keyring_auto_add_pickup(&mut world, Some(&mut player), character_id, key_item_id,),
            Some(KeyringAutoAddPickupResult::Added {
                key_name: "Copper Key".to_string(),
            })
        );

        assert_eq!(
            player.keyring_key_name(IID_AREA1_SKELKEY1),
            Some("Copper Key")
        );
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        let key = world.items.get(&key_item_id).unwrap();
        assert_eq!(key.carried_by, None);
        assert!(!key.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn apply_keyring_auto_add_pickup_leaves_duplicate_key_on_cursor() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let key_item_id = ItemId(44);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(key_item_id);
        world.add_character(character);
        let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = IID_AREA1_SKELKEY1;
        key.carried_by = Some(character_id);
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(character_id);
        player.set_keyring_auto_add(true);
        assert_eq!(
            player.add_keyring_key(IID_AREA1_SKELKEY1, "Copper Key"),
            KeyringAddResult::Added
        );

        assert_eq!(
            apply_keyring_auto_add_pickup(&mut world, Some(&mut player), character_id, key_item_id,),
            Some(KeyringAutoAddPickupResult::Duplicate {
                key_name: "Copper Key".to_string(),
            })
        );

        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(key_item_id)
        );
        assert!(world
            .items
            .get(&key_item_id)
            .unwrap()
            .flags
            .contains(ItemFlags::USED));
    }

    #[test]
    fn apply_chest_treasure_tracks_legacy_hour_cooldown() {
        let mut loader = chest_loader();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0, 0, 0, 0, 1, 0];
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: None,
            }
        );
        world
            .characters
            .get_mut(&CharacterId(7))
            .unwrap()
            .cursor_item = None;

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100 + 3599,
            ),
            ChestTreasureApplyResult::Empty
        );

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100 + 3600,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: None,
            }
        );
    }

    #[test]
    fn apply_chest_treasure_requires_and_accepts_exact_inventory_key() {
        let mut loader = chest_loader();
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        world.add_character(character);
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
        world.add_item(chest);
        let mut key = test_item(ItemId(20), 701, ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = 0x1122_3344;
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: Some("Copper Key".to_string()),
            }
        );
    }

    #[test]
    fn apply_chest_treasure_accepts_skeleton_key_for_keyed_chest() {
        let mut loader = chest_loader();
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        world.add_character(character);
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
        world.add_item(chest);
        let mut key = test_item(ItemId(20), 701, ItemFlags::TAKE);
        key.name = "Skeleton Key".to_string();
        key.template_id = IID_SKELETON_KEY;
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: Some("Skeleton Key".to_string()),
            }
        );
    }

    #[test]
    fn apply_chest_treasure_blocks_keyed_chest_without_key() {
        let mut loader = chest_loader();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::KeyRequired
        );
        assert_eq!(player.chest_last_access_seconds(9), 0);
    }

    #[test]
    fn apply_chest_treasure_accepts_keyring_key_for_keyed_chest() {
        let mut loader = chest_loader();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Keyring Key"),
            ugaris_core::player::KeyringAddResult::Added
        );

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: Some("Keyring Key".to_string()),
            }
        );
    }

    #[test]
    fn item_driver_context_supplies_keyring_key_for_keyed_door() {
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut door = test_item(ItemId(10), 700, ItemFlags::USE);
        door.driver = ugaris_core::item_driver::IDR_DOOR;
        door.driver_data = vec![0, 0x44, 0x33, 0x22, 0x11];
        world.add_item(door);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Keyring Key"),
            ugaris_core::player::KeyringAddResult::Added
        );

        let request = ugaris_core::item_driver::ItemDriverRequest::Driver {
            driver: ugaris_core::item_driver::IDR_DOOR,
            item_id: ItemId(10),
            character_id: CharacterId(7),
            spec: 0,
        };

        assert_eq!(
            item_driver_context_for_request(&world, Some(&player), &request).door_key,
            Some(ugaris_core::item_driver::DoorKeyAccess {
                key_id: 0x1122_3344,
                name: "Keyring Key".to_string(),
                source: ugaris_core::item_driver::DoorKeySource::Keyring,
            })
        );
    }

    #[test]
    fn apply_chest_treasure_respects_death_gate() {
        let mut loader = chest_loader();
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0, 0, 0, 0, 0, 0, 2];
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Empty
        );
        assert_eq!(player.chest_last_access_seconds(9), 0);

        world.characters.get_mut(&CharacterId(7)).unwrap().deaths = 2;
        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                101,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Coins".to_string(),
                key_name: None,
            }
        );
    }

    #[test]
    fn apply_chest_treasure_records_chest_achievements_only_on_success() {
        let mut loader = chest_loader_with_gold_room();
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.deaths = 1;
        world.add_character(character);
        let mut gated_chest = test_item(ItemId(10), 700, ItemFlags::USE);
        gated_chest.driver_data = vec![9, 0, 0, 0, 0, 0, 0, 2];
        world.add_item(gated_chest);
        let mut gold_room_chest = test_item(ItemId(11), 701, ItemFlags::USE);
        gold_room_chest.driver_data = vec![63];
        world.add_item(gold_room_chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::Empty
        );
        assert_eq!(player.achievements.chests_opened, 0);

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(11),
                CharacterId(7),
                63,
                101,
            ),
            ChestTreasureApplyResult::Granted {
                item_name: "Gold".to_string(),
                key_name: None,
            }
        );
        assert_eq!(player.achievements.chests_opened, 1);
        assert!(player.achievements.gold_looter);
    }

    #[test]
    fn apply_random_chest_grants_money_and_enforces_daily_cooldown() {
        let mut loader = ZoneLoader::new();
        let mut world = random_chest_world(10, 0);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        let seed = seed_for_legacy_random(4, 0);

        let result = apply_random_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            1,
            100,
            seed,
        );
        let RandomChestApplyResult::Money { amount } = result else {
            panic!("expected money result, got {result:?}");
        };
        assert_eq!(amount, random_chest_money_amount(10, seed));
        assert_eq!(player.achievements.chests_opened, 1);
        assert_eq!(
            player.random_chest_last_used_seconds(random_chest_location_id(5, 6, 1)),
            Some(100)
        );

        let character = world.characters.get_mut(&CharacterId(7)).unwrap();
        let money_id = character.cursor_item.take().unwrap();
        let money = world.items.get(&money_id).unwrap();
        assert!(money.flags.contains(ItemFlags::MONEY));
        assert_eq!(money.value, amount);

        assert_eq!(
            apply_random_chest(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                1,
                100 + RANDCHEST_COOLDOWN_SECONDS - 1,
                seed,
            ),
            RandomChestApplyResult::Empty
        );
        assert_eq!(player.achievements.chests_opened, 1);
    }

    #[test]
    fn apply_random_chest_no_tier_empty_roll_consumes_daily_access() {
        let mut loader = ZoneLoader::new();
        let mut world = random_chest_world(10, 0);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        let seed = seed_for_legacy_random(4, 1);

        assert_eq!(
            apply_random_chest(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                1,
                100,
                seed,
            ),
            RandomChestApplyResult::Empty
        );
        assert_eq!(player.achievements.chests_opened, 0);
        assert_eq!(
            player.random_chest_last_used_seconds(random_chest_location_id(5, 6, 1)),
            Some(100)
        );
    }

    #[test]
    fn apply_random_chest_can_grant_template_loot_for_tier_rolls() {
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                healing_potion1:
                    name="Healing Potion"
                    sprite=200
                    flag=IF_TAKE
                ;
                "#,
            )
            .unwrap();
        let mut world = random_chest_world(10, 1);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        let seed = seed_for_legacy_random(28, 21);

        assert_eq!(
            apply_random_chest(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                1,
                100,
                seed,
            ),
            RandomChestApplyResult::Item {
                item_name: "Healing Potion".to_string()
            }
        );
        let item_id = world
            .characters
            .get(&CharacterId(7))
            .unwrap()
            .cursor_item
            .unwrap();
        assert_eq!(world.items.get(&item_id).unwrap().name, "Healing Potion");
        assert_eq!(player.achievements.chests_opened, 1);
    }

    #[test]
    fn apply_chest_treasure_sees_cursor_key_but_keeps_cursor_occupied_rule() {
        let mut loader = chest_loader();
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        world.add_character(character);
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
        world.add_item(chest);
        let mut key = test_item(ItemId(20), 701, ItemFlags::TAKE);
        key.name = "Copper Key".to_string();
        key.template_id = 0x1122_3344;
        world.add_item(key);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::CursorOccupied
        );
        assert_eq!(player.chest_last_access_seconds(9), 0);
    }

    #[test]
    fn apply_chest_treasure_reports_cursor_occupied_before_cooldown() {
        let mut loader = chest_loader();
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(99));
        world.add_character(character);
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0, 0, 0, 0, 1, 0];
        world.add_item(chest);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        assert_eq!(
            apply_chest_treasure(
                &mut world,
                &mut loader,
                Some(&mut player),
                ItemId(10),
                CharacterId(7),
                9,
                100,
            ),
            ChestTreasureApplyResult::CursorOccupied
        );
        assert_eq!(player.chest_last_access_seconds(9), 0);
    }

    #[test]
    fn chest_helpers_decode_legacy_driver_data() {
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 2, 0, 3];

        assert_eq!(chest_required_key_id(&chest), 0x1122_3344);
        assert_eq!(chest_timeout_seconds(&chest), 2 * 60 * 60);
        assert_eq!(chest_required_deaths(&chest), 3);
    }

    #[test]
    fn chest_blocked_message_prefers_key_requirement_like_legacy_driver() {
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(99));
        world.add_character(character);
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.driver_data = vec![9, 1, 0, 0, 0];
        world.add_item(chest);

        assert_eq!(
            chest_blocked_message(&world, ItemId(10), CharacterId(7)),
            CHEST_KEY_REQUIRED_MESSAGE
        );
    }

    fn chest_loader() -> ZoneLoader {
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"
                treasure_9:
                    name="Coins"
                    sprite=105
                    flag=IF_TAKE
                    flag=IF_MONEY
                    value=2500
                ;
                "#,
            )
            .unwrap();
        loader
    }

    fn chest_loader_with_gold_room() -> ZoneLoader {
        let mut loader = chest_loader();
        loader
            .load_item_templates_str(
                r#"
                treasure_63:
                    name="Gold"
                    sprite=106
                    flag=IF_TAKE
                    flag=IF_MONEY
                    value=5000
                ;
                "#,
            )
            .unwrap();
        loader
    }

    fn random_chest_world(money_level: u8, loot_tier: u8) -> World {
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(7),
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
        chest.x = 5;
        chest.y = 6;
        chest.driver_data = vec![money_level, loot_tier];
        world.add_item(chest);
        world
    }

    fn seed_for_legacy_random(max: u32, target: u32) -> u64 {
        (0..10_000)
            .find(|seed| legacy_random(*seed, max) == target)
            .expect("test seed exists")
    }

    #[test]
    fn choose_spawn_tile_skips_blocked_default_spawn() {
        let mut world = World::default();
        world
            .map
            .tile_mut(LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        let (x, y) = choose_spawn_tile(&world);

        assert_ne!((x, y), (LOGIN_SPAWN_X, LOGIN_SPAWN_Y));
        assert!(is_spawn_tile_open(&world, x, y));
    }

    #[test]
    fn next_available_character_id_follows_loaded_world_characters() {
        let mut world = World::default();
        assert!(world.spawn_character(
            login_character(CharacterId(12), &login_block("Npc"), 1, 10, 10),
            10,
            10,
        ));

        assert_eq!(next_available_character_id(&world), 13);
    }

    #[test]
    fn client_center_map_position_matches_legacy_cmap_index() {
        assert_eq!(client_center_map_position(25), 25 + 25 * 51);
        assert_eq!(client_center_map_position(40), 40 + 40 * 81);
    }

    #[test]
    fn resource_percent_matches_legacy_scaled_resource_math() {
        assert_eq!(resource_percent(50 * POWERSCALE, 50), 100);
        assert_eq!(resource_percent(25 * POWERSCALE, 50), 50);
        assert_eq!(resource_percent(-1, 50), 0);
    }

    fn login_block(name: &str) -> LoginBlock {
        LoginBlock {
            name: name.into(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        }
    }

    fn unique_temp_zone_root(test_name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("ugaris-server-{test_name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
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

    let character_repository = if let Some(database_url) = args.database_url.as_deref() {
        let db = ugaris_db::Database::connect(database_url, 8).await?;
        db.ping().await?;
        info!("connected to PostgreSQL");
        Some(db.characters())
    } else {
        warn!("DATABASE_URL not set; starting without persistence");
        None
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
    let mut tick = time::interval(TickRate::default().interval());
    info!(
        area_id = config.area_id,
        mirror_id = config.mirror_id,
        "entering Rust game loop"
    );

    loop {
        tokio::select! {
            _ = tick.tick() => {
                world.advance();
                world.tick_effects();
                let timer_outcomes = world.process_due_timers(config.area_id);
                if !timer_outcomes.is_empty() {
                    info!(count = timer_outcomes.len(), tick = world.tick.0, "processed timer callbacks");
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
                let mut command_inventory_refresh = Vec::new();
                let mut command_container_refresh = Vec::new();
                for (session_id, action) in queued {
                    let Some(player) = runtime.players.get(&session_id) else {
                        continue;
                    };
                    let Some(character_id) = player.character_id else {
                        continue;
                    };
                    match action {
                        ClientAction::Text(bytes) => {
                            let Some(command) = normalize_text_command(&bytes) else {
                                continue;
                            };
                            if command.eq_ignore_ascii_case("accountdepotsort") {
                                if let Some(depot) = runtime.account_depots.get_mut(&character_id) {
                                    account_depot_sort(depot);
                                    command_container_refresh.push(character_id);
                                    command_feedback.push((character_id, "Account depot sorted.".to_string()));
                                } else {
                                    command_feedback.push((character_id, "You must have the account depot open to use this command.".to_string()));
                                }
                                continue;
                            }
                            let Some(player) = runtime.players.get_mut(&session_id) else {
                                continue;
                            };
                            if let Some(result) = apply_keyring_command(&mut world, &mut zone_loader, player, character_id, &command) {
                                for message in result.messages {
                                    command_feedback.push((character_id, message));
                                }
                                if result.inventory_changed {
                                    command_inventory_refresh.push(character_id);
                                }
                            }
                        }
                        ClientAction::Container { .. } | ClientAction::LookContainer { .. } => {
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
                        _ => {}
                    }
                }
                if !command_feedback.is_empty() || !command_inventory_refresh.is_empty() || !command_container_refresh.is_empty() {
                    let mut feedback_sessions = 0;
                    for (character_id, message) in command_feedback {
                        let payload = ugaris_protocol::packet::system_text(&message);
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
                        let payload = current_container_payload(
                            &world,
                            runtime.account_depots.get(&character_id),
                            character_id,
                        );
                        let Some(payload) = payload else { continue };
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            if runtime.send_to_session(session_id, payload.clone()) {
                                container_sessions += 1;
                            }
                        }
                    }
                    info!(feedback_sessions, inventory_sessions, container_sessions, tick = world.tick.0, "processed text/container commands");
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
                let completed_actions = world.tick_basic_actions();
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
                        match apply_keyring_auto_add_pickup(
                            &mut world,
                            runtime.player_for_character_mut(completion.character_id),
                            completion.character_id,
                            item_id,
                        ) {
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
                        .filter_map(|completion| completion.item_use)
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
                        let mut area_feedback = Vec::new();
                        let mut container_refresh = Vec::new();
                        for request in item_use_requests {
                            let use_character_id = request.character_id;
                            match world.use_item_request(request, true) {
                                Ok(ugaris_core::item_driver::UseItemOutcome::OpenContainer { .. })
                                | Ok(ugaris_core::item_driver::UseItemOutcome::OpenDepot { .. }) => {
                                    opened += 1;
                                }
                                Ok(ugaris_core::item_driver::UseItemOutcome::OpenAccountDepot { .. }) => {
                                    runtime.ensure_account_depot(use_character_id);
                                    container_refresh.push(use_character_id);
                                    opened += 1;
                                }
                                Ok(ugaris_core::item_driver::UseItemOutcome::Dispatch(request)) => {
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
                                    match world.execute_item_driver_request_with_context(request, config.area_id, &driver_context) {
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
                                                }
                                                RandomChestApplyResult::Item { item_name } => {
                                                    feedback.push((character_id, format!("You found a {item_name}.")));
                                                    executed += 1;
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
                                                DemonShrineResult::Learned { .. } => {
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
                                        ugaris_core::item_driver::ItemDriverOutcome::XmasTree { character_id, .. } => {
                                            let (is_xmas, event_year) = current_xmas_event();
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
                                        ugaris_core::item_driver::ItemDriverOutcome::PotionDrunk { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FoodEaten { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LollipopLicked { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LollipopMemories { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ChristmasPopInspected { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StatScrollUsed { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoubleDoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Teleport { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeleportDoor { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Recall { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::CityRecall { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BallTrapProjectile { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::EdemonBallProjectile { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerExtinguished { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapTriggered { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapReset { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TriggerMapItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StepTrapDiscoverTarget { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LightChanged { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DecayItemToggled { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitAnimating { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitExpired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitUse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BeyondPotion { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::EnchantCursorItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::AntiEnchantCursorItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletAssemble { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyAssemble { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyCombine { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::AccountDepotOpened { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LookItem { .. } => {
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
                                        ugaris_core::item_driver::ItemDriverOutcome::LabExitWrongOwner { character_id, .. } => {
                                            feedback.push((character_id, "This gate has not been created for you. You cannot use it.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EmptyPotionTemplateNeeded { .. } => {
                                            deferred_templates += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { item_id, character_id }
                                            if is_no_potion_area_blocked_item(&world, item_id) =>
                                        {
                                            feedback.push((character_id, "You sense that the potion would not work.".to_string()));
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                            if is_beyond_potion_item(&world, item_id) =>
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
                                        ugaris_core::item_driver::ItemDriverOutcome::Unsupported { .. } => {
                                            unsupported += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::TorchExpired { .. } => {
                                            executed += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::DecayItemExpired { character_id, item_name, .. } => {
                                            let item_name = String::from_utf8_lossy(&item_name)
                                                .trim_end_matches('\0')
                                                .to_string();
                                            feedback.push((character_id, format!("Your {item_name} expired.")));
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
                        for (session_id, view_distance) in runtime.sessions_for_character(completion.character_id) {
                            let mut payloads = if completion.ok
                                && completion.action_id == ugaris_core::legacy::action::WALK
                            {
                                let payloads = movement_scroll_payload(
                                    &world,
                                    character,
                                    completion.old_x,
                                    completion.old_y,
                                    view_distance,
                                )
                                .map(|payload| vec![payload])
                                .unwrap_or_else(|| map_refresh_payloads(&world, character, view_distance));
                                runtime.map_caches.insert(
                                    session_id,
                                    visible_map_cache(&world, character, view_distance),
                                );
                                payloads
                            } else {
                                match runtime.map_caches.get_mut(&session_id) {
                                    Some(cache) => map_diff_payloads(
                                        &world,
                                        character,
                                        view_distance,
                                        cache,
                                    ),
                                    None => {
                                        let payloads =
                                            map_refresh_payloads(&world, character, view_distance);
                                        runtime.map_caches.insert(
                                            session_id,
                                            visible_map_cache(&world, character, view_distance),
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
                }

                let simple_baddy_message_characters: Vec<_> = world
                    .characters
                    .iter()
                    .filter_map(|(&character_id, character)| {
                        (!character.driver_messages.is_empty()
                            && (character.driver == CDR_SIMPLEBADDY
                                || matches!(
                                    character.driver_state.as_ref(),
                                    Some(CharacterDriverState::SimpleBaddy(_))
                                )))
                        .then_some(character_id)
                    })
                    .collect();
                if !simple_baddy_message_characters.is_empty() {
                    let mut simple_baddy_outcomes = 0;
                    for character_id in simple_baddy_message_characters {
                        simple_baddy_outcomes += world
                            .process_simple_baddy_message_actions(character_id, config.area_id)
                            .len();
                    }
                    info!(simple_baddy_outcomes, tick = world.tick.0, "processed simple-baddy driver messages");
                }

                let simple_baddy_attacks = world.process_simple_baddy_attack_actions(config.area_id);
                if simple_baddy_attacks != 0 {
                    info!(simple_baddy_attacks, tick = world.tick.0, "queued simple-baddy attack actions");
                }

                let simple_baddy_noncombat = world.process_simple_baddy_noncombat_actions(config.area_id);
                if simple_baddy_noncombat != 0 {
                    info!(simple_baddy_noncombat, tick = world.tick.0, "queued simple-baddy noncombat actions");
                }

                let (periodic_diff_sessions, periodic_empty_frames) =
                    queue_periodic_player_frames(&mut runtime, &world);
                if periodic_diff_sessions != 0 {
                    info!(periodic_diff_sessions, tick = world.tick.0, "queued periodic map/action diffs");
                }
                if periodic_empty_frames != 0 {
                    tracing::trace!(periodic_empty_frames, tick = world.tick.0, "queued empty legacy tick frames");
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
                        let mut loaded_from_database = false;
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
                                    }
                                    match repository.load_character_snapshot(db_character_id).await {
                                        Ok(Some(snapshot)) => {
                                            if let Some(player) = runtime.players.get_mut(&id.0) {
                                                loaded_from_database = apply_character_snapshot(
                                                    &mut world,
                                                    player,
                                                    snapshot,
                                                    spawn_tile.0,
                                                    spawn_tile.1,
                                                );
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
                                Ok(outcome) => {
                                    warn!(%id, code = outcome.legacy_find_login_code(), "DB login did not return a local ready character; using scaffold");
                                }
                                Err(err) => {
                                    warn!(%id, error = %err, "DB login failed; using scaffold");
                                }
                            }
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
                            }
                        }
                        let view_distance = runtime
                            .players
                            .get(&id.0)
                            .map(|player| player.view_distance)
                            .unwrap_or(ugaris_core::legacy::DIST_OLD);
                        let payloads = world
                            .characters
                            .get(&character_id)
                            .map(|character| {
                                runtime.map_caches.insert(
                                    id.0,
                                    visible_map_cache(&world, character, view_distance),
                                );
                                login_bootstrap_payloads(
                                    &world,
                                    character,
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
                                    runtime.effect_caches.entry(id.0).or_default(),
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
                                    visible_map_cache(&world, &fallback_character, view_distance),
                                );
                                login_bootstrap_payloads(
                                    &world,
                                    &fallback_character,
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
                                    runtime.effect_caches.entry(id.0).or_default(),
                                )
                            });
                        let payload_count = payloads.len();
                        if !runtime.send_many_to_session(id.0, payloads) {
                            warn!(%id, "failed to queue complete login bootstrap for session");
                        }
                        info!(%id, name = %login.name, client_version = ?login.client_version, payload_count, "login accepted by compatibility scaffold");
                    }
                    SessionEvent::Action { id, command_kind, action } => {
                        runtime.queue_action(id.0, action, world.tick.0);
                        info!(%id, command = command_kind, "action queued for gameplay port");
                    }
                    SessionEvent::Disconnected { id } => {
                        if let Some(player) = runtime.disconnect(id.0) {
                            if let Some(character_id) = player.character_id {
                                if let Some(repository) = &character_repository {
                                    if let Some(character) = world.characters.get(&character_id) {
                                        let request = character_save_request(
                                            &world,
                                            &player,
                                            character,
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
                            }
                        }
                        info!(%id, "session removed");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown requested");
                break;
            }
        }
    }

    Ok(())
}
