// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::character_driver::{
    mem_add_driver, parse_aclerk_driver_args, AclerkDriverData, CDR_ACLERK,
};
use crate::world::aclerk::ACLERK_TALK_INTERVAL_TICKS;

fn aclerk_npc(id: u32, pricemulti: i32) -> Character {
    let mut aclerk = character(id);
    aclerk.name = "Aravon".into();
    aclerk.driver = CDR_ACLERK;
    aclerk.driver_state = Some(CharacterDriverState::Aclerk(AclerkDriverData {
        pricemulti,
        ..AclerkDriverData::default()
    }));
    aclerk
}

fn player(id: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player
}

#[test]
fn aclerk_driver_args_parse_c_fields_with_default_hours() {
    let data = parse_aclerk_driver_args("dir=3;pricemulti=600;ignore=2;special=1;");
    assert_eq!(data.dir, 3);
    assert_eq!(data.pricemulti, 600);
    assert_eq!(data.ignore, 2);
    assert_eq!(data.special, 1);
    // C `aclerk_driver`: `dat->open = 6; dat->close = 23;` before parsing.
    assert_eq!(data.open, 6);
    assert_eq!(data.close, 23);
}

#[test]
fn aclerk_store_created_from_carried_inventory_like_merchant() {
    // C: `aclerk_driver` calls the same `create_store` as `merchant_driver`.
    let mut world = World::default();
    let mut aclerk = aclerk_npc(1, 600);
    aclerk.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(aclerk, 10, 10));
    let mut ware = item(900, ItemFlags::TAKE);
    ware.value = 500;
    ware.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), ware);

    assert!(world.ensure_merchant_store(CharacterId(1)));

    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store.price_multi, 600);
    assert!(store.wares[0].as_ref().unwrap().always);
    match world.characters.get(&CharacterId(1)).unwrap().driver_state {
        Some(CharacterDriverState::Aclerk(ref data)) => assert!(data.store_created),
        _ => panic!("expected aclerk driver state"),
    }
}

#[test]
fn aclerk_greets_visible_players_once_within_five_tiles() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    world.process_aclerk_actions();
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Welcome to the Cameron Arena!"));
    // C: the greeting never mentions the visitor by name (unlike merchant).
    assert!(!texts[0].message.contains("Godmode"));

    // Second pass: memory suppresses the repeat greeting.
    world.process_aclerk_actions();
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn aclerk_does_not_greet_players_beyond_five_tiles() {
    // C: `if (char_dist(cn, co) > 5) { remove_message(...); continue; }`.
    let mut world = World::default();
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 16, 10));

    world.process_aclerk_actions();

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn aclerk_trade_text_never_opens_store_unlike_merchant() {
    // C: `aclerk_driver`'s `NT_TEXT` handler never sets
    // `ch[co].merchant = cn` - only `merchant_driver` does that.
    let mut world = World::default();
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));
    assert!(world.spawn_character(player(2), 11, 10));

    if let Some(aclerk) = world.characters.get_mut(&CharacterId(1)) {
        aclerk.push_driver_text_message(CharacterId(2), "Aravon, trade!");
    }
    world.process_aclerk_actions();

    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().merchant,
        None
    );
}

#[test]
fn aclerk_reacts_to_abuser_trade_request() {
    // C `abuser(ch[co].ID)`: the hardcoded ID list reacts with a
    // murmur/emote on a "<name> ... trade" message.
    let mut world = World::default();
    world.legacy_random_seed = 1; // RANDOM(3) == 0 -> "I hate cheaters."
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));
    let speaker = player(676); // first entry in the C `abuser()` list
    assert!(world.spawn_character(speaker, 11, 10));

    if let Some(aclerk) = world.characters.get_mut(&CharacterId(1)) {
        aclerk.push_driver_text_message(CharacterId(676), "Aravon, trade!");
    }
    world.process_aclerk_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message.contains("I hate cheaters.")),
        "expected an abuser reaction among {texts:?}"
    );
}

#[test]
fn aclerk_ignores_non_abuser_trade_request() {
    let mut world = World::default();
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));
    assert!(world.spawn_character(player(2), 11, 10));

    if let Some(aclerk) = world.characters.get_mut(&CharacterId(1)) {
        aclerk.push_driver_text_message(CharacterId(2), "Aravon, trade!");
    }
    world.process_aclerk_actions();

    // A non-abuser speaker gets no cheater reaction (unrelated welcome
    // greetings from the same tick are fine - only the abuser murmur/emote
    // texts are excluded here).
    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(
        |text| text.message.contains("cheater") || text.message.contains("clenches his fists")
    ));
}

#[test]
fn aclerk_given_item_vanishes() {
    // C: `if (msg->type == NT_GIVE) { ... destroy_item(ch[cn].citem); }`.
    let mut world = World::default();
    let mut aclerk = aclerk_npc(1, 400);
    aclerk.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(aclerk, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(aclerk) = world.characters.get_mut(&CharacterId(1)) {
        aclerk.push_driver_message(crate::character_driver::NT_GIVE, 2, 0, 0);
    }
    world.process_aclerk_actions();

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
}

#[test]
fn aclerk_idle_chatter_murmurs_on_lucky_roll() {
    let mut world = World::default();
    // seed=80: RANDOM(25) == 0 (hit), RANDOM(11) == 0 (case 0 murmur).
    world.legacy_random_seed = 80;
    world.tick = Tick(ACLERK_TALK_INTERVAL_TICKS + 1);
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));

    world.process_aclerk_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message == "Aravon murmurs: \"Oh, these sand fleas are a nuisance.\""),
        "expected the case-0 murmur among {texts:?}"
    );
    let aclerk = world.characters.get(&CharacterId(1)).unwrap();
    match aclerk.driver_state.as_ref() {
        Some(CharacterDriverState::Aclerk(data)) => {
            assert_eq!(data.last_talk, ACLERK_TALK_INTERVAL_TICKS + 1);
        }
        _ => panic!("expected aclerk driver state"),
    }
}

#[test]
fn aclerk_idle_chatter_skips_unlucky_roll() {
    let mut world = World::default();
    // seed=17: RANDOM(25) == 1, missing the 1-in-25 hit.
    world.legacy_random_seed = 17;
    world.tick = Tick(ACLERK_TALK_INTERVAL_TICKS + 1);
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));

    world.process_aclerk_actions();

    assert!(world.drain_pending_area_texts().is_empty());
    let aclerk = world.characters.get(&CharacterId(1)).unwrap();
    match aclerk.driver_state.as_ref() {
        Some(CharacterDriverState::Aclerk(data)) => assert_eq!(data.last_talk, 0),
        _ => panic!("expected aclerk driver state"),
    }
}

#[test]
fn aclerk_idle_chatter_stays_quiet_below_talk_interval() {
    let mut world = World::default();
    world.tick = Tick(ACLERK_TALK_INTERVAL_TICKS);
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));

    world.process_aclerk_actions();

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn aclerk_idle_chatter_outdoor_emote_doubles_embedded_period() {
    // C: `emote(cn, "eyeballs deep within the forest.")` already ends in a
    // period, and `emote()`'s own `"%s %s."` wrapper adds a second one.
    let mut world = World::default();
    // seed=118: RANDOM(25) == 0 (hit), RANDOM(11) == 9 (outdoor emote).
    world.legacy_random_seed = 118;
    world.tick = Tick(ACLERK_TALK_INTERVAL_TICKS + 1);
    assert!(world.spawn_character(aclerk_npc(1, 400), 10, 10));

    world.process_aclerk_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message == "Aravon eyeballs deep within the forest.."),
        "expected the doubled-period outdoor emote among {texts:?}"
    );
}

#[test]
fn aclerk_memory_clears_after_twelve_hours() {
    // C: `if (ticker > dat->memcleartimer) { mem_erase_driver(cn, 7); ... }`.
    let mut world = World::default();
    let mut aclerk = aclerk_npc(1, 400);
    mem_add_driver(&mut aclerk.driver_memory, 7, 2);
    if let Some(CharacterDriverState::Aclerk(data)) = aclerk.driver_state.as_mut() {
        data.memory_clear_tick = 5;
    }
    assert!(world.spawn_character(aclerk, 10, 10));
    world.tick = Tick(6);

    world.process_aclerk_actions();

    assert!(!crate::character_driver::mem_check_driver(
        &world.characters.get(&CharacterId(1)).unwrap().driver_memory,
        7,
        2
    ));
}
