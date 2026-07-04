use super::*;
use crate::character_driver::{TraderDriverData, CDR_TRADER};
use crate::world::trader::TraderEvent;

const TALK_INTERVAL: u64 = TICKS_PER_SECOND * 60;
const TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 60 * 3;

fn trader_npc(id: u32) -> Character {
    let mut trader = character(id);
    trader.name = "Ishtar".into();
    trader.driver = CDR_TRADER;
    trader.driver_state = Some(CharacterDriverState::Trader(TraderDriverData::default()));
    trader
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn trader_state(world: &World, trader_id: CharacterId) -> TraderDriverData {
    match world
        .characters
        .get(&trader_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Trader(data)) => data,
        _ => panic!("expected trader driver state"),
    }
}

#[test]
fn trader_greets_visible_players_once() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.process_trader_actions();
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Hello Godmode!"));
    assert!(texts[0]
        .message
        .contains("I will work as middleman in any deal"));

    // Second pass: memory suppresses the repeat greeting.
    world.process_trader_actions();
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn trader_replies_to_small_talk_keyword() {
    let mut world = World::default();
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message.contains("Hello, Godmode!")),
        "expected a qa reply among {texts:?}"
    );
}

#[test]
fn trader_help_qa_explains_trade_commands_without_color_markers() {
    let mut world = World::default();
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "help");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| {
        text.message.contains("trade with <name>")
            && text.message.contains("stop trade")
            && text.message.contains("accept trade")
            && text.message.contains("show trade")
    }));
}

#[test]
fn trade_with_starts_a_trade_between_named_players() {
    let mut world = World::default();
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "trade with Egbert");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I will handle a trade between Godmode and Egbert")));

    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.state, 1);
    assert_eq!(data.c1_id, Some(CharacterId(2)));
    assert_eq!(data.c2_id, Some(CharacterId(3)));
    assert_eq!(data.timeout, TIMEOUT_TICKS);
}

#[test]
fn trade_with_busy_trader_replies_sorry_busy() {
    let mut world = World::default();
    let mut trader = trader_npc(1);
    trader.driver_state = Some(CharacterDriverState::Trader(TraderDriverData {
        state: 1,
        ..TraderDriverData::default()
    }));
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "trade with Egbert");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Sorry, I am busy.")));
    assert_eq!(trader_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn trade_with_unknown_player_replies_not_around() {
    let mut world = World::default();
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "trade with Nobody");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Sorry, Nobody does not seem to be around.")));
    assert_eq!(trader_state(&world, CharacterId(1)).state, 0);
}

#[test]
fn trade_with_full_inventory_rejects_start() {
    let mut world = World::default();
    assert!(world.spawn_character(trader_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    for slot in godmode.inventory.iter_mut().skip(30) {
        *slot = Some(ItemId(999));
    }
    assert!(world.spawn_character(godmode, 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "trade with Egbert");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("your inventory is too filled to trade")));
    assert_eq!(trader_state(&world, CharacterId(1)).state, 0);
}

fn started_trade_trader(id: u32, c1: u32, c2: u32) -> Character {
    let mut trader = trader_npc(id);
    trader.driver_state = Some(CharacterDriverState::Trader(TraderDriverData {
        state: 1,
        c1_id: Some(CharacterId(c1)),
        c2_id: Some(CharacterId(c2)),
        timeout: TIMEOUT_TICKS,
        ..TraderDriverData::default()
    }));
    trader
}

#[test]
fn give_item_adds_to_correct_side_and_notifies_other_partner() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.cursor_item = Some(ItemId(900));
        trader.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_trader_actions();

    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.c1_items, vec![ItemId(900)]);
    assert!(world
        .items
        .get(&ItemId(900))
        .unwrap()
        .flags
        .contains(ItemFlags::VOID));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());

    let events = world.drain_pending_trader_events();
    assert_eq!(
        events,
        vec![TraderEvent::ItemAddedToTrade {
            notify_id: CharacterId(3),
            giver_name: "Godmode".to_string(),
            item_id: ItemId(900),
        }]
    );
}

#[test]
fn give_item_from_non_trading_player_is_returned() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    assert!(world.spawn_character(player(4, "Stranger"), 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.cursor_item = Some(ItemId(900));
        trader.push_driver_message(NT_GIVE, 4, 0, 0);
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I am not trading at your behalf at the moment, Stranger.")));
    // Item is handed back to the stranger rather than destroyed.
    assert_eq!(
        world.characters.get(&CharacterId(4)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    assert!(world.items.contains_key(&ItemId(900)));
}

#[test]
fn give_item_when_side_already_has_ten_is_returned_not_added() {
    let mut world = World::default();
    let mut trader = started_trade_trader(1, 2, 3);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.c1_items = (1..=10).map(ItemId).collect();
    }
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.cursor_item = Some(ItemId(900));
        trader.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("I cannot trade more than ten items at once")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    assert_eq!(trader_state(&world, CharacterId(1)).c1_items.len(), 10);
}

#[test]
fn stop_trade_returns_items_and_resets_state() {
    let mut world = World::default();
    let mut trader = started_trade_trader(1, 2, 3);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.c1_items = vec![ItemId(900)];
        data.c2_items = vec![ItemId(901)];
    }
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world
        .items
        .insert(ItemId(900), item(900, ItemFlags::TAKE | ItemFlags::VOID));
    world
        .items
        .insert(ItemId(901), item(901, ItemFlags::TAKE | ItemFlags::VOID));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "stop trade");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("The trade is cancelled.")));

    // Items return to their original owners (no side-swap on cancel).
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    assert_eq!(
        world.characters.get(&CharacterId(3)).unwrap().cursor_item,
        Some(ItemId(901))
    );
    assert!(!world
        .items
        .get(&ItemId(900))
        .unwrap()
        .flags
        .contains(ItemFlags::VOID));

    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.state, 0);
    assert!(data.c1_items.is_empty());
    assert!(data.c2_items.is_empty());
}

#[test]
fn stop_trade_from_outsider_is_rejected() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    assert!(world.spawn_character(player(4, "Stranger"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(4), "stop trade");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("not trading on your behalf")));
    assert_eq!(trader_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn accept_trade_needs_both_sides_before_swapping_items() {
    let mut world = World::default();
    let mut trader = started_trade_trader(1, 2, 3);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.c1_items = vec![ItemId(900)];
        data.c2_items = vec![ItemId(901)];
    }
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world
        .items
        .insert(ItemId(900), item(900, ItemFlags::TAKE | ItemFlags::VOID));
    world
        .items
        .insert(ItemId(901), item(901, ItemFlags::TAKE | ItemFlags::VOID));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "accept trade");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Godmode is satisfied with the deal.")));
    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.state, 2);
    assert!(data.c1_ok);
    assert!(!data.c2_ok);
    // Not swapped yet - only one side has accepted.
    assert!(world
        .characters
        .get(&CharacterId(2))
        .unwrap()
        .cursor_item
        .is_none());

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(3), "accept trade");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Deal.")));

    // C `return_items(dat, 1)`: items are swapped between sides.
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(901))
    );
    assert_eq!(
        world.characters.get(&CharacterId(3)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.state, 0);
    assert!(!data.c1_ok && !data.c2_ok);

    // C: "Award Trust But Verify achievement to both traders"
    // (`base.c:4420-4428`) - queued as a `DealCompleted` event for
    // `ugaris-server` to apply.
    let events = world.drain_pending_trader_events();
    assert_eq!(
        events,
        vec![TraderEvent::DealCompleted {
            c1_id: CharacterId(2),
            c2_id: CharacterId(3),
        }]
    );
}

#[test]
fn accept_trade_only_one_side_does_not_queue_deal_completed_event() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "accept trade");
    }
    world.process_trader_actions();

    // Only one side accepted - no deal, no achievement event yet.
    assert!(world.drain_pending_trader_events().is_empty());
}

#[test]
fn accept_trade_as_part_of_a_longer_sentence_asks_for_exact_phrase() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "I accept trade now");
    }
    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("by itself, not as part of a longer sentence")));
    assert!(texts.iter().any(|text| text
        .message
        .contains("No leading or trailing spaces, either.")));
    assert!(!trader_state(&world, CharacterId(1)).c1_ok);
}

#[test]
fn show_trade_queues_trader_event_with_current_items() {
    let mut world = World::default();
    let mut trader = started_trade_trader(1, 2, 3);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.c1_items = vec![ItemId(900)];
        data.c2_items = vec![ItemId(901)];
    }
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));

    if let Some(trader) = world.characters.get_mut(&CharacterId(1)) {
        trader.push_driver_text_message(CharacterId(2), "show trade");
    }
    world.process_trader_actions();

    let events = world.drain_pending_trader_events();
    assert_eq!(
        events,
        vec![TraderEvent::ShowTrade {
            viewer_id: CharacterId(2),
            c1_items: vec![ItemId(900)],
            c2_items: vec![ItemId(901)],
        }]
    );
}

#[test]
fn timeout_cancels_trade_and_returns_items() {
    let mut world = World::default();
    let mut trader = started_trade_trader(1, 2, 3);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.c1_items = vec![ItemId(900)];
    }
    assert!(world.spawn_character(trader, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world
        .items
        .insert(ItemId(900), item(900, ItemFlags::TAKE | ItemFlags::VOID));
    world.tick = Tick(TIMEOUT_TICKS + 1);

    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("The trade is cancelled!")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.state, 0);
    assert!(data.c1_items.is_empty());
}

#[test]
fn timeout_does_not_fire_before_deadline() {
    let mut world = World::default();
    assert!(world.spawn_character(started_trade_trader(1, 2, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 10, 10));
    world.tick = Tick(TIMEOUT_TICKS);

    world.process_trader_actions();

    assert!(!world
        .drain_pending_area_texts()
        .iter()
        .any(|text| text.message.contains("cancelled")));
    assert_eq!(trader_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn trader_idle_chatter_murmurs_on_lucky_roll() {
    let mut world = World::default();
    // seed=118: RANDOM(25) == 0 (hit), RANDOM(12) == 0 (mutterings[0]).
    world.legacy_random_seed = 118;
    world.tick = Tick(TALK_INTERVAL + 1);
    assert!(world.spawn_character(trader_npc(1), 10, 10));

    world.process_trader_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts.iter().any(|text| {
            text.message
            == "Ishtar murmurs: \"Trust is the currency of trade. Well, that and actual currency.\""
        }),
        "expected the case-0 murmur among {texts:?}"
    );
    let data = trader_state(&world, CharacterId(1));
    assert_eq!(data.last_talk, TALK_INTERVAL + 1);
}

#[test]
fn trader_idle_chatter_stays_quiet_below_talk_interval() {
    let mut world = World::default();
    world.tick = Tick(TALK_INTERVAL);
    world.legacy_random_seed = 118;
    assert!(world.spawn_character(trader_npc(1), 10, 10));

    world.process_trader_actions();

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn trader_memory_clears_after_twelve_hours() {
    let mut world = World::default();
    let mut trader = trader_npc(1);
    if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
        data.memory_clear_tick = 5;
    }
    crate::character_driver::mem_add_driver(&mut trader.driver_memory, 7, 2);
    assert!(world.spawn_character(trader, 10, 10));
    world.tick = Tick(6);

    world.process_trader_actions();

    assert!(!crate::character_driver::mem_check_driver(
        &world.characters.get(&CharacterId(1)).unwrap().driver_memory,
        7,
        2
    ));
}
