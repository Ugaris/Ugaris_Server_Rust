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
    do_action::{can_attack_in_area, can_attack_in_area_with_clan_policy, ClanAttackPolicy},
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode,
        CHARACTER_VALUE_NAMES, POWERSCALE,
    },
    game_settings::GameSettings,
    game_time::{
        GameDate, DAYS_PER_MOON_CYCLE, DAYS_PER_YEAR, FALL_EQUINOX_DAY, HALF_MOON_CYCLE, HOUR_LEN,
        MIN_LEN, SPRING_EQUINOX_DAY, START_TIME, SUMMER_SOLSTICE_DAY,
    },
    ids::{CharacterId, ItemId},
    item_driver::{
        legacy_lucky_die_from_rolls, ForestSpadeFind, IDR_ACCOUNT_DEPOT, IDR_BOOKCASE,
        IDR_DECAYITEM, IDR_DEMONCHIP, IDR_DEMONSHRINE, IDR_FOOD, IDR_KEY_RING, IDR_SPECIAL_POTION,
        IDR_TORCH, IID_AREA17_LIBRARYKEY, IID_AREA2_ZOMBIESKULL1, IID_AREA2_ZOMBIESKULL2,
        IID_AREA2_ZOMBIESKULL3,
    },
    item_ops::{
        consume_item, give_item_to_character, replace_item_in_character, GiveItemFlags,
        GiveItemResult,
    },
    key_registry::{is_registered_key, REGISTERED_KEY_IDS},
    legacy::{action, profession, INVENTORY_START_INVENTORY},
    log_text::{holler_message, sanitize_log_bytes, say_message, shout_message, whisper_message},
    map::{MapFlags, MapTile},
    player::{
        CommandAlias, DemonShrineResult, IgnoreToggleResult, KeyringAddResult, PlayerActionCode,
        PlayerConnectionState, PlayerRuntime, QueuedAction, XmasTreeResult,
    },
    spell::{
        EF_BALL, EF_BLESS, EF_BUBBLE, EF_BURN, EF_CAP, EF_CURSE, EF_EARTHMUD, EF_EARTHRAIN,
        EF_EDEMONBALL, EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE, EF_HEAL, EF_LAG,
        EF_MAGICSHIELD, EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE, EF_WARCRY,
        IDR_ARMOR, IDR_HP, IDR_MANA, IDR_WEAPON,
    },
    tell::tell_not_listening_message,
    text::{
        runtime_color, COL_DARK_GRAY, COL_LIGHT_BLUE, COL_LIGHT_GREEN, COL_LIGHT_RED, COL_ORANGE,
        COL_RESET,
    },
    tick::TICKS_PER_SECOND,
    world::LookMapRequest,
    zone::ZoneLoader,
    ServerConfig, TickRate, World,
};

struct RuntimePlayerAttackPolicy<'a> {
    attacker_runtime: &'a PlayerRuntime,
}

impl ClanAttackPolicy for RuntimePlayerAttackPolicy<'_> {
    fn has_pk_hate(&self, _attacker: &Character, defender: &Character) -> bool {
        self.attacker_runtime.has_pk_hate_for(defender.id.0)
    }
}

fn remove_stale_pvp_hate_if_effect_check_fails(
    player: &mut PlayerRuntime,
    attacker: &Character,
    target: &Character,
    area_id: u16,
) {
    if area_id == 1 {
        return;
    }
    if !attacker.flags.contains(CharacterFlags::PLAYER)
        || !target.flags.contains(CharacterFlags::PLAYER)
        || !attacker.flags.contains(CharacterFlags::PK)
    {
        return;
    }
    if attacker.id == target.id
        || !target.flags.contains(CharacterFlags::PK)
        || attacker.level.abs_diff(target.level) > 3
    {
        player.remove_pk_hate(target.id.0);
    }
}

fn apply_pk_hate_from_hurt_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    realtime_seconds: u64,
) -> usize {
    let mut applied = 0;
    for event in world.drain_legacy_hurt_events() {
        let eligible = match (
            world.characters.get(&event.target_id),
            world.characters.get(&event.cause_id),
        ) {
            (Some(target), Some(cause)) => {
                target.id != cause.id
                    && target
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && cause
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && target.level.abs_diff(cause.level) <= 3
            }
            _ => false,
        };
        if !eligible {
            continue;
        }
        let Some(player) = runtime.player_for_character_mut(event.target_id) else {
            continue;
        };
        let Some(target) = world.characters.get_mut(&event.target_id) else {
            continue;
        };
        player.add_pk_hate_from_hit(target, event.cause_id.0);
        applied += 1;

        if event.outcome.killed {
            if let Some(player) = runtime.player_for_character_mut(event.target_id) {
                player.add_pk_death(realtime_seconds);
            }
            if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
                player.add_pk_kill(realtime_seconds);
            }
        }
    }
    applied
}

fn send_pending_world_system_texts(runtime: &mut ServerRuntime, world: &mut World) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_system_texts() {
        let payload = ugaris_protocol::packet::system_text(&event.message);
        for (session_id, _) in runtime.sessions_for_character(event.character_id) {
            if runtime.send_to_session(session_id, payload.clone()) {
                sent += 1;
            }
        }
    }
    sent
}
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
    dlight_override: i32,
    show_attack: bool,
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

    fn refresh_known_character_name(&mut self, character: &Character) -> Vec<u64> {
        let character_id = client_character_id(character);
        let packet = character_name_packet(character).to_vec();
        let mut sessions = Vec::new();
        for (session_id, cache) in &mut self.map_caches {
            if cache.known_character_names.contains_key(&character_id) {
                cache
                    .known_character_names
                    .insert(character_id, packet.clone());
                sessions.push(*session_id);
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

#[derive(Debug, Clone)]
pub(crate) struct AccountDepotState {
    slots: Vec<Option<Item>>,
}

#[allow(dead_code)]
mod legacy_account_depot_codec {
    use super::*;

    pub(crate) const LEGACY_ACCOUNT_DEPOT_ITEM_SIZE: usize = 232;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX: usize = 224;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET: usize = 0;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_NAME_OFFSET: usize = 8;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET: usize = 48;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET: usize = 128;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET: usize = 132;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET: usize = 133;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET: usize = 134;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET: usize = 136;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET: usize = 140;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET: usize = 150;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET: usize = 168;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET: usize = 170;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET: usize = 172;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET: usize = 212;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET: usize = 216;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET: usize = 220;

    pub(crate) fn write_fixed_c_string(dst: &mut [u8], value: &str) {
        dst.fill(0);
        let bytes = value.as_bytes();
        let len = bytes.len().min(dst.len().saturating_sub(1));
        dst[..len].copy_from_slice(&bytes[..len]);
    }

    pub(crate) fn read_fixed_c_string(src: &[u8]) -> String {
        let len = src.iter().position(|&byte| byte == 0).unwrap_or(src.len());
        String::from_utf8_lossy(&src[..len]).into_owned()
    }

    pub(crate) fn encode_legacy_account_depot_item(
        item: &Item,
    ) -> [u8; LEGACY_ACCOUNT_DEPOT_ITEM_SIZE] {
        let mut bytes = [0u8; LEGACY_ACCOUNT_DEPOT_ITEM_SIZE];
        bytes[LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET..LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET + 8]
            .copy_from_slice(&item.flags.bits().to_le_bytes());
        write_fixed_c_string(
            &mut bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 40],
            &item.name,
        );
        write_fixed_c_string(
            &mut bytes[LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET
                ..LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET + 80],
            &item.description,
        );
        bytes[LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET..LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET + 4]
            .copy_from_slice(&item.value.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET] = item.min_level;
        bytes[LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET] = item.max_level;
        bytes[LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET] = item.needs_class;
        bytes[LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET..LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET + 4]
            .copy_from_slice(&item.owner_id.to_le_bytes());
        for index in 0..ugaris_core::entity::MAX_MODIFIERS {
            let base = LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + index * 2;
            bytes[base..base + 2].copy_from_slice(&item.modifier_index[index].to_le_bytes());
            let base = LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET + index * 2;
            bytes[base..base + 2].copy_from_slice(&item.modifier_value[index].to_le_bytes());
        }
        bytes[LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET..LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET + 2]
            .copy_from_slice(&item.content_id.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET..LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET + 2]
            .copy_from_slice(&item.driver.to_le_bytes());
        let drdata_len = item.driver_data.len().min(40);
        bytes[LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + drdata_len]
            .copy_from_slice(&item.driver_data[..drdata_len]);
        bytes[LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET..LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET + 4]
            .copy_from_slice(&item.template_id.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET..LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET + 4]
            .copy_from_slice(&item.serial.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET..LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET + 4]
            .copy_from_slice(&item.sprite.to_le_bytes());
        bytes
    }

    pub(crate) fn decode_legacy_account_depot_item(bytes: &[u8], slot: usize) -> Option<Item> {
        if bytes.len() < LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX {
            return None;
        }
        let read_u16 = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let read_i16 = |offset: usize| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let read_u32 = |offset: usize| {
            u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        let read_i32 = |offset: usize| {
            i32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        let flags = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let mut modifier_index = [0i16; ugaris_core::entity::MAX_MODIFIERS];
        let mut modifier_value = [0i16; ugaris_core::entity::MAX_MODIFIERS];
        for index in 0..ugaris_core::entity::MAX_MODIFIERS {
            modifier_index[index] = read_i16(LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + index * 2);
            modifier_value[index] = read_i16(LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET + index * 2);
        }
        Some(Item {
            id: ItemId((slot + 1) as u32),
            name: read_fixed_c_string(
                &bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 40],
            ),
            description: read_fixed_c_string(
                &bytes[LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET
                    ..LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET + 80],
            ),
            flags: ItemFlags::from_bits_retain(flags),
            sprite: read_i32(LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET),
            value: read_u32(LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET),
            min_level: bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET],
            max_level: bytes[LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET],
            needs_class: bytes[LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET],
            template_id: read_u32(LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET),
            owner_id: read_i32(LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET),
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: read_u16(LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET),
            driver: read_u16(LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET),
            driver_data: bytes
                [LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + 40]
                .to_vec(),
            serial: read_u32(LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET),
        })
    }

    pub(crate) fn encode_legacy_account_depot_blob(depot: &AccountDepotState) -> Vec<u8> {
        let mut bytes = Vec::new();
        for item in depot.slots.iter().flatten() {
            bytes.extend_from_slice(&encode_legacy_account_depot_item(item));
        }
        bytes
    }

    pub(crate) fn decode_legacy_account_depot_blob(bytes: &[u8]) -> AccountDepotState {
        let mut depot = AccountDepotState::default();
        for (slot, chunk) in bytes
            .chunks_exact(LEGACY_ACCOUNT_DEPOT_ITEM_SIZE)
            .take(depot.slots.len())
            .enumerate()
        {
            depot.slots[slot] = decode_legacy_account_depot_item(chunk, slot);
        }
        depot
    }
}

use legacy_account_depot_codec::{
    decode_legacy_account_depot_blob, encode_legacy_account_depot_blob,
};
#[cfg(test)]
use legacy_account_depot_codec::{
    encode_legacy_account_depot_item, LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET,
    LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET, LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX,
    LEGACY_ACCOUNT_DEPOT_ITEM_SIZE, LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET,
    LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET, LEGACY_ACCOUNT_DEPOT_NAME_OFFSET,
    LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET, LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET,
    LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET,
};

const DRD_ACCOUNT_WIDE_DEPOT: u32 =
    (ugaris_core::player::DEV_ID_ED << 24) | (6 | ugaris_core::player::PERSISTENT_SUBSCRIBER_DATA);

#[derive(Debug, Clone, Copy)]
struct LegacySubscriberBlock<'a> {
    id: u32,
    data: &'a [u8],
}

fn parse_legacy_subscriber_blocks(bytes: &[u8]) -> Option<Vec<LegacySubscriberBlock<'_>>> {
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes.len().saturating_sub(offset) < 8 {
            return None;
        }
        let id = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
        offset += 8;
        if bytes.len().saturating_sub(offset) < size {
            return None;
        }
        blocks.push(LegacySubscriberBlock {
            id,
            data: &bytes[offset..offset + size],
        });
        offset += size;
    }
    Some(blocks)
}

fn write_legacy_subscriber_block(bytes: &mut Vec<u8>, id: u32, data: &[u8]) {
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
}

fn account_depot_has_items(depot: &AccountDepotState) -> bool {
    depot.slots.iter().any(Option::is_some)
}

fn decode_legacy_account_depot_subscriber_blob(bytes: &[u8]) -> Option<AccountDepotState> {
    parse_legacy_subscriber_blocks(bytes)?
        .into_iter()
        .find(|block| block.id == DRD_ACCOUNT_WIDE_DEPOT)
        .map(|block| decode_legacy_account_depot_blob(block.data))
}

fn encode_legacy_account_depot_subscriber_blob(
    existing: &[u8],
    depot: Option<&AccountDepotState>,
) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(existing.len());
    let Some(blocks) = parse_legacy_subscriber_blocks(existing) else {
        return existing.to_vec();
    };
    let mut had_account_depot = false;
    for block in blocks {
        if block.id == DRD_ACCOUNT_WIDE_DEPOT {
            had_account_depot = true;
            if let Some(depot) = depot.filter(|depot| account_depot_has_items(depot)) {
                write_legacy_subscriber_block(
                    &mut encoded,
                    DRD_ACCOUNT_WIDE_DEPOT,
                    &encode_legacy_account_depot_blob(depot),
                );
            }
        } else {
            write_legacy_subscriber_block(&mut encoded, block.id, block.data);
        }
    }
    if !had_account_depot {
        if let Some(depot) = depot.filter(|depot| account_depot_has_items(depot)) {
            write_legacy_subscriber_block(
                &mut encoded,
                DRD_ACCOUNT_WIDE_DEPOT,
                &encode_legacy_account_depot_blob(depot),
            );
        }
    }
    encoded
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

#[derive(Debug, Clone)]
struct CharacterSnapshotApplyResult {
    loaded: bool,
    account_depot: Option<AccountDepotState>,
}

fn apply_character_snapshot(
    world: &mut World,
    player: &mut PlayerRuntime,
    snapshot: CharacterSnapshot,
    fallback_x: usize,
    fallback_y: usize,
    realtime_seconds: u64,
) -> CharacterSnapshotApplyResult {
    let CharacterSnapshot {
        mut character,
        items,
        ppd_blob,
        subscriber_blob,
        ..
    } = snapshot;

    player.ppd_blob = ppd_blob;
    player.subscriber_blob = subscriber_blob;
    let account_depot = decode_legacy_account_depot_subscriber_blob(&player.subscriber_blob);
    let ppd_blob = player.ppd_blob.clone();
    if !ppd_blob.is_empty() && !player.decode_legacy_ppd_blob(&ppd_blob) {
        warn!(
            character_id = character.id.0,
            "failed to decode legacy PPD blob for DB character"
        );
    }
    if player.shutup_until_seconds > realtime_seconds {
        character.flags.insert(CharacterFlags::SHUTUP);
    } else {
        player.shutup_until_seconds = 0;
        character.flags.remove(CharacterFlags::SHUTUP);
    }

    let spawn_x = usize::from(character.x).max(1);
    let spawn_y = usize::from(character.y).max(1);
    if !world.spawn_character(character.clone(), spawn_x, spawn_y) {
        character.x = 0;
        character.y = 0;
        if !world.spawn_character(character, fallback_x, fallback_y) {
            return CharacterSnapshotApplyResult {
                loaded: false,
                account_depot: None,
            };
        }
    }

    for item in items {
        world.add_item(item);
    }
    CharacterSnapshotApplyResult {
        loaded: true,
        account_depot,
    }
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
    account_depot: Option<&AccountDepotState>,
    area_id: u16,
    mirror_id: u16,
) -> CharacterSaveRequest {
    let save_mirror_id = if player.current_mirror_id == 0 {
        mirror_id
    } else {
        player.current_mirror_id
    };
    CharacterSaveRequest {
        character: character.clone(),
        items: character_snapshot_items(world, character),
        ppd_blob: player.encode_legacy_ppd_blob(&player.ppd_blob),
        subscriber_blob: encode_legacy_account_depot_subscriber_blob(
            &player.subscriber_blob,
            account_depot,
        ),
        mode: CharacterSaveMode::Logout {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            allowed_area: i32::from(area_id),
            mirror: i32::from(save_mirror_id),
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
enum ForestSpadeApplyResult {
    Found { item_name: String },
    FoundMoney { amount: u32 },
    AlreadyDug,
    Nothing,
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
    message_bytes: Vec<Vec<u8>>,
    target_message_bytes: Vec<(CharacterId, Vec<u8>)>,
    inventory_changed: bool,
    name_changed: bool,
    name_refresh: Vec<CharacterId>,
}

fn legacy_light_red_text_bytes(message: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(COL_LIGHT_RED.len() + message.len() + COL_RESET.len());
    bytes.extend_from_slice(COL_LIGHT_RED);
    bytes.extend_from_slice(message.as_bytes());
    bytes.extend_from_slice(COL_RESET);
    bytes
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TellCommandResult {
    sender_messages: Vec<String>,
    delivered_messages: Vec<(CharacterId, String)>,
    delivered_message_bytes: Vec<(CharacterId, Vec<u8>)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ChatCommandResult {
    sender_messages: Vec<String>,
    delivered_message_bytes: Vec<(CharacterId, Vec<u8>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalSpeechKind {
    Holler,
    Shout,
    Say,
    Murmur,
    Whisper,
}

impl LocalSpeechKind {
    fn from_verb(verb: &str) -> Option<Self> {
        match verb.to_ascii_lowercase().as_str() {
            "holler" => Some(Self::Holler),
            "shout" => Some(Self::Shout),
            "say" => Some(Self::Say),
            "murmur" => Some(Self::Murmur),
            "whisper" => Some(Self::Whisper),
            _ => None,
        }
    }

    fn max_distance(self, settings: &GameSettings) -> i32 {
        match self {
            Self::Holler => settings.holler_dist,
            Self::Shout => settings.shout_dist,
            Self::Say => settings.say_dist,
            Self::Murmur | Self::Whisper => settings.whisper_dist,
        }
    }

    fn endurance_cost(self, settings: &GameSettings) -> i32 {
        match self {
            Self::Holler => settings.holler_cost,
            Self::Shout => settings.shout_cost,
            Self::Say | Self::Murmur | Self::Whisper => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ChatChannelInfo {
    number: u8,
    name: &'static str,
    description: &'static str,
}

const LEGACY_CHAT_CHANNELS: &[ChatChannelInfo] = &[
    ChatChannelInfo {
        number: 0,
        name: "Announce",
        description: "Announcements from management - NOLEAVE",
    },
    ChatChannelInfo {
        number: 1,
        name: "Info",
        description: "Requesting staff help, technical and gameplay questions",
    },
    ChatChannelInfo {
        number: 2,
        name: "Gossip",
        description: "Talk about Life, the Universe and Everything",
    },
    ChatChannelInfo {
        number: 3,
        name: "Auction",
        description: "Buy and sell stuff",
    },
    ChatChannelInfo {
        number: 4,
        name: "Astonia",
        description: "Other Astonia versions (2.0, 3.5)",
    },
    ChatChannelInfo {
        number: 5,
        name: "Clan",
        description: "Public channel for clan related matters",
    },
    ChatChannelInfo {
        number: 6,
        name: "Grats",
        description: "Grats on leveling!",
    },
    ChatChannelInfo {
        number: 7,
        name: "Clan2",
        description: "Channel only visible to members of your clan",
    },
    ChatChannelInfo {
        number: 8,
        name: "Area",
        description: "Channel only visible to those in your area",
    },
    ChatChannelInfo {
        number: 9,
        name: "Mirror",
        description: "Only visible to those in your area and mirror",
    },
    ChatChannelInfo {
        number: 10,
        name: "Games",
        description: "Discussions of computer games",
    },
    ChatChannelInfo {
        number: 11,
        name: "Kill",
        description: "Playerkiller related topics",
    },
    ChatChannelInfo {
        number: 12,
        name: "ClanA",
        description: "Channel only visible to clan members and allies",
    },
    ChatChannelInfo {
        number: 13,
        name: "Club",
        description: "Channel only visible to your club members",
    },
    ChatChannelInfo {
        number: 14,
        name: "Development",
        description: "Channel only visible to developers",
    },
    ChatChannelInfo {
        number: 31,
        name: "Staff",
        description: "Staff member's private channel",
    },
    ChatChannelInfo {
        number: 32,
        name: "God",
        description: "Ye God's private channel",
    },
];

fn legacy_help_line_bytes(line: &str) -> Vec<u8> {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 16);
    if line.starts_with("===") || line.starts_with("==") || line.starts_with("---") {
        out.extend_from_slice(COL_LIGHT_RED);
        out.extend_from_slice(bytes);
        out.extend_from_slice(COL_RESET);
        return out;
    }
    if line.starts_with("Note:") {
        out.extend_from_slice(COL_ORANGE);
        out.extend_from_slice(bytes);
        out.extend_from_slice(COL_RESET);
        return out;
    }
    if line.starts_with('/') || line.starts_with('#') {
        let split_at = bytes
            .iter()
            .position(|byte| byte.is_ascii_whitespace())
            .unwrap_or(bytes.len());
        out.extend_from_slice(COL_LIGHT_BLUE);
        out.extend_from_slice(&bytes[..split_at]);
        out.extend_from_slice(COL_RESET);
        color_help_parameters(&bytes[split_at..], &mut out);
        return out;
    }
    color_help_parameters(bytes, &mut out);
    out
}

fn color_help_parameters(bytes: &[u8], out: &mut Vec<u8>) {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            if let Some(end) = bytes[index..].iter().position(|byte| *byte == b'>') {
                let end = index + end + 1;
                out.extend_from_slice(COL_LIGHT_GREEN);
                out.extend_from_slice(&bytes[index..end]);
                out.extend_from_slice(COL_RESET);
                index = end;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
}

fn legacy_help_result(messages: Vec<String>) -> KeyringCommandResult {
    let message_bytes = messages
        .iter()
        .map(|message| legacy_help_line_bytes(message))
        .collect();
    KeyringCommandResult {
        messages,
        message_bytes,
        ..Default::default()
    }
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
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
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
    Some(text.to_string())
}

fn legacy_alias_command_verb(verb: &str) -> Option<&'static str> {
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() >= 2 && "alias".starts_with(&lower) {
        return Some("alias");
    }
    if lower == "clearaliases" {
        return Some("clearaliases");
    }
    None
}

fn apply_alias_command(player: &mut PlayerRuntime, command: &str) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = legacy_alias_command_verb(verb)?;
    if verb == "clearaliases" {
        player.aliases.clear();
        return Some(KeyringCommandResult {
            messages: vec!["Done. All gone now.".to_string()],
            ..Default::default()
        });
    }

    let rest = rest.trim_start();
    let mut from_end = rest.len();
    for (index, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            from_end = index;
            break;
        }
    }
    let from = rest[..from_end].chars().take(7).collect::<String>();
    if from.is_empty() {
        let messages = if player.aliases.is_empty() {
            vec!["None defined.".to_string()]
        } else {
            player
                .aliases
                .iter()
                .map(|alias| format!("{} -> {}", alias.from, alias.to))
                .collect()
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    let to = rest[from_end..]
        .trim_start()
        .chars()
        .take(55)
        .collect::<String>();
    if let Some(alias) = player
        .aliases
        .iter_mut()
        .find(|alias| alias.from.eq_ignore_ascii_case(&from))
    {
        if to.is_empty() {
            let old_from = alias.from.clone();
            let old_to = alias.to.clone();
            player
                .aliases
                .retain(|alias| !alias.from.eq_ignore_ascii_case(&from));
            return Some(KeyringCommandResult {
                messages: vec![format!("Erased {old_from} -> {old_to}.")],
                ..Default::default()
            });
        }
        alias.to = to.clone();
        return Some(KeyringCommandResult {
            messages: vec![format!("Replaced {from} -> {to}.")],
            ..Default::default()
        });
    }

    if to.is_empty() {
        return Some(KeyringCommandResult {
            messages: vec![format!("Alias {from} not found, could not delete.")],
            ..Default::default()
        });
    }
    if player.aliases.len() >= ugaris_core::player::ALIAS_MAX_ENTRIES {
        return Some(KeyringCommandResult {
            messages: vec!["Alias memory is full, cannot add.".to_string()],
            ..Default::default()
        });
    }
    player.aliases.push(CommandAlias {
        from: from.clone(),
        to: to.clone(),
    });
    Some(KeyringCommandResult {
        messages: vec![format!("Created {from} -> {to}.")],
        ..Default::default()
    })
}

fn find_online_character_by_name(world: &World, name: &str) -> Option<CharacterId> {
    world
        .characters
        .values()
        .find(|character| character.name.eq_ignore_ascii_case(name))
        .map(|character| character.id)
}

fn take_legacy_alpha_name(text: &str) -> (&str, &str) {
    let end = text
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_alphabetic()).then_some(index))
        .unwrap_or(text.len());
    text.split_at(end)
}

fn pk_hate_prerequisites(source: &Character, target: &Character) -> bool {
    source.id != target.id
        && source
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && target
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && source.level.abs_diff(target.level) <= 3
}

fn legacy_pk_command_verb(verb: &str) -> Option<&'static str> {
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("playerkiller") {
        return Some("playerkiller");
    }
    if verb.eq_ignore_ascii_case("iwilldie") {
        return Some("iwilldie");
    }
    if verb.len() >= 2 && "listhate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("listhate");
    }
    if verb.len() >= 3 && "hate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("hate");
    }
    if verb.len() >= 3 && "nohate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("nohate");
    }
    if verb.eq_ignore_ascii_case("clearhate") {
        return Some("clearhate");
    }
    None
}

fn legacy_atoi_prefix(input: &str) -> i64 {
    let input = input.trim_start();
    let mut chars = input.chars().peekable();
    let sign = match chars.peek().copied() {
        Some('-') => {
            chars.next();
            -1
        }
        Some('+') => {
            chars.next();
            1
        }
        _ => 1,
    };
    let mut value = 0i64;
    let mut seen_digit = false;
    while let Some(ch) = chars.peek().copied() {
        let Some(digit) = ch.to_digit(10) else {
            break;
        };
        seen_digit = true;
        chars.next();
        value = value.saturating_mul(10).saturating_add(i64::from(digit));
    }
    if seen_digit {
        value.saturating_mul(sign)
    } else {
        0
    }
}

fn legacy_color_word(red: i64, green: i64, blue: i64) -> u16 {
    ((red << 10) + (green << 5) + blue) as u16
}

fn parse_legacy_color_triplet(rest: &str) -> [i64; 3] {
    let mut values = [0; 3];
    for (index, part) in rest.split_whitespace().take(3).enumerate() {
        values[index] = legacy_atoi_prefix(part);
    }
    values
}

fn apply_color_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 4 && "color".starts_with(&lower) {
        let character = world.characters.get(&character_id)?;
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "c1={:X}, c2={:X}, c3={:X}",
                character.c1, character.c2, character.c3
            )],
            ..Default::default()
        });
    }

    let color_slot = match lower.as_str() {
        "col1" => Some(1),
        "col2" => Some(2),
        "col3" => Some(3),
        _ => None,
    }?;
    let [red, green, blue] = parse_legacy_color_triplet(rest);
    let color = legacy_color_word(red, green, blue);
    let character = world.characters.get_mut(&character_id)?;
    match color_slot {
        1 => character.c1 = color,
        2 => character.c2 = color,
        3 => character.c3 = color,
        _ => unreachable!(),
    }
    Some(KeyringCommandResult {
        name_changed: true,
        ..Default::default()
    })
}

fn legacy_description_text(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|ch| match ch {
            '"' => '\'',
            '%' => ' ',
            _ => ch,
        })
        .collect::<String>();

    let mut out = String::new();
    for ch in sanitized.chars() {
        if out.len() + ch.len_utf8() >= 160 {
            break;
        }
        out.push(ch);
    }
    out
}

fn apply_description_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"description".starts_with(&lower) {
        return None;
    }

    let text = rest.trim_start();
    if text.is_empty() {
        return Some(KeyringCommandResult {
            messages: vec!["Sorry, you need to enter some text.".to_string()],
            ..Default::default()
        });
    }

    let description = legacy_description_text(text);
    let character = world.characters.get_mut(&character_id)?;
    character.description = description;
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Your description reads now: {}",
            character.description
        )],
        ..Default::default()
    })
}

fn apply_gold_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("ggold") {
        let Some(character) = world.characters.get_mut(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let amount = legacy_atoi_prefix(rest).saturating_mul(100);
        if amount >= 0 {
            let amount = u32::try_from(amount).unwrap_or(u32::MAX);
            character.gold = character.gold.saturating_add(amount);
        } else {
            let amount = u32::try_from(amount.unsigned_abs()).unwrap_or(u32::MAX);
            character.gold = character.gold.saturating_sub(amount);
        }
        character.flags.insert(CharacterFlags::ITEMS);
        return Some(KeyringCommandResult {
            inventory_changed: true,
            ..Default::default()
        });
    }
    if !verb.eq_ignore_ascii_case("gold") {
        return None;
    }

    let Some(amount) = legacy_atoi_prefix(rest).checked_mul(100) else {
        return Some(KeyringCommandResult {
            messages: vec!["Hu?".to_string()],
            ..Default::default()
        });
    };
    if amount < 1 {
        return Some(KeyringCommandResult {
            messages: vec!["Hu?".to_string()],
            ..Default::default()
        });
    }
    let amount = amount as u64;

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if amount > u64::from(character.gold) {
        return Some(KeyringCommandResult {
            messages: vec!["You do not have that much gold.".to_string()],
            ..Default::default()
        });
    }
    if character.cursor_item.is_some() {
        return Some(KeyringCommandResult {
            messages: vec!["Please free your hand (mouse cursor) first.".to_string()],
            ..Default::default()
        });
    }

    let amount = amount as u32;
    if !grant_money_to_cursor(world, loader, character_id, amount) {
        return Some(KeyringCommandResult {
            messages: vec!["Please free your hand (mouse cursor) first.".to_string()],
            ..Default::default()
        });
    }
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.gold = character.gold.saturating_sub(amount);
    }
    Some(KeyringCommandResult {
        inventory_changed: true,
        ..Default::default()
    })
}

fn apply_maxlag_command(player: &mut PlayerRuntime, command: &str) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 4 || !"maxlag".starts_with(&lower) {
        return None;
    }

    let lag = legacy_atoi_prefix(rest);
    if !(3..=20).contains(&lag) {
        return Some(KeyringCommandResult {
            messages: vec!["Number must be between 3 and 20.".to_string()],
            ..Default::default()
        });
    }

    player.set_max_lag_seconds(lag as u8);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Set delay for lag control to kick in to {} seconds.",
            player.max_lag_seconds
        )],
        ..Default::default()
    })
}

fn apply_lag_command(
    world: &mut World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("lag") {
        return None;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let turning_on = !character.flags.contains(CharacterFlags::LAG);
    let in_arena = world
        .map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::ARENA));

    if turning_on && in_arena {
        return Some(KeyringCommandResult {
            messages: vec!["You cannot simulate lag in an arena.".to_string()],
            ..Default::default()
        });
    }
    if turning_on && player.has_any_pk_hate() {
        return Some(KeyringCommandResult {
            messages: vec!["You cannot simulate lag while your hate list is not empty.".to_string()],
            ..Default::default()
        });
    }

    let Some(character) = world.characters.get_mut(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    character.flags.toggle(CharacterFlags::LAG);
    let enabled = character.flags.contains(CharacterFlags::LAG);
    let mut messages = vec![format!(
        "Turned artificial lag {}.",
        if enabled { "on" } else { "off" }
    )];
    if enabled {
        messages.push(
            "PLEASE turn this option off (type /lag again) before you complain about lag!"
                .to_string(),
        );
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

fn apply_status_command(
    character: &Character,
    player: &PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() || !"status".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![
        "Lag Control Settings:".to_string(),
        format!("Max. Lag [/MAXLAG]: {} sec.", player.max_lag_seconds),
    ];

    let has_spell = |value: CharacterValue| character.values[1][value as usize] > 0;
    if has_spell(CharacterValue::Flash) {
        messages.push("Don't use Ball Lightning [/NOBALL]: Off.".to_string());
    }
    if has_spell(CharacterValue::Bless) {
        messages.push("Don't use Bless [/NOBLESS]: Off.".to_string());
    }
    if has_spell(CharacterValue::Fireball) {
        messages.push("Don't use Fireball [/NOFIREBALL]: Off.".to_string());
    }
    if has_spell(CharacterValue::Flash) {
        messages.push("Don't use Lightning Flash [/NOFLASH]: Off.".to_string());
    }
    if has_spell(CharacterValue::Freeze) {
        messages.push("Don't use Freeze [/NOFREEZE]: Off.".to_string());
    }
    if has_spell(CharacterValue::Heal) {
        messages.push("Don't use Heal [/NOHEAL]: Off.".to_string());
    }
    if has_spell(CharacterValue::MagicShield) {
        messages.push("Don't use Magic Shield [/NOSHIELD]: Off.".to_string());
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push("Don't use Pulse [/NOPULSE]: Off.".to_string());
    }
    if has_spell(CharacterValue::Warcry) {
        messages.push("Don't use Warcry [/NOWARCRY]: Off.".to_string());
    }

    messages.extend([
        "Don't use Healing Potions [/NOLIFE]: Off.".to_string(),
        "Don't use Mana Potions [/NOMANA]: Off.".to_string(),
        "Don't use Combo Potions [/NOCOMBO]: Off.".to_string(),
        "Don't use Recall Scroll [/NORECALL]: Off.".to_string(),
        "Don't Move [/NOMOVE]: Off.".to_string(),
        "Automation Settings:".to_string(),
    ]);
    if has_spell(CharacterValue::Bless) {
        messages.push("Automatic Re-Bless [/AUTOBLESS]: Off.".to_string());
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push("Automatic Pulse [/AUTOPULSE]: Off.".to_string());
    }
    messages.extend([
        "Automatic Turning [/AUTOTURN]: Off.".to_string(),
        "Protection Settings:".to_string(),
        format!(
            "Allow others to bless me [/ALLOWBLESS]: {}.",
            if character.flags.contains(CharacterFlags::NOBLESS) {
                "No"
            } else {
                "Yes"
            }
        ),
        "Account Status:".to_string(),
        if character.flags.contains(CharacterFlags::PAID) {
            "Paid Account".to_string()
        } else {
            "Trial Account".to_string()
        },
    ]);

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

fn apply_admin_character_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let Some(character) = world.characters.get_mut(&character_id) else {
        return Some(KeyringCommandResult::default());
    };

    let is_lqmaster = character.flags.contains(CharacterFlags::GOD)
        || character.flags.contains(CharacterFlags::EVENTMASTER)
        || (area_id == 20 && character.flags.contains(CharacterFlags::LQMASTER));
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 4 && "saves".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let saves = legacy_atoi_prefix(rest).clamp(0, i64::from(u8::MAX)) as u8;
        character.saves = saves;
        return Some(KeyringCommandResult::default());
    }

    if lower.len() >= 2 && "immortal".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::IMMORTAL);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Immortal is {}.",
                if character.flags.contains(CharacterFlags::IMMORTAL) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "infrared".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INFRARED);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Infrared is {}.",
                if character.flags.contains(CharacterFlags::INFRARED) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "invisible".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INVISIBLE);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Invisible is {}.",
                if character.flags.contains(CharacterFlags::INVISIBLE) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower == "xray" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::XRAY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned x-ray mode {}.",
                if character.flags.contains(CharacterFlags::XRAY) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "spy".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::SPY);
        let enabled = character.flags.contains(CharacterFlags::SPY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned spy mode {}. You will {} see all tells, clan, alliance, club, area, and mirror chat.",
                if enabled { "on" } else { "off" },
                if enabled { "now" } else { "no longer" }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 6 && "dlight".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.dlight_override = legacy_atoi_prefix(rest) as i32;
        let override_value = (runtime.dlight_override != 0).then_some(runtime.dlight_override);
        world.date = GameDate::calculate(
            START_TIME + world.date.realtime,
            area_id as i32,
            override_value,
        );
        return Some(KeyringCommandResult::default());
    }

    if lower.len() >= 6 && "showattack".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.show_attack = !runtime.show_attack;
        world.show_attack_debug = runtime.show_attack;
        return Some(KeyringCommandResult::default());
    }

    None
}

fn apply_shutup_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("shutup") {
        return None;
    }

    let caller = world.characters.get(&character_id)?;
    if !caller
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return None;
    }

    let rest = rest.trim_start();
    let (name, minute_text) = take_legacy_alpha_name(rest);
    let minutes = if minute_text.trim_start().is_empty() {
        10
    } else {
        legacy_atoi_prefix(minute_text.trim_start())
    };

    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(KeyringCommandResult {
            messages: vec![format!("Sorry, no player by the name {name}.")],
            ..Default::default()
        });
    };
    if !world
        .characters
        .get(&target_id)
        .is_some_and(|target| target.flags.contains(CharacterFlags::PLAYER))
    {
        return Some(KeyringCommandResult {
            messages: vec![format!("Sorry, no player by the name {name}.")],
            ..Default::default()
        });
    }

    if !(0..=60).contains(&minutes) {
        return Some(KeyringCommandResult {
            messages: vec![
                "Sorry, can only shutup for 0 to 60 minutes (use 0 to disable).".to_string(),
            ],
            ..Default::default()
        });
    }

    if let Some(target) = world.characters.get_mut(&target_id) {
        if minutes == 0 {
            target.flags.remove(CharacterFlags::SHUTUP);
        } else {
            target.flags.insert(CharacterFlags::SHUTUP);
        }
    }
    if let Some(target_player) = runtime.player_for_character_mut(target_id) {
        target_player.shutup_until_seconds = if minutes == 0 {
            0
        } else {
            realtime_seconds.saturating_add(minutes as u64 * 60)
        };
    }

    let message = if minutes == 0 {
        "Your ability to talk has been enabled."
    } else {
        "Your ability to talk has been disabled."
    };
    Some(KeyringCommandResult {
        target_message_bytes: vec![(target_id, legacy_light_red_text_bytes(message))],
        ..Default::default()
    })
}

fn drain_expired_shutup_feedback(
    world: &mut World,
    runtime: &mut ServerRuntime,
    realtime_seconds: u64,
) -> Vec<(CharacterId, Vec<u8>)> {
    let mut feedback = Vec::new();
    let expired: Vec<CharacterId> = runtime
        .players
        .values_mut()
        .filter_map(|player| {
            let character_id = player.character_id?;
            (player.shutup_until_seconds != 0 && player.shutup_until_seconds <= realtime_seconds)
                .then(|| {
                    player.shutup_until_seconds = 0;
                    character_id
                })
        })
        .collect();

    for character_id in expired {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.flags.remove(CharacterFlags::SHUTUP);
        }
        feedback.push((
            character_id,
            legacy_light_red_text_bytes("Your ability to talk has been enabled."),
        ));
    }

    feedback
}

fn apply_notells_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"notells".starts_with(&lower) {
        return None;
    }

    let character = world.characters.get_mut(&character_id)?;
    character.flags.toggle(CharacterFlags::NOTELL);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Turned no-tell mode {}.",
            if character.flags.contains(CharacterFlags::NOTELL) {
                "on"
            } else {
                "off"
            }
        )],
        ..Default::default()
    })
}

fn chat_command_verb(command: &str) -> (&str, &str) {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    (verb.trim_start_matches('/').trim_start_matches('#'), rest)
}

fn legacy_chat_channel(number: u8) -> Option<ChatChannelInfo> {
    LEGACY_CHAT_CHANNELS
        .iter()
        .copied()
        .find(|channel| channel.number == number)
}

fn legacy_chat_channel_color(channel: u8) -> u8 {
    match channel {
        0 => 3,
        1 => 12,
        2 => 2,
        3 => 9,
        4 => 14,
        5 => 15,
        6 => 10,
        7 => 16,
        8 => 13,
        9 => 11,
        10 | 11 => 14,
        12 | 13 => 16,
        14 => 11,
        31 => 7,
        32 => 8,
        _ => 2,
    }
}

fn legacy_chat_command_channel(command: &str) -> Option<(u8, &str)> {
    let (verb, rest) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    for channel in LEGACY_CHAT_CHANNELS {
        let alias = format!("c{}", channel.number);
        if alias.starts_with(&lower) || channel.name.to_ascii_lowercase().starts_with(&lower) {
            return Some((channel.number, rest));
        }
    }
    None
}

fn legacy_chat_line(
    sender: &Character,
    mirror: u16,
    channel: ChatChannelInfo,
    text: &str,
) -> Vec<u8> {
    let color = legacy_chat_channel_color(channel.number);
    if channel.number == 0 {
        let mut out = runtime_color(color);
        out.extend_from_slice(text.as_bytes());
        return out;
    }

    let mut sender_name = if sender.flags.contains(CharacterFlags::STAFF) {
        sender.name.to_ascii_uppercase()
    } else {
        sender.name.clone()
    };
    sender_name.truncate(75);

    let player_color = if sender.flags.contains(CharacterFlags::GOD) {
        COL_LIGHT_RED
    } else if sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::EVENTMASTER)
    {
        COL_LIGHT_GREEN
    } else {
        COL_RESET
    };

    let mut out = runtime_color(color);
    out.extend_from_slice(channel.name.as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(player_color);
    out.extend_from_slice(sender_name.as_bytes());
    out.extend_from_slice(&runtime_color(color));
    if sender.flags.contains(CharacterFlags::STAFF) && !sender.flags.contains(CharacterFlags::GOD) {
        out.extend_from_slice(b"[]");
    }
    out.extend_from_slice(b" ");
    if channel.number == 4 {
        out.extend_from_slice(format!("(OW) says: \"{text}\"").as_bytes());
    } else {
        out.extend_from_slice(format!("({mirror}) says: \"{text}\"").as_bytes());
    }
    out
}

fn legacy_spy_line(kind: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(COL_DARK_GRAY.len() + kind.len() + payload.len() + 8);
    out.extend_from_slice(COL_DARK_GRAY);
    out.extend_from_slice(b"[SPY/");
    out.extend_from_slice(kind.as_bytes());
    out.extend_from_slice(b"] ");
    out.extend_from_slice(payload);
    out.extend_from_slice(COL_RESET);
    out
}

fn local_speech_payload(kind: LocalSpeechKind, name: &str, text: &str) -> Option<Vec<u8>> {
    match kind {
        LocalSpeechKind::Holler => holler_message(name, text),
        LocalSpeechKind::Shout => shout_message(name, text),
        LocalSpeechKind::Say => Some(say_message(name, text)),
        LocalSpeechKind::Murmur => (!text.contains('"'))
            .then(|| sanitize_log_bytes(format!("{name} murmurs: \"{text}\"").as_bytes())),
        LocalSpeechKind::Whisper => whisper_message(name, text),
    }
}

fn apply_local_speech_command(
    world: &mut World,
    runtime: &ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
) -> Option<ChatCommandResult> {
    let (verb, raw_text) = chat_command_verb(command);
    let kind = LocalSpeechKind::from_verb(verb)?;
    let settings = GameSettings::default();

    let sender = world.characters.get(&sender_id)?;
    let mut text = raw_text.trim_start();
    if sender.flags.contains(CharacterFlags::SHUTUP) {
        return Some(ChatCommandResult {
            sender_messages: vec!["Sorry, you cannot say anything right now.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let underwater = world
        .map
        .tile(usize::from(sender.x), usize::from(sender.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::UNDERWATER));
    let actual_kind = if underwater {
        text = "Blub.";
        LocalSpeechKind::Say
    } else {
        kind
    };

    let cost = actual_kind.endurance_cost(&settings);
    if cost > 0 && sender.endurance < cost {
        let message = match actual_kind {
            LocalSpeechKind::Holler => "You're too exhausted to holler.",
            LocalSpeechKind::Shout => "You're too exhausted to shout.",
            _ => unreachable!(),
        };
        return Some(ChatCommandResult {
            sender_messages: vec![message.to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let Some(payload) = local_speech_payload(actual_kind, &sender.name, text) else {
        return Some(ChatCommandResult::default());
    };
    let max_distance = actual_kind.max_distance(&settings);
    let sender_x = i32::from(sender.x);
    let sender_y = i32::from(sender.y);

    if cost > 0 {
        if let Some(sender) = world.characters.get_mut(&sender_id) {
            sender.endurance = sender.endurance.saturating_sub(cost);
            sender.regen_ticker = u32::try_from(current_tick).unwrap_or(u32::MAX);
        }
    }

    let mut delivered_message_bytes = Vec::new();
    for player in runtime.players.values() {
        let Some(target_id) = player.character_id else {
            continue;
        };
        let Some(target) = world.characters.get(&target_id) else {
            continue;
        };
        if !target
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::USED)
        {
            continue;
        }
        if (i32::from(target.x) - sender_x).abs() > max_distance
            || (i32::from(target.y) - sender_y).abs() > max_distance
        {
            continue;
        }
        delivered_message_bytes.push((target_id, payload.clone()));
    }

    Some(ChatCommandResult {
        sender_messages: Vec::new(),
        delivered_message_bytes,
    })
}

fn apply_chat_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    area_id: u16,
) -> Option<ChatCommandResult> {
    let (channel_nr, raw_text) = legacy_chat_command_channel(command)?;
    let Some(channel) = legacy_chat_channel(channel_nr) else {
        return None;
    };
    let Some(sender) = world.characters.get(&sender_id) else {
        return Some(ChatCommandResult::default());
    };
    let text = raw_text.trim_start();
    if text.is_empty() {
        return Some(ChatCommandResult {
            sender_messages: vec!["You cannot send empty chat messages.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if text.len() > 200 {
        return Some(ChatCommandResult {
            sender_messages: vec!["This chat message is too long.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    if sender.flags.contains(CharacterFlags::PLAYER)
        && (channel_nr == 0 || channel_nr == 32)
        && !sender.flags.contains(CharacterFlags::GOD)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && channel_nr == 31
        && !sender
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && channel_nr == 14
        && !sender
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::DEVELOPER)
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let sender_runtime = runtime.player_for_character(sender_id);
    if sender.flags.contains(CharacterFlags::PLAYER) && channel_nr != 0 {
        let bit = 1_u32 << (channel_nr - 1);
        if !sender_runtime.is_some_and(|player| player.chat_channels & bit != 0) {
            return Some(ChatCommandResult {
                sender_messages: vec!["You must join a channel before you can use it.".to_string()],
                delivered_message_bytes: Vec::new(),
            });
        }
    }
    if sender.flags.contains(CharacterFlags::PLAYER)
        && (channel_nr == 7 || channel_nr == 12)
        && sender.clan == 0
    {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied - clan members only.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }
    if sender.flags.contains(CharacterFlags::PLAYER) && channel_nr == 13 {
        return Some(ChatCommandResult {
            sender_messages: vec!["Access denied - club members only.".to_string()],
            delivered_message_bytes: Vec::new(),
        });
    }

    let sender_mirror = sender_runtime
        .map(|player| player.current_mirror_id)
        .unwrap_or_default();
    let line = legacy_chat_line(sender, sender_mirror, channel, text);
    let sender_public_id = if sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        0
    } else {
        sender.id.0
    };
    let sender_clan = sender.clan;

    let mut delivered_message_bytes = Vec::new();
    for player in runtime.players.values_mut() {
        let Some(target_id) = player.character_id else {
            continue;
        };
        let Some(target) = world.characters.get(&target_id) else {
            continue;
        };
        if channel_nr == 0 {
            delivered_message_bytes.push((target_id, line.clone()));
            continue;
        }

        let bit = 1_u32 << (channel_nr - 1);
        if player.chat_channels & bit == 0 {
            continue;
        }
        if channel_nr == 14
            && !target
                .flags
                .intersects(CharacterFlags::DEVELOPER | CharacterFlags::GOD)
        {
            player.chat_channels &= !bit;
            continue;
        }
        if channel_nr == 31
            && !target.flags.intersects(
                CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER,
            )
        {
            player.chat_channels &= !bit;
            continue;
        }
        if channel_nr == 32 && !target.flags.contains(CharacterFlags::GOD) {
            player.chat_channels &= !bit;
            continue;
        }
        if sender_public_id != 0 && player.ignores_character(sender_public_id) {
            continue;
        }
        if channel_nr == 7 && target.clan != sender_clan {
            continue;
        }
        if channel_nr == 12 && target.clan != sender_clan {
            continue;
        }
        if channel_nr == 8 && area_id == 0 {
            continue;
        }
        if channel_nr == 9 && (area_id == 0 || player.current_mirror_id != sender_mirror) {
            continue;
        }
        if channel_nr == 13 {
            continue;
        }
        delivered_message_bytes.push((target_id, line.clone()));
    }

    let spy_kind = match channel_nr {
        7 => Some("CLAN"),
        8 => Some("AREA"),
        9 => Some("MIRROR"),
        12 => Some("ALLIANCE"),
        13 => Some("CLUB"),
        _ => None,
    };
    if let Some(spy_kind) = spy_kind {
        let bit = 1_u32 << (channel_nr - 1);
        let spy_line = legacy_spy_line(spy_kind, &line);
        for player in runtime.players.values() {
            let Some(target_id) = player.character_id else {
                continue;
            };
            let Some(target) = world.characters.get(&target_id) else {
                continue;
            };
            if !target
                .flags
                .contains(CharacterFlags::GOD | CharacterFlags::SPY)
            {
                continue;
            }
            let would_see_normally = match channel_nr {
                7 | 12 => player.chat_channels & bit != 0 && target.clan == sender_clan,
                8 => player.chat_channels & bit != 0 && area_id != 0,
                9 => {
                    player.chat_channels & bit != 0
                        && area_id != 0
                        && player.current_mirror_id == sender_mirror
                }
                13 => false,
                _ => false,
            };
            if would_see_normally {
                continue;
            }
            delivered_message_bytes.push((target_id, spy_line.clone()));
        }
    }

    Some(ChatCommandResult {
        sender_messages: Vec::new(),
        delivered_message_bytes,
    })
}

fn apply_channels_command(command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() || !"channels".starts_with(&lower) {
        return None;
    }

    Some(KeyringCommandResult {
        messages: LEGACY_CHAT_CHANNELS
            .iter()
            .map(|channel| {
                format!(
                    "{:>2}: {:<10.10} - {}",
                    channel.number, channel.name, channel.description
                )
            })
            .collect(),
        ..Default::default()
    })
}

fn apply_join_leave_chat_command(
    player: &mut PlayerRuntime,
    character_flags: CharacterFlags,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = chat_command_verb(command);
    let lower = verb.to_ascii_lowercase();
    if lower.len() > "join".len() && "joinall".starts_with(&lower) {
        for nr in 1..=13 {
            player.chat_channels |= 1_u32 << (nr - 1);
        }
        return Some(KeyringCommandResult {
            messages: vec!["You have joined all channels.".to_string()],
            ..Default::default()
        });
    }

    let is_join = !lower.is_empty() && "join".starts_with(&lower);
    let is_leave = !lower.is_empty() && "leave".starts_with(&lower);
    if !is_join && !is_leave {
        return None;
    }

    let nr = legacy_atoi_prefix(rest.trim_start());
    if !(1..=32).contains(&nr) {
        return Some(KeyringCommandResult {
            messages: vec!["Channel number must be between 1 and 32.".to_string()],
            ..Default::default()
        });
    }
    let nr = nr as u8;
    let Some(channel) = legacy_chat_channel(nr) else {
        return Some(KeyringCommandResult {
            messages: vec![format!("Channel number must be between 1 and 32.")],
            ..Default::default()
        });
    };
    let bit = 1_u32 << (nr - 1);

    if is_join {
        if player.chat_channels & bit != 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "You have already joined channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        if nr == 31
            && !character_flags.intersects(
                CharacterFlags::STAFF | CharacterFlags::GOD | CharacterFlags::EVENTMASTER,
            )
        {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Permission denied to join channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        if nr == 32 && !character_flags.contains(CharacterFlags::GOD) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Permission denied to join channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        player.chat_channels |= bit;
        Some(KeyringCommandResult {
            messages: vec![format!(
                "You have joined channel {} ({}).",
                nr, channel.name
            )],
            ..Default::default()
        })
    } else {
        if player.chat_channels & bit == 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "You have already left channel {} ({}).",
                    nr, channel.name
                )],
                ..Default::default()
            });
        }
        player.chat_channels &= !bit;
        Some(KeyringCommandResult {
            messages: vec![format!("You have left channel {} ({}).", nr, channel.name)],
            ..Default::default()
        })
    }
}

fn apply_clearignore_command(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("clearignore") {
        return None;
    }

    let player = runtime.player_for_character_mut(character_id)?;
    player.clear_ignored_characters();
    Some(KeyringCommandResult {
        messages: vec!["Ignore list is now empty.".to_string()],
        ..Default::default()
    })
}

fn apply_ignore_command(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"ignore".starts_with(&lower) {
        return None;
    }

    let rest = rest.trim_start();
    if rest.is_empty() {
        let Some(player) = runtime.player_for_character_mut(character_id) else {
            return Some(KeyringCommandResult::default());
        };
        let mut messages = Vec::new();
        player.ignored_characters.retain(|ignored_id| {
            if let Some(character) = world
                .characters
                .values()
                .find(|character| character.id.0 == *ignored_id)
            {
                messages.push(format!("Ignoring: {}", character.name));
                true
            } else {
                messages.push("Removed deleted char from list.".to_string());
                false
            }
        });
        if messages.is_empty() {
            messages.push("Ignore list is empty.".to_string());
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    let (name, _) = take_legacy_alpha_name(rest);
    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(KeyringCommandResult {
            messages: vec!["No player by that name.".to_string()],
            ..Default::default()
        });
    };
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let result = player.toggle_ignored_character(target_id.0);
    let message = match result {
        IgnoreToggleResult::Added => "Added to ignore list.",
        IgnoreToggleResult::Removed => "Deleted from ignore list.",
        IgnoreToggleResult::Full => "Ignore list is full, cannot add.",
    };
    Some(KeyringCommandResult {
        messages: vec![message.to_string()],
        ..Default::default()
    })
}

fn apply_tell_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
) -> Option<TellCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("tell") {
        return None;
    }

    let rest = rest.trim_start();
    let (name, message) = take_legacy_alpha_name(rest);
    let message = message.trim_start();
    let Some(sender) = world.characters.get(&sender_id) else {
        return Some(TellCommandResult::default());
    };

    let Some(target_id) = find_online_character_by_name(world, name) else {
        return Some(TellCommandResult {
            sender_messages: vec![format!("Sorry, no player by the name {name}.")],
            delivered_messages: Vec::new(),
            delivered_message_bytes: Vec::new(),
        });
    };
    let Some(target) = world.characters.get(&target_id) else {
        return Some(TellCommandResult::default());
    };
    if message.is_empty() {
        return Some(TellCommandResult {
            sender_messages: vec!["Tell, yes, tell it will be, but tell what?".to_string()],
            delivered_messages: Vec::new(),
            delivered_message_bytes: Vec::new(),
        });
    }

    let staffmode = sender
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD);
    if let Some(sender_runtime) = runtime.player_for_character_mut(sender_id) {
        sender_runtime
            .tell_data
            .register_sent_tell(target.id.0, current_tick);
    }

    let mut result = TellCommandResult {
        sender_messages: vec![format!("Told {}: \"{}\"", target.name, message)],
        delivered_messages: Vec::new(),
        delivered_message_bytes: Vec::new(),
    };
    let sender_name = if sender.flags.contains(CharacterFlags::STAFF) {
        sender.name.to_ascii_uppercase()
    } else {
        sender.name.clone()
    };
    let mirror = runtime
        .player_for_character(sender_id)
        .map(|player| player.current_mirror_id)
        .unwrap_or_default();
    let staff_code = if sender.flags.contains(CharacterFlags::STAFF) {
        " []"
    } else {
        ""
    };
    let tell_text = format!(
        "{}{} ({}) tells you: \"{}\"",
        sender_name, staff_code, mirror, message
    );
    let tell_payload = tell_text.as_bytes().to_vec();
    let spy_line = legacy_spy_line("TELL", &tell_payload);
    for player in runtime.players.values() {
        let Some(spy_character_id) = player.character_id else {
            continue;
        };
        if spy_character_id == sender_id || spy_character_id == target_id {
            continue;
        }
        let Some(spy_character) = world.characters.get(&spy_character_id) else {
            continue;
        };
        if spy_character
            .flags
            .contains(CharacterFlags::GOD | CharacterFlags::SPY)
        {
            result
                .delivered_message_bytes
                .push((spy_character_id, spy_line.clone()));
        }
    }

    if target.flags.contains(CharacterFlags::NOTELL) && !staffmode {
        return Some(result);
    }
    if !staffmode
        && runtime
            .player_for_character(target_id)
            .is_some_and(|player| player.ignores_character(sender.id.0))
    {
        return Some(result);
    }
    result.delivered_messages.push((target_id, tell_text));

    if let Some(target_runtime) = runtime.player_for_character_mut(target_id) {
        target_runtime.tell_data.register_received_tell(sender.id.0);
    }
    if target_id == sender_id {
        result
            .sender_messages
            .push("Do you like talking to yourself?".to_string());
    }

    Some(result)
}

fn drain_expired_tell_feedback(
    world: &World,
    runtime: &mut ServerRuntime,
    current_tick: u64,
) -> Vec<(CharacterId, Vec<u8>)> {
    let mut feedback = Vec::new();
    for player in runtime.players.values_mut() {
        let Some(character_id) = player.character_id else {
            continue;
        };
        for target_id in player.tell_data.check_tells(current_tick, TICKS_PER_SECOND) {
            let name = world
                .characters
                .values()
                .find(|character| character.id.0 == target_id)
                .map(|character| character.name.as_str())
                .unwrap_or("Someone");
            feedback.push((character_id, tell_not_listening_message(name)));
        }
    }
    feedback
}

fn apply_nowho_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower != "nowho" {
        return None;
    }

    let character = world.characters.get_mut(&character_id)?;
    if !character
        .flags
        .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return None;
    }

    character.flags.toggle(CharacterFlags::NOWHO);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "NoWho {}.",
            if character.flags.contains(CharacterFlags::NOWHO) {
                "enabled"
            } else {
                "disabled"
            }
        )],
        ..Default::default()
    })
}

fn apply_who_command(
    world: &World,
    caller_flags: CharacterFlags,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() >= 4
        && "whostaff".starts_with(&lower)
        && caller_flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        let mut characters = world.characters.values().collect::<Vec<_>>();
        characters.sort_by_key(|character| character.id.0);

        let mut messages = Vec::new();
        for character in characters {
            if character.flags.contains(CharacterFlags::INVISIBLE) {
                continue;
            }
            if !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if !character
                .flags
                .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
            {
                continue;
            }

            messages.push(format!(
                "{} []{}",
                character.name,
                if character.driver == 0 {
                    ""
                } else {
                    " (lagging)"
                }
            ));
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.is_empty() || !"who".starts_with(&lower) {
        return None;
    }

    let mut characters = world.characters.values().collect::<Vec<_>>();
    characters.sort_by_key(|character| character.id.0);

    let mut messages = vec!["Currently online in this area:".to_string()];
    for character in characters {
        if character.flags.contains(CharacterFlags::INVISIBLE) {
            continue;
        }
        if !character.flags.contains(CharacterFlags::PLAYER) {
            continue;
        }
        if character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
            && character.flags.contains(CharacterFlags::NOWHO)
        {
            continue;
        }

        let arch = if character.flags.contains(CharacterFlags::ARCH) {
            "A"
        } else {
            ""
        };
        let warrior = if character.flags.contains(CharacterFlags::WARRIOR) {
            "W"
        } else {
            ""
        };
        let mage = if character.flags.contains(CharacterFlags::MAGE) {
            "M"
        } else {
            ""
        };
        messages.push(format!(
            "{} ({}{}{}{})",
            character.name, arch, warrior, mage, character.level
        ));
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

fn apply_time_command(date: GameDate, command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 2 || !"time".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![format!(
        "It's {:02}:{:02} on the {}/{}/{}. Sunrise is at {:02}:{:02}, sunset at {:02}:{:02}. Moonrise is at {:02}:{:02}, moonset at {:02}:{:02}.",
        date.hour,
        date.minute,
        date.month + 1,
        date.mday + 1,
        date.year,
        date.sunrise / HOUR_LEN,
        (date.sunrise % HOUR_LEN) / MIN_LEN,
        date.sunset / HOUR_LEN,
        (date.sunset % HOUR_LEN) / MIN_LEN,
        date.moonrise / HOUR_LEN,
        (date.moonrise % HOUR_LEN) / MIN_LEN,
        date.moonset / HOUR_LEN,
        (date.moonset % HOUR_LEN) / MIN_LEN,
    )];

    if !date.fullmoon && !date.newmoon {
        if date.moonsize < 3 {
            messages.push("Quarter Moon.".to_string());
        } else if date.moonsize < 10 {
            messages.push("Half Moon.".to_string());
        } else {
            messages.push("Three Quarter Moon.".to_string());
        }
    }
    if date.newmoon {
        messages.push("Be careful, New Moon tonight!".to_string());
    }
    if date.fullmoon {
        messages.push("It's a fine day, Full Moon tonight!".to_string());
    }
    if date.summer_solstice {
        messages.push("It's a great day, it's Summer Solstice today!".to_string());
    }
    if date.winter_solstice {
        messages.push("It's a scary day, it's Winter Solstice today!".to_string());
    }
    if date.spring_equinox {
        messages.push("Everything is in balance, it's Spring Equinox today!".to_string());
    }
    if date.fall_equinox {
        messages.push("Everything is in balance, it's Fall Equinox today!".to_string());
    }

    if date.moonday < HALF_MOON_CYCLE {
        messages.push(format!(
            "Next full moon is in {} days.",
            HALF_MOON_CYCLE - date.moonday
        ));
    } else {
        messages.push(format!(
            "Next new moon is in {} days.",
            DAYS_PER_MOON_CYCLE - date.moonday
        ));
    }

    if date.yday < SPRING_EQUINOX_DAY {
        messages.push(format!(
            "Spring Equinox will be in {} days.",
            SPRING_EQUINOX_DAY - date.yday
        ));
    } else if date.yday < SUMMER_SOLSTICE_DAY {
        messages.push(format!(
            "Summer Solstice will be in {} days.",
            SUMMER_SOLSTICE_DAY - date.yday
        ));
    } else if date.yday < FALL_EQUINOX_DAY {
        messages.push(format!(
            "Fall Equinox will be in {} days.",
            FALL_EQUINOX_DAY - date.yday
        ));
    } else {
        messages.push(format!(
            "Winter Solstice will be in {} days.",
            DAYS_PER_YEAR - date.yday
        ));
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WeatherState {
    current_weather: i32,
    weather_intensity: usize,
    weather_effects: u32,
    is_transitioning: bool,
    transition_start: u64,
    transition_duration: u64,
    prev_weather: i32,
    weather_change_time: u64,
    affected_areas: &'static [u16],
}

const WEATHER_EFFECT_SLOW: u32 = 0x01;
const WEATHER_EFFECT_BLIND: u32 = 0x02;
const WEATHER_EFFECT_DAMAGE: u32 = 0x04;
const WEATHER_EFFECT_SLIP: u32 = 0x08;
const WEATHER_INTENSITY_NAMES: [&str; 4] = ["None", "Light", "Moderate", "Heavy"];

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current_weather: 0,
            weather_intensity: 0,
            weather_effects: 0,
            is_transitioning: false,
            transition_start: 0,
            transition_duration: 0,
            prev_weather: 0,
            weather_change_time: 0,
            affected_areas: &[],
        }
    }
}

fn weather_name(weather_type: i32) -> &'static str {
    match weather_type {
        0 => "Clear",
        1 => "Rain",
        2 => "Storm",
        3 => "Snow",
        4 => "Sandstorm",
        5 => "Fog",
        _ => "Unknown",
    }
}

fn weather_description(weather_type: i32, intensity: usize) -> &'static str {
    if weather_type == 0 || intensity == 0 {
        return "Clear skies";
    }
    match (weather_type, intensity.min(3)) {
        (1, 1) => "Light rain",
        (1, 2) => "Moderate rain",
        (1, _) => "Heavy rain",
        (2, 1) => "Light storm",
        (2, 2) => "Moderate storm",
        (2, _) => "Heavy storm",
        (3, 1) => "Light snow",
        (3, 2) => "Moderate snow",
        (3, _) => "Heavy snow",
        (4, 1) => "Light sandstorm",
        (4, 2) => "Moderate sandstorm",
        (4, _) => "Heavy sandstorm",
        (5, 1) => "Light fog",
        (5, 2) => "Moderate fog",
        (5, _) => "Heavy fog",
        _ => "Unknown weather",
    }
}

fn apply_weather_command(
    world: &World,
    character_id: CharacterId,
    area_id: u16,
    weather: WeatherState,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("weather") {
        return None;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let mut messages = vec![format!(
        "Current weather in this area: {}",
        weather_description(weather.current_weather, weather.weather_intensity)
    )];

    if character.flags.contains(CharacterFlags::GOD) {
        let intensity = WEATHER_INTENSITY_NAMES
            .get(weather.weather_intensity)
            .copied()
            .unwrap_or("Unknown");
        messages.extend([
            "Global Weather Debug Info:".to_string(),
            format!(
                "- Current Weather: {}",
                weather_name(weather.current_weather)
            ),
            format!("- Intensity: {intensity}"),
            format!("- Effects: 0x{:x}", weather.weather_effects),
        ]);
        if weather.is_transitioning {
            let end = weather
                .transition_start
                .saturating_add(weather.transition_duration);
            let time_left = end.saturating_sub(world.tick.0) / TICKS_PER_SECOND;
            let progress = if weather.transition_duration == 0 {
                100.0
            } else {
                (world.tick.0.saturating_sub(weather.transition_start) as f64
                    / weather.transition_duration as f64)
                    .clamp(0.0, 1.0)
                    * 100.0
            };
            messages.push(format!("- Transitioning: Yes ({time_left} seconds left)"));
            messages.push(format!(
                "- Previous Weather: {}",
                weather_name(weather.prev_weather)
            ));
            messages.push(format!("- Progress: {progress:.1}%"));
        } else {
            messages.push("- Transitioning: No".to_string());
        }
        messages.push(format!(
            "- Next Change: {} seconds",
            weather.weather_change_time.saturating_sub(world.tick.0) / TICKS_PER_SECOND
        ));
        messages.push(format!(
            "- Affected Areas ({}):",
            weather.affected_areas.len()
        ));
        if !weather.affected_areas.is_empty() {
            let mut areas = weather
                .affected_areas
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            areas.push(' ');
            messages.push(format!("  {areas}"));
        }
    }

    let indoors = world
        .map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
    if indoors {
        messages.push("You are indoors and protected from weather effects.".to_string());
    } else if weather.affected_areas.is_empty() || weather.affected_areas.contains(&area_id) {
        if weather.weather_effects & WEATHER_EFFECT_SLOW != 0 {
            messages.push("Movement is affected by the weather.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_BLIND != 0 {
            messages.push("Visibility is reduced by the weather.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_DAMAGE != 0 {
            messages.push("The weather is causing damage.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_SLIP != 0 {
            messages.push("The weather makes the ground slippery.".to_string());
        }
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

fn apply_help_command(
    command: &str,
    flags: CharacterFlags,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("achelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(anti_cheat_help_lines()));
    }
    if verb.eq_ignore_ascii_case("macrohelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(macro_help_lines()));
    }
    if verb.eq_ignore_ascii_case("penthelp") {
        if !flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(pentagram_help_lines()));
    }
    if !verb.eq_ignore_ascii_case("help") {
        return None;
    }

    let mut messages = vec![
        "=== PLAYER COMMANDS ===".to_string(),
        "== Communication Commands ==".to_string(),
        "/holler <text> - Say something with very long range (costs endurance points)".to_string(),
        "/shout <text> - Say something with extended range (costs endurance points)".to_string(),
        "/say <text> - Make your character say text to nearby players".to_string(),
        "/murmur <text> - Say something with reduced range (whisper alternative)".to_string(),
        "/whisper <text> - Say something with very short range".to_string(),
        "/tell <name> <text> - Send a private message to another player".to_string(),
        "/emote <text> - Express an action (Example: /emote jumps shows Player jumps)".to_string(),
        "/me <text> - Same as /emote (Example:  /me smiles  shows Player smiles)".to_string(),
        "== Emote Shortcuts ==".to_string(),
        "/wave - Wave at others (shortcut for /emote waves happily)".to_string(),
        "/bow - Bow to others (shortcut for /emote bows deeply)".to_string(),
        "/eg - Evil grin (shortcut for /emote grins evilly)".to_string(),
        "/slap <name> - Slap someone with a large trout (humorous emote)".to_string(),
        "/hugme - Show that you need a hug (shortcut for /emote is in need of a hug)".to_string(),
        "== Chat Channel Commands ==".to_string(),
        "/channels - List all available chat channels".to_string(),
        "/join <nr> - Join chat channel number <nr>".to_string(),
        "/leave <nr> - Leave chat channel number <nr>".to_string(),
        "/joinall - Join all channels from 1-13 at once".to_string(),
        "/ah - Various auction house commands".to_string(),
        "== Character & Interaction Commands ==".to_string(),
        "/description <text> - Change your character's description".to_string(),
        "/status - Show your lag control settings and account info".to_string(),
        "/time - Show the current game time and date".to_string(),
        "/weather - Display current weather conditions".to_string(),
        "/swap - Swap places with the player you're facing".to_string(),
        "/allow <name> - Allow another player to search your grave if you die".to_string(),
        "/lastseen <player> - Check when a player last logged into the game".to_string(),
        "/showvalues <player> - Show your stats to another player".to_string(),
        "/who - List all players currently in your area".to_string(),
        "/achievements - View your unlocked achievements".to_string(),
        "/achstats - View your achievement statistics".to_string(),
        "== Command Aliases ==".to_string(),
        "/aliases - Show your active command aliases".to_string(),
        "/alias <short> <long> - Create an alias (Example: \"/alias ty Thank you!\")".to_string(),
        "/alias <short> - Remove an existing alias".to_string(),
        "/clearaliases - Delete ALL your command aliases".to_string(),
        "== PvP & Security Commands ==".to_string(),
        "/playerkiller - Toggle player killing mode on/off".to_string(),
        "/iwilldie <id> - Confirm enabling player killer mode".to_string(),
        "/hate <name> - Add player to your PK list (only works in PK mode)".to_string(),
        "/nohate <name> - Remove player from your PK list".to_string(),
        "/listhate - Show all players on your PK list".to_string(),
        "/clearhate - Clear your entire PK list at once".to_string(),
        "/ignore <name> - Ignore a player in chat and tells".to_string(),
        "/clearignore - Remove ALL players from your ignore list".to_string(),
        "/notells - Toggle receiving private messages on/off".to_string(),
        "/complain <player> [reason] - Report abuse or scamming by a player".to_string(),
        "== Inventory & Gold Commands ==".to_string(),
        "/gold <amount> - Move gold coins to your cursor".to_string(),
        "/sort - Sort items in your inventory by value and type".to_string(),
        "/depotsort - Sort the contents of your storage depot".to_string(),
        "/accountdepotsort - Sort your account-wide storage depot".to_string(),
        "/keyring - View keys stored on your keyring".to_string(),
        "/keyring addall - Add all keys from inventory to keyring".to_string(),
        "/keyring remove <n> - Remove key number <n> from keyring".to_string(),
        "== Clan & Club Commands ==".to_string(),
        "/clan - Show information about the clans".to_string(),
        "/relation <nr> - Show clan <nr>'s diplomatic relations".to_string(),
        "/clanpots - Display information about your clan's potions".to_string(),
        "/clanlog - Check the clan logs (/clanlog -h for more details)".to_string(),
        "/club - Show information about clubs".to_string(),
        "== Character Development Commands ==".to_string(),
        "/set <spell nr> <key> - Change spell key mappings".to_string(),
        "/noexp - Toggle gaining experience on/off".to_string(),
        "/nolevel - Toggle preventing level-ups while continuing to earn exp".to_string(),
        "/hints - Toggle game hints on/off".to_string(),
        "/killbless - Remove all Bless effects from your character".to_string(),
        "== Thief-Specific Commands ==".to_string(),
        "/thief - Toggle thief mode on/off (thief characters only)".to_string(),
        "/steal - Attempt to steal an item from the character you're facing".to_string(),
        "== Game Information Commands ==".to_string(),
        "/orbs - Show available orbs and respawn timers".to_string(),
        "/tunnel <level> - Show progress on a specific tunnel level".to_string(),
        "/tunnels - Show list of all tunnel levels and their status".to_string(),
        "/treasures - Show information on treasures (mine chests, etc.)".to_string(),
        "/demonlords - Show information on demon lords and their status".to_string(),
        "== Lag Control Commands ==".to_string(),
        "/lag - Toggle artificial lag (for testing purposes)".to_string(),
        "/maxlag <seconds> - Set delay for lag control to activate (3-20 seconds)".to_string(),
        "/noball - Toggle using Ball Lightning spell during lag".to_string(),
        "/nobless - Toggle using Bless spell during lag".to_string(),
        "/nofireball - Toggle using Fireball spell during lag".to_string(),
        "/noflash - Toggle using Lightning Flash spell during lag".to_string(),
        "/nofreeze - Toggle using Freeze spell during lag".to_string(),
        "/noheal - Toggle using Heal spell during lag".to_string(),
        "/noshield - Toggle using Magic Shield spell during lag".to_string(),
        "/nowarcry - Toggle using Warcry during lag".to_string(),
        "/nopulse - Toggle using Pulse spell during lag".to_string(),
        "/nolife - Toggle using Healing Potions during lag".to_string(),
        "/nomana - Toggle using Mana Potions during lag".to_string(),
        "/nocombo - Toggle using Combo Potions during lag".to_string(),
        "/norecall - Toggle using Recall Scroll during lag".to_string(),
        "/nomove - Toggle character movement during lag".to_string(),
        "== Automation Commands ==".to_string(),
        "/autobless - Toggle automatic re-blessing when spell expires".to_string(),
        "/autoturn - Toggle automatic turning toward enemies".to_string(),
        "/autopulse - Toggle automatic pulse casting".to_string(),
        "/allowbless - Toggle allowing other players to bless you".to_string(),
        "== Miscellaneous Commands ==".to_string(),
        "/logout - Safely log out when standing on a blue square".to_string(),
        "/wimp - Exit from a Live Quest (may have consequences)".to_string(),
        "/help - Display this help text".to_string(),
    ];

    if flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
        messages.extend([
            "=== STAFF COMMANDS ===".to_string(),
            "== Player Management ==".to_string(),
            "/jump <name> <mirror> - Jump to a location or player in specified mirror".to_string(),
            "/look <name> - View a player's character information".to_string(),
            "/values <name> - View a player's stats and values".to_string(),
            "/kick <name> - Disconnect a player from the server".to_string(),
            "/nowho - Hide yourself from /who listings".to_string(),
            "/whostaff - List all staff members online".to_string(),
            "== Disciplinary Actions ==".to_string(),
            "/punish <name> <level> <reason> - Apply punishment to a player".to_string(),
            "/shutup <name> <minutes> - Prevent a player from talking".to_string(),
            "/exterminate <name> - Remove a player from the game".to_string(),
            "/jail <name> - Send a player to jail".to_string(),
            "/unjail <name> - Release a player from jail".to_string(),
            "/klog - Check karma logs".to_string(),
        ]);
    }

    if flags
        .intersects(CharacterFlags::EVENTMASTER | CharacterFlags::LQMASTER | CharacterFlags::GOD)
    {
        messages.push("=== EVENT/QUEST MASTER COMMANDS ===".to_string());
        if flags.contains(CharacterFlags::EVENTMASTER) {
            messages.extend([
                "== Event Master Commands ==".to_string(),
                "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            ]);
        }
        if flags.intersects(CharacterFlags::LQMASTER | CharacterFlags::GOD) {
            messages.extend([
                "== Quest Master Commands ==".to_string(),
                "/immortal - Toggle immortality status".to_string(),
                "/infrared - Toggle infrared vision".to_string(),
                "/invisible - Toggle invisibility".to_string(),
            ]);
            if area_id == 20 || area_id == 35 {
                messages.push(
                    "Note: Additional LQ commands are available in the Live Quest area".to_string(),
                );
            }
        }
    }

    if flags.contains(CharacterFlags::GOD) {
        messages.extend([
            "=== GOD COMMANDS ===".to_string(),
            "== Movement & Teleportation ==".to_string(),
            "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            "/gotolist - List all available goto locations".to_string(),
            "/gotosearch <term> - Search for goto locations".to_string(),
            "/office - Teleport to staff office in Aston".to_string(),
            "/summon <name> - Bring a player to your location".to_string(),
            "/summonall - Bring all online players to your location".to_string(),
            "== Item Management ==".to_string(),
            "/create <name> - Create an item by template name".to_string(),
            "/create_orb [type] [value] - Create an orb with specific properties".to_string(),
            "/itemmod <pos> <skill> <val> - Modify item in cursor (position, skill, value)"
                .to_string(),
            "/itemname <name> - Change name of item in cursor".to_string(),
            "/itemdesc <text> - Change description of item in cursor".to_string(),
            "/listitem <id> - Show detailed information about an item".to_string(),
            "== Player Modification ==".to_string(),
            "/ggold <amount> - Give yourself gold coins".to_string(),
            "/exp [name] [amount] - Give experience to a player".to_string(),
            "/milexp [name] [amount] - Give military experience to a player".to_string(),
            "/setskill <name> <skill> <value> - Set a player's skill level".to_string(),
            "/setlevel <level> - Set your character level".to_string(),
            "/heal - Fully restore your health".to_string(),
            "/setseyan <name> - Make a player a Seyan'Du".to_string(),
            "/rmdeath <name> - Remove one death from player's record".to_string(),
            "/setkarma <name> <value> - Set a player's karma".to_string(),
            "/toggleflag <name> <flag> - Toggles a flag for a character - use with caution"
                .to_string(),
            "/saves <amount> - Set number of saves".to_string(),
            "== Quest & Progress Management ==".to_string(),
            "/resetgift <name> <area> - Reset a player's gift status for an area".to_string(),
            "/fixit <name> - Fix a player's questlog".to_string(),
            "/questfix <name> - Fix quests for a player".to_string(),
            "/reset <name> - Reset a player's skills".to_string(),
            "/noarch <name> - Remove arch status from a player".to_string(),
            "/noprof <name> - Remove professions from a player".to_string(),
            "/questlog <name> - View a player's quest log".to_string(),
            "/labsolved <name> [lab] - Show or toggle lab completion status".to_string(),
            "== Achievements ==".to_string(),
            "/achgive <name> <id> - Award achievement to player".to_string(),
            "/achfix [name] - Recheck and award earned achievements".to_string(),
            "/achclear [name] - Clear all achievements (dev only)".to_string(),
            "/achsync [name] - Force sync achievements to client".to_string(),
            "== Account Management ==".to_string(),
            "/rename <oldname> <newname> - Rename a player character".to_string(),
            "/lockname <name> - Lock a character name".to_string(),
            "/unlockname <name> - Unlock a character name".to_string(),
            "/unpunish <name> <id> - Remove a punishment".to_string(),
            "== Character Information ==".to_string(),
            "/showppd <name> <ppd> - Show player persistent data".to_string(),
            "/showflags <name> - Show which flags are enabled on a character".to_string(),
            "/listchars - List all active characters".to_string(),
            "== God Status Management ==".to_string(),
            "/immortal - Toggle immortality status".to_string(),
            "/invisible - Toggle invisibility".to_string(),
            "/infrared - Toggle infrared vision".to_string(),
            "/xray - Toggle x-ray vision mode".to_string(),
            "/sprite <num> - Change your sprite".to_string(),
            "/color - Show your color values".to_string(),
            "/col1 <r> <g> <b> - Set your primary colors".to_string(),
            "/col2 <r> <g> <b> - Set your secondary colors".to_string(),
            "/col3 <r> <g> <b> - Set your tertiary colors".to_string(),
            "/dlight <value> - Override dynamic lighting".to_string(),
            "/showattack - Toggle attack display".to_string(),
            "/spy - Toggle spy mode (see all tells, clan, alliance, club, area, mirror chat)"
                .to_string(),
            "== Server Management ==".to_string(),
            "/shutdown <minutes> <downtime> - Schedule server shutdown".to_string(),
            "/respawn - Force respawn check".to_string(),
            "/setxmas <value> - Set Christmas special flag".to_string(),
            "/global - Display current global game settings".to_string(),
            "/checksanity - Run consistency checks on game data".to_string(),
            "/saveall - Force save of all player data".to_string(),
            "== Diagnostics & Monitoring ==".to_string(),
            "/memstats - Show memory usage statistics".to_string(),
            "/profinfo - Show profiling information".to_string(),
            "/poolstats - Show database connection pool statistics".to_string(),
            "/querystats - Show database query statistics".to_string(),
            "/prof - Show memory profiling information".to_string(),
            "== Game Settings Management ==".to_string(),
            "/setexpmod <value> - Set global experience modifier".to_string(),
            "/sethardcoreexpbonus <value> - Set hardcore experience bonus".to_string(),
            "/sethardcoremilexpbonus <value> - Set hardcore military exp bonus".to_string(),
            "/sethardcorekillexpbonus <value> - Set hardcore kill exp bonus".to_string(),
            "/setdecaytime <ticks> - Set item decay time".to_string(),
            "/setplayerbodytime <ticks> - Set player body decay time".to_string(),
            "/setnpcbodytime <ticks> - Set NPC body decay time".to_string(),
            "/setnpcbodytimearea32 <ticks> - Set area 32 NPC body decay time".to_string(),
            "/setrespawntime <ticks> - Set NPC respawn time".to_string(),
            "/setlagouttime <ticks> - Set lagout time".to_string(),
            "/setregentime <ticks> - Set regeneration time".to_string(),
            "/setsewerrespawntime <seconds> - Set sewer item respawn time".to_string(),
            "== Communication Settings ==".to_string(),
            "/sethollerdist <tiles> - Set holler distance".to_string(),
            "/setshoutdist <tiles> - Set shout distance".to_string(),
            "/setsaydist <tiles> - Set say distance".to_string(),
            "/setemotedist <tiles> - Set emote distance".to_string(),
            "/setquietsaydist <tiles> - Set quiet say distance".to_string(),
            "/setwhisperdist <tiles> - Set whisper distance".to_string(),
            "/sethollercost <points> - Set holler endurance cost".to_string(),
            "/setshoutcost <points> - Set shout endurance cost".to_string(),
            "== Special Item Settings ==".to_string(),
            "/setsplots <value> - Set special item probability 'lots'".to_string(),
            "/setspmany <value> - Set special item probability 'many'".to_string(),
            "/setspsome <value> - Set special item probability 'some'".to_string(),
            "/setspfew <value> - Set special item probability 'few'".to_string(),
            "/setsprare <value> - Set special item probability 'rare'".to_string(),
            "/setspultra <value> - Set special item probability 'ultra'".to_string(),
            "== Orb & Tunnel Management ==".to_string(),
            "/setorbrespawndays <days> - Set orb respawn time".to_string(),
            "/settunnelexpdivider <value> - Set tunnel exp base value divider".to_string(),
            "/settunnelmillexp <value> - Set tunnel mill exp base value".to_string(),
            "/changetunnel <name> <level> - Change player's tunnel level".to_string(),
            "/settunnel <name> <level> <amount> - Set completion amount for tunnel".to_string(),
            "/cleartunnel <name> <level> - Clear tunnel completion status".to_string(),
            "/solvetunnel <type> - Simulate solving the current tunnel".to_string(),
            "== Shrine & Dungeon Management ==".to_string(),
            "/setrd <name> <number> - Set continuity shrine number".to_string(),
            "/clearrd <name> <number> - Clear used shrine bits".to_string(),
            "/solverd <name> <number> - Mark non-continuity shrines as used".to_string(),
            "== Clan & Club Management ==".to_string(),
            "/killclan <nr> - Destroy a clan".to_string(),
            "/killclub <nr> - Destroy a club".to_string(),
            "/joinclan <nr> - Join a specific clan".to_string(),
            "/joinclub <nr> - Join a specific club".to_string(),
            "/setmaxjewelcount <value> - Set maximum clan jewel count".to_string(),
            "/clearclanlog <clan> - Clear the clan log for a specific clan".to_string(),
            "/setclanjewels <clan> <count> [log] - Set clan jewel count".to_string(),
            "/renclan <nr> <name> - Rename clan with specified number".to_string(),
            "/renclub <nr> <name> - Rename club with specified number".to_string(),
            "== Military Administration ==".to_string(),
            "/milinfo [name] - View a player's military data and mission status".to_string(),
            "/milpref <name> <type> <difficulty> - Set a player's mission preferences".to_string(),
            "/milreset [name] - Reset a player's mission cooldowns and advisor timers".to_string(),
            "/milpoints <name> <points> - Grant military points to a player".to_string(),
            "/milrec <name> <points> - Grant recommendation points to a player".to_string(),
            "/milstats - View statistics about the military system".to_string(),
            "/milsolve [name] [announce] - Complete a player's current military mission"
                .to_string(),
            "== Weather System Management ==".to_string(),
            "/setweather <type> <intensity> - Set global weather".to_string(),
            "/clearweather - Clear weather globally".to_string(),
            "/setareaweather <area> <type> - Set weather for specific area".to_string(),
            "== Player Status Management ==".to_string(),
            "/god <name> - Toggle god status for a player".to_string(),
            "/staff <name> - Toggle staff status for a player".to_string(),
            "/staffcode <name> <code> - Set staff code for a player".to_string(),
            "/qmaster <name> - Toggle quest master status".to_string(),
            "/emaster <name> - Toggle event master status".to_string(),
            "/devel <name> - Toggle developer status".to_string(),
            "/setsir <name> - Toggle sir/lady status".to_string(),
            "/hardcore <name> - Toggle hardcore mode for a player".to_string(),
            "== Miscellaneous God Commands ==".to_string(),
            "/laugh - Play laugh sound effect".to_string(),
            "/ls <name> <file> - List files for a player".to_string(),
            "/cat <name> <file> - View file content for a player".to_string(),
            "/lollipop <name> - Send lollipop to a player".to_string(),
            "/clearmerchantstores <id> - Reset a merchant's inventory".to_string(),
        ]);
    }

    messages.push(
        "Type a command without parameters to get more information in some cases.".to_string(),
    );

    Some(legacy_help_result(messages))
}

fn anti_cheat_help_lines() -> Vec<String> {
    [
        "--- Anti-Cheat Commands ---",
        "#achelp - Show this help",
        "#acstats - Global AC statistics",
        "#aclist - List online players with AC status",
        "#acsuspicious - List suspicious/flagged players",
        "--- Player Commands ---",
        "#acstatus <name> - Show player's AC status",
        "#achistory <name> - Show player's violation history",
        "#acsessions <name> - Show player's recent sessions",
        "#acviolations <name> - Show player's violations",
        "#acflag <name> - Flag player for review",
        "#acunflag <name> - Remove flagged status",
        "#actrust <name> - Mark player as trusted",
        "#acuntrust <name> - Remove trusted status",
        "#acreset <name> - Reset player's AC data (God)",
        "#acwarn <name> [reason] - Issue AC warning",
        "#acwatch <name> - Toggle detailed logging",
        "--- Multi-Account Detection ---",
        "#acsharedip <name> - Show accounts sharing IP",
        "#acsharedhw <name> - Show accounts sharing hardware",
        "--- Database Queries ---",
        "#achighrisk - Show high-risk players",
        "#aclookup <id> - Lookup by subscriber ID",
        "--- Signature Management ---",
        "#acsiglist - List known bad signatures",
        "#acsigadd <type> <value> <name> - Add signature (God)",
        "#acsigdel <id> - Delete signature (God)",
        "--- Maintenance ---",
        "#accleanup <days> - Cleanup old records (God)",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn macro_help_lines() -> Vec<String> {
    [
        "=== Macro Daemon Admin Commands ===",
        "/macrostats <player> - Show player's macro stats",
        "/macrohistory <player> - Show challenge history",
        "/macrolist - List all players with macro status",
        "/summonmacro <player> - Force summon (GOD only)",
        "/macroimmune <player> <mins> - Grant immunity (GOD only)",
        "/macrosuspicion <player> <amt> - Adjust suspicion (GOD)",
        "/macrokarma <player> <val> - Set karma 0-100 (GOD)",
        "/macrofailures <player> <n> - Set failure count (GOD)",
        "/macroreset <player> - Reset all macro stats (GOD)",
        "/macrohelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn pentagram_help_lines() -> Vec<String> {
    [
        "=== Pentagram Debug Commands (GOD) ===",
        "/pentinfo <player> - Show pentagram data",
        "/setpentcount <player> <n> - Set pent_cnt (run count)",
        "/setpentstatus <player> <0|1> - Set status",
        "/setpentbonus <player> <n> - Set bonus exp",
        "/resetpent <player> - Reset all pent data",
        "/penthelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn apply_pk_hate_command(
    world: &mut World,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let Some(verb) = legacy_pk_command_verb(verb) else {
        return None;
    };
    let name = rest.trim();

    match verb {
        "playerkiller" => {
            let mut messages = Vec::new();
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };

            if character.flags.contains(CharacterFlags::PK) {
                if character.action != action::IDLE
                    || world
                        .tick
                        .0
                        .saturating_sub(u64::from(character.regen_ticker))
                        < TICKS_PER_SECOND * 3
                {
                    messages.push("Pant, pant. Too tired.".to_string());
                } else if player.pk_last_kill.saturating_add(60 * 60 * 24 * 28)
                    > realtime_seconds.min(u64::from(u32::MAX)) as u32
                {
                    let elapsed = realtime_seconds.saturating_sub(u64::from(player.pk_last_kill))
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    let remaining = (u64::from(player.pk_last_kill) + 60 * 60 * 24 * 28)
                        .saturating_sub(realtime_seconds)
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    messages.push(format!(
                        "You have killed {elapsed:.2} days ago, you need to wait {remaining:.2} more days."
                    ));
                } else {
                    character.flags.remove(CharacterFlags::PK);
                    player.pk_kills = 0;
                    player.pk_deaths = 0;
                    player.pk_last_kill = 0;
                    player.pk_last_death = 0;
                    player.pk_hate.clear();
                }
            } else if character.level < 10 {
                messages.push(
                    "Sorry, you may not become a player killer before reaching level 10."
                        .to_string(),
                );
            } else if !character.flags.contains(CharacterFlags::PAID) {
                messages.push("Sorry, only paying players may become player killers.".to_string());
            } else {
                messages.push(format!(
                    "Please take a moment to consider this decision. If another player kills you, he will be able to take all your belongings, or kill you over and over again. Do you really want this? Type: '/iwilldie {}' to confirm.",
                    character.id.0
                ));
            }

            let status = if character.flags.contains(CharacterFlags::PK) {
                "on"
            } else {
                "off"
            };
            messages.push(format!("PK is {status}."));
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "iwilldie" => {
            let mut messages = Vec::new();
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };

            if character.flags.contains(CharacterFlags::PK) {
                if character.action != action::IDLE
                    || world
                        .tick
                        .0
                        .saturating_sub(u64::from(character.regen_ticker))
                        < TICKS_PER_SECOND * 3
                {
                    messages.push("Pant, pant. Too tired.".to_string());
                } else if player.pk_last_kill.saturating_add(60 * 60 * 24 * 28)
                    > realtime_seconds.min(u64::from(u32::MAX)) as u32
                {
                    let elapsed = realtime_seconds.saturating_sub(u64::from(player.pk_last_kill))
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    let remaining = (u64::from(player.pk_last_kill) + 60 * 60 * 24 * 28)
                        .saturating_sub(realtime_seconds)
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    messages.push(format!(
                        "You have killed {elapsed:.2} days ago, you need to wait {remaining:.2} more days."
                    ));
                } else {
                    character.flags.remove(CharacterFlags::PK);
                    player.pk_kills = 0;
                    player.pk_deaths = 0;
                    player.pk_last_kill = 0;
                    player.pk_last_death = 0;
                    player.pk_hate.clear();
                }
            } else if character.level < 10 {
                messages.push(
                    "Sorry, you may not become a player killer before reaching level 10."
                        .to_string(),
                );
            } else if !character.flags.contains(CharacterFlags::PAID) {
                messages.push("Sorry, only paying players may become player killers.".to_string());
            } else if name.parse::<u32>().ok() != Some(character.id.0) {
                messages.push("Please type: '/playerkiller' first.".to_string());
            } else {
                player.pk_kills = 0;
                player.pk_deaths = 0;
                player.pk_last_kill = 0;
                player.pk_last_death = 0;
                player.pk_hate.clear();
                character.flags.insert(CharacterFlags::PK);
            }

            let status = if character.flags.contains(CharacterFlags::PK) {
                "on"
            } else {
                "off"
            };
            messages.push(format!("PK is {status}."));
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "listhate" => {
            if !world
                .characters
                .get(&character_id)
                .is_some_and(|character| {
                    character
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                })
            {
                return Some(KeyringCommandResult::default());
            }
            let messages = if !player.has_any_pk_hate() {
                vec!["List is empty.".to_string()]
            } else {
                player
                    .active_pk_hate_ids()
                    .map(|hated_id| {
                        let name = world
                            .characters
                            .get(&CharacterId(hated_id))
                            .map(|character| character.name.as_str())
                            .unwrap_or("Unknown");
                        format!("Hate: {name}")
                    })
                    .collect()
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "clearhate" => {
            if world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.flags.contains(CharacterFlags::PK))
            {
                player.pk_hate.clear();
            }
            Some(KeyringCommandResult {
                messages: vec!["Hate list has been erased.".to_string()],
                inventory_changed: false,
                ..Default::default()
            })
        }
        "hate" => {
            let Some(target_id) = find_online_character_by_name(world, name) else {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Sorry, no one by the name {name} around.")],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let can_add = match (
                world.characters.get(&character_id),
                world.characters.get(&target_id),
            ) {
                (Some(source), Some(target)) => pk_hate_prerequisites(source, target),
                _ => false,
            };
            if can_add {
                player.add_pk_hate(target_id.0);
                if let Some(source) = world.characters.get_mut(&character_id) {
                    source.flags.remove(CharacterFlags::LAG);
                }
                return Some(KeyringCommandResult {
                    name_refresh: vec![character_id, target_id],
                    ..Default::default()
                });
            }
            Some(KeyringCommandResult::default())
        }
        "nohate" => {
            let Some(target_id) = find_online_character_by_name(world, name) else {
                if let Ok(target_id) = name.parse::<u32>() {
                    let removed = world
                        .characters
                        .get(&character_id)
                        .is_some_and(|source| source.flags.contains(CharacterFlags::PK))
                        && player.remove_pk_hate(target_id);
                    let mut name_refresh = Vec::new();
                    if removed {
                        name_refresh.push(character_id);
                    }
                    return Some(KeyringCommandResult {
                        messages: if removed {
                            vec!["Removed from hate list".to_string()]
                        } else {
                            Vec::new()
                        },
                        inventory_changed: false,
                        name_refresh,
                        ..Default::default()
                    });
                }
                return Some(KeyringCommandResult {
                    messages: vec![format!("Sorry, no player by the name {name}.")],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(source) = world.characters.get(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            if !source.flags.contains(CharacterFlags::PK) {
                return Some(KeyringCommandResult::default());
            }
            let removed = player.remove_pk_hate(target_id.0);
            let mut name_refresh = Vec::new();
            let messages = if removed {
                name_refresh.push(character_id);
                name_refresh.push(target_id);
                let target_name = world
                    .characters
                    .get(&target_id)
                    .map(|character| character.name.as_str())
                    .unwrap_or(name);
                vec![format!("Removed {target_name} from hate list")]
            } else {
                Vec::new()
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                name_refresh,
                ..Default::default()
            })
        }
        _ => None,
    }
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
    let lower = command.to_ascii_lowercase();
    let rest = if lower.starts_with("#keyring") || lower.starts_with("/keyring") {
        &command[8..]
    } else {
        return None;
    };
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
            ..Default::default()
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
            ..Default::default()
        });
    }

    let mut words = rest.split_whitespace();
    match words.next().unwrap_or_default().to_ascii_lowercase().as_str() {
        "remove" => {
            let Some(number) = words.next().and_then(|word| word.parse::<usize>().ok()) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Usage: #keyring remove <number>".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(index) = number.checked_sub(1) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(entry) = player.keyring.get(index).cloned() else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            if let Err(message) = give_removed_keyring_entry(world, loader, character_id, &entry) {
                return Some(KeyringCommandResult {
                    messages: vec![message.to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            }
            player.remove_keyring_key_at(index);
            Some(KeyringCommandResult {
                messages: vec![format!("Removed {} from your keyring.", entry.name)],
                inventory_changed: true,
                ..Default::default()
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
                ..Default::default()
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
                    ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            })
        }
        _ => Some(KeyringCommandResult {
            messages: vec!["Unknown keyring subcommand. Use: #keyring, #keyring remove <n>, #keyring addall, #keyring auto".to_string()],
            inventory_changed: false,
            ..Default::default()
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
    let character = world.characters.get(&character_id);
    let name = character
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    let his = legacy_hisname(character);
    let him = legacy_himname(character);

    match kind {
        8 => Some(format!(
            "You see {name} hit {him}self on the head with a mug."
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
        14 => Some(format!("{name} cracks {his} strong knuckles.")),
        15 => Some(format!("{name} starts frothing at the mouth.")),
        _ => None,
    }
}

fn legacy_hisname(character: Option<&Character>) -> &'static str {
    match character.map(|character| character.flags) {
        Some(flags) if flags.contains(CharacterFlags::MALE) => "his",
        Some(flags) if flags.contains(CharacterFlags::FEMALE) => "her",
        _ => "its",
    }
}

fn legacy_himname(character: Option<&Character>) -> &'static str {
    match character.map(|character| character.flags) {
        Some(flags) if flags.contains(CharacterFlags::MALE) => "him",
        Some(flags) if flags.contains(CharacterFlags::FEMALE) => "her",
        _ => "it",
    }
}

fn lollipop_area_message(world: &World, character_id: CharacterId) -> String {
    let name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    format!("{name} licks a lollipop.")
}

fn potion_area_message(world: &World, character_id: CharacterId) -> String {
    let name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    format!("{name} drinks a potion.")
}

fn apply_empty_potion_drink(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    empty_kind: u8,
) -> bool {
    let template = format!("empty_potion{empty_kind}");
    let Ok(mut empty_item) = loader.instantiate_item_template(&template, Some(character_id)) else {
        return false;
    };

    let Some(mut potion) = world.items.remove(&item_id) else {
        return false;
    };
    let Some(character) = world.characters.get_mut(&character_id) else {
        world.items.insert(item_id, potion);
        return false;
    };
    if potion.carried_by != Some(character_id) {
        world.items.insert(item_id, potion);
        return false;
    }

    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;

    character.hp = capped_resource(
        character.hp,
        potion.driver_data.get(1).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Hp),
    );
    character.mana = capped_resource(
        character.mana,
        potion.driver_data.get(2).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Mana),
    );
    character.endurance = capped_resource(
        character.endurance,
        potion.driver_data.get(3).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Endurance),
    );

    if !replace_item_in_character(character, &mut potion, &mut empty_item) {
        character.hp = old_hp;
        character.mana = old_mana;
        character.endurance = old_endurance;
        world.items.insert(item_id, potion);
        return false;
    }
    world.add_item(empty_item);
    true
}

fn capped_resource(current: i32, added_units: u8, max_units: i32) -> i32 {
    (current + i32::from(added_units) * POWERSCALE).min(max_units * POWERSCALE)
}

fn max_character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

fn christmas_pop_inspection_messages() -> [&'static str; 4] {
    [
        "You notice a tiny inscription on the magical lollipop. It reads:",
        "\"Place me under a Christmas tree to receive a special gift from the gods.\"",
        "In shimmering letters below, you see:",
        "\"Each tree grants but one wish per adventurer.\"",
    ]
}

fn is_torch_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_TORCH)
}

fn is_timed_potion_source_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| matches!(item.driver, IDR_BEYONDPOTION | IDR_FLASK))
}

fn is_no_potion_area_blocked_item(world: &World, item_id: ItemId) -> bool {
    world.items.get(&item_id).is_some_and(|item| {
        matches!(
            item.driver,
            ugaris_core::item_driver::IDR_POTION
                | IDR_BEYONDPOTION
                | IDR_SPECIAL_POTION
                | IDR_FLASK
        )
    })
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
        || *driver == ugaris_core::item_driver::IDR_FLASK
        || *driver == ugaris_core::item_driver::IDR_ARKHATA
    {
        let cursor_item = world
            .characters
            .get(character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| world.items.get(&cursor_item_id));
        return ugaris_core::item_driver::ItemDriverContext {
            door_key: None,
            cursor_template_id: cursor_item.map(|item| item.template_id),
            cursor_driver: cursor_item.map(|item| item.driver),
            cursor_sprite: cursor_item.map(|item| item.sprite),
            cursor_drdata0: cursor_item.and_then(|item| item.driver_data.first().copied()),
            hour: world.date.hour as u8,
            fullmoon: world.date.fullmoon,
            newmoon: world.date.newmoon,
            solstice: world.date.solstice,
            equinox: world.date.equinox,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_BONEHINT {
        let needs_init = world
            .items
            .get(item_id)
            .map(|item| item.driver_data.get(1).copied().unwrap_or_default() == 0)
            .unwrap_or(false);
        if needs_init {
            return ugaris_core::item_driver::ItemDriverContext {
                bone_hint_nr: Some((runtime_random_below(25) as f64).sqrt() as u8),
                bone_hint_pos: Some(runtime_random_below(3).max(0) as u8),
                ..ugaris_core::item_driver::ItemDriverContext::default()
            };
        }
        return ugaris_core::item_driver::ItemDriverContext::default();
    }
    if *driver == IDR_BOOKCASE {
        let has_area17_library_key = world.characters.get(character_id).is_some_and(|character| {
            character.inventory.iter().flatten().any(|item_id| {
                world
                    .items
                    .get(item_id)
                    .is_some_and(|item| item.template_id == IID_AREA17_LIBRARYKEY)
            })
        });
        return ugaris_core::item_driver::ItemDriverContext {
            has_area17_library_key,
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

const FOREST_SPADE_DIG_COOLDOWN_SECONDS: u64 = 365 * 24 * 60 * 60;

fn apply_forest_spade_find(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    find: ForestSpadeFind,
    realtime_seconds: u64,
    random_seed: u64,
) -> ForestSpadeApplyResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return ForestSpadeApplyResult::CursorOccupied;
    }

    match find {
        ForestSpadeFind::ForestNote1 => {
            grant_template_item_to_cursor(world, loader, character_id, "forest_note1")
                .map(|item_name| ForestSpadeApplyResult::Found { item_name })
                .unwrap_or(ForestSpadeApplyResult::Nothing)
        }
        ForestSpadeFind::BranningtonTreasure { dig_index } => {
            let Some(player) = player else {
                return ForestSpadeApplyResult::MissingPlayer;
            };
            let last_dig = player.treasure_dig_last_seconds(dig_index);
            if last_dig != 0
                && realtime_seconds.saturating_sub(last_dig) < FOREST_SPADE_DIG_COOLDOWN_SECONDS
            {
                return ForestSpadeApplyResult::AlreadyDug;
            }
            let amount = 100_000 + legacy_random(random_seed, 100_000);
            if !grant_money_to_cursor(world, loader, character_id, amount) {
                return ForestSpadeApplyResult::Nothing;
            }
            if !player.mark_treasure_dig(dig_index, realtime_seconds) {
                return ForestSpadeApplyResult::MissingPlayer;
            }
            ForestSpadeApplyResult::FoundMoney { amount }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PickBerryApplyResult {
    Picked(String),
    NotRipe,
    CursorOccupied,
    MissingPlayer,
    Bug,
}

fn pick_berry_template(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("lizard_brown_berry"),
        2 => Some("picked_flower_h"),
        3 => Some("picked_flower_i"),
        4 => Some("picked_flower_j"),
        _ => None,
    }
}

fn alchemy_flower_template(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("alc_flower1"),
        2 => Some("alc_flower2"),
        3 => Some("alc_flower3"),
        4 => Some("alc_flower4"),
        5 => Some("alc_flower5"),
        6 => Some("alc_flower6"),
        7 => Some("alc_flower7"),
        8 => Some("alc_mushroom1"),
        9 => Some("alc_mushroom2"),
        10 => Some("alc_mushroom3"),
        11 => Some("alc_mushroom4"),
        12 => Some("alc_mushroom5"),
        13 => Some("alc_mushroom6"),
        14 => Some("alc_mushroom7"),
        15 => Some("alc_mushroom8"),
        16 => Some("alc_mushroom9"),
        17 => Some("alc_berry1"),
        18 => Some("alc_berry2"),
        19 => Some("alc_berry3"),
        20 => Some("alc_berry4"),
        _ => None,
    }
}

fn pick_berry_ripe_seconds(character: &Character) -> u64 {
    match character.professions[profession::HERBALIST] {
        value if value >= 30 => 60 * 60 * 4,
        value if value >= 20 => 60 * 60 * 8,
        value if value >= 10 => 60 * 60 * 12,
        _ => 60 * 60 * 24,
    }
}

fn apply_pick_berry(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    kind: u8,
    location_id: u32,
    realtime_seconds: u64,
) -> PickBerryApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return PickBerryApplyResult::CursorOccupied;
    }

    let Some(template) = pick_berry_template(kind) else {
        return PickBerryApplyResult::Bug;
    };

    let ripe_seconds = pick_berry_ripe_seconds(character);
    let Some(player) = player else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if let Some(last_used) = player.flower_last_used_seconds(location_id) {
        if realtime_seconds.saturating_sub(last_used) < ripe_seconds {
            return PickBerryApplyResult::NotRipe;
        }
    }

    let Some(item_name) = grant_template_item_to_cursor(world, loader, character_id, template)
    else {
        return PickBerryApplyResult::CursorOccupied;
    };
    player.mark_flower_used(location_id, realtime_seconds);
    PickBerryApplyResult::Picked(item_name)
}

fn apply_pick_alchemy_flower(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    kind: u8,
    location_id: u32,
    realtime_seconds: u64,
) -> PickBerryApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return PickBerryApplyResult::CursorOccupied;
    }

    let Some(template) = alchemy_flower_template(kind) else {
        return PickBerryApplyResult::Bug;
    };

    let ripe_seconds = pick_berry_ripe_seconds(character);
    let Some(player) = player else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if let Some(last_used) = player.flower_last_used_seconds(location_id) {
        if realtime_seconds.saturating_sub(last_used) < ripe_seconds {
            return PickBerryApplyResult::NotRipe;
        }
    }

    let Some(item_name) = grant_template_item_to_cursor(world, loader, character_id, template)
    else {
        return PickBerryApplyResult::CursorOccupied;
    };
    player.mark_flower_used(location_id, realtime_seconds);
    PickBerryApplyResult::Picked(item_name)
}

fn apply_flask_ingredient_added(
    world: &mut World,
    character_id: CharacterId,
    flask_id: ItemId,
    cursor_item_id: ItemId,
    ingredient_kind: u8,
) -> Option<String> {
    let ingredient_name = world.items.get(&cursor_item_id)?.name.clone();
    let flask = world.items.get_mut(&flask_id)?;
    let size = flask.driver_data.first().copied().unwrap_or_default();
    let used = flask.driver_data.get(1).copied().unwrap_or_default();

    flask.name = "Unfinished Potion".to_string();
    match size {
        1 => {
            flask.sprite = 50204 + i32::from(used);
            flask.description = "A small flask containing some strange liquid.".to_string();
        }
        2 => {
            flask.sprite = 50207 + i32::from(used);
            flask.description = "A flask containing some strange liquid.".to_string();
        }
        3 => {
            flask.sprite = 50243 + i32::from(used);
            flask.description = "A big flask containing some strange liquid.".to_string();
        }
        _ => {}
    }
    if flask.driver_data.len() <= usize::from(ingredient_kind) + 10 {
        flask
            .driver_data
            .resize(usize::from(ingredient_kind) + 11, 0);
    }
    flask.driver_data[1] = flask.driver_data[1].saturating_add(1);
    let ingredient_slot = usize::from(ingredient_kind) + 10;
    flask.driver_data[ingredient_slot] = flask.driver_data[ingredient_slot].saturating_add(1);

    world.items.remove(&cursor_item_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        if character.cursor_item == Some(cursor_item_id) {
            character.cursor_item = None;
        }
        character.flags.insert(CharacterFlags::ITEMS);
    }
    Some(ingredient_name)
}

const ALCHEMY_INGREDIENT_NAMES: [&str; 29] = [
    "Adygalah",
    "Bhalkissa",
    "Chrysado",
    "Domari",
    "Elithah",
    "Firuba",
    "Ghethiye",
    "Akond",
    "Barun",
    "Chylmoth",
    "Dizul",
    "Edyak",
    "Forud",
    "Ghestroz",
    "Hangot",
    "Ivnan",
    "Azmey",
    "Beelough",
    "Ciuba",
    "Dyelshi",
    "Fiery Stone",
    "Icy Stone",
    "Earth Stone",
    "Hell Stone",
    "",
    "",
    "",
    "",
    "",
];

fn flask_ingredient_feedback(ingredient_counts: [u8; 29]) -> Vec<String> {
    ALCHEMY_INGREDIENT_NAMES
        .iter()
        .enumerate()
        .filter_map(|(idx, name)| {
            let count = ingredient_counts[idx];
            if count == 0 || name.is_empty() {
                None
            } else {
                Some(format!("Contains {count} parts {name}."))
            }
        })
        .collect()
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

fn legacy_nomad_dice_roll(seed: u64, luck: u8) -> ([u8; 3], u8) {
    let mut next_roll = 0_u64;
    let mut roll_die = || {
        let rolls = (0..=luck).map(|_| {
            let roll = legacy_random(seed.wrapping_add(next_roll), 6) as u8 + 1;
            next_roll = next_roll.wrapping_add(1);
            roll
        });
        legacy_lucky_die_from_rolls(6, luck, rolls)
    };

    let dice = [roll_die(), roll_die(), roll_die()];
    let total = dice.iter().copied().sum();
    (dice, total)
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ZombieShrineApplyResult {
    NeedsOffering(u8),
    Gift(String),
    Experience(u32),
    Bonus {
        message: &'static str,
        driver: u16,
        strength: i32,
        duration_ticks: i32,
    },
    MissingGift,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArkhataPoolApplyResult {
    Gift(String),
    Vanished,
    MissingGift,
    MissingPlayer,
    MissingCursor,
}

fn zombie_shrine_required_skull(shrine_type: u8) -> u32 {
    match shrine_type {
        0 => IID_AREA2_ZOMBIESKULL1,
        1 => IID_AREA2_ZOMBIESKULL2,
        _ => IID_AREA2_ZOMBIESKULL3,
    }
}

fn zombie_shrine_offering_message(shrine_type: u8) -> &'static str {
    match shrine_type {
        0 => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone.",
        1 => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone and silver.",
        _ => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone and gold.",
    }
}

fn zombie_shrine_reward_template(
    shrine_type: u8,
    roll: u32,
    flags: CharacterFlags,
) -> Option<&'static str> {
    match shrine_type {
        0 => match roll {
            0 | 1 | 20 | 21 => Some("zombie_skull2"),
            2..=9 => Some("torch"),
            10 | 11 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion1"
            } else {
                "healing_potion1"
            }),
            12 | 13 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion1"
            } else {
                "mana_potion1"
            }),
            _ => None,
        },
        1 => match roll {
            0 | 1 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion2"
            } else {
                "healing_potion2"
            }),
            2 | 11 | 12 => Some("zombie_skull3"),
            3 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion2"
            } else {
                "mana_potion2"
            }),
            _ => None,
        },
        _ => match roll {
            0 | 1 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion3"
            } else {
                "healing_potion3"
            }),
            2 | 3 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion3"
            } else {
                "mana_potion3"
            }),
            _ => None,
        },
    }
}

fn zombie_shrine_experience(shrine_type: u8, roll: u32) -> Option<u32> {
    match shrine_type {
        0 if roll == 14 || roll == 15 => Some(250),
        1 if (4..=6).contains(&roll) => Some(750),
        2..=u8::MAX if (4..=6).contains(&roll) => Some(2250),
        _ => None,
    }
}

fn zombie_shrine_bonus(
    shrine_type: u8,
    roll: u32,
    flags: CharacterFlags,
) -> Option<(&'static str, u16, i32, i32)> {
    match shrine_type {
        0 => match roll {
            16 => Some((
                "You have been protected for a short while.",
                IDR_ARMOR,
                5 * 20,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            17 => Some((
                "You are more dangerous for a short while.",
                IDR_WEAPON,
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            18 => Some((
                "Your capacity was increased for a short while.",
                if flags.contains(CharacterFlags::WARRIOR) {
                    IDR_HP
                } else {
                    IDR_MANA
                },
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            19 => Some((
                "Your capacity was increased for a short while.",
                if flags.contains(CharacterFlags::MAGE) {
                    IDR_MANA
                } else {
                    IDR_HP
                },
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            _ => None,
        },
        1 => match roll {
            7 => Some((
                "You have been protected for a while.",
                IDR_ARMOR,
                10 * 20,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            8 => Some((
                "You are more dangerous for a while.",
                IDR_WEAPON,
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            9 => Some((
                "Your capacity was increased for a while.",
                if flags.contains(CharacterFlags::WARRIOR) {
                    IDR_HP
                } else {
                    IDR_MANA
                },
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            10 => Some((
                "Your capacity was increased for a while.",
                if flags.contains(CharacterFlags::MAGE) {
                    IDR_MANA
                } else {
                    IDR_HP
                },
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            _ => None,
        },
        _ => None,
    }
}

fn apply_zombie_shrine(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    shrine_type: u8,
    random_seed: u64,
) -> ZombieShrineApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return ZombieShrineApplyResult::MissingPlayer;
    };
    let Some(cursor_item_id) = character.cursor_item else {
        return ZombieShrineApplyResult::NeedsOffering(shrine_type);
    };
    if world
        .items
        .get(&cursor_item_id)
        .is_none_or(|item| item.template_id != zombie_shrine_required_skull(shrine_type))
    {
        return ZombieShrineApplyResult::NeedsOffering(shrine_type);
    }
    let character_flags = character.flags;

    let Some(character) = world.characters.get_mut(&character_id) else {
        return ZombieShrineApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    world.items.remove(&cursor_item_id);

    let roll_max = match shrine_type {
        0 => 22,
        1 => 13,
        _ => 7,
    };
    let roll = legacy_random(random_seed, roll_max);
    if let Some(template) = zombie_shrine_reward_template(shrine_type, roll, character_flags) {
        return match grant_template_item_to_cursor(world, loader, character_id, template) {
            Some(item_name) => ZombieShrineApplyResult::Gift(item_name),
            None => ZombieShrineApplyResult::MissingGift,
        };
    }
    if let Some(exp_added) = zombie_shrine_experience(shrine_type, roll) {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.exp = character.exp.saturating_add(exp_added);
        }
        return ZombieShrineApplyResult::Experience(exp_added);
    }
    if let Some((message, driver, strength, duration_ticks)) =
        zombie_shrine_bonus(shrine_type, roll, character_flags)
    {
        world.install_bonus_spell(character_id, driver, strength, duration_ticks);
        return ZombieShrineApplyResult::Bonus {
            message,
            driver,
            strength,
            duration_ticks,
        };
    }

    ZombieShrineApplyResult::MissingGift
}

fn apply_arkhata_pool(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    cursor_item_id: ItemId,
    random_seed: u64,
) -> ArkhataPoolApplyResult {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return ArkhataPoolApplyResult::MissingPlayer;
    };
    if character.cursor_item != Some(cursor_item_id) {
        return ArkhataPoolApplyResult::MissingCursor;
    }
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    world.items.remove(&cursor_item_id);

    let template = match legacy_random(random_seed, 70) {
        22 | 33 => Some("Red_Scroll"),
        42 => Some("Buddah_Statue"),
        _ => None,
    };
    let Some(template) = template else {
        return ArkhataPoolApplyResult::Vanished;
    };
    match grant_template_item_smart(world, loader, character_id, template) {
        Some(item_name) => ArkhataPoolApplyResult::Gift(item_name),
        None => ArkhataPoolApplyResult::MissingGift,
    }
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

const XMAS_TREE_GIFT_GODS: [&str; 3] = ["Eddow", "Freya", "Sauron"];
const XMAS_MAX_SKILLS: usize = 3;
const XMAS_MAX_SKILL_VALUE: i16 = 20;
const XMAS_SPECIAL_MAX_VALUE: i16 = 20;
const XMAS_ENHANCE_SKILLS: [CharacterValue; 35] = [
    CharacterValue::Hp,
    CharacterValue::Endurance,
    CharacterValue::Mana,
    CharacterValue::Wisdom,
    CharacterValue::Intelligence,
    CharacterValue::Agility,
    CharacterValue::Strength,
    CharacterValue::Light,
    CharacterValue::Speed,
    CharacterValue::Pulse,
    CharacterValue::Dagger,
    CharacterValue::Hand,
    CharacterValue::Staff,
    CharacterValue::Sword,
    CharacterValue::TwoHand,
    CharacterValue::Attack,
    CharacterValue::Parry,
    CharacterValue::Warcry,
    CharacterValue::Tactics,
    CharacterValue::Surround,
    CharacterValue::BodyControl,
    CharacterValue::SpeedSkill,
    CharacterValue::Barter,
    CharacterValue::Percept,
    CharacterValue::Stealth,
    CharacterValue::Bless,
    CharacterValue::Heal,
    CharacterValue::Freeze,
    CharacterValue::MagicShield,
    CharacterValue::Flash,
    CharacterValue::Fireball,
    CharacterValue::Regenerate,
    CharacterValue::Meditate,
    CharacterValue::Immunity,
    CharacterValue::Duration,
];

#[derive(Debug, Clone)]
struct XmasTreeRng {
    state: u64,
}

impl XmasTreeRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn random(&mut self, limit: usize) -> usize {
        if limit == 0 {
            return 0;
        }
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.state >> 32) as usize) % limit
    }
}

fn random_xmas_skill_value(rng: &mut XmasTreeRng) -> i16 {
    let mut value = rng.random((XMAS_MAX_SKILL_VALUE / 2 + 1) as usize) as i16;
    if rng.random(100) < 30 {
        value += rng.random((XMAS_MAX_SKILL_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 10 {
        value += rng.random((XMAS_MAX_SKILL_VALUE / 4) as usize) as i16;
    }
    value.min(XMAS_MAX_SKILL_VALUE)
}

fn random_xmas_special_value(rng: &mut XmasTreeRng) -> i16 {
    let mut value = rng.random((XMAS_SPECIAL_MAX_VALUE / 2 + 1) as usize) as i16;
    if rng.random(100) < 20 {
        value += rng.random((XMAS_SPECIAL_MAX_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 10 {
        value += rng.random((XMAS_SPECIAL_MAX_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 5 {
        value = XMAS_SPECIAL_MAX_VALUE;
    }
    value.min(XMAS_SPECIAL_MAX_VALUE)
}

fn enhance_xmas_item(item: &mut Item, rng: &mut XmasTreeRng) {
    item.modifier_index.fill(0);
    item.modifier_value.fill(0);

    let mut available = XMAS_ENHANCE_SKILLS.to_vec();
    let num_skills = (rng.random(XMAS_MAX_SKILLS) + 1).min(item.modifier_index.len());
    let mut immunity_selected = false;

    for slot in 0..num_skills.min(XMAS_MAX_SKILLS) {
        if available.is_empty() {
            break;
        }
        let selected = rng.random(available.len());
        let skill = available.swap_remove(selected);
        if skill == CharacterValue::Immunity {
            immunity_selected = true;
        }
        let value = random_xmas_skill_value(rng);
        if value > 0 {
            item.modifier_index[slot] = skill as i16;
            item.modifier_value[slot] = value;
        }
    }

    if !immunity_selected && num_skills < item.modifier_index.len() && num_skills < XMAS_MAX_SKILLS
    {
        let special = random_xmas_special_value(rng);
        if special > 0 {
            item.modifier_index[num_skills] = CharacterValue::Immunity as i16;
            item.modifier_value[num_skills] = special;
        }
    }
}

fn grant_xmas_tree_gift(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    seed: u64,
) -> Option<String> {
    let mut rng = XmasTreeRng::new(seed);
    let template = XMAS_TREE_GIFT_TEMPLATES[(seed as usize) % XMAS_TREE_GIFT_TEMPLATES.len()];
    let recipient_name = world.characters.get(&character_id)?.name.clone();
    let mut item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    enhance_xmas_item(&mut item, &mut rng);
    let god = XMAS_TREE_GIFT_GODS[rng.random(XMAS_TREE_GIFT_GODS.len())];
    item.description =
        format!("To {recipient_name}, with holiday blessings from {god}.\nMerry Christmas!");
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
            let Some(item_name) = grant_xmas_tree_gift(world, loader, character_id, gift_seed)
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

fn apply_caligar_key_final(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_item_id: ItemId,
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

    let Ok(new_item) =
        loader.instantiate_item_template("caligar_palace_chest_key", Some(character_id))
    else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    let new_item_id = new_item.id;

    world.items.remove(&cursor_item_id);
    world.items.remove(&item_id);
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = Some(new_item_id);
    character.inventory[slot] = None;
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
        return NomadStackApplyResult::Bug(
            if world
                .items
                .get(&item_id)
                .is_some_and(|item| item.driver == IDR_DEMONCHIP)
            {
                "Bug #1445y"
            } else {
                "Bug #1442y"
            },
        );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TransportDestination {
    name: &'static str,
    x: u16,
    y: u16,
    area: u16,
}

const LEGACY_TRANSPORT_DESTINATIONS: [TransportDestination; 26] = [
    TransportDestination {
        name: "Cameron",
        x: 139,
        y: 75,
        area: 1,
    },
    TransportDestination {
        name: "Chapel",
        x: 139,
        y: 75,
        area: 1,
    },
    TransportDestination {
        name: "Aston",
        x: 129,
        y: 201,
        area: 3,
    },
    TransportDestination {
        name: "Tribe of the Isara",
        x: 239,
        y: 249,
        area: 6,
    },
    TransportDestination {
        name: "Tribe of the Cerasa",
        x: 92,
        y: 164,
        area: 6,
    },
    TransportDestination {
        name: "Maze of the Cerasa",
        x: 49,
        y: 135,
        area: 6,
    },
    TransportDestination {
        name: "Defense Tunnels of the Cerasa",
        x: 14,
        y: 114,
        area: 6,
    },
    TransportDestination {
        name: "Zalina Entrance",
        x: 5,
        y: 4,
        area: 6,
    },
    TransportDestination {
        name: "Tribe of the Zalina",
        x: 172,
        y: 36,
        area: 6,
    },
    TransportDestination {
        name: "Teufelheim",
        x: 225,
        y: 249,
        area: 34,
    },
    TransportDestination {
        name: "Aston Mines",
        x: 57,
        y: 124,
        area: 3,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "Ice 1",
        x: 93,
        y: 102,
        area: 10,
    },
    TransportDestination {
        name: "Ice 2",
        x: 11,
        y: 113,
        area: 10,
    },
    TransportDestination {
        name: "Ice 3",
        x: 241,
        y: 87,
        area: 10,
    },
    TransportDestination {
        name: "Ice 4",
        x: 213,
        y: 156,
        area: 11,
    },
    TransportDestination {
        name: "Ice 5",
        x: 189,
        y: 80,
        area: 11,
    },
    TransportDestination {
        name: "Nomad Plains",
        x: 16,
        y: 124,
        area: 19,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "Forest",
        x: 181,
        y: 117,
        area: 16,
    },
    TransportDestination {
        name: "Exkordon",
        x: 65,
        y: 106,
        area: 17,
    },
    TransportDestination {
        name: "Brannington",
        x: 202,
        y: 226,
        area: 29,
    },
    TransportDestination {
        name: "Grimroot",
        x: 210,
        y: 246,
        area: 31,
    },
    TransportDestination {
        name: "Caligar",
        x: 230,
        y: 62,
        area: 36,
    },
    TransportDestination {
        name: "Arkhata",
        x: 28,
        y: 20,
        area: 37,
    },
];

const LEGACY_TRANSPORT_CLAN_DESTINATIONS: [TransportDestination; 32] = [
    TransportDestination {
        name: "Clan1",
        x: 28,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan2",
        x: 59,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan3",
        x: 90,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan4",
        x: 121,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan5",
        x: 152,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan6",
        x: 183,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan7",
        x: 214,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan8",
        x: 245,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan9",
        x: 28,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan10",
        x: 59,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan11",
        x: 90,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan12",
        x: 121,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan13",
        x: 152,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan14",
        x: 183,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan15",
        x: 214,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan16",
        x: 245,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan17",
        x: 28,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan18",
        x: 59,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan19",
        x: 90,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan20",
        x: 121,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan21",
        x: 152,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan22",
        x: 183,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan23",
        x: 28,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan24",
        x: 59,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan25",
        x: 90,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan26",
        x: 121,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan27",
        x: 152,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan28",
        x: 183,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan29",
        x: 28,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan30",
        x: 59,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan31",
        x: 90,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan32",
        x: 28,
        y: 231,
        area: 3,
    },
];

fn may_enter_clan(character: &Character, clan: u16) -> bool {
    (1..=32).contains(&clan) && character.clan == clan
}

fn transport_clan_access(world: &World, character_id: CharacterId) -> [u8; 4] {
    let Some(character) = world.characters.get(&character_id) else {
        return [0; 4];
    };
    let mut access = [0_u8; 4];
    for clan in 1..=32_u16 {
        if may_enter_clan(character, clan) {
            let index = (clan - 1) as usize;
            access[index / 8] |= 1_u8 << (index % 8);
        }
    }
    access
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransportTravelResult {
    SameArea {
        x: u16,
        y: u16,
        mirror: u32,
    },
    CrossArea {
        area: u16,
        x: u16,
        y: u16,
        mirror: u32,
    },
    Busy,
    Blocked(String),
    Bug(String),
}

fn resolve_transport_travel(
    world: &World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
) -> TransportTravelResult {
    resolve_transport_travel_with_random(
        world,
        player,
        character_id,
        current_area,
        spec,
        runtime_random_below,
    )
}

fn runtime_random_below(max: i32) -> i32 {
    if max <= 0 {
        return 0;
    }

    legacy_random(runtime_random_seed(), max as u32) as i32
}

fn runtime_random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default()
}

fn legacy_questlog_payload(player: &PlayerRuntime) -> bytes::BytesMut {
    let mut quest_bytes = Vec::with_capacity(ugaris_protocol::packet::QUESTLOG_QUEST_COUNT);
    for entry in player
        .quest_log
        .entries()
        .iter()
        .take(ugaris_protocol::packet::QUESTLOG_QUEST_COUNT)
    {
        quest_bytes.push((entry.done & 0x3f) | ((entry.flags & 0x03) << 6));
    }

    ugaris_protocol::packet::questlog(&quest_bytes, &[])
}

fn resolve_transport_travel_with_random(
    world: &World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
    mut random_below: impl FnMut(i32) -> i32,
) -> TransportTravelResult {
    let nr = (spec & 255) - 1;
    let mirror = match spec / 256 {
        1..=26 => (spec / 256) as u32,
        _ => (random_below(26).clamp(0, 25) + 1) as u32,
    };

    if (64..96).contains(&nr) {
        let clan = (nr - 63) as u16;
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| may_enter_clan(character, clan))
        {
            return TransportTravelResult::Blocked(format!("You may not enter ({}).", clan));
        }
        let destination = LEGACY_TRANSPORT_CLAN_DESTINATIONS[(clan - 1) as usize];
        if destination.area != current_area {
            return TransportTravelResult::CrossArea {
                area: destination.area,
                x: destination.x,
                y: destination.y,
                mirror,
            };
        }
        return TransportTravelResult::SameArea {
            x: destination.x,
            y: destination.y,
            mirror,
        };
    }

    if !(0..64).contains(&nr) {
        return TransportTravelResult::Bug("You've confused me. (BUG #1123)".to_string());
    }

    let point = nr as usize;
    let bit = 1_u64 << point;
    let Some(destination) = LEGACY_TRANSPORT_DESTINATIONS.get(point).copied() else {
        return TransportTravelResult::Bug(format!("Nothing happens - BUG ({nr},#2)."));
    };
    if player.transport_seen & bit == 0 {
        return TransportTravelResult::Blocked(format!(
            "You've never been to {} before. You cannot go there.",
            destination.name
        ));
    }
    if point == 22
        && !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::ARCH))
    {
        return TransportTravelResult::Blocked("Sorry, Arches only!".to_string());
    }
    if destination.x < 1 || destination.x > 254 || destination.y < 1 || destination.y > 254 {
        return TransportTravelResult::Bug(format!(
            "Nothing happens - BUG ({},{},{}).",
            destination.x, destination.y, destination.area
        ));
    }
    if destination.area != current_area {
        return TransportTravelResult::CrossArea {
            area: destination.area,
            x: destination.x,
            y: destination.y,
            mirror,
        };
    }
    TransportTravelResult::SameArea {
        x: destination.x,
        y: destination.y,
        mirror,
    }
}

fn apply_transport_travel(
    world: &mut World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
) -> TransportTravelResult {
    let resolved = resolve_transport_travel(world, player, character_id, current_area, spec);
    if let TransportTravelResult::SameArea { x, y, mirror } = resolved {
        if world.teleport_character_same_area(character_id, x, y, false) {
            TransportTravelResult::SameArea { x, y, mirror }
        } else {
            TransportTravelResult::Busy
        }
    } else {
        resolved
    }
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
    builder.gold(character.gold);

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

    for (slot_index, slot) in cache.slots.iter_mut().enumerate() {
        if !used[slot_index] {
            *slot = None;
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
    let colors = if character.sprite == 27 {
        [0, 0, 0]
    } else {
        [character.c1, character.c2, character.c3]
    };

    ugaris_protocol::packet::character_name(
        client_character_id(character),
        character.level.min(u32::from(u8::MAX)) as u8,
        colors,
        character.clan.min(u16::from(u8::MAX)) as u8,
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
        SV_GOLD, SV_LOGINDONE, SV_MAP01, SV_MAP10, SV_MAP11, SV_MAPPOS, SV_MIRROR, SV_ORIGIN,
        SV_PROTOCOL, SV_QUESTLOG, SV_SETCITEM, SV_SETHP, SV_SETITEM, SV_SETVAL0, SV_SETVAL1,
        SV_SPECIAL, SV_TEXT, SV_TICKER,
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
    fn legacy_questlog_payload_packs_c_bitfield_shape() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.quest_log.open(0);
        player.quest_log.mark_done(1);

        let payload = legacy_questlog_payload(&player);

        assert_eq!(payload.len(), 1 + 100 + 36);
        assert_eq!(payload[0], SV_QUESTLOG);
        assert_eq!(payload[1], 0x40);
        assert_eq!(payload[2], 0x81);
        assert!(payload[3..101].iter().all(|byte| *byte == 0));
        assert!(payload[101..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn transport_travel_moves_to_seen_same_area_destination() {
        let mut world = World::default();
        world.map = ugaris_core::map::MapGrid::new(300, 300);
        let login = login_block("Ralph");
        assert!(world.spawn_character(login_character(CharacterId(1), &login, 1, 10, 10), 10, 10));
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.touch_transport(0);

        let result = apply_transport_travel(&mut world, &player, CharacterId(1), 1, 1 + 2 * 256);

        assert_eq!(
            result,
            TransportTravelResult::SameArea {
                x: 139,
                y: 75,
                mirror: 2
            }
        );
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (139, 75));
    }

    #[test]
    fn transport_travel_rejects_unseen_destination_with_legacy_text() {
        let world = World::default();
        let player = PlayerRuntime::connected(1, 0);

        let result = resolve_transport_travel(&world, &player, CharacterId(1), 1, 1);

        assert_eq!(
            result,
            TransportTravelResult::Blocked(
                "You've never been to Cameron before. You cannot go there.".to_string()
            )
        );
    }

    #[test]
    fn pick_berry_runtime_grants_template_and_marks_flower_ppd() {
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(1),
            &login_block("Ralph"),
            31,
            10,
            10,
        ));
        let mut loader = ZoneLoader::new();
        loader.item_templates.insert(
            "picked_flower_h".to_string(),
            ugaris_core::zone::ItemTemplate {
                key: "picked_flower_h".to_string(),
                name: "Flower H".to_string(),
                description: String::new(),
                flags: ItemFlags::USED,
                sprite: 11190,
                value: 0,
                min_level: 0,
                max_level: 0,
                needs_class: 0,
                template_id: 0x1f02,
                modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
                modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
                driver: ugaris_core::item_driver::IDR_LIZARDFLOWER,
                driver_data: vec![2],
            },
        );
        let mut player = PlayerRuntime::connected(1, 0);

        let result = apply_pick_berry(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(1),
            2,
            0x001f_2030,
            50_000,
        );

        assert_eq!(result, PickBerryApplyResult::Picked("Flower H".to_string()));
        assert_eq!(player.flower_last_used_seconds(0x001f_2030), Some(50_000));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let cursor_item = character.cursor_item.unwrap();
        assert_eq!(world.items.get(&cursor_item).unwrap().name, "Flower H");
    }

    #[test]
    fn flask_ingredient_feedback_matches_legacy_order_and_names() {
        let mut counts = [0; 29];
        counts[0] = 2;
        counts[20] = 1;
        counts[24] = 7;

        assert_eq!(
            flask_ingredient_feedback(counts),
            vec![
                "Contains 2 parts Adygalah.".to_string(),
                "Contains 1 parts Fiery Stone.".to_string(),
            ]
        );
    }

    #[test]
    fn pick_berry_runtime_enforces_herbalist_ripe_time() {
        let mut world = World::default();
        let mut character = login_character(CharacterId(1), &login_block("Ralph"), 31, 10, 10);
        character.professions[profession::HERBALIST] = 20;
        world.add_character(character);
        let mut loader = ZoneLoader::new();
        loader.item_templates.insert(
            "picked_flower_h".to_string(),
            ugaris_core::zone::ItemTemplate {
                key: "picked_flower_h".to_string(),
                name: "Flower H".to_string(),
                description: String::new(),
                flags: ItemFlags::USED,
                sprite: 11190,
                value: 0,
                min_level: 0,
                max_level: 0,
                needs_class: 0,
                template_id: 0x1f02,
                modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
                modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
                driver: ugaris_core::item_driver::IDR_LIZARDFLOWER,
                driver_data: vec![2],
            },
        );
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_flower_used(7, 100);

        let blocked = apply_pick_berry(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(1),
            2,
            7,
            100 + 8 * 60 * 60 - 1,
        );

        assert_eq!(blocked, PickBerryApplyResult::NotRipe);

        let picked = apply_pick_berry(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(1),
            2,
            7,
            100 + 8 * 60 * 60,
        );

        assert_eq!(picked, PickBerryApplyResult::Picked("Flower H".to_string()));
    }

    #[test]
    fn alchemy_flower_runtime_grants_ingredient_and_marks_flower_ppd() {
        let mut world = World::default();
        world.add_character(login_character(
            CharacterId(1),
            &login_block("Ralph"),
            1,
            10,
            10,
        ));
        let mut loader = ZoneLoader::new();
        loader.item_templates.insert(
            "alc_berry1".to_string(),
            ugaris_core::zone::ItemTemplate {
                key: "alc_berry1".to_string(),
                name: "Berry A".to_string(),
                description: String::new(),
                flags: ItemFlags::USED,
                sprite: 50280,
                value: 0,
                min_level: 0,
                max_level: 0,
                needs_class: 0,
                template_id: 0x1f11,
                modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
                modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
                driver: ugaris_core::item_driver::IDR_FLOWER,
                driver_data: vec![17],
            },
        );
        let mut player = PlayerRuntime::connected(1, 0);

        let result = apply_pick_alchemy_flower(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(1),
            17,
            0x0001_2814,
            60_000,
        );

        assert_eq!(result, PickBerryApplyResult::Picked("Berry A".to_string()));
        assert_eq!(player.flower_last_used_seconds(0x0001_2814), Some(60_000));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let cursor_item = character.cursor_item.unwrap();
        assert_eq!(world.items.get(&cursor_item).unwrap().name, "Berry A");
    }

    #[test]
    fn transport_travel_keeps_cross_area_as_handoff_boundary() {
        let world = World::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.touch_transport(2);

        let result = resolve_transport_travel(&world, &player, CharacterId(1), 1, 3 + 4 * 256);

        assert_eq!(
            result,
            TransportTravelResult::CrossArea {
                area: 3,
                x: 129,
                y: 201,
                mirror: 4,
            }
        );
    }

    #[test]
    fn transport_travel_randomizes_invalid_mirror_like_c() {
        let world = World::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.touch_transport(2);

        let low =
            resolve_transport_travel_with_random(&world, &player, CharacterId(1), 1, 3, |_| 7);
        let high = resolve_transport_travel_with_random(
            &world,
            &player,
            CharacterId(1),
            1,
            3 + 27 * 256,
            |_| 25,
        );

        assert_eq!(
            low,
            TransportTravelResult::CrossArea {
                area: 3,
                x: 129,
                y: 201,
                mirror: 8,
            }
        );
        assert_eq!(
            high,
            TransportTravelResult::CrossArea {
                area: 3,
                x: 129,
                y: 201,
                mirror: 26,
            }
        );
    }

    #[test]
    fn transport_travel_clamps_injected_random_mirror_roll() {
        let world = World::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.touch_transport(2);

        let result =
            resolve_transport_travel_with_random(&world, &player, CharacterId(1), 1, 3, |_| 99);

        assert_eq!(
            result,
            TransportTravelResult::CrossArea {
                area: 3,
                x: 129,
                y: 201,
                mirror: 26,
            }
        );
    }

    #[test]
    fn transport_clan_access_marks_direct_member_byte() {
        let mut world = World::default();
        let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
        character.clan = 17;
        world.add_character(character);

        assert_eq!(transport_clan_access(&world, CharacterId(1)), [0, 0, 1, 0]);
    }

    #[test]
    fn transport_clan_travel_uses_legacy_hall_coordinates() {
        let mut world = World::default();
        world.map = ugaris_core::map::MapGrid::new(300, 300);
        let mut character = login_character(CharacterId(1), &login_block("Ralph"), 3, 10, 10);
        character.clan = 17;
        assert!(world.spawn_character(character, 10, 10));
        let player = PlayerRuntime::connected(1, 0);

        let result = apply_transport_travel(&mut world, &player, CharacterId(1), 3, 81 + 2 * 256);

        assert_eq!(
            result,
            TransportTravelResult::SameArea {
                x: 28,
                y: 58,
                mirror: 2,
            }
        );
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (28, 58));
    }

    #[test]
    fn transport_clan_travel_rejects_non_member_with_legacy_text() {
        let world = World::default();
        let player = PlayerRuntime::connected(1, 0);

        let result = resolve_transport_travel(&world, &player, CharacterId(1), 3, 65);

        assert_eq!(
            result,
            TransportTravelResult::Blocked("You may not enter (1).".to_string())
        );
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
        let mut character = login_character(CharacterId(1), &login, 1, 10, 10);
        character.flags |= CharacterFlags::MALE;
        world.add_character(character);

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
    fn special_potion_fun_message_uses_legacy_gender_pronouns() {
        let mut world = World::default();
        let login = login_block("Maggie");
        let mut female = login_character(CharacterId(1), &login, 1, 10, 10);
        female.flags |= CharacterFlags::FEMALE;
        world.add_character(female);
        let login = login_block("Snowball");
        world.add_character(login_character(CharacterId(2), &login, 1, 10, 11));

        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 8).as_deref(),
            Some("You see Maggie hit herself on the head with a mug.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(1), 14).as_deref(),
            Some("Maggie cracks her strong knuckles.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(2), 8).as_deref(),
            Some("You see Snowball hit itself on the head with a mug.")
        );
        assert_eq!(
            special_potion_fun_message(&world, CharacterId(2), 14).as_deref(),
            Some("Snowball cracks its strong knuckles.")
        );
    }

    #[test]
    fn lollipop_area_message_matches_legacy_text() {
        let mut world = World::default();
        let login = login_block("Ralph");
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));

        assert_eq!(
            lollipop_area_message(&world, CharacterId(1)),
            "Ralph licks a lollipop."
        );
        assert_eq!(
            lollipop_area_message(&world, CharacterId(99)),
            "Someone licks a lollipop."
        );
    }

    #[test]
    fn potion_area_message_matches_legacy_text() {
        let mut world = World::default();
        let login = login_block("Ralph");
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));

        assert_eq!(
            potion_area_message(&world, CharacterId(1)),
            "Ralph drinks a potion."
        );
        assert_eq!(
            potion_area_message(&world, CharacterId(99)),
            "Someone drinks a potion."
        );
    }

    #[test]
    fn apply_empty_potion_drink_replaces_carried_potion_with_template() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.hp = 20 * POWERSCALE;
        character.mana = 45 * POWERSCALE;
        character.endurance = 49 * POWERSCALE;
        character.inventory[INVENTORY_START_INVENTORY] = Some(ItemId(100));
        let mut world = World::default();
        world.add_character(character);

        let mut potion = test_item(ItemId(100), 1234, ItemFlags::USED | ItemFlags::USE);
        potion.driver = ugaris_core::item_driver::IDR_POTION;
        potion.carried_by = Some(character_id);
        potion.driver_data = vec![2, 10, 10, 10];
        world.add_item(potion);

        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"empty_potion2: name="Empty Potion" sprite=5678 ;"#)
            .unwrap();

        assert!(apply_empty_potion_drink(
            &mut world,
            &mut loader,
            ItemId(100),
            character_id,
            2,
        ));

        assert!(!world.items.contains_key(&ItemId(100)));
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.hp, 30 * POWERSCALE);
        assert_eq!(character.mana, 50 * POWERSCALE);
        assert_eq!(character.endurance, 50 * POWERSCALE);
        let empty_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
        let empty = world.items.get(&empty_id).unwrap();
        assert_eq!(empty.name, "Empty Potion");
        assert_eq!(empty.sprite, 5678);
        assert_eq!(empty.carried_by, Some(character_id));
    }

    #[test]
    fn christmas_pop_inspection_messages_match_legacy_text() {
        assert_eq!(
            christmas_pop_inspection_messages(),
            [
                "You notice a tiny inscription on the magical lollipop. It reads:",
                "\"Place me under a Christmas tree to receive a special gift from the gods.\"",
                "In shimmering letters below, you see:",
                "\"Each tree grants but one wish per adventurer.\"",
            ]
        );
    }

    #[test]
    fn no_potion_area_feedback_applies_to_all_potion_items() {
        let mut world = World::default();
        world.add_item(test_item_with_driver(ItemId(1), IDR_SPECIAL_POTION));
        world.add_item(test_item_with_driver(ItemId(2), IDR_BEYONDPOTION));
        world.add_item(test_item_with_driver(
            ItemId(3),
            ugaris_core::item_driver::IDR_POTION,
        ));
        world.add_item(test_item_with_driver(ItemId(4), IDR_TORCH));

        assert!(is_no_potion_area_blocked_item(&world, ItemId(1)));
        assert!(is_no_potion_area_blocked_item(&world, ItemId(2)));
        assert!(is_no_potion_area_blocked_item(&world, ItemId(3)));
        assert!(!is_no_potion_area_blocked_item(&world, ItemId(4)));
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
        character.gold = 12345;
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
        assert!(payload
            .windows(5)
            .any(|window| window == [SV_GOLD, 0x39, 0x30, 0, 0]));
    }

    #[test]
    fn gold_command_moves_character_gold_to_cursor_money_item() {
        let mut world = World::default();
        let mut loader = ZoneLoader::new();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.gold = 12_500;
        world.add_character(character);

        let result = apply_gold_command(&mut world, &mut loader, character_id, "/gold 12")
            .expect("gold command should be recognized");

        assert!(result.messages.is_empty());
        assert!(result.inventory_changed);
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.gold, 11_300);
        let money_id = character.cursor_item.expect("money should be on cursor");
        let money = world.items.get(&money_id).unwrap();
        assert!(money.flags.contains(ItemFlags::MONEY));
        assert_eq!(money.value, 1_200);
        assert_eq!(money.carried_by, Some(character_id));
    }

    #[test]
    fn gold_command_preserves_c_guard_order_and_atoi_prefix() {
        let mut world = World::default();
        let mut loader = ZoneLoader::new();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.gold = 500;
        world.add_character(character);

        let invalid = apply_gold_command(&mut world, &mut loader, character_id, "/gold abc")
            .expect("gold command should be recognized");
        assert_eq!(invalid.messages, vec!["Hu?"]);

        let too_much = apply_gold_command(&mut world, &mut loader, character_id, "/gold 6")
            .expect("gold command should be recognized");
        assert_eq!(too_much.messages, vec!["You do not have that much gold."]);

        world.characters.get_mut(&character_id).unwrap().gold = 1_000;
        let cursor_item = test_item(ItemId(99), 100, ItemFlags::TAKE);
        world.add_item(cursor_item);
        world.characters.get_mut(&character_id).unwrap().cursor_item = Some(ItemId(99));
        let occupied = apply_gold_command(&mut world, &mut loader, character_id, "/gold 6abc")
            .expect("gold command should be recognized");
        assert_eq!(
            occupied.messages,
            vec!["Please free your hand (mouse cursor) first."]
        );
    }

    #[test]
    fn ggold_command_is_god_only_and_uses_atoi_prefix() {
        let mut world = World::default();
        let mut loader = ZoneLoader::new();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.gold = 500;
        world.add_character(character);

        assert!(apply_gold_command(&mut world, &mut loader, character_id, "/ggold 12").is_none());
        assert_eq!(world.characters.get(&character_id).unwrap().gold, 500);

        world
            .characters
            .get_mut(&character_id)
            .unwrap()
            .flags
            .insert(CharacterFlags::GOD);
        let result = apply_gold_command(&mut world, &mut loader, character_id, "/ggold 12abc")
            .expect("god gold command should be recognized");

        assert!(result.messages.is_empty());
        assert!(result.inventory_changed);
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.gold, 1_700);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn maxlag_command_matches_legacy_range_and_feedback() {
        let mut player = PlayerRuntime::connected(1, 0);

        let invalid = apply_maxlag_command(&mut player, "/maxlag 2")
            .expect("maxlag command should be recognized");
        assert_eq!(
            invalid.messages,
            vec!["Number must be between 3 and 20.".to_string()]
        );
        assert_eq!(player.max_lag_seconds, 0);

        let high = apply_maxlag_command(&mut player, "/maxlag 21")
            .expect("maxlag command should be recognized");
        assert_eq!(
            high.messages,
            vec!["Number must be between 3 and 20.".to_string()]
        );
        assert_eq!(player.max_lag_seconds, 0);

        let result = apply_maxlag_command(&mut player, "/maxl 12abc")
            .expect("legacy maxlag abbreviation should be recognized");
        assert_eq!(player.max_lag_seconds, 12);
        assert_eq!(
            result.messages,
            vec!["Set delay for lag control to kick in to 12 seconds.".to_string()]
        );

        assert!(apply_maxlag_command(&mut player, "/ma 12").is_none());
    }

    #[test]
    fn lag_command_toggles_artificial_lag_with_legacy_feedback() {
        let mut world = World::default();
        world.map = ugaris_core::map::MapGrid::new(20, 20);
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let player = PlayerRuntime::connected(1, 0);

        let enabled = apply_lag_command(&mut world, &player, character_id, "/lag")
            .expect("lag command should be recognized");
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));
        assert_eq!(
            enabled.messages,
            vec![
                "Turned artificial lag on.".to_string(),
                "PLEASE turn this option off (type /lag again) before you complain about lag!"
                    .to_string(),
            ]
        );

        let disabled = apply_lag_command(&mut world, &player, character_id, "/lag")
            .expect("lag command should be recognized");
        assert!(!world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));
        assert_eq!(
            disabled.messages,
            vec!["Turned artificial lag off.".to_string()]
        );
        assert!(apply_lag_command(&mut world, &player, character_id, "/la").is_none());
    }

    #[test]
    fn lag_command_blocks_enabling_in_arena_or_with_hate() {
        let mut world = World::default();
        world.map = ugaris_core::map::MapGrid::new(20, 20);
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        world.map.set_flags(10, 10, MapFlags::ARENA);
        let mut player = PlayerRuntime::connected(1, 0);

        let arena = apply_lag_command(&mut world, &player, character_id, "/lag")
            .expect("lag command should be recognized");
        assert_eq!(arena.messages, vec!["You cannot simulate lag in an arena."]);
        assert!(!world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));

        world.map.set_flags(10, 10, MapFlags::empty());
        assert!(player.add_pk_hate(99));
        let hate = apply_lag_command(&mut world, &player, character_id, "/lag")
            .expect("lag command should be recognized");
        assert_eq!(
            hate.messages,
            vec!["You cannot simulate lag while your hate list is not empty."]
        );
    }

    #[test]
    fn description_command_sanitizes_and_reports_legacy_text() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        world.add_character(login_character(
            character_id,
            &login_block("Tester"),
            1,
            10,
            10,
        ));

        let result = apply_description_command(
            &mut world,
            character_id,
            "/desc I am \"great\" and 100% ready",
        )
        .expect("description minlen prefix should be recognized");

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.description, "I am 'great' and 100  ready");
        assert_eq!(
            result.messages,
            vec!["Your description reads now: I am 'great' and 100  ready"]
        );
    }

    #[test]
    fn description_command_preserves_empty_and_truncation_guards() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        world.add_character(login_character(
            character_id,
            &login_block("Tester"),
            1,
            10,
            10,
        ));

        let empty = apply_description_command(&mut world, character_id, "/description   ")
            .expect("description command should be recognized");
        assert_eq!(empty.messages, vec!["Sorry, you need to enter some text."]);
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .description
            .is_empty());

        let long_text = "x".repeat(200);
        apply_description_command(
            &mut world,
            character_id,
            &format!("/description {long_text}"),
        )
        .expect("description command should be recognized");
        assert_eq!(
            world
                .characters
                .get(&character_id)
                .unwrap()
                .description
                .len(),
            159
        );
        assert!(apply_description_command(&mut world, character_id, "/de Text").is_none());
        assert!(
            apply_description_command(&mut world, character_id, "/descriptionx Text").is_none()
        );
    }

    #[test]
    fn status_command_shows_represented_lostcon_and_account_state() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character
            .flags
            .insert(CharacterFlags::PAID | CharacterFlags::NOBLESS);
        character.values[1][CharacterValue::Bless as usize] = 10;
        character.values[1][CharacterValue::Pulse as usize] = 8;
        character.values[1][CharacterValue::Fireball as usize] = 5;
        let mut player = PlayerRuntime::connected(1, 0);
        player.set_max_lag_seconds(12);

        let result = apply_status_command(&character, &player, "/status")
            .expect("status command should be recognized");

        assert_eq!(result.messages[0], "Lag Control Settings:");
        assert!(result
            .messages
            .contains(&"Max. Lag [/MAXLAG]: 12 sec.".to_string()));
        assert!(result
            .messages
            .contains(&"Don't use Bless [/NOBLESS]: Off.".to_string()));
        assert!(result
            .messages
            .contains(&"Don't use Fireball [/NOFIREBALL]: Off.".to_string()));
        assert!(result
            .messages
            .contains(&"Automatic Pulse [/AUTOPULSE]: Off.".to_string()));
        assert!(result
            .messages
            .contains(&"Allow others to bless me [/ALLOWBLESS]: No.".to_string()));
        assert!(result.messages.contains(&"Account Status:".to_string()));
        assert!(result.messages.contains(&"Paid Account".to_string()));
    }

    #[test]
    fn status_command_preserves_cmdcmp_prefix_shape() {
        let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        let player = PlayerRuntime::connected(1, 0);

        assert!(apply_status_command(&character, &player, "/s").is_some());
        assert!(apply_status_command(&character, &player, "/stat").is_some());
        assert!(apply_status_command(&character, &player, "/statusx").is_none());
    }

    #[test]
    fn saves_command_is_god_only_and_uses_legacy_prefix_parsing() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.saves = 3;
        world.add_character(character);
        let mut runtime = ServerRuntime::default();

        assert!(apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/saves 12",
            1
        )
        .is_none());
        assert_eq!(world.characters.get(&character_id).unwrap().saves, 3);

        world
            .characters
            .get_mut(&character_id)
            .unwrap()
            .flags
            .insert(CharacterFlags::GOD);
        let result =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/save 12abc", 1)
                .expect("god saves command should be recognized by minlen 4 abbreviation");

        assert!(result.messages.is_empty());
        assert!(!result.inventory_changed);
        assert_eq!(world.characters.get(&character_id).unwrap().saves, 12);

        apply_admin_character_command(&mut world, &mut runtime, character_id, "/saves nope", 1)
            .expect("god saves command should be recognized");
        assert_eq!(world.characters.get(&character_id).unwrap().saves, 0);
    }

    #[test]
    fn live_quest_toggle_commands_preserve_legacy_gates_and_feedback() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::LQMASTER);
        world.add_character(character);
        let mut runtime = ServerRuntime::default();

        assert!(
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/im", 1)
                .is_none()
        );

        let immortal =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/im", 20)
                .expect("area-20 lqmaster immortal command should be recognized");
        assert_eq!(immortal.messages, vec!["Immortal is on."]);
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::IMMORTAL));

        let infrared =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/inf", 20)
                .expect("area-20 lqmaster infrared command should be recognized");
        assert_eq!(infrared.messages, vec!["Infrared is on."]);
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::INFRARED));

        let invisible =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/inv", 20)
                .expect("area-20 lqmaster invisible command should be recognized");
        assert_eq!(invisible.messages, vec!["Invisible is on."]);
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::INVISIBLE));
    }

    #[test]
    fn god_visibility_toggle_commands_preserve_legacy_feedback() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::GOD);
        world.add_character(character);
        let mut runtime = ServerRuntime::default();

        let xray =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/xray", 1)
                .expect("god xray command should be recognized");
        assert_eq!(xray.messages, vec!["Turned x-ray mode on."]);
        assert!(xray.inventory_changed);
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::XRAY));

        let spy = apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
            .expect("god spy command should be recognized");
        assert_eq!(
            spy.messages,
            vec!["Turned spy mode on. You will now see all tells, clan, alliance, club, area, and mirror chat."]
        );
        assert!(world
            .characters
            .get(&character_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SPY));

        let spy_off =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
                .expect("god spy command should toggle off");
        assert_eq!(
            spy_off.messages,
            vec!["Turned spy mode off. You will no longer see all tells, clan, alliance, club, area, and mirror chat."]
        );
    }

    #[test]
    fn god_dlight_and_showattack_commands_mutate_runtime_without_feedback() {
        let mut world = World::default();
        world.date = GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None);
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::GOD);
        world.add_character(character);
        let mut runtime = ServerRuntime::default();

        let dlight = apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/dlight 123abc",
            1,
        )
        .expect("god dlight command should be recognized");
        assert!(dlight.messages.is_empty());
        assert_eq!(runtime.dlight_override, 123);
        assert_eq!(world.date.daylight, 123);

        apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 0", 23)
            .expect("god dlight zero command should clear the override");
        assert_eq!(runtime.dlight_override, 0);
        assert_eq!(
            world.date.daylight,
            GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None).daylight
        );

        let showattack =
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/showat", 1)
                .expect("god showattack command should allow C minlen 6 abbreviation");
        assert!(showattack.messages.is_empty());
        assert!(runtime.show_attack);
        assert!(world.show_attack_debug);

        apply_admin_character_command(&mut world, &mut runtime, character_id, "/showattack", 1)
            .expect("god showattack command should toggle back off");
        assert!(!runtime.show_attack);
        assert!(!world.show_attack_debug);
    }

    #[test]
    fn dlight_and_showattack_are_god_only_and_keep_full_dlight_minlen() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        world.add_character(login_character(
            character_id,
            &login_block("Tester"),
            1,
            10,
            10,
        ));
        let mut runtime = ServerRuntime::default();

        assert!(apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/dlight 1",
            1,
        )
        .is_none());
        assert!(apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/showattack",
            1,
        )
        .is_none());
        assert_eq!(runtime.dlight_override, 0);
        assert!(!runtime.show_attack);

        world
            .characters
            .get_mut(&character_id)
            .unwrap()
            .flags
            .insert(CharacterFlags::GOD);
        assert!(apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/dligh 7",
            1,
        )
        .is_none());
        assert_eq!(runtime.dlight_override, 0);
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, character_id, "/showa", 1,)
                .is_none()
        );
        assert!(!runtime.show_attack);
    }

    #[test]
    fn shutup_command_is_staff_only_and_toggles_target_flag() {
        let mut world = World::default();
        let staff_id = CharacterId(7);
        let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 10, 10);
        staff.flags.insert(CharacterFlags::STAFF);
        world.add_character(staff);

        let player_id = CharacterId(8);
        let player = login_character(player_id, &login_block("Target"), 1, 11, 10);
        world.add_character(player);

        let ordinary_id = CharacterId(9);
        let ordinary = login_character(ordinary_id, &login_block("Ordinary"), 1, 12, 10);
        world.add_character(ordinary);
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.insert(2, PlayerRuntime::connected(2, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(player_id);
        runtime.players.get_mut(&2).unwrap().character_id = Some(staff_id);

        assert!(apply_shutup_command(
            &mut world,
            &mut runtime,
            ordinary_id,
            "/shutup Target 10",
            100
        )
        .is_none());
        assert!(!world
            .characters
            .get(&player_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));

        let result =
            apply_shutup_command(&mut world, &mut runtime, staff_id, "/shutup Target", 100)
                .expect("staff shutup command should be recognized");
        assert!(result.messages.is_empty());
        assert_eq!(
            runtime
                .player_for_character(player_id)
                .unwrap()
                .shutup_until_seconds,
            700
        );
        assert_eq!(result.target_message_bytes.len(), 1);
        assert!(world
            .characters
            .get(&player_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));

        apply_shutup_command(&mut world, &mut runtime, staff_id, "/shutup Target 0", 101)
            .expect("zero minutes should disable shutup");
        assert_eq!(
            runtime
                .player_for_character(player_id)
                .unwrap()
                .shutup_until_seconds,
            0
        );
        assert!(!world
            .characters
            .get(&player_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));
    }

    #[test]
    fn shutup_command_preserves_legacy_name_and_range_errors() {
        let mut world = World::default();
        let staff_id = CharacterId(7);
        let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 10, 10);
        staff.flags.insert(CharacterFlags::GOD);
        world.add_character(staff);
        let mut runtime = ServerRuntime::default();

        let result = apply_shutup_command(
            &mut world,
            &mut runtime,
            staff_id,
            "/shutup Missing 10",
            100,
        )
        .expect("god shutup command should be recognized");
        assert_eq!(
            result.messages,
            vec!["Sorry, no player by the name Missing."]
        );

        let target_id = CharacterId(8);
        let target = login_character(target_id, &login_block("Alpha"), 1, 11, 10);
        world.add_character(target);

        let result = apply_shutup_command(
            &mut world,
            &mut runtime,
            staff_id,
            "/shutup Alpha 61abc",
            100,
        )
        .expect("out-of-range shutup should still be handled");
        assert_eq!(
            result.messages,
            vec!["Sorry, can only shutup for 0 to 60 minutes (use 0 to disable)."]
        );
        assert!(!world
            .characters
            .get(&target_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));

        assert!(
            apply_shutup_command(&mut world, &mut runtime, staff_id, "/shut Alpha 10", 100)
                .is_none()
        );
    }

    #[test]
    fn shutup_expiry_clears_flag_and_notifies_target() {
        let mut world = World::default();
        let target_id = CharacterId(8);
        let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::SHUTUP);
        world.add_character(target);

        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(target_id);
        runtime.players.get_mut(&1).unwrap().shutup_until_seconds = 700;

        assert!(drain_expired_shutup_feedback(&mut world, &mut runtime, 699).is_empty());
        assert!(world
            .characters
            .get(&target_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));

        let feedback = drain_expired_shutup_feedback(&mut world, &mut runtime, 700);
        assert_eq!(feedback.len(), 1);
        assert_eq!(feedback[0].0, target_id);
        assert!(feedback[0].1.starts_with(COL_LIGHT_RED));
        assert!(feedback[0].1.ends_with(COL_RESET));
        assert_eq!(
            runtime
                .player_for_character(target_id)
                .unwrap()
                .shutup_until_seconds,
            0
        );
        assert!(!world
            .characters
            .get(&target_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));
    }

    #[test]
    fn character_snapshot_restores_active_legacy_shutup_ppd() {
        let target_id = CharacterId(8);
        let character = login_character(target_id, &login_block("Target"), 1, 11, 10);

        let mut persisted = PlayerRuntime::connected(99, 0);
        persisted.shutup_until_seconds = 700;
        let ppd_blob = persisted.encode_legacy_ppd_blob(&[]);

        let snapshot = CharacterSnapshot {
            character,
            items: Vec::new(),
            ppd_blob,
            subscriber_blob: Vec::new(),
            current_area: 1,
            current_mirror: 1,
            allowed_area: 1,
            mirror: 1,
        };
        let mut world = World::default();
        let mut player = PlayerRuntime::connected(1, 0);

        let result = apply_character_snapshot(&mut world, &mut player, snapshot, 11, 10, 100);
        assert!(result.loaded);
        assert_eq!(player.shutup_until_seconds, 700);
        assert!(world
            .characters
            .get(&target_id)
            .unwrap()
            .flags
            .contains(CharacterFlags::SHUTUP));
    }

    #[test]
    fn tell_command_delivers_local_private_message_and_acknowledges_receipt() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
        world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.insert(2, PlayerRuntime::connected(2, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
        runtime.players.get_mut(&1).unwrap().current_mirror_id = 3;
        runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

        let result = apply_tell_command(
            &world,
            &mut runtime,
            sender_id,
            "/tell Bob Hello MixedCase",
            100,
        )
        .expect("tell command should be recognized");

        assert_eq!(
            result.sender_messages,
            vec!["Told Bob: \"Hello MixedCase\""]
        );
        assert_eq!(
            result.delivered_messages,
            vec![(
                target_id,
                "Alice (3) tells you: \"Hello MixedCase\"".to_string()
            )]
        );
        assert!(drain_expired_tell_feedback(&world, &mut runtime, 112).is_empty());
    }

    #[test]
    fn tell_command_preserves_legacy_errors_and_self_feedback() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

        let missing = apply_tell_command(&world, &mut runtime, sender_id, "/tell Bob Hi", 100)
            .expect("tell command should be recognized");
        assert_eq!(
            missing.sender_messages,
            vec!["Sorry, no player by the name Bob."]
        );

        let empty = apply_tell_command(&world, &mut runtime, sender_id, "/tell Alice", 100)
            .expect("tell command should be recognized");
        assert_eq!(
            empty.sender_messages,
            vec!["Tell, yes, tell it will be, but tell what?"]
        );

        let self_tell = apply_tell_command(&world, &mut runtime, sender_id, "/tell Alice Hi", 100)
            .expect("tell command should be recognized");
        assert_eq!(
            self_tell.sender_messages,
            vec!["Told Alice: \"Hi\"", "Do you like talking to yourself?"]
        );
    }

    #[test]
    fn notells_blocks_non_staff_tells_until_timeout_feedback() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
        world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.insert(2, PlayerRuntime::connected(2, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
        runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

        let toggle = apply_notells_command(&mut world, target_id, "/not").unwrap();
        assert_eq!(toggle.messages, vec!["Turned no-tell mode on."]);

        let result = apply_tell_command(&world, &mut runtime, sender_id, "/tell Bob Hi", 100)
            .expect("tell command should be recognized");
        assert!(result.delivered_messages.is_empty());
        assert!(
            drain_expired_tell_feedback(&world, &mut runtime, 100 + TICKS_PER_SECOND).is_empty()
        );
        assert_eq!(
            drain_expired_tell_feedback(&world, &mut runtime, 100 + TICKS_PER_SECOND + 1),
            vec![(sender_id, b"Bob is not listening.".to_vec())]
        );
    }

    #[test]
    fn channels_command_lists_legacy_channel_table() {
        let result = apply_channels_command("/chan").expect("channels prefix should be recognized");

        assert_eq!(
            result.messages[0],
            " 0: Announce   - Announcements from management - NOLEAVE"
        );
        assert!(result.messages.contains(
            &" 7: Clan2      - Channel only visible to members of your clan".to_string()
        ));
        assert_eq!(
            result.messages.last().unwrap(),
            "32: God        - Ye God's private channel"
        );
        assert!(apply_channels_command("/clearhate").is_none());
    }

    #[test]
    fn join_leave_chat_commands_preserve_legacy_feedback_and_bits() {
        let mut player = PlayerRuntime::connected(1, 0);

        let joined =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 2extra")
                .expect("join should be recognized");
        assert_eq!(joined.messages, vec!["You have joined channel 2 (Gossip)."]);
        assert_eq!(player.chat_channels, 1_u32 << 1);

        let duplicate =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 2").unwrap();
        assert_eq!(
            duplicate.messages,
            vec!["You have already joined channel 2 (Gossip)."]
        );

        let left =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/leave 2").unwrap();
        assert_eq!(left.messages, vec!["You have left channel 2 (Gossip)."]);
        assert_eq!(player.chat_channels, 0);

        let left_again =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/leave 2").unwrap();
        assert_eq!(
            left_again.messages,
            vec!["You have already left channel 2 (Gossip)."]
        );
    }

    #[test]
    fn join_chat_command_gates_staff_and_god_channels() {
        let mut player = PlayerRuntime::connected(1, 0);

        let staff_denied =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 31").unwrap();
        assert_eq!(
            staff_denied.messages,
            vec!["Permission denied to join channel 31 (Staff)."]
        );
        assert_eq!(player.chat_channels, 0);

        let staff_joined = apply_join_leave_chat_command(
            &mut player,
            CharacterFlags::PLAYER | CharacterFlags::EVENTMASTER,
            "/join 31",
        )
        .unwrap();
        assert_eq!(
            staff_joined.messages,
            vec!["You have joined channel 31 (Staff)."]
        );

        let god_denied =
            apply_join_leave_chat_command(&mut player, CharacterFlags::STAFF, "/join 32").unwrap();
        assert_eq!(
            god_denied.messages,
            vec!["Permission denied to join channel 32 (God)."]
        );

        let joined_all =
            apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/joinall").unwrap();
        assert_eq!(joined_all.messages, vec!["You have joined all channels."]);
        for nr in 1..=13 {
            assert_ne!(player.chat_channels & (1_u32 << (nr - 1)), 0);
        }
    }

    #[test]
    fn chat_command_delivers_joined_local_channel_with_legacy_formatting() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
        world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.insert(2, PlayerRuntime::connected(2, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
        runtime.players.get_mut(&1).unwrap().current_mirror_id = 3;
        runtime.players.get_mut(&1).unwrap().chat_channels = 1_u32 << 1;
        runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);
        runtime.players.get_mut(&2).unwrap().chat_channels = 1_u32 << 1;

        let result = apply_chat_command(&world, &mut runtime, sender_id, "/gossip Hello", 1)
            .expect("chat channel command should be recognized");

        assert!(result.sender_messages.is_empty());
        assert_eq!(result.delivered_message_bytes.len(), 2);
        let text = String::from_utf8_lossy(&result.delivered_message_bytes[0].1);
        assert!(text.contains("Gossip: "));
        assert!(text.contains("Alice"));
        assert!(text.contains("(3) says: \"Hello\""));
        assert!(result.delivered_message_bytes[0]
            .1
            .starts_with(&[0xb0, b'c']));
    }

    #[test]
    fn local_speech_command_delivers_legacy_say_to_nearby_players() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let nearby_id = CharacterId(8);
        let far_id = CharacterId(9);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.x = 10;
        sender.y = 10;
        world.add_character(sender);
        let mut nearby = login_character(nearby_id, &login_block("Bob"), 1, 11, 10);
        nearby.x = 11;
        nearby.y = 10;
        world.add_character(nearby);
        let mut far = login_character(far_id, &login_block("Far"), 1, 250, 250);
        far.x = 250;
        far.y = 250;
        world.add_character(far);

        let mut runtime = ServerRuntime::default();
        for (session, id) in [(1, sender_id), (2, nearby_id), (3, far_id)] {
            runtime
                .players
                .insert(session, PlayerRuntime::connected(session, 0));
            runtime.players.get_mut(&session).unwrap().character_id = Some(id);
        }

        let result = apply_local_speech_command(
            &mut world,
            &runtime,
            sender_id,
            "/say Hello \"quoted\"",
            123,
        )
        .expect("say command should be recognized");

        assert!(result.sender_messages.is_empty());
        let mut deliveries = result
            .delivered_message_bytes
            .iter()
            .map(|(id, bytes)| (*id, String::from_utf8_lossy(bytes).into_owned()))
            .collect::<Vec<_>>();
        deliveries.sort_by_key(|(id, _)| id.0);
        assert_eq!(
            deliveries,
            vec![
                (sender_id, "Alice says: \"Hello \"quoted\"\"".to_string()),
                (nearby_id, "Alice says: \"Hello \"quoted\"\"".to_string()),
            ]
        );
    }

    #[test]
    fn local_speech_command_preserves_mute_and_quote_rejection() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.flags.insert(CharacterFlags::SHUTUP);
        sender.x = 10;
        sender.y = 10;
        world.add_character(sender);
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

        let muted = apply_local_speech_command(&mut world, &runtime, sender_id, "/whisper Hi", 1)
            .expect("whisper should be recognized");
        assert_eq!(
            muted.sender_messages,
            vec!["Sorry, you cannot say anything right now."]
        );

        world
            .characters
            .get_mut(&sender_id)
            .unwrap()
            .flags
            .remove(CharacterFlags::SHUTUP);
        let quoted =
            apply_local_speech_command(&mut world, &runtime, sender_id, "/whisper bad\"quote", 1)
                .expect("whisper should be recognized");
        assert!(quoted.sender_messages.is_empty());
        assert!(quoted.delivered_message_bytes.is_empty());
    }

    #[test]
    fn shout_and_holler_apply_legacy_endurance_costs() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.endurance = GameSettings::default().shout_cost;
        sender.x = 10;
        sender.y = 10;
        world.add_character(sender);
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

        let shouted = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Hi", 77)
            .expect("shout should be recognized");
        assert_eq!(shouted.delivered_message_bytes.len(), 1);
        let sender = world.characters.get(&sender_id).unwrap();
        assert_eq!(sender.endurance, 0);
        assert_eq!(sender.regen_ticker, 77);

        let tired = apply_local_speech_command(&mut world, &runtime, sender_id, "/holler Hi", 78)
            .expect("holler should be recognized");
        assert_eq!(
            tired.sender_messages,
            vec!["You're too exhausted to holler."]
        );
        assert!(tired.delivered_message_bytes.is_empty());
    }

    #[test]
    fn underwater_speech_falls_back_to_blub_without_shout_cost() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.endurance = 0;
        sender.x = 10;
        sender.y = 10;
        world.add_character(sender);
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::UNDERWATER);
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

        let result = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Help", 77)
            .expect("shout should be recognized");
        assert_eq!(result.sender_messages, Vec::<String>::new());
        assert_eq!(
            String::from_utf8_lossy(&result.delivered_message_bytes[0].1),
            "Alice says: \"Blub.\""
        );
        assert_eq!(world.characters.get(&sender_id).unwrap().endurance, 0);
    }

    #[test]
    fn chat_command_preserves_join_and_access_gates() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
        let mut runtime = ServerRuntime::default();
        runtime.players.insert(1, PlayerRuntime::connected(1, 0));
        runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

        let unjoined = apply_chat_command(&world, &mut runtime, sender_id, "/gossip Hello", 1)
            .expect("chat command should be recognized");
        assert_eq!(
            unjoined.sender_messages,
            vec!["You must join a channel before you can use it."]
        );

        runtime.players.get_mut(&1).unwrap().chat_channels = 1_u32 << 31;
        let god_denied = apply_chat_command(&world, &mut runtime, sender_id, "/god Hello", 1)
            .expect("god chat command should be recognized");
        assert_eq!(god_denied.sender_messages, vec!["Access denied."]);

        let empty = apply_chat_command(&world, &mut runtime, sender_id, "/c2   ", 1)
            .expect("c2 chat command should be recognized");
        assert_eq!(
            empty.sender_messages,
            vec!["You cannot send empty chat messages."]
        );
    }

    #[test]
    fn chat_command_filters_ignored_sender_and_private_clan_channel() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        let outsider_id = CharacterId(9);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.clan = 4;
        world.add_character(sender);
        let mut target = login_character(target_id, &login_block("Bob"), 1, 11, 10);
        target.clan = 4;
        world.add_character(target);
        let mut outsider = login_character(outsider_id, &login_block("Eve"), 1, 12, 10);
        outsider.clan = 5;
        world.add_character(outsider);

        let mut runtime = ServerRuntime::default();
        for (session, id) in [(1, sender_id), (2, target_id), (3, outsider_id)] {
            runtime
                .players
                .insert(session, PlayerRuntime::connected(session, 0));
            let player = runtime.players.get_mut(&session).unwrap();
            player.character_id = Some(id);
            player.chat_channels = 1_u32 << 6;
        }
        runtime
            .player_for_character_mut(target_id)
            .unwrap()
            .ignored_characters
            .push(sender_id.0);

        let result = apply_chat_command(&world, &mut runtime, sender_id, "/clan2 Secret", 1)
            .expect("clan chat command should be recognized");

        assert_eq!(
            result
                .delivered_message_bytes
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![sender_id]
        );
    }

    #[test]
    fn chat_command_forwards_private_channel_to_spying_god() {
        let mut world = World::default();
        let sender_id = CharacterId(7);
        let same_clan_id = CharacterId(8);
        let spy_id = CharacterId(9);
        let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
        sender.clan = 4;
        world.add_character(sender);
        let mut same_clan = login_character(same_clan_id, &login_block("Bob"), 1, 11, 10);
        same_clan.clan = 4;
        world.add_character(same_clan);
        let mut spy = login_character(spy_id, &login_block("God"), 1, 12, 10);
        spy.flags.insert(CharacterFlags::GOD | CharacterFlags::SPY);
        world.add_character(spy);

        let mut runtime = ServerRuntime::default();
        for (session, id) in [(1, sender_id), (2, same_clan_id), (3, spy_id)] {
            runtime
                .players
                .insert(session, PlayerRuntime::connected(session, 0));
            let player = runtime.players.get_mut(&session).unwrap();
            player.character_id = Some(id);
            player.chat_channels = 1_u32 << 6;
        }

        let result = apply_chat_command(&world, &mut runtime, sender_id, "/clan2 Secret", 1)
            .expect("clan chat command should be recognized");

        let deliveries = result
            .delivered_message_bytes
            .iter()
            .map(|(id, bytes)| (*id, String::from_utf8_lossy(bytes).into_owned()))
            .collect::<Vec<_>>();
        assert_eq!(deliveries.len(), 3);
        assert!(deliveries
            .iter()
            .any(|(id, text)| *id == spy_id && text.contains("[SPY/CLAN]")));
        assert!(result
            .delivered_message_bytes
            .iter()
            .find(|(id, _)| *id == spy_id)
            .unwrap()
            .1
            .starts_with(COL_DARK_GRAY));
    }

    #[test]
    fn who_command_lists_visible_players_like_legacy_command() {
        let mut world = World::default();
        let mut warrior = login_character(CharacterId(7), &login_block("Warrior"), 1, 10, 10);
        warrior.level = 12;
        warrior.flags.insert(CharacterFlags::WARRIOR);
        world.add_character(warrior);

        let mut archmage = login_character(CharacterId(8), &login_block("Archmage"), 1, 11, 10);
        archmage.level = 33;
        archmage
            .flags
            .insert(CharacterFlags::ARCH | CharacterFlags::MAGE);
        world.add_character(archmage);

        let mut hidden = login_character(CharacterId(9), &login_block("Hidden"), 1, 12, 10);
        hidden.flags.insert(CharacterFlags::INVISIBLE);
        world.add_character(hidden);

        let mut staff_hidden = login_character(CharacterId(10), &login_block("Staff"), 1, 13, 10);
        staff_hidden
            .flags
            .insert(CharacterFlags::STAFF | CharacterFlags::NOWHO);
        world.add_character(staff_hidden);

        let mut npc = login_character(CharacterId(11), &login_block("Npc"), 1, 14, 10);
        npc.flags.remove(CharacterFlags::PLAYER);
        world.add_character(npc);

        let result = apply_who_command(&world, CharacterFlags::empty(), "/who")
            .expect("who command should be recognized");

        assert_eq!(
            result.messages,
            vec![
                "Currently online in this area:",
                "Warrior (W12)",
                "Archmage (AM33)",
            ]
        );
    }

    #[test]
    fn who_command_preserves_legacy_short_prefix_match() {
        let mut world = World::default();
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        character.level = 9;
        world.add_character(character);

        let result = apply_who_command(&world, CharacterFlags::empty(), "/w")
            .expect("legacy cmdcmp accepts short who");

        assert_eq!(
            result.messages,
            vec!["Currently online in this area:", "Tester (9)"]
        );
        assert!(apply_who_command(&world, CharacterFlags::empty(), "/whostaff").is_none());
    }

    #[test]
    fn staff_who_command_lists_visible_staff_with_legacy_prefix_gate() {
        let mut world = World::default();
        let caller_flags = CharacterFlags::STAFF;

        let mut staff = login_character(CharacterId(7), &login_block("Staffer"), 1, 10, 10);
        staff.flags.insert(CharacterFlags::STAFF);
        world.add_character(staff);

        let mut god = login_character(CharacterId(8), &login_block("LagGod"), 1, 11, 10);
        god.flags.insert(CharacterFlags::GOD);
        god.driver = 42;
        world.add_character(god);

        let mut hidden_staff = login_character(CharacterId(9), &login_block("Hidden"), 1, 12, 10);
        hidden_staff
            .flags
            .insert(CharacterFlags::STAFF | CharacterFlags::INVISIBLE);
        world.add_character(hidden_staff);

        let player = login_character(CharacterId(10), &login_block("Player"), 1, 13, 10);
        world.add_character(player);

        assert!(apply_who_command(&world, caller_flags, "/who").is_some());
        assert!(apply_who_command(&world, caller_flags, "/whoo").is_none());

        let result = apply_who_command(&world, caller_flags, "/whos")
            .expect("legacy cmdcmp accepts whostaff prefix length four");

        assert_eq!(result.messages, vec!["Staffer []", "LagGod [] (lagging)"]);
    }

    #[test]
    fn time_command_preserves_legacy_showtime_output_and_prefix_gate() {
        let date = GameDate::calculate(ugaris_core::game_time::START_TIME, 1, None);

        let result = apply_time_command(date, "/ti").expect("legacy cmdcmp accepts /ti");

        assert_eq!(
            result.messages,
            vec![
                "It's 00:00 on the 1/1/0. Sunrise is at 07:30, sunset at 16:30. Moonrise is at 06:00, moonset at 18:00.",
                "Be careful, New Moon tonight!",
                "It's a scary day, it's Winter Solstice today!",
                "Next full moon is in 14 days.",
                "Spring Equinox will be in 90 days.",
            ]
        );
        assert!(apply_time_command(date, "/t").is_none());
        assert!(apply_time_command(date, "/timex").is_none());
    }

    #[test]
    fn time_command_reports_legacy_moon_phase_and_next_events() {
        let date = GameDate::calculate(
            ugaris_core::game_time::START_TIME + ugaris_core::game_time::DAY_LEN * 7,
            1,
            None,
        );

        let result = apply_time_command(date, "/time").expect("time command should match");

        assert!(result.messages.contains(&"Half Moon.".to_string()));
        assert!(result
            .messages
            .contains(&"Next full moon is in 7 days.".to_string()));
        assert!(result
            .messages
            .contains(&"Spring Equinox will be in 83 days.".to_string()));
    }

    #[test]
    fn weather_command_reports_default_clear_area_weather() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Weather"), 1, 10, 10);
        character.x = 10;
        character.y = 10;
        world.add_character(character);

        let result =
            apply_weather_command(&world, character_id, 1, WeatherState::default(), "/weather")
                .expect("weather command should be recognized");

        assert_eq!(
            result.messages,
            vec!["Current weather in this area: Clear skies"]
        );
        assert!(
            apply_weather_command(&world, character_id, 1, WeatherState::default(), "/weath")
                .is_none()
        );
    }

    #[test]
    fn weather_command_reports_indoor_protection_and_outdoor_effects() {
        let mut world = World::default();
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Weather"), 1, 10, 10);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let weather = WeatherState {
            current_weather: 2,
            weather_intensity: 3,
            weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP,
            affected_areas: &[1],
            ..WeatherState::default()
        };

        let outdoor = apply_weather_command(&world, character_id, 1, weather, "/weather")
            .expect("weather command should be recognized");
        assert_eq!(
            outdoor.messages,
            vec![
                "Current weather in this area: Heavy storm",
                "Movement is affected by the weather.",
                "Visibility is reduced by the weather.",
                "The weather makes the ground slippery.",
            ]
        );

        world.map.set_flags(10, 10, MapFlags::INDOORS);
        let indoor = apply_weather_command(&world, character_id, 1, weather, "/weather")
            .expect("weather command should be recognized");
        assert_eq!(
            indoor.messages,
            vec![
                "Current weather in this area: Heavy storm",
                "You are indoors and protected from weather effects.",
            ]
        );
    }

    #[test]
    fn weather_command_reports_god_debug_info() {
        let mut world = World::default();
        world.tick = ugaris_core::Tick(24);
        let character_id = CharacterId(7);
        let mut god = login_character(character_id, &login_block("WeatherGod"), 1, 10, 10);
        god.x = 10;
        god.y = 10;
        god.flags.insert(CharacterFlags::GOD);
        world.add_character(god);
        let weather = WeatherState {
            current_weather: 1,
            weather_intensity: 2,
            weather_effects: WEATHER_EFFECT_DAMAGE,
            is_transitioning: true,
            transition_start: 0,
            transition_duration: 48,
            prev_weather: 0,
            weather_change_time: 240,
            affected_areas: &[1, 3],
        };

        let result = apply_weather_command(&world, character_id, 1, weather, "/weather")
            .expect("weather command should be recognized");

        assert_eq!(
            result.messages,
            vec![
                "Current weather in this area: Moderate rain",
                "Global Weather Debug Info:",
                "- Current Weather: Rain",
                "- Intensity: Moderate",
                "- Effects: 0x4",
                "- Transitioning: Yes (1 seconds left)",
                "- Previous Weather: Clear",
                "- Progress: 50.0%",
                "- Next Change: 9 seconds",
                "- Affected Areas (2):",
                "  1 3 ",
                "The weather is causing damage.",
            ]
        );
    }

    #[test]
    fn nowho_command_toggles_staff_visibility_only_for_staff_or_gods() {
        let mut world = World::default();
        let mut staff = login_character(CharacterId(7), &login_block("Staffer"), 1, 10, 10);
        staff.flags.insert(CharacterFlags::STAFF);
        world.add_character(staff);

        let result = apply_nowho_command(&mut world, CharacterId(7), "/nowho")
            .expect("staff nowho should be recognized");
        assert_eq!(result.messages, vec!["NoWho enabled."]);
        assert!(world
            .characters
            .get(&CharacterId(7))
            .expect("staff exists")
            .flags
            .contains(CharacterFlags::NOWHO));

        let result = apply_nowho_command(&mut world, CharacterId(7), "/nowho")
            .expect("staff nowho should toggle off");
        assert_eq!(result.messages, vec!["NoWho disabled."]);

        let player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
        world.add_character(player);
        assert!(apply_nowho_command(&mut world, CharacterId(8), "/nowho").is_none());
        assert!(apply_nowho_command(&mut world, CharacterId(7), "/now").is_none());
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
    fn apply_zombie_shrine_requires_matching_skull() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        skull.template_id = IID_AREA2_ZOMBIESKULL1;
        skull.carried_by = Some(character_id);
        world.add_item(skull);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_zombie_shrine(&mut world, &mut loader, character_id, 1, 0),
            ZombieShrineApplyResult::NeedsOffering(1)
        );
        assert_eq!(
            world.characters.get(&character_id).unwrap().cursor_item,
            Some(ItemId(20))
        );
        assert!(world.items.contains_key(&ItemId(20)));
    }

    #[test]
    fn apply_zombie_shrine_consumes_skull_and_grants_item_to_cursor() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        skull.template_id = IID_AREA2_ZOMBIESKULL1;
        skull.carried_by = Some(character_id);
        world.add_item(skull);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(
                r#"zombie_skull2: name="Silver Skull" ID=01000026 flag=IF_TAKE ;"#,
            )
            .unwrap();

        assert_eq!(
            apply_zombie_shrine(
                &mut world,
                &mut loader,
                character_id,
                0,
                seed_for_legacy_random(22, 0)
            ),
            ZombieShrineApplyResult::Gift("Silver Skull".to_string())
        );
        assert!(!world.items.contains_key(&ItemId(20)));
        let cursor_item_id = world
            .characters
            .get(&character_id)
            .unwrap()
            .cursor_item
            .unwrap();
        let gift = world.items.get(&cursor_item_id).unwrap();
        assert_eq!(gift.name, "Silver Skull");
        assert_eq!(gift.carried_by, Some(character_id));
    }

    #[test]
    fn apply_arkhata_pool_consumes_scroll_without_reward() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut scroll = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        scroll.template_id = 0x0100_00C2;
        scroll.carried_by = Some(character_id);
        world.add_item(scroll);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_arkhata_pool(
                &mut world,
                &mut loader,
                character_id,
                ItemId(20),
                seed_for_legacy_random(70, 0)
            ),
            ArkhataPoolApplyResult::Vanished
        );
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(!world.items.contains_key(&ItemId(20)));
    }

    #[test]
    fn apply_arkhata_pool_consumes_scroll_and_grants_reward() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut scroll = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        scroll.template_id = 0x0100_00C2;
        scroll.carried_by = Some(character_id);
        world.add_item(scroll);
        let mut loader = ZoneLoader::new();
        loader
            .load_item_templates_str(r#"Red_Scroll: name="Red Scroll" ID=010000C3 flag=IF_TAKE ;"#)
            .unwrap();

        assert_eq!(
            apply_arkhata_pool(
                &mut world,
                &mut loader,
                character_id,
                ItemId(20),
                seed_for_legacy_random(70, 22)
            ),
            ArkhataPoolApplyResult::Gift("Red Scroll".to_string())
        );
        assert!(!world.items.contains_key(&ItemId(20)));
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(character.inventory.iter().flatten().any(|item_id| {
            world
                .items
                .get(item_id)
                .is_some_and(|item| item.name == "Red Scroll")
        }));
    }

    #[test]
    fn apply_zombie_shrine_consumes_skull_and_grants_experience() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        character.exp = 100;
        let mut world = World::default();
        world.add_character(character);
        let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        skull.template_id = IID_AREA2_ZOMBIESKULL3;
        skull.carried_by = Some(character_id);
        world.add_item(skull);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_zombie_shrine(
                &mut world,
                &mut loader,
                character_id,
                2,
                seed_for_legacy_random(7, 4)
            ),
            ZombieShrineApplyResult::Experience(2250)
        );
        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        assert_eq!(character.exp, 2350);
        assert!(!world.items.contains_key(&ItemId(20)));
    }

    #[test]
    fn apply_zombie_shrine_installs_timed_bonus_spell() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.cursor_item = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
        skull.template_id = IID_AREA2_ZOMBIESKULL1;
        skull.carried_by = Some(character_id);
        world.add_item(skull);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_zombie_shrine(
                &mut world,
                &mut loader,
                character_id,
                0,
                seed_for_legacy_random(22, 16)
            ),
            ZombieShrineApplyResult::Bonus {
                message: "You have been protected for a short while.",
                driver: IDR_ARMOR,
                strength: 100,
                duration_ticks: TICKS_PER_SECOND as i32 * 60 * 5,
            }
        );

        let character = world.characters.get(&character_id).unwrap();
        assert_eq!(character.cursor_item, None);
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_ARMOR);
        assert_eq!(spell.modifier_index[0], CharacterValue::Armor as i16);
        assert_eq!(spell.modifier_value[0], 100);
        assert_eq!(
            spell.driver_data,
            (TICKS_PER_SECOND as u32 * 60 * 5).to_le_bytes().to_vec()
        );
        assert_eq!(character.values[0][CharacterValue::Armor as usize], 100);
        assert!(!world.items.contains_key(&ItemId(20)));
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
        let gift = world.items.get(&gift_id).unwrap();
        assert!(gift
            .description
            .starts_with("To Tester, with holiday blessings from "));
        assert!(gift.description.ends_with(".\nMerry Christmas!"));
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, true),
            XmasTreeResult::AlreadyGranted
        );
    }

    #[test]
    fn enhance_xmas_item_uses_unique_legacy_skill_pool_and_caps_values() {
        let mut gift = test_item(ItemId(30), 1, ItemFlags::USED | ItemFlags::TAKE);
        gift.modifier_index = [CharacterValue::Armor as i16; ugaris_core::entity::MAX_MODIFIERS];
        gift.modifier_value = [99; ugaris_core::entity::MAX_MODIFIERS];
        let mut rng = XmasTreeRng::new(42);

        enhance_xmas_item(&mut gift, &mut rng);

        let mut seen = Vec::new();
        for (&index, &value) in gift.modifier_index.iter().zip(gift.modifier_value.iter()) {
            if value == 0 {
                assert_eq!(value, 0);
                continue;
            }
            assert!(value > 0 && value <= XMAS_MAX_SKILL_VALUE);
            assert!(XMAS_ENHANCE_SKILLS
                .iter()
                .any(|skill| *skill as i16 == index));
            assert!(!seen.contains(&index));
            seen.push(index);
        }
        assert!(seen.len() <= XMAS_MAX_SKILLS);
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
    fn nomad_dice_roll_uses_three_lucky_six_sided_dice() {
        let seed = 42;
        let luck = 2;
        let expected = [
            legacy_lucky_die_from_rolls(
                6,
                luck,
                (0..=luck).map(|offset| legacy_random(seed + u64::from(offset), 6) as u8 + 1),
            ),
            legacy_lucky_die_from_rolls(
                6,
                luck,
                (3..=5).map(|offset| legacy_random(seed + offset, 6) as u8 + 1),
            ),
            legacy_lucky_die_from_rolls(
                6,
                luck,
                (6..=8).map(|offset| legacy_random(seed + offset, 6) as u8 + 1),
            ),
        ];

        let (dice, total) = legacy_nomad_dice_roll(seed, luck);

        assert_eq!(dice, expected);
        assert_eq!(total, expected.iter().copied().sum::<u8>());
        assert!(dice.iter().all(|die| (1..=6).contains(die)));
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
    fn demon_chip_stack_invalid_template_reports_legacy_chip_bug() {
        let character_id = CharacterId(7);
        let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
        character.inventory[30] = Some(ItemId(20));
        let mut world = World::default();
        world.add_character(character);
        let mut stack = test_item(ItemId(20), 53007, ItemFlags::USED | ItemFlags::USE);
        stack.template_id = 0xDEAD_BEEF;
        stack.driver = IDR_DEMONCHIP;
        stack.carried_by = Some(character_id);
        world.add_item(stack);
        let mut loader = ZoneLoader::new();

        assert_eq!(
            apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
            NomadStackApplyResult::Bug("Bug #1445y")
        );
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
    fn account_depot_blob_encodes_c_struct_item_layout() {
        let mut depot = AccountDepotState::default();
        let mut item = test_item(
            ItemId(99),
            -12345,
            ItemFlags::USED | ItemFlags::TAKE | ItemFlags::NODEPOT,
        );
        item.name = "Long Stored Relic Name That Fits".to_string();
        item.description = "A relic in the account depot.".to_string();
        item.value = 12_345;
        item.min_level = 7;
        item.max_level = 77;
        item.needs_class = 3;
        item.owner_id = -44;
        item.modifier_index = [1, -2, 3, -4, 5];
        item.modifier_value = [10, 20, 30, 40, 50];
        item.content_id = 17;
        item.driver = IDR_TORCH;
        item.driver_data = (0..50).collect();
        item.template_id = 0x0102_0304;
        item.serial = 0xAABB_CCDD;
        depot.slots[5] = Some(item);

        let bytes = encode_legacy_account_depot_blob(&depot);

        assert_eq!(bytes.len(), LEGACY_ACCOUNT_DEPOT_ITEM_SIZE);
        assert_eq!(
            u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            (ItemFlags::USED | ItemFlags::TAKE | ItemFlags::NODEPOT).bits()
        );
        assert_eq!(
            &bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 4],
            b"Long"
        );
        assert_eq!(
            u32::from_le_bytes(
                bytes[LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET..LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET + 4]
                    .try_into()
                    .unwrap()
            ),
            12_345
        );
        assert_eq!(bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET], 7);
        assert_eq!(
            i16::from_le_bytes(
                bytes[LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + 2
                    ..LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + 4]
                    .try_into()
                    .unwrap()
            ),
            -2
        );
        assert_eq!(
            u16::from_le_bytes(
                bytes[LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET..LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET + 2]
                    .try_into()
                    .unwrap()
            ),
            IDR_TORCH
        );
        assert_eq!(
            &bytes[LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + 40],
            &(0u8..40).collect::<Vec<_>>()[..]
        );
        assert_eq!(
            u32::from_le_bytes(
                bytes[LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET
                    ..LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET + 4]
                    .try_into()
                    .unwrap()
            ),
            0x0102_0304
        );
        assert_eq!(
            i32::from_le_bytes(
                bytes[LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET..LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET + 4]
                    .try_into()
                    .unwrap()
            ),
            -12345
        );
        assert!(bytes[LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX..]
            .iter()
            .all(|&b| b == 0));
    }

    #[test]
    fn account_depot_blob_decodes_items_into_dense_legacy_slots() {
        let mut item = test_item(ItemId(99), 4321, ItemFlags::USED | ItemFlags::TAKE);
        item.name = "Stored Gem".to_string();
        item.description = "It sparkles.".to_string();
        item.value = 88;
        item.modifier_index = [7, 0, 0, 0, 0];
        item.modifier_value = [9, 0, 0, 0, 0];
        item.driver = IDR_FOOD;
        item.driver_data = vec![3, 2, 1];
        item.template_id = 0x1234_5678;
        item.serial = 123;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&encode_legacy_account_depot_item(&item));
        bytes.extend_from_slice(&[0xFF; 17]);

        let depot = decode_legacy_account_depot_blob(&bytes);
        let decoded = depot.slots[0].as_ref().unwrap();

        assert_eq!(decoded.id, ItemId(1));
        assert_eq!(decoded.name, "Stored Gem");
        assert_eq!(decoded.description, "It sparkles.");
        assert_eq!(decoded.flags, ItemFlags::USED | ItemFlags::TAKE);
        assert_eq!(decoded.sprite, 4321);
        assert_eq!(decoded.value, 88);
        assert_eq!(decoded.modifier_index[0], 7);
        assert_eq!(decoded.modifier_value[0], 9);
        assert_eq!(decoded.driver, IDR_FOOD);
        assert_eq!(&decoded.driver_data[..3], &[3, 2, 1]);
        assert_eq!(decoded.template_id, 0x1234_5678);
        assert_eq!(decoded.serial, 123);
        assert_eq!(decoded.x, 0);
        assert_eq!(decoded.y, 0);
        assert_eq!(decoded.carried_by, None);
        assert_eq!(decoded.contained_in, None);
        assert!(depot.slots[1].is_none());
    }

    #[test]
    fn account_depot_subscriber_blob_replaces_block_and_preserves_unknown() {
        let unknown_id = (77 << 24) | 9;
        let mut existing = Vec::new();
        write_legacy_subscriber_block(&mut existing, unknown_id, &[1, 2, 3]);
        write_legacy_subscriber_block(&mut existing, DRD_ACCOUNT_WIDE_DEPOT, &[9, 9, 9]);

        let mut depot = AccountDepotState::default();
        let mut item = test_item(ItemId(99), 1234, ItemFlags::USED | ItemFlags::TAKE);
        item.name = "Stored Gem".to_string();
        depot.slots[2] = Some(item);

        let encoded = encode_legacy_account_depot_subscriber_blob(&existing, Some(&depot));
        let blocks = parse_legacy_subscriber_blocks(&encoded).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].id, unknown_id);
        assert_eq!(blocks[0].data, &[1, 2, 3]);
        assert_eq!(blocks[1].id, DRD_ACCOUNT_WIDE_DEPOT);
        let decoded = decode_legacy_account_depot_subscriber_blob(&encoded).unwrap();
        assert_eq!(decoded.slots[0].as_ref().unwrap().name, "Stored Gem");
    }

    #[test]
    fn account_depot_subscriber_blob_omits_empty_depot_like_c_del_data() {
        let mut existing = Vec::new();
        write_legacy_subscriber_block(&mut existing, DRD_ACCOUNT_WIDE_DEPOT, &[9, 9, 9]);

        let encoded = encode_legacy_account_depot_subscriber_blob(
            &existing,
            Some(&AccountDepotState::default()),
        );

        assert!(parse_legacy_subscriber_blocks(&encoded).unwrap().is_empty());
        assert!(decode_legacy_account_depot_subscriber_blob(&encoded).is_none());
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
    fn alias_command_create_list_replace_delete_and_clear_match_legacy_feedback() {
        let mut player = PlayerRuntime::connected(1, 0);

        let created = apply_alias_command(&mut player, "/alias thanks Thank you!")
            .expect("alias command should be recognized");
        let listed =
            apply_alias_command(&mut player, "/alias").expect("alias list should be recognized");
        let replaced = apply_alias_command(&mut player, "/alias thanks Thanks!")
            .expect("alias replacement should be recognized");
        let erased = apply_alias_command(&mut player, "/alias thanks")
            .expect("alias deletion should be recognized");
        let missing = apply_alias_command(&mut player, "/alias thanks")
            .expect("missing alias deletion should be recognized");
        let cleared = apply_alias_command(&mut player, "/clearaliases")
            .expect("clearaliases should be recognized");

        assert_eq!(created.messages, vec!["Created thanks -> Thank you!."]);
        assert_eq!(listed.messages, vec!["thanks -> Thank you!"]);
        assert_eq!(replaced.messages, vec!["Replaced thanks -> Thanks!."]);
        assert_eq!(erased.messages, vec!["Erased thanks -> Thanks!."]);
        assert_eq!(
            missing.messages,
            vec!["Alias thanks not found, could not delete."]
        );
        assert_eq!(cleared.messages, vec!["Done. All gone now."]);
    }

    #[test]
    fn alias_command_truncates_legacy_from_and_to_lengths() {
        let mut player = PlayerRuntime::connected(1, 0);
        let result = apply_alias_command(
            &mut player,
            "/alias abcdefghijklmnopqrstuvwxyz 123456789012345678901234567890123456789012345678901234567890",
        )
        .expect("alias command should be recognized");

        assert_eq!(player.aliases[0].from, "abcdefg");
        assert_eq!(player.aliases[0].to.len(), 55);
        assert_eq!(
            result.messages[0],
            format!("Created abcdefg -> {}.", player.aliases[0].to)
        );
        assert!(apply_alias_command(&mut player, "/al abc").is_some());
        assert!(apply_alias_command(&mut player, "/a abc").is_none());
    }

    #[test]
    fn ignore_command_toggles_lists_and_clears_legacy_feedback() {
        let source_id = CharacterId(7);
        let target_id = CharacterId(8);
        let mut world = World::default();
        world.add_character(login_character(
            source_id,
            &login_block("Source"),
            1,
            10,
            10,
        ));
        world.add_character(login_character(
            target_id,
            &login_block("Target"),
            1,
            11,
            10,
        ));
        let mut runtime = ServerRuntime::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(source_id);
        runtime.players.insert(1, player);

        let added = apply_ignore_command(&world, &mut runtime, source_id, "/ign target")
            .expect("ignore abbreviation should be recognized");
        let listed = apply_ignore_command(&world, &mut runtime, source_id, "/ignore")
            .expect("ignore list should be recognized");
        let removed = apply_ignore_command(&world, &mut runtime, source_id, "/ignore target")
            .expect("ignore toggle should be recognized");
        let missing = apply_ignore_command(&world, &mut runtime, source_id, "/ignore unknown")
            .expect("ignore missing target should be recognized");
        runtime
            .player_for_character_mut(source_id)
            .unwrap()
            .toggle_ignored_character(target_id.0);
        let cleared = apply_clearignore_command(&mut runtime, source_id, "/clearignore")
            .expect("clearignore should be recognized");

        assert_eq!(added.messages, vec!["Added to ignore list."]);
        assert_eq!(listed.messages, vec!["Ignoring: Target"]);
        assert_eq!(removed.messages, vec!["Deleted from ignore list."]);
        assert_eq!(missing.messages, vec!["No player by that name."]);
        assert_eq!(cleared.messages, vec!["Ignore list is now empty."]);
        assert!(runtime
            .player_for_character(source_id)
            .unwrap()
            .ignored_characters
            .is_empty());
    }

    #[test]
    fn tell_command_respects_ignore_except_staff_mode() {
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        let mut world = World::default();
        world.add_character(login_character(
            sender_id,
            &login_block("Sender"),
            1,
            10,
            10,
        ));
        world.add_character(login_character(
            target_id,
            &login_block("Target"),
            1,
            11,
            10,
        ));
        let mut runtime = ServerRuntime::default();
        let mut sender = PlayerRuntime::connected(1, 0);
        sender.character_id = Some(sender_id);
        let mut target = PlayerRuntime::connected(2, 0);
        target.character_id = Some(target_id);
        target.toggle_ignored_character(sender_id.0);
        runtime.players.insert(1, sender);
        runtime.players.insert(2, target);

        let blocked = apply_tell_command(&world, &mut runtime, sender_id, "/tell target hello", 10)
            .expect("tell should be recognized");
        world
            .characters
            .get_mut(&sender_id)
            .unwrap()
            .flags
            .insert(CharacterFlags::STAFF);
        let staff = apply_tell_command(&world, &mut runtime, sender_id, "/tell target hello", 11)
            .expect("staff tell should be recognized");

        assert_eq!(blocked.sender_messages, vec!["Told Target: \"hello\""]);
        assert!(blocked.delivered_messages.is_empty());
        assert_eq!(staff.delivered_messages.len(), 1);
        assert!(staff.delivered_messages[0].1.contains("SENDER"));
    }

    #[test]
    fn tell_command_forwards_to_spying_god_even_when_recipient_blocks() {
        let sender_id = CharacterId(7);
        let target_id = CharacterId(8);
        let spy_id = CharacterId(9);
        let mut world = World::default();
        world.add_character(login_character(
            sender_id,
            &login_block("Sender"),
            1,
            10,
            10,
        ));
        let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::NOTELL);
        world.add_character(target);
        let mut spy = login_character(spy_id, &login_block("God"), 1, 12, 10);
        spy.flags.insert(CharacterFlags::GOD | CharacterFlags::SPY);
        world.add_character(spy);

        let mut runtime = ServerRuntime::default();
        for (session, id) in [(1, sender_id), (2, target_id), (3, spy_id)] {
            runtime
                .players
                .insert(session, PlayerRuntime::connected(session, 0));
            runtime.players.get_mut(&session).unwrap().character_id = Some(id);
        }

        let result = apply_tell_command(&world, &mut runtime, sender_id, "/tell target secret", 10)
            .expect("tell should be recognized");

        assert!(result.delivered_messages.is_empty());
        assert_eq!(result.delivered_message_bytes.len(), 1);
        assert_eq!(result.delivered_message_bytes[0].0, spy_id);
        assert!(result.delivered_message_bytes[0]
            .1
            .starts_with(COL_DARK_GRAY));
        assert!(
            String::from_utf8_lossy(&result.delivered_message_bytes[0].1)
                .contains("[SPY/TELL] Sender (0) tells you: \"secret\"")
        );
    }

    #[test]
    fn pk_hate_command_adds_online_player_and_clears_lag() {
        let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        attacker
            .flags
            .insert(CharacterFlags::PK | CharacterFlags::LAG);
        attacker.level = 12;
        let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::PK);
        target.level = 10;
        let mut world = World::default();
        world.add_character(attacker);
        world.add_character(target);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));

        let result =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate target", 0)
                .expect("hate command should be recognized");

        assert!(result.messages.is_empty());
        assert_eq!(result.name_refresh, vec![CharacterId(7), CharacterId(8)]);
        assert!(player.has_pk_hate_for(8));
        assert!(!world
            .characters
            .get(&CharacterId(7))
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));
    }

    #[test]
    fn pk_hate_command_list_and_remove_match_legacy_feedback() {
        let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        attacker.flags.insert(CharacterFlags::PK);
        let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::PK);
        let mut world = World::default();
        world.add_character(attacker);
        world.add_character(target);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        assert!(player.add_pk_hate(8));

        let listed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/listhate", 0)
            .expect("listhate command should be recognized");
        let removed =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/nohate target", 0)
                .expect("nohate command should be recognized");
        let empty = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/listhate", 0)
            .expect("listhate command should be recognized");

        assert_eq!(listed.messages, vec!["Hate: Target"]);
        assert_eq!(removed.messages, vec!["Removed Target from hate list"]);
        assert_eq!(removed.name_refresh, vec![CharacterId(7), CharacterId(8)]);
        assert_eq!(empty.messages, vec!["List is empty."]);
        assert!(!player.has_pk_hate_for(8));
    }

    #[test]
    fn pk_nohate_numeric_id_uses_legacy_del_hate_id_feedback() {
        let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        attacker.flags.insert(CharacterFlags::PK);
        let mut world = World::default();
        world.add_character(attacker);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        assert!(player.add_pk_hate(1234));

        let removed =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/nohate 1234", 0)
                .expect("nohate command should be recognized");

        assert_eq!(removed.messages, vec!["Removed from hate list"]);
        assert_eq!(removed.name_refresh, vec![CharacterId(7)]);
        assert!(!player.has_pk_hate_for(1234));
    }

    #[test]
    fn pk_hate_commands_accept_legacy_abbreviations() {
        let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        attacker
            .flags
            .insert(CharacterFlags::PK | CharacterFlags::LAG);
        attacker.level = 12;
        let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::PK);
        target.level = 10;
        let mut world = World::default();
        world.add_character(attacker);
        world.add_character(target);
        let mut player = PlayerRuntime::connected(1, 0);

        let added =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hat target", 0)
                .expect("abbreviated hate command should be recognized");
        let listed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/li", 0)
            .expect("abbreviated listhate command should be recognized");
        let removed =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/noh target", 0)
                .expect("abbreviated nohate command should be recognized");

        assert!(added.messages.is_empty());
        assert_eq!(listed.messages, vec!["Hate: Target"]);
        assert_eq!(removed.messages, vec!["Removed Target from hate list"]);
        assert!(!player.has_pk_hate_for(8));
        assert!(
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/ha target", 0)
                .is_none()
        );
    }

    #[test]
    fn pk_hate_command_uses_legacy_front_priority_on_duplicate_add() {
        let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 12;
        let mut first = login_character(CharacterId(8), &login_block("First"), 1, 11, 10);
        first.flags.insert(CharacterFlags::PK);
        first.level = 10;
        let mut second = login_character(CharacterId(9), &login_block("Second"), 1, 12, 10);
        second.flags.insert(CharacterFlags::PK);
        second.level = 10;
        let mut world = World::default();
        world.add_character(attacker);
        world.add_character(first);
        world.add_character(second);
        let mut player = PlayerRuntime::connected(1, 0);

        apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate first", 0)
            .expect("hate command should be recognized");
        apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate second", 0)
            .expect("hate command should be recognized");
        let refreshed =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate first", 0)
                .expect("hate command should be recognized");

        assert_eq!(player.pk_hate, vec![8, 9]);
        assert_eq!(refreshed.name_refresh, vec![CharacterId(7), CharacterId(8)]);
    }

    #[test]
    fn help_command_includes_legacy_pk_security_lines() {
        let result = apply_help_command("/help", CharacterFlags::empty(), 1)
            .expect("help command should be recognized");

        assert_eq!(result.messages[0], "=== PLAYER COMMANDS ===");
        assert_eq!(
            result.message_bytes[0],
            b"\xb0c3=== PLAYER COMMANDS ===\xb0c0".to_vec()
        );
        assert!(result
            .messages
            .contains(&"== Communication Commands ==".to_string()));
        assert!(result.messages.contains(
            &"/holler <text> - Say something with very long range (costs endurance points)"
                .to_string()
        ));
        assert!(result
            .messages
            .contains(&"/playerkiller - Toggle player killing mode on/off".to_string()));
        assert!(result
            .messages
            .contains(&"/iwilldie <id> - Confirm enabling player killer mode".to_string()));
        assert!(result
            .messages
            .contains(&"/clearhate - Clear your entire PK list at once".to_string()));
        assert!(result
            .messages
            .contains(&"== Miscellaneous Commands ==".to_string()));
        assert!(result
            .messages
            .contains(&"/help - Display this help text".to_string()));
        let help_line_index = result
            .messages
            .iter()
            .position(|message| message == "/help - Display this help text")
            .expect("help line should be present");
        assert_eq!(
            result.message_bytes[help_line_index],
            b"\xb0c4/help\xb0c0 - Display this help text".to_vec()
        );
        assert!(result.messages.contains(
            &"Type a command without parameters to get more information in some cases.".to_string()
        ));
        assert!(!result
            .messages
            .contains(&"=== STAFF COMMANDS ===".to_string()));
        assert!(apply_help_command("/hel", CharacterFlags::empty(), 1).is_none());
        assert!(!result.inventory_changed);
    }

    #[test]
    fn color_commands_pack_legacy_rgb_words_and_report_for_gods() {
        let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        let mut world = World::default();
        world.add_character(character);

        let changed = apply_color_command(&mut world, CharacterId(7), "/col1 1 2 3")
            .expect("col1 should be recognized");
        assert!(changed.name_changed);
        let character = world.characters.get(&CharacterId(7)).unwrap();
        assert_eq!(character.c1, (1 << 10) + (2 << 5) + 3);
        assert_eq!(character.c2, 0);
        assert_eq!(character.c3, 0);

        assert!(apply_color_command(&mut world, CharacterId(7), "/color").is_none());
        world
            .characters
            .get_mut(&CharacterId(7))
            .unwrap()
            .flags
            .insert(CharacterFlags::GOD);
        let report = apply_color_command(&mut world, CharacterId(7), "/color")
            .expect("god color command should be recognized");
        assert_eq!(report.messages, vec!["c1=443, c2=0, c3=0"]);
    }

    #[test]
    fn character_name_packet_uses_legacy_color_words() {
        let mut character = login_character(CharacterId(0x1234), &login_block("Tester"), 1, 10, 10);
        character.c1 = 0x0443;
        character.c2 = 0x0884;
        character.c3 = 0x0cc5;

        let packet = character_name_packet(&character);

        assert_eq!(packet[0], ugaris_protocol::packet::SV_NAME);
        assert_eq!(&packet[1..3], &0x1234_u16.to_le_bytes());
        assert_eq!(&packet[4..6], &0x0443_u16.to_le_bytes());
        assert_eq!(&packet[6..8], &0x0884_u16.to_le_bytes());
        assert_eq!(&packet[8..10], &0x0cc5_u16.to_le_bytes());
    }

    #[test]
    fn character_name_packet_uses_legacy_clan_and_demon_color_rules() {
        let mut character = login_character(CharacterId(0x1234), &login_block("Tester"), 1, 10, 10);
        character.c1 = 0x0443;
        character.c2 = 0x0884;
        character.c3 = 0x0cc5;
        character.clan = 42;

        let packet = character_name_packet(&character);
        assert_eq!(packet[10], 42);
        assert_eq!(&packet[4..10], &[0x43, 0x04, 0x84, 0x08, 0xc5, 0x0c]);

        character.sprite = 27;
        let demon_packet = character_name_packet(&character);
        assert_eq!(&demon_packet[4..10], &[0, 0, 0, 0, 0, 0]);
        assert_eq!(demon_packet[10], 42);
    }

    #[test]
    fn runtime_refreshes_known_character_name_cache_after_color_change() {
        let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
        let old_packet = character_name_packet(&character).to_vec();
        let mut runtime = ServerRuntime::default();
        runtime.map_caches.insert(
            11,
            VisibleMapCache {
                center_x: 10,
                center_y: 10,
                view_distance: 8,
                cells: HashMap::new(),
                known_character_names: HashMap::from([(7, old_packet.clone())]),
            },
        );

        character.c1 = 0x0443;
        let sessions = runtime.refresh_known_character_name(&character);

        assert_eq!(sessions, vec![11]);
        assert_ne!(
            runtime
                .map_caches
                .get(&11)
                .unwrap()
                .known_character_names
                .get(&7)
                .unwrap(),
            &old_packet
        );
    }

    #[test]
    fn help_command_includes_staff_and_god_sections_by_flag() {
        let staff = apply_help_command("/help", CharacterFlags::STAFF, 1)
            .expect("staff help should be recognized");

        assert!(staff
            .messages
            .contains(&"=== STAFF COMMANDS ===".to_string()));
        assert!(staff
            .messages
            .contains(&"/kick <name> - Disconnect a player from the server".to_string()));
        assert!(!staff.messages.contains(&"=== GOD COMMANDS ===".to_string()));

        let god = apply_help_command("/help", CharacterFlags::GOD, 1)
            .expect("god help should be recognized");

        assert!(god.messages.contains(&"=== STAFF COMMANDS ===".to_string()));
        assert!(god
            .messages
            .contains(&"=== EVENT/QUEST MASTER COMMANDS ===".to_string()));
        assert!(god.messages.contains(&"=== GOD COMMANDS ===".to_string()));
        assert!(god
            .messages
            .contains(&"/clearmerchantstores <id> - Reset a merchant's inventory".to_string()));
    }

    #[test]
    fn help_command_includes_event_and_live_quest_sections_by_flag() {
        let event = apply_help_command("/help", CharacterFlags::EVENTMASTER, 1)
            .expect("event help should be recognized");

        assert!(event
            .messages
            .contains(&"=== EVENT/QUEST MASTER COMMANDS ===".to_string()));
        assert!(event
            .messages
            .contains(&"== Event Master Commands ==".to_string()));
        assert!(!event
            .messages
            .contains(&"== Quest Master Commands ==".to_string()));

        let lq = apply_help_command("/help", CharacterFlags::LQMASTER, 20)
            .expect("lq help should be recognized");

        assert!(lq
            .messages
            .contains(&"== Quest Master Commands ==".to_string()));
        assert!(lq.messages.contains(
            &"Note: Additional LQ commands are available in the Live Quest area".to_string()
        ));
    }

    #[test]
    fn admin_subhelp_commands_match_legacy_privilege_gates_and_text() {
        assert!(apply_help_command("#achelp", CharacterFlags::empty(), 1).is_none());
        let ac = apply_help_command("#achelp", CharacterFlags::STAFF, 1)
            .expect("staff anti-cheat help should be recognized");
        assert_eq!(ac.messages[0], "--- Anti-Cheat Commands ---");
        assert_eq!(
            ac.message_bytes[0],
            b"\xb0c3--- Anti-Cheat Commands ---\xb0c0".to_vec()
        );
        assert!(ac
            .messages
            .contains(&"#acwarn <name> [reason] - Issue AC warning".to_string()));
        let acwarn_index = ac
            .messages
            .iter()
            .position(|message| message == "#acwarn <name> [reason] - Issue AC warning")
            .expect("acwarn line should be present");
        assert_eq!(
            ac.message_bytes[acwarn_index],
            b"\xb0c4#acwarn\xb0c0 \xb0c2<name>\xb0c0 [reason] - Issue AC warning".to_vec()
        );
        assert!(ac
            .messages
            .contains(&"#accleanup <days> - Cleanup old records (God)".to_string()));
        assert!(!ac.inventory_changed);

        assert!(apply_help_command("/macrohelp", CharacterFlags::empty(), 1).is_none());
        let macro_help = apply_help_command("/macrohelp", CharacterFlags::STAFF, 1)
            .expect("staff macro help should be recognized");
        assert_eq!(
            macro_help.messages[0],
            "=== Macro Daemon Admin Commands ==="
        );
        assert!(macro_help
            .messages
            .contains(&"/macroimmune <player> <mins> - Grant immunity (GOD only)".to_string()));
        assert!(macro_help
            .messages
            .contains(&"/macrohelp - Show this help".to_string()));

        assert!(apply_help_command("/penthelp", CharacterFlags::STAFF, 1).is_none());
        let pent = apply_help_command("/penthelp", CharacterFlags::GOD, 1)
            .expect("god pentagram help should be recognized");
        assert_eq!(pent.messages[0], "=== Pentagram Debug Commands (GOD) ===");
        assert!(pent
            .messages
            .contains(&"/setpentcount <player> <n> - Set pent_cnt (run count)".to_string()));
        assert!(pent
            .messages
            .contains(&"/penthelp - Show this help".to_string()));
    }

    #[test]
    fn pk_hate_command_clear_requires_pk_and_clears_runtime_list() {
        let mut character = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
        character.flags.remove(CharacterFlags::PK);
        let mut world = World::default();
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(7));
        assert!(player.add_pk_hate(8));

        let not_pk =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/clearhate", 0)
                .expect("clearhate command should be recognized");
        assert_eq!(not_pk.messages, vec!["Hate list has been erased."]);
        assert!(player.has_pk_hate_for(8));

        world
            .characters
            .get_mut(&CharacterId(7))
            .unwrap()
            .flags
            .insert(CharacterFlags::PK);
        let cleared =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/clearhate", 0)
                .expect("clearhate command should be recognized");
        assert_eq!(cleared.messages, vec!["Hate list has been erased."]);
        assert!(player.pk_hate.is_empty());
    }

    #[test]
    fn pk_playerkiller_command_requires_level_and_paid_before_confirmation() {
        let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
        character.level = 9;
        let mut world = World::default();
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);

        let low_level =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
                .expect("playerkiller command should be recognized");
        assert_eq!(
            low_level.messages,
            vec![
                "Sorry, you may not become a player killer before reaching level 10.",
                "PK is off."
            ]
        );

        let character = world.characters.get_mut(&CharacterId(77)).unwrap();
        character.level = 10;
        let unpaid =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
                .expect("playerkiller command should be recognized");
        assert_eq!(
            unpaid.messages,
            vec![
                "Sorry, only paying players may become player killers.",
                "PK is off."
            ]
        );

        world
            .characters
            .get_mut(&CharacterId(77))
            .unwrap()
            .flags
            .insert(CharacterFlags::PAID);
        let confirm =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
                .expect("playerkiller command should be recognized");
        assert_eq!(confirm.messages.len(), 2);
        assert!(confirm.messages[0].contains("Type: '/iwilldie 77' to confirm."));
        assert_eq!(confirm.messages[1], "PK is off.");
    }

    #[test]
    fn pk_iwilldie_command_toggles_pk_and_clears_ppd_like_state() {
        let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
        character.level = 10;
        character.flags.insert(CharacterFlags::PAID);
        let mut world = World::default();
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.pk_kills = 3;
        player.pk_deaths = 2;
        player.pk_last_kill = 123;
        player.pk_last_death = 456;
        assert!(player.add_pk_hate(999));

        let wrong =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/iwilldie 76", 0)
                .expect("iwilldie command should be recognized");
        assert_eq!(
            wrong.messages,
            vec!["Please type: '/playerkiller' first.", "PK is off."]
        );

        let joined =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/iwilldie 77", 0)
                .expect("iwilldie command should be recognized");
        assert_eq!(joined.messages, vec!["PK is on."]);
        assert!(world
            .characters
            .get(&CharacterId(77))
            .unwrap()
            .flags
            .contains(CharacterFlags::PK));
        assert_eq!(player.pk_kills, 0);
        assert_eq!(player.pk_deaths, 0);
        assert_eq!(player.pk_last_kill, 0);
        assert_eq!(player.pk_last_death, 0);
        assert!(player.pk_hate.is_empty());
    }

    #[test]
    fn pk_playerkiller_leave_respects_tired_and_kill_cooldown() {
        let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
        character.flags.insert(CharacterFlags::PK);
        character.regen_ticker = 10;
        let mut world = World::default();
        world.tick.0 = 20;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);

        let tired =
            apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
                .expect("playerkiller command should be recognized");
        assert_eq!(tired.messages, vec!["Pant, pant. Too tired.", "PK is on."]);

        world.tick.0 = TICKS_PER_SECOND * 4;
        player.pk_last_kill = 60 * 60 * 24 * 27;
        let blocked = apply_pk_hate_command(
            &mut world,
            &mut player,
            CharacterId(77),
            "/playerkiller",
            60 * 60 * 24 * 27,
        )
        .expect("playerkiller command should be recognized");
        assert_eq!(
            blocked.messages,
            vec![
                "You have killed 0.00 days ago, you need to wait 28.00 more days.",
                "PK is on."
            ]
        );

        let left = apply_pk_hate_command(
            &mut world,
            &mut player,
            CharacterId(77),
            "/playerkiller",
            60 * 60 * 24 * 56,
        )
        .expect("playerkiller command should be recognized");
        assert_eq!(left.messages, vec!["PK is off."]);
        assert!(!world
            .characters
            .get(&CharacterId(77))
            .unwrap()
            .flags
            .contains(CharacterFlags::PK));
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
        assert!(cache.slots.iter().all(Option::is_none));
    }

    #[test]
    fn client_effect_payloads_reuse_slot_after_effect_disappears() {
        let login = login_block("Tester");
        let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
        character.x = 10;
        character.y = 10;
        let mut world = World::default();
        let mut first = Effect::new(EF_FIREBALL, 123, 55, 65);
        first.from_x = 10;
        first.from_y = 10;
        first.to_x = 12;
        first.to_y = 10;
        first.x = 11 * 1024 + 512;
        first.y = 10 * 1024 + 512;
        world.effects.insert(123, first);
        let mut cache = ClientEffectCache::default();

        let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
        assert_eq!(payloads[0][1], 0);

        world.effects.clear();
        assert_eq!(
            &client_effect_payloads(&world, &character, 2, &mut cache)[0][..],
            &ugaris_protocol::packet::used_effects(0)[..]
        );

        let mut second = Effect::new(EF_BALL, 124, 56, 66);
        second.from_x = 10;
        second.from_y = 10;
        second.to_x = 12;
        second.to_y = 10;
        second.x = 11 * 1024 + 512;
        second.y = 10 * 1024 + 512;
        world.effects.insert(124, second);

        let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
        assert_eq!(payloads[0][1], 0);
        assert_eq!(&payloads[0][2..10], &[124, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(
            &payloads[1][..],
            &ugaris_protocol::packet::used_effects(1)[..]
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
        let mut depot = AccountDepotState::default();
        let mut stored = test_item(ItemId(201), 4321, ItemFlags::USED | ItemFlags::TAKE);
        stored.name = "Depot Relic".to_string();
        depot.slots[4] = Some(stored);

        let request = character_save_request(&world, &player, &character, Some(&depot), 1, 2);

        assert_eq!(request.items.len(), 2);
        assert!(request.items.iter().any(|item| item.id == ItemId(101)));
        assert!(request.items.iter().any(|item| item.id == ItemId(102)));
        assert!(matches!(
            request.mode,
            ugaris_db::character::CharacterSaveMode::Logout { mirror: 2, .. }
        ));
        let mut decoded = PlayerRuntime::connected(6, 0);
        assert!(decoded.decode_legacy_ppd_blob(&request.ppd_blob));
        assert_eq!(decoded.keyring.len(), 1);
        assert_eq!(decoded.chest_last_access_seconds(9), 1234);
        let decoded_depot = decode_legacy_account_depot_subscriber_blob(&request.subscriber_blob)
            .expect("account depot subscriber block");
        assert_eq!(decoded_depot.slots[0].as_ref().unwrap().name, "Depot Relic");
    }

    #[test]
    fn character_save_request_persists_runtime_transport_mirror() {
        let login = login_block("Tester");
        let character = login_character(CharacterId(7), &login, 1, 10, 10);
        let mut world = World::default();
        world.add_character(character.clone());
        let mut player = PlayerRuntime::connected(5, 0);
        player.set_current_mirror(9);

        let request = character_save_request(&world, &player, &character, None, 1, 2);

        assert!(matches!(
            request.mode,
            ugaris_db::character::CharacterSaveMode::Logout { mirror: 9, .. }
        ));
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
    fn hurt_events_add_pk_hate_and_clear_lag_for_valid_player_hit() {
        let mut world = World::default();
        let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
        target
            .flags
            .insert(CharacterFlags::PK | CharacterFlags::LAG);
        target.level = 10;
        let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 12;
        world.add_character(target);
        world.add_character(attacker);

        let mut runtime = ServerRuntime::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        runtime.players.insert(1, player);

        world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

        assert_eq!(
            apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123),
            1
        );
        assert!(runtime
            .player_for_character(CharacterId(1))
            .unwrap()
            .has_pk_hate_for(2));
        assert!(!world
            .characters
            .get(&CharacterId(1))
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));
    }

    #[test]
    fn hurt_events_respect_legacy_pk_hate_level_gate() {
        let mut world = World::default();
        let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
        target
            .flags
            .insert(CharacterFlags::PK | CharacterFlags::LAG);
        target.level = 10;
        let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 14;
        world.add_character(target);
        world.add_character(attacker);

        let mut runtime = ServerRuntime::default();
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        runtime.players.insert(1, player);

        world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

        assert_eq!(
            apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123),
            0
        );
        assert!(!runtime
            .player_for_character(CharacterId(1))
            .unwrap()
            .has_pk_hate_for(2));
        assert!(world
            .characters
            .get(&CharacterId(1))
            .unwrap()
            .flags
            .contains(CharacterFlags::LAG));
    }

    #[test]
    fn lethal_pk_hurt_events_update_kill_and_death_counters() {
        let mut world = World::default();
        let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
        target.flags.insert(CharacterFlags::PK);
        target.level = 10;
        target.hp = 100;
        let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 11;
        world.add_character(target);
        world.add_character(attacker);

        let mut runtime = ServerRuntime::default();
        let mut target_player = PlayerRuntime::connected(1, 0);
        target_player.character_id = Some(CharacterId(1));
        let mut attacker_player = PlayerRuntime::connected(2, 0);
        attacker_player.character_id = Some(CharacterId(2));
        runtime.players.insert(1, target_player);
        runtime.players.insert(2, attacker_player);

        world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 1_000, 1, 0, 0);

        assert_eq!(
            apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 12_345),
            1
        );
        let target_player = runtime.player_for_character(CharacterId(1)).unwrap();
        assert_eq!(target_player.pk_deaths, 1);
        assert_eq!(target_player.pk_last_death, 12_345);
        let attacker_player = runtime.player_for_character(CharacterId(2)).unwrap();
        assert_eq!(attacker_player.pk_kills, 1);
        assert_eq!(attacker_player.pk_last_kill, 12_345);
    }

    #[test]
    fn retained_effect_policy_removes_stale_pk_hate_when_level_gate_fails() {
        let mut attacker = login_character(CharacterId(1), &login_block("Attacker"), 1, 10, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 10;
        let mut target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::PK);
        target.level = 14;
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.add_pk_hate(2));

        remove_stale_pvp_hate_if_effect_check_fails(&mut player, &attacker, &target, 2);

        assert!(!player.has_pk_hate_for(2));
    }

    #[test]
    fn retained_effect_policy_preserves_hate_for_area_one_town_block() {
        let mut attacker = login_character(CharacterId(1), &login_block("Attacker"), 1, 10, 10);
        attacker.flags.insert(CharacterFlags::PK);
        attacker.level = 10;
        let mut target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
        target.flags.insert(CharacterFlags::PK);
        target.level = 14;
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.add_pk_hate(2));

        remove_stale_pvp_hate_if_effect_check_fails(&mut player, &attacker, &target, 1);

        assert!(player.has_pk_hate_for(2));
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
                world.tick_effects_with_attack_policy(|caster_id, caster, target, map| {
                    if let Some(player) = runtime.player_for_character_mut(caster_id) {
                        let attack_policy = RuntimePlayerAttackPolicy { attacker_runtime: &*player };
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
                            let character_flags = world
                                .characters
                                .get(&character_id)
                                .map(|character| character.flags)
                                .unwrap_or_else(CharacterFlags::empty);
                            if let Some(result) = apply_weather_command(
                                &world,
                                character_id,
                                config.area_id,
                                WeatherState::default(),
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
                                &runtime,
                                character_id,
                                &command,
                                current_tick,
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
                            if let Some(result) = apply_who_command(&world, character_flags, &command) {
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
                        ClientAction::GetQuestLog => {
                            if let Some(player) = runtime.players.get(&session_id) {
                                let payload = legacy_questlog_payload(player);
                                runtime.send_to_session(session_id, payload);
                            }
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
                    let mut name_sessions = 0;
                    command_name_refresh.sort_unstable_by_key(|id| id.0);
                    command_name_refresh.dedup();
                    for character_id in command_name_refresh {
                        let Some(character) = world.characters.get(&character_id).cloned() else {
                            continue;
                        };
                        let payload = character_name_packet(&character);
                        for session_id in runtime.refresh_known_character_name(&character) {
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
                let mut completed_actions = world.tick_basic_actions_with_attack_policy(|caster_id, caster, target, map| {
                    if let Some(player) = runtime.player_for_character_mut(caster_id) {
                        let attack_policy = RuntimePlayerAttackPolicy { attacker_runtime: &*player };
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
                                        ugaris_core::item_driver::ItemDriverOutcome::FoodEaten { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StatScrollUsed { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DoubleDoorToggle { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::Teleport { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::TeleportDoor { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::Recall { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CityRecall { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::FireballMachineProjectile { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::BallTrapProjectile { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::EdemonBallProjectile { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderChanged { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerPulse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerExtinguished { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapTriggered { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapReset { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TriggerMapItem { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::StepTrapDiscoverTarget { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LightChanged { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::PalaceGateTick { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::DecayItemToggled { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitAnimating { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitExpired { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LabExitUse { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BeyondPotion { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::OxygenPotion { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::EnchantCursorItem { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::AntiEnchantCursorItem { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyAssemble { .. }
                                         | ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble { final_key: false, .. }
                                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyCombine { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::AccountDepotOpened { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::LookItem { .. } => {
                                            executed += 1;
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
                                        ugaris_core::item_driver::ItemDriverOutcome::BoneBridgePlace { .. }
                                        | ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeTimerTick { .. } => {
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

                let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                let pk_hate_updates =
                    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, realtime_seconds);
                if pk_hate_updates != 0 {
                    info!(pk_hate_updates, tick = world.tick.0, "applied PK hate updates from hurt events");
                }

                let world_text_sessions = send_pending_world_system_texts(&mut runtime, &mut world);
                if world_text_sessions != 0 {
                    info!(world_text_sessions, tick = world.tick.0, "queued world system text feedback");
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
                        if let Some(player) = runtime.players.get_mut(&id.0) {
                            player.set_current_mirror(u32::from(config.mirror_id));
                        }
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
                                        player.set_current_mirror(mirror.max(0) as u32);
                                    }
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
                        let account_depot = runtime
                            .players
                            .get(&id.0)
                            .and_then(|player| player.character_id)
                            .and_then(|character_id| runtime.account_depots.get(&character_id).cloned());
                        if let Some(player) = runtime.disconnect(id.0) {
                            if let Some(character_id) = player.character_id {
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
