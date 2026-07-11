use super::*;
use crate::item_driver::{IDR_STR_DEPOT, IDR_STR_MINE, IDR_STR_STORAGE};

fn place_item(world: &mut World, id: u32, driver: u16, x: u16, y: u16) -> ItemId {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.x = x;
    it.y = y;
    world.add_item(it);
    world.map.tile_mut(x as usize, y as usize).unwrap().item = id;
    ItemId(id)
}

fn speaker_at(id: u32, x: u16, y: u16) -> Character {
    Character {
        flags: CharacterFlags::USED | CharacterFlags::MALE,
        x,
        y,
        ..character(id)
    }
}

// --- targeting/addressing gate ---

#[test]
fn message_not_addressed_and_too_far_is_ignored() {
    let world = World::default();
    let speaker = speaker_at(2, 100, 100);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (0, 0), // far away (map_dist > 30)
        1,
        &speaker,
        "Someone says: guard",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert!(messages.is_empty());
}

#[test]
fn message_with_no_number_prefix_and_within_range_is_processed() {
    let world = World::default();
    let speaker = speaker_at(2, 50, 50);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 55),
        1,
        &speaker,
        "Someone says: guard",
    );
    assert_eq!(order, StrategyWorkerOrder::Guard { x: 50, y: 50 });
    assert_eq!(messages.len(), 1);
}

#[test]
fn message_addressed_to_a_different_worker_number_is_ignored() {
    let world = World::default();
    let speaker = speaker_at(2, 50, 50);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        42,
        &speaker,
        "Someone says: 7, guard",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert!(messages.is_empty());
}

#[test]
fn message_addressed_to_this_worker_number_is_processed() {
    let world = World::default();
    let speaker = speaker_at(2, 50, 50);
    let (order, _messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        42,
        &speaker,
        "Someone says: 42, guard",
    );
    assert_eq!(order, StrategyWorkerOrder::Guard { x: 50, y: 50 });
}

// --- simple keyword orders ---

#[test]
fn follow_order_targets_the_speaker() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: follow",
    );
    assert_eq!(order, StrategyWorkerOrder::Follow { leader: speaker.id });
    assert_eq!(messages[0], "nobody, sir, yes, sir, follow, sir!");
}

#[test]
fn fight_order_targets_the_speaker() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: fight",
    );
    assert_eq!(order, StrategyWorkerOrder::Fighter { leader: speaker.id });
    assert_eq!(messages[0], "nobody, sir, yes, sir, fight, sir!");
}

#[test]
fn home_order_resets_to_none() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);
    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::Guard { x: 1, y: 1 },
        (50, 50),
        1,
        &speaker,
        "Boss says: home",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(messages[0], "nobody, sir, yes, sir, go home, sir!");
}

#[test]
fn later_keyword_overrides_earlier_one_in_the_same_message() {
    // C evaluates every `if (strstr(...))` independently, so a message
    // containing multiple keywords lets the last matching one win.
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);
    let (order, _messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: guard and fight",
    );
    assert_eq!(order, StrategyWorkerOrder::Fighter { leader: speaker.id });
}

// --- mine ---

#[test]
fn mine_order_succeeds_when_mine_and_depot_are_found_and_close() {
    let mut world = World::default();
    place_item(&mut world, 10, IDR_STR_MINE, 50, 48);
    place_item(&mut world, 11, IDR_STR_DEPOT, 50, 53);
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: mine",
    );
    assert_eq!(
        order,
        StrategyWorkerOrder::Mine {
            mine_item: ItemId(10),
            depot_item: ItemId(11),
        }
    );
    assert_eq!(messages[0], "sir, nobody, yes, sir, mine, sir!");
}

#[test]
fn mine_order_fails_with_sorry_message_when_no_mine_nearby() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: mine",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(
        messages[0],
        "Sir, nobody, sir, sorry sir, but I cannot find that mine."
    );
}

#[test]
fn mine_order_fails_with_sorry_message_when_no_depot_nearby() {
    let mut world = World::default();
    place_item(&mut world, 10, IDR_STR_MINE, 50, 48);
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: mine",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(
        messages[0],
        "Sir, nobody, sir, sorry sir, but I cannot find a depot."
    );
}

#[test]
fn mine_order_fails_when_mine_and_depot_are_too_far_apart() {
    let mut world = World::default();
    // Mine 9 tiles north of speaker, depot 19 tiles south - both within
    // their own search radius, but 28 tiles apart from each other.
    place_item(&mut world, 10, IDR_STR_MINE, 50, 41);
    place_item(&mut world, 11, IDR_STR_DEPOT, 50, 69);
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: mine",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(
        messages[0],
        "Sir, nobody, sir, sorry sir, but those are too far apart."
    );
}

// --- take (silent no-op on failure) ---

#[test]
fn take_order_succeeds_when_depot_is_found() {
    let mut world = World::default();
    place_item(&mut world, 12, IDR_STR_DEPOT, 50, 48);
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: take",
    );
    assert_eq!(
        order,
        StrategyWorkerOrder::Take {
            depot_item: ItemId(12),
            leader: speaker.id,
        }
    );
    assert_eq!(messages[0], "nobody, sir, yes, sir, take, sir!");
}

#[test]
fn take_order_is_a_silent_no_op_when_no_depot_nearby() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: take",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert!(messages.is_empty());
}

// --- transfer ---

#[test]
fn transfer_order_succeeds_between_two_nearby_depots() {
    let mut world = World::default();
    place_item(&mut world, 13, IDR_STR_DEPOT, 50, 48);
    // Speaker faces up (dy = -1), so the second-depot search radiates
    // from (50, 50 - 16) = (50, 34) - the ring spiral never checks that
    // exact center tile (it starts at dist = 1), so place the storage one
    // tile off it rather than exactly on it.
    place_item(&mut world, 14, IDR_STR_STORAGE, 50, 35);
    let mut speaker = speaker_at(9, 50, 50);
    speaker.dir = Direction::Up as u8;

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: transfer",
    );
    assert_eq!(
        order,
        StrategyWorkerOrder::Transfer {
            from_item: ItemId(13),
            to_item: ItemId(14),
        }
    );
    assert_eq!(messages[0], "nobody, sir, yes, sir, transfer, sir!");
}

#[test]
fn transfer_order_fails_with_sorry_when_no_first_depot() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: transfer",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(
        messages[0],
        "Sir, nobody, sir, sorry sir, but I cannot find the first depot."
    );
}

// --- train ---

#[test]
fn train_order_succeeds_when_storage_is_found() {
    let mut world = World::default();
    place_item(&mut world, 15, IDR_STR_STORAGE, 50, 48);
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: train",
    );
    assert_eq!(
        order,
        StrategyWorkerOrder::Train {
            storage_item: ItemId(15),
        }
    );
    assert_eq!(messages[0], "nobody, sir, yes, sir, train, sir!");
}

#[test]
fn train_order_fails_with_sorry_when_no_storage_nearby() {
    let world = World::default();
    let speaker = speaker_at(9, 50, 50);

    let (order, messages) = world.strategy_worker_apply_order_text(
        StrategyWorkerOrder::None,
        (50, 50),
        1,
        &speaker,
        "Boss says: train",
    );
    assert_eq!(order, StrategyWorkerOrder::None);
    assert_eq!(
        messages[0],
        "Sir, nobody, sir, sorry sir, but I cannot find a storage."
    );
}

// --- spiral search helpers directly ---

#[test]
fn strategy_find_item_near_matches_only_the_requested_driver() {
    let mut world = World::default();
    place_item(&mut world, 20, IDR_STR_DEPOT, 50, 48);
    place_item(&mut world, 21, IDR_STR_MINE, 50, 47);

    assert_eq!(
        world.strategy_find_item_near(50, 50, IDR_STR_MINE),
        Some(ItemId(21))
    );
    assert_eq!(
        world.strategy_find_item_near(50, 50, IDR_STR_DEPOT),
        Some(ItemId(20))
    );
    assert_eq!(world.strategy_find_item_near(50, 50, IDR_STR_STORAGE), None);
}

#[test]
fn strategy_find_depot_or_storage_near_matches_either_driver() {
    let mut world = World::default();
    place_item(&mut world, 22, IDR_STR_STORAGE, 50, 48);

    assert_eq!(
        world.strategy_find_depot_or_storage_near(50, 50),
        Some(ItemId(22))
    );
}

#[test]
fn strategy_find_item_near_gives_up_outside_the_search_radius() {
    let mut world = World::default();
    // 10 tiles away - just outside finditem's dist < 10 (1..=9) radius.
    place_item(&mut world, 23, IDR_STR_MINE, 50, 40);

    assert_eq!(world.strategy_find_item_near(50, 50, IDR_STR_MINE), None);
}

// --- setname (strategy_train_price/strategy_worker_name/_description) ---

#[test]
fn strategy_train_price_matches_the_c_macro() {
    assert_eq!(strategy_train_price(45), 0);
    assert_eq!(strategy_train_price(55), 100);
    assert_eq!(strategy_train_price(40), -50);
}

#[test]
fn strategy_worker_name_picks_the_right_label_per_order() {
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Mine {
                mine_item: ItemId(1),
                depot_item: ItemId(2)
            },
            "Neutral",
            42
        ),
        "Neutral's Miner 42"
    );
    assert_eq!(
        strategy_worker_name(StrategyWorkerOrder::None, "Neutral", 7),
        "Neutral's Worker 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Follow {
                leader: CharacterId(3)
            },
            "Neutral",
            7
        ),
        "Neutral's Minion 7"
    );
    assert_eq!(
        strategy_worker_name(StrategyWorkerOrder::Guard { x: 1, y: 1 }, "Neutral", 7),
        "Neutral's Guard 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::EternalGuard { x: 1, y: 1 },
            "Neutral",
            7
        ),
        "Neutral's E-Guard 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Fighter {
                leader: CharacterId(3)
            },
            "Neutral",
            7
        ),
        "Neutral's Fighter 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Take {
                depot_item: ItemId(1),
                leader: CharacterId(3)
            },
            "Neutral",
            7
        ),
        "Neutral's Fighter 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Transfer {
                from_item: ItemId(1),
                to_item: ItemId(2)
            },
            "Neutral",
            7
        ),
        "Neutral's Transfer 7"
    );
    assert_eq!(
        strategy_worker_name(
            StrategyWorkerOrder::Train {
                storage_item: ItemId(1)
            },
            "Neutral",
            7
        ),
        "Neutral's Trainee 7"
    );
}

#[test]
fn strategy_worker_description_matches_c_format() {
    assert_eq!(
        strategy_worker_description(120, 30, 55),
        "Carrying 120 Platinum, 30 of 100 exp"
    );
}

// --- findstorage ---

#[test]
fn strategy_find_storage_owned_by_group_matches_owner_code() {
    let mut world = World::default();
    let mut storage_a = item(30, ItemFlags::USED);
    storage_a.driver = IDR_STR_STORAGE;
    set_str_item_owner(&mut storage_a, 5);
    world.add_item(storage_a);

    let mut storage_b = item(31, ItemFlags::USED);
    storage_b.driver = IDR_STR_STORAGE;
    set_str_item_owner(&mut storage_b, 9);
    world.add_item(storage_b);

    assert_eq!(
        world.strategy_find_storage_owned_by_group(9),
        Some(ItemId(31))
    );
    assert_eq!(world.strategy_find_storage_owned_by_group(1234), None);
}

#[test]
fn strategy_find_storage_owned_by_group_ignores_wrong_driver() {
    let mut world = World::default();
    let mut mine = item(32, ItemFlags::USED);
    mine.driver = IDR_STR_MINE;
    set_str_item_owner(&mut mine, 5);
    world.add_item(mine);

    assert_eq!(world.strategy_find_storage_owned_by_group(5), None);
}

// --- restplace ---

#[test]
fn rest_place_returns_base_tile_when_no_cached_offset_and_nothing_blocked() {
    let world = World::default();
    let (offset, pos) = world.strategy_worker_rest_place(CharacterId(1), (50, 50), None);
    // Nearest fallback candidate in search order is (-3, -5).
    assert_eq!(offset, Some((-3, -5)));
    assert_eq!(pos, (47, 45));
}

#[test]
fn rest_place_reuses_a_still_free_cached_offset() {
    let world = World::default();
    let (offset, pos) = world.strategy_worker_rest_place(CharacterId(1), (50, 50), Some((4, 5)));
    assert_eq!(offset, Some((4, 5)));
    assert_eq!(pos, (54, 55));
}

#[test]
fn rest_place_falls_back_when_the_cached_offset_becomes_blocked() {
    let mut world = World::default();
    world.map.set_flags(54, 55, MapFlags::MOVEBLOCK);

    let (offset, pos) = world.strategy_worker_rest_place(CharacterId(1), (50, 50), Some((4, 5)));
    // Falls through to the next free candidate in STRATEGY_REST_OFFSETS.
    assert_eq!(offset, Some((-3, -5)));
    assert_eq!(pos, (47, 45));
}

#[test]
fn rest_place_treats_the_worker_itself_as_a_free_occupant() {
    let mut world = World::default();
    world.map.set_flags(47, 45, MapFlags::TMOVEBLOCK);
    world.map.tile_mut(47, 45).unwrap().character = 1;

    let (offset, pos) = world.strategy_worker_rest_place(CharacterId(1), (50, 50), None);
    assert_eq!(offset, Some((-3, -5)));
    assert_eq!(pos, (47, 45));
}

#[test]
fn rest_place_returns_base_unchanged_when_every_candidate_is_blocked() {
    let mut world = World::default();
    for &(dx, dy) in &[
        (-3, -5),
        (-4, -5),
        (-5, -5),
        (-6, -5),
        (-3, 5),
        (-4, 5),
        (-5, 5),
        (-6, 5),
        (3, -5),
        (4, -5),
        (5, -5),
        (6, -5),
        (3, 5),
        (4, 5),
        (5, 5),
        (6, 5),
        (-3, -3),
        (-4, -3),
        (-5, -3),
        (-6, -3),
        (-3, 3),
        (-4, 3),
        (-5, 3),
        (-6, 3),
        (3, -3),
        (4, -3),
        (5, -3),
        (6, -3),
        (3, 3),
        (4, 3),
        (5, 3),
        (6, 3),
    ] {
        world.map.set_flags(
            (50i32 + dx) as usize,
            (50i32 + dy) as usize,
            MapFlags::MOVEBLOCK,
        );
    }

    let (offset, pos) = world.strategy_worker_rest_place(CharacterId(1), (50, 50), None);
    assert_eq!(offset, None);
    assert_eq!(pos, (50, 50));
}
