use super::*;
use crate::character_driver::{mem_add_driver, parse_bank_driver_args, BankDriverData, CDR_BANK};
use crate::world::bank::{BankEvent, BANK_TALK_INTERVAL_TICKS};

fn bank_npc(id: u32) -> Character {
    let mut bank = character(id);
    bank.name = "Scrooge".into();
    bank.driver = CDR_BANK;
    bank.driver_state = Some(CharacterDriverState::Bank(BankDriverData::default()));
    bank
}

fn player(id: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player
}

#[test]
fn bank_driver_args_parse_c_fields_and_defaults() {
    let data = parse_bank_driver_args(
        "dir=2;dayx=10;dayy=11;daydir=1;nightx=20;nighty=21;nightdir=3;\
         storefx=5;storefy=6;storetx=8;storety=9;doorx=7;doory=7;",
    );
    assert_eq!(data.dir, 2);
    assert_eq!(data.dayx, 10);
    assert_eq!(data.dayy, 11);
    assert_eq!(data.daydir, 1);
    assert_eq!(data.nightx, 20);
    assert_eq!(data.nighty, 21);
    assert_eq!(data.nightdir, 3);
    assert_eq!(data.storefx, 5);
    assert_eq!(data.storefy, 6);
    assert_eq!(data.storetx, 8);
    assert_eq!(data.storety, 9);
    assert_eq!(data.doorx, 7);
    assert_eq!(data.doory, 7);
    // C `bank_driver` defaults opening hours to 6..23 before parsing.
    assert_eq!(data.open, 6);
    assert_eq!(data.close, 23);
}

#[test]
fn bank_greets_visible_players_once() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    world.process_bank_actions(0);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Hello Godmode!"));
    assert!(texts[0]
        .message
        .contains("open an account with the Imperial Bank?"));

    // Second pass: memory suppresses the repeat greeting.
    world.process_bank_actions(0);
    assert!(world.drain_pending_area_texts().is_empty());
}

fn bank_npc_already_greeted(id: u32, greeted: u32) -> Character {
    let mut bank = bank_npc(id);
    mem_add_driver(&mut bank.driver_memory, 7, greeted);
    bank
}

#[test]
fn bank_replies_to_small_talk_keyword() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(bank_npc_already_greeted(1, 2), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message.contains("Hello, Godmode!")),
        "expected a qa reply among {texts:?}"
    );
}

#[test]
fn bank_account_qa_explains_deposit_withdraw_and_balance_without_color_markers() {
    // C wraps the referenced keywords in COL_LIGHT_BLUE/COL_RESET; this
    // port drops the color styling (see `character_driver::BANK_QA`'s doc
    // comment) but keeps the wording byte-for-byte identical otherwise.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(bank_npc_already_greeted(1, 2), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "account");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| {
        text.message.contains("first deposit (explain deposit)")
            && text.message.contains("balance (explain balance)")
            && text.message.contains("withdraw (explain withdraw)")
    }));
}

#[test]
fn bank_deposit_charges_carried_gold_and_queues_ppd_credit() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    let mut visitor = player(2);
    visitor.gold = 10_000;
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "deposit 38");
    }
    world.process_bank_actions(0);

    // C: `ch[co].gold -= val` where `val = atoi("38") * 100`.
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 6_200);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou hast deposited 38 gold coins.")));
    let events = world.drain_pending_bank_events();
    assert_eq!(
        events,
        vec![BankEvent::Deposit {
            player_id: CharacterId(2),
            amount: 3_800,
        }]
    );
}

#[test]
fn bank_deposit_rejects_insufficient_carried_gold() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    let mut visitor = player(2);
    visitor.gold = 100;
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "deposit 38");
    }
    world.process_bank_actions(0);

    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou dost not have that much gold.")));
    assert!(world.drain_pending_bank_events().is_empty());
}

#[test]
fn bank_deposit_without_amount_asks_to_name_one() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    assert!(world.spawn_character(player(2), 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "deposit");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou must name an amount.")));
}

#[test]
fn bank_withdraw_queues_ppd_check_without_local_gold_mutation() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    assert!(world.spawn_character(player(2), 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "withdraw 38");
    }
    world.process_bank_actions(0);

    // C `bank_driver` cannot decide the withdraw outcome without
    // `ppd->imperial_gold` (session-owned `PlayerRuntime`, outside
    // `World`); the carried-gold credit only happens once that side
    // applies the event.
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 0);
    let events = world.drain_pending_bank_events();
    assert_eq!(
        events,
        vec![BankEvent::Withdraw {
            bank_id: CharacterId(1),
            player_id: CharacterId(2),
            amount: 3_800,
        }]
    );
}

#[test]
fn bank_withdraw_without_amount_asks_to_name_one() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    assert!(world.spawn_character(player(2), 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "withdraw");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou must name an amount.")));
    assert!(world.drain_pending_bank_events().is_empty());
}

#[test]
fn bank_withdraw_negative_amount_fails_without_ppd_lookup() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    assert!(world.spawn_character(player(2), 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "withdraw -5");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Thou dost not have that much gold in thine account.")));
    assert!(world.drain_pending_bank_events().is_empty());
}

#[test]
fn bank_balance_request_is_queued_for_ppd_lookup() {
    let mut world = World::default();
    assert!(world.spawn_character(bank_npc(1), 10, 10));
    assert!(world.spawn_character(player(2), 10, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "balance");
    }
    world.process_bank_actions(0);

    let events = world.drain_pending_bank_events();
    assert_eq!(
        events,
        vec![BankEvent::Balance {
            bank_id: CharacterId(1),
            player_id: CharacterId(2),
        }]
    );
}

#[test]
fn bank_explain_deposit_matches_both_qa_and_deposit_paths() {
    // C `strcasestr` matches "deposit" inside "explain deposit" too, so
    // both the qa table's explain reply *and* the deposit "name an
    // amount" reply fire for the same message - preserved as-is.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(bank_npc_already_greeted(1, 2), 10, 10));
    let mut visitor = player(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_text_message(CharacterId(2), "explain deposit");
    }
    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("just say: 'deposit 38'")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou must name an amount.")));
}

#[test]
fn bank_given_item_is_destroyed() {
    let mut world = World::default();
    let mut bank = bank_npc(1);
    bank.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(bank, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(bank) = world.characters.get_mut(&CharacterId(1)) {
        bank.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_bank_actions(0);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
}

/// C `bank_driver`'s idle-murmur block (`bank.c:459-480`): once per
/// minute, on a `RANDOM(25)` 1-in-25 hit, murmur a random line. Seeds are
/// pinned to precomputed values landing on a known `(RANDOM(25),
/// RANDOM(16))` pair, matching the pattern `world/tests/merchant.rs` uses.
#[test]
fn bank_idle_chatter_murmurs_on_lucky_roll() {
    let mut world = World::default();
    // seed=546: RANDOM(25) == 0 (hit), RANDOM(16) == 0 ("I love the
    // clicking of coins.").
    world.legacy_random_seed = 546;
    world.tick = Tick(BANK_TALK_INTERVAL_TICKS + 1);
    assert!(world.spawn_character(bank_npc(1), 10, 10));

    world.process_bank_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message == "Scrooge murmurs: \"I love the clicking of coins.\""),
        "expected the case-0 murmur among {texts:?}"
    );
    let bank = world.characters.get(&CharacterId(1)).unwrap();
    match bank.driver_state.as_ref() {
        Some(CharacterDriverState::Bank(data)) => {
            assert_eq!(data.last_talk, BANK_TALK_INTERVAL_TICKS + 1);
        }
        _ => panic!("expected bank driver state"),
    }
}

#[test]
fn bank_idle_chatter_stays_quiet_below_talk_interval() {
    let mut world = World::default();
    world.tick = Tick(BANK_TALK_INTERVAL_TICKS);
    world.legacy_random_seed = 546;
    assert!(world.spawn_character(bank_npc(1), 10, 10));

    world.process_bank_actions(0);

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn bank_without_day_positions_returns_to_spawn_tile_and_faces_configured_dir() {
    // C: `dat->dayx == 0` -> `move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, 0)`
    // then `turn(cn, dat->dir)` (`bank.c:450-457`).
    let mut world = World::default();
    let mut bank = bank_npc(1);
    bank.driver_state = Some(CharacterDriverState::Bank(BankDriverData {
        dir: 2,
        ..BankDriverData::default()
    }));
    assert!(world.spawn_character(bank, 10, 10));
    // `rest_x/rest_y` (C `ch.tmpx/tmpy`) is set to the spawn tile by
    // `spawn_character`; the bank is already there, so no walk fires and
    // it should just turn to face `dir`.
    world.process_bank_actions(0);

    let bank = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(bank.dir, 2);
}

#[test]
fn bank_opening_hours_gate_day_vs_night_position() {
    use crate::world::bank::bank_opening_time;
    // C `opening_time(from, to)` (`bank.c:276-290`).
    assert!(bank_opening_time(6, 23, 12));
    assert!(!bank_opening_time(6, 23, 4));
    // Wrap-around (from > to): open overnight.
    assert!(bank_opening_time(22, 4, 23));
    assert!(bank_opening_time(22, 4, 2));
    assert!(!bank_opening_time(22, 4, 12));
}
