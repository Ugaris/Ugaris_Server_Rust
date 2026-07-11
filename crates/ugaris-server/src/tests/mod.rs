// Legacy PPD/subscriber-blob decoders are `#[deprecated]` (migration 0020's
// `player_state_json` is authoritative now) but several tests exercise them
// directly to lock down the pre-0020 read-fallback byte layout - see the
// "Retire legacy blob writes" PORTING_TODO.md task.
#![allow(deprecated)]

use super::*;

mod achievement;
mod area8_army;
mod area_apply;
mod auction;
mod chests;
mod clan_command;
mod clan_log;
mod commands_admin;
mod commands_chat;
mod commands_player;
mod containers;
mod cross_area;
mod depot;
mod dungeon;
mod effects_sync;
mod events;
mod inventory;
mod item_apply;
mod keyring;
mod lab;
mod login;
mod lostcon;
mod macro_daemon;
mod map_sync;
mod merchants;
mod military;
mod mine;
mod pents;
mod player_actions;
mod resource_sync;
mod server_misc;
mod shutdown;
mod snapshots;
mod spawns;
mod stacks;
mod strategy;
mod transport;
mod tutorial;
mod weather;
mod world_events;
mod xmas;
mod zone;

use ugaris_protocol::packet::{
    MAP_CHARACTER_ACTION, MAP_CHARACTER_SPRITE, MAP_CHARACTER_STATUS, MAP_EFFECT_0, MAP_EFFECT_1,
    MAP_EFFECT_2, MAP_EFFECT_3, MAP_TILE_FLAGS, MAP_TILE_FSPRITE, MAP_TILE_GSPRITE,
    MAP_TILE_ISPRITE, SV_CONCNT, SV_CONNAME, SV_CONTAINER, SV_CONTYPE, SV_GOLD, SV_LOGINDONE,
    SV_MAP01, SV_MAP10, SV_MAP11, SV_MAPPOS, SV_MIRROR, SV_ORIGIN, SV_PROTOCOL, SV_QUESTLOG,
    SV_SETCITEM, SV_SETHP, SV_SETITEM, SV_SETVAL0, SV_SETVAL1, SV_SPECIAL, SV_TEXT, SV_TICKER,
};

fn apply_tell_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
) -> Option<TellCommandResult> {
    super::apply_tell_command(world, runtime, sender_id, command, current_tick, 1_000)
}

fn apply_chat_command(
    world: &World,
    runtime: &mut ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    area_id: u16,
) -> Option<ChatCommandResult> {
    super::apply_chat_command(world, runtime, sender_id, command, area_id, 1_000)
}

fn apply_local_speech_command(
    world: &mut World,
    runtime: &ServerRuntime,
    sender_id: CharacterId,
    command: &str,
    current_tick: u64,
) -> Option<ChatCommandResult> {
    let mut runtime = ServerRuntime {
        players: runtime.players.clone(),
        staff_codes: runtime.staff_codes.clone(),
        holler_dist: runtime.holler_dist,
        shout_dist: runtime.shout_dist,
        say_dist: runtime.say_dist,
        emote_dist: runtime.emote_dist,
        quietsay_dist: runtime.quietsay_dist,
        whisper_dist: runtime.whisper_dist,
        holler_cost: runtime.holler_cost,
        shout_cost: runtime.shout_cost,
        ..ServerRuntime::default()
    };
    super::apply_local_speech_command(world, &mut runtime, sender_id, command, current_tick, 1_000)
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

fn test_item_with_driver(id: ugaris_core::ids::ItemId, driver: u16) -> ugaris_core::entity::Item {
    let mut item = test_item(id, 0, ItemFlags::USED | ItemFlags::USE);
    item.driver = driver;
    item
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
