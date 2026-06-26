use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::Parser;
use tokio::{sync::mpsc, time};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use ugaris_core::{
    entity::{Character, CharacterFlags, CharacterValue, ItemFlags, SpeedMode, POWERSCALE},
    ids::{CharacterId, ItemId},
    map::{MapFlags, MapTile},
    player::{PlayerActionCode, PlayerConnectionState, PlayerRuntime, QueuedAction},
    tick::TICKS_PER_SECOND,
    world::LookMapRequest,
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
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

    fn disconnect(&mut self, session_id: u64) -> Option<CharacterId> {
        let character_id = self
            .players
            .remove(&session_id)
            .and_then(|player| player.character_id);
        self.sessions.remove(&session_id);
        self.action_queue.retain(|(id, _)| *id != session_id);
        character_id
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
}

const LOGIN_SPAWN_X: usize = 128;
const LOGIN_SPAWN_Y: usize = 128;
const LOGIN_ACCEPTED_MESSAGE: &str = "Rust Ugaris compatibility login accepted.";
const CHEST_EMPTY_MESSAGE: &str = "The chest is empty.";
const CHEST_CURSOR_OCCUPIED_MESSAGE: &str = "Please empty your 'hand' (mouse cursor) first.";
const CHEST_KEY_REQUIRED_MESSAGE: &str = "You need a key to open this chest.";
const RANDCHEST_CURSOR_OCCUPIED_MESSAGE: &str = "Please empty your hand (mouse cursor) first.";
const RANDCHEST_EMPTY_MESSAGE: &str = "You didn't find anything.";
const MAP_BOOTSTRAP_CHUNK_TARGET: usize = MAX_LEGACY_TICK_PAYLOAD - 512;
const DEFAULT_PLAYER_TEMPLATE: &str = "new_warrior_m";
const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;
const RANDCHEST_COOLDOWN_SECONDS: u64 = 60 * 60 * 24;

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
enum RandomChestApplyResult {
    Money { amount: u32 },
    Item { item_name: String },
    Empty,
    CursorOccupied,
    MissingPlayer,
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
        deaths: 0,
        cursor_item: None,
        current_container: None,
        values,
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
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
    if *driver != ugaris_core::item_driver::IDR_DOOR {
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

    ugaris_core::item_driver::ItemDriverContext {
        door_key: door_key_access(world, player, *character_id, required_key_id),
    }
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

fn login_bootstrap_payloads(
    world: &World,
    character: &Character,
    mirror_id: u16,
    tick: u64,
    view_distance: usize,
) -> Vec<bytes::BytesMut> {
    let mut payloads = vec![login_payload(world, character, mirror_id, tick)];
    payloads.extend(initial_map_payloads(world, character, view_distance));
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

fn look_map_payloads(world: &World, request: LookMapRequest) -> Vec<bytes::BytesMut> {
    if !request.visible {
        return vec![ugaris_protocol::packet::system_text(
            "Too far away or hidden.",
        )];
    }

    let mut messages = Vec::new();
    messages.push(format!("({},{})", request.x, request.y));

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

        append_map_packet(
            &mut payloads,
            &mut current,
            map_tile_packet(world, tile, client_pos),
        );

        if let Some(character) = tile_character(world, tile) {
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
            queued1(spell_to_player_action(*spell, false), *character)
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
        MAP_CHARACTER_ACTION, MAP_CHARACTER_SPRITE, MAP_CHARACTER_STATUS, MAP_TILE_FLAGS,
        MAP_TILE_FSPRITE, MAP_TILE_GSPRITE, MAP_TILE_ISPRITE, SV_LOGINDONE, SV_MAP10, SV_MAP11,
        SV_MAPPOS, SV_MIRROR, SV_ORIGIN, SV_PROTOCOL, SV_SETCITEM, SV_SETHP, SV_SETITEM,
        SV_SETVAL0, SV_SETVAL1, SV_TEXT, SV_TICKER,
    };

    use super::*;

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
    fn look_map_payload_hidden_target_matches_legacy_feedback() {
        let payloads = look_map_payloads(
            &World::default(),
            LookMapRequest {
                character_id: CharacterId(7),
                x: 12,
                y: 13,
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
            LookMapRequest {
                character_id: CharacterId(7),
                x: 12,
                y: 13,
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
        assert_eq!(runtime.disconnect(5), Some(CharacterId(1)));
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

    if let Some(database_url) = args.database_url.as_deref() {
        let db = ugaris_db::Database::connect(database_url, 8).await?;
        db.ping().await?;
        info!("connected to PostgreSQL");
    } else {
        warn!("DATABASE_URL not set; starting without persistence");
    }

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
                let _timer_events = world.timers.tick(world.tick.0);
                let due_tasks = world.scheduler.due_tasks(world.tick.0);
                if !due_tasks.is_empty() {
                    info!(count = due_tasks.len(), tick = world.tick.0, "scheduled tasks are due");
                }
                let queued = runtime.drain_actions_for_tick();
                if !queued.is_empty() {
                    info!(count = queued.len(), tick = world.tick.0, "drained queued client actions");
                }
                let setup_count = runtime.setup_world_actions(&mut world, config.area_id);
                if setup_count != 0 {
                    info!(count = setup_count, tick = world.tick.0, "prepared player actions");
                }
                let look_map_requests = world.drain_look_map_requests();
                if !look_map_requests.is_empty() {
                    let mut look_sessions = 0;
                    for request in look_map_requests {
                        let payloads = look_map_payloads(&world, request);
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
                        for request in item_use_requests {
                            match world.use_item_request(request, false) {
                                Ok(ugaris_core::item_driver::UseItemOutcome::OpenContainer { .. })
                                | Ok(ugaris_core::item_driver::UseItemOutcome::OpenDepot { .. }) => {
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
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Teleport { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TeleportDoor { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Recall { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LookItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::KeyringShow { .. } => {
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
                                        ugaris_core::item_driver::ItemDriverOutcome::KeyringAddCursorItem { .. } => {
                                            deferred_templates += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::EmptyPotionTemplateNeeded { .. } => {
                                            deferred_templates += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { .. } => {
                                            blocked += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::Unsupported { .. } => {
                                            unsupported += 1;
                                        }
                                        ugaris_core::item_driver::ItemDriverOutcome::UnsupportedSpecialFood { .. } => {
                                            unsupported += 1;
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
                        info!(opened, executed, unsupported, deferred_templates, blocked, failed, feedback_sessions, tick = world.tick.0, "processed item-use requests");
                    }
                    let mut refreshed_sessions = 0;
                    for completion in &completed_actions {
                        let Some(character) = world.characters.get(&completion.character_id) else {
                            continue;
                        };
                        for (session_id, view_distance) in runtime.sessions_for_character(completion.character_id) {
                            let mut payloads = if completion.ok
                                && completion.action_id == ugaris_core::legacy::action::WALK
                            {
                                movement_scroll_payload(
                                    &world,
                                    character,
                                    completion.old_x,
                                    completion.old_y,
                                    view_distance,
                                )
                                .map(|payload| vec![payload])
                                .unwrap_or_else(|| map_refresh_payloads(&world, character, view_distance))
                            } else {
                                map_refresh_payloads(&world, character, view_distance)
                            };
                            if completion.action_id != ugaris_core::legacy::action::WALK {
                                payloads.push(inventory_snapshot_payload(&world, character));
                            }
                            if runtime.send_many_to_session(session_id, payloads) {
                                refreshed_sessions += 1;
                            }
                        }
                    }
                    if refreshed_sessions != 0 {
                        info!(refreshed_sessions, tick = world.tick.0, "queued map refreshes for completed actions");
                    }
                }
            }
            Some(event) = events_rx.recv() => {
                match event {
                    SessionEvent::Connected { id, peer_addr, commands } => {
                        runtime.connect(id.0, commands, world.tick.0);
                        info!(%id, %peer_addr, "session registered");
                    }
                    SessionEvent::Login { id, login } => {
                        let character_id = runtime.login(id.0, &login, world.tick.0);
                        if !world.characters.contains_key(&character_id) {
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
                                login_bootstrap_payloads(
                                    &world,
                                    character,
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
                                )
                            })
                            .unwrap_or_else(|| {
                                login_bootstrap_payloads(
                                    &world,
                                    &login_character(
                                        character_id,
                                        &login,
                                        config.area_id,
                                        spawn_tile.0,
                                        spawn_tile.1,
                                    ),
                                    config.mirror_id,
                                    world.tick.0,
                                    view_distance,
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
                        if let Some(character_id) = runtime.disconnect(id.0) {
                            world.remove_character(character_id);
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
