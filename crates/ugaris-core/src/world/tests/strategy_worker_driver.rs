use super::*;
use crate::character_driver::{CDR_STRATEGY, NT_CREATE, NT_GIVE, NT_GOTHIT, NT_TEXT};
use crate::item_driver::{IDR_STR_DEPOT, IDR_STR_STORAGE};
use crate::world::npc::area23_24::StrategyWorkerDriverData;

fn worker_npc(id: u32, group: u16) -> Character {
    let mut worker = character(id);
    worker.name = "Neutral's Worker 2".into();
    worker.driver = CDR_STRATEGY;
    worker.group = group;
    worker.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData::default(),
    ));
    worker
}

fn worker_with_order(id: u32, group: u16, order: StrategyWorkerOrder) -> Character {
    let mut worker = worker_npc(id, group);
    worker.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData {
            order,
            ..StrategyWorkerDriverData::default()
        },
    ));
    worker
}

fn worker_state(world: &World, id: CharacterId) -> StrategyWorkerDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::StrategyWorker(data)) => data,
        _ => panic!("expected strategy-worker driver state"),
    }
}

fn place_item(world: &mut World, id: u32, driver: u16, flags: ItemFlags, x: u16, y: u16) -> ItemId {
    let mut it = item(id, flags);
    it.driver = driver;
    it.x = x;
    it.y = y;
    world.items.insert(ItemId(id), it);
    world.map.tile_mut(x as usize, y as usize).unwrap().item = id;
    ItemId(id)
}

#[test]
fn nt_create_sets_level_from_base_wisdom() {
    let mut world = World::default();
    assert!(world.spawn_character(worker_npc(1, 5), 100, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.values[1][CharacterValue::Wisdom as usize] = 42;
        worker.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    world.process_strategy_worker_actions(1);

    assert_eq!(world.characters[&CharacterId(1)].level, 42);
}

#[test]
fn nt_gothit_tracks_victim_and_attacks_once_visible() {
    let mut world = World::default();
    assert!(world.spawn_character(worker_npc(1, 5), 100, 100));
    let mut attacker = character(2);
    attacker.group = 9;
    attacker.flags = CharacterFlags::USED;
    assert!(world.spawn_character(attacker, 101, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        worker_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn nt_gothit_from_same_group_is_not_tracked_as_a_victim() {
    let mut world = World::default();
    assert!(world.spawn_character(worker_npc(1, 5), 100, 100));
    let mut ally = character(2);
    ally.group = 5;
    ally.flags = CharacterFlags::USED;
    assert!(world.spawn_character(ally, 101, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    world.process_strategy_worker_actions(1);

    assert_eq!(worker_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn nt_give_destroys_the_carried_cursor_item() {
    let mut world = World::default();
    let mut worker = worker_npc(1, 5);
    worker.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(worker, 100, 100));
    world.items.insert(ItemId(9), item(9, ItemFlags::USED));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_GIVE, 0, 0, 0);
    }

    world.process_strategy_worker_actions(1);

    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
}

#[test]
fn nt_text_from_matching_group_player_assigns_a_new_order() {
    let mut world = World::default();
    assert!(world.spawn_character(worker_npc(1, 5), 100, 100));
    let mut speaker = character(2);
    speaker.group = 5;
    speaker.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(speaker, 101, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_TEXT, 0, 0, 2);
        worker.driver_messages.last_mut().unwrap().text = Some("1 guard".into());
    }

    world.process_strategy_worker_actions(1);

    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::Guard { x: 101, y: 100 }
    );
}

#[test]
fn nt_text_from_a_different_group_is_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(worker_npc(1, 5), 100, 100));
    let mut speaker = character(2);
    speaker.group = 9;
    speaker.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(speaker, 101, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_TEXT, 0, 0, 2);
        worker.driver_messages.last_mut().unwrap().text = Some("1 guard".into());
    }

    world.process_strategy_worker_actions(1);

    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::None
    );
}

#[test]
fn eternal_guard_ignores_new_order_text() {
    let mut world = World::default();
    let worker = worker_with_order(1, 5, StrategyWorkerOrder::EternalGuard { x: 100, y: 100 });
    assert!(world.spawn_character(worker, 100, 100));
    let mut speaker = character(2);
    speaker.group = 5;
    speaker.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(speaker, 101, 100));
    if let Some(worker) = world.characters.get_mut(&CharacterId(1)) {
        worker.push_driver_message(NT_TEXT, 0, 0, 2);
        worker.driver_messages.last_mut().unwrap().text = Some("1 follow".into());
    }

    world.process_strategy_worker_actions(1);

    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::EternalGuard { x: 100, y: 100 }
    );
}

#[test]
fn guard_order_walks_toward_the_guard_post_when_away_from_it() {
    let mut world = World::default();
    let worker = worker_with_order(1, 5, StrategyWorkerOrder::Guard { x: 110, y: 100 });
    assert!(world.spawn_character(worker, 100, 100));

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(world.characters[&CharacterId(1)].action, action::WALK);
}

#[test]
fn guard_order_idles_once_already_at_the_guard_post() {
    let mut world = World::default();
    let worker = worker_with_order(1, 5, StrategyWorkerOrder::Guard { x: 100, y: 100 });
    assert!(world.spawn_character(worker, 100, 100));

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(world.characters[&CharacterId(1)].action, action::IDLE);
}

#[test]
fn follow_order_gives_up_once_the_leader_is_gone() {
    let mut world = World::default();
    let worker = worker_with_order(
        1,
        5,
        StrategyWorkerOrder::Follow {
            leader: CharacterId(99),
        },
    );
    assert!(world.spawn_character(worker, 100, 100));

    world.process_strategy_worker_actions(1);

    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::None
    );
}

#[test]
fn follow_order_walks_toward_a_distant_leader() {
    let mut world = World::default();
    let worker = worker_with_order(
        1,
        5,
        StrategyWorkerOrder::Follow {
            leader: CharacterId(2),
        },
    );
    assert!(world.spawn_character(worker, 100, 100));
    let mut leader = character(2);
    leader.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(leader, 110, 100));

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(world.characters[&CharacterId(1)].action, action::WALK);
}

#[test]
fn take_order_becomes_fighter_once_the_depot_is_already_owned() {
    let mut world = World::default();
    let depot_id = 7;
    let worker = worker_with_order(
        1,
        5,
        StrategyWorkerOrder::Take {
            depot_item: ItemId(depot_id),
            leader: CharacterId(2),
        },
    );
    assert!(world.spawn_character(worker, 100, 100));
    let mut depot = item(depot_id, ItemFlags::USE);
    depot.driver = IDR_STR_DEPOT;
    set_str_item_owner(&mut depot, 5);
    world.items.insert(ItemId(depot_id), depot);

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::Fighter {
            leader: CharacterId(2)
        }
    );
}

#[test]
fn default_order_walks_toward_the_owning_group_storage_when_far() {
    let mut world = World::default();
    let worker = worker_with_order(1, 5, StrategyWorkerOrder::None);
    assert!(world.spawn_character(worker, 100, 100));
    place_item(&mut world, 7, IDR_STR_STORAGE, ItemFlags::USE, 110, 100);
    set_str_item_owner(world.items.get_mut(&ItemId(7)).unwrap(), 5);

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(world.characters[&CharacterId(1)].action, action::WALK);
}

#[test]
fn train_order_promotes_the_worker_once_enough_exp_accrues() {
    let mut world = World::default();
    let mut worker = worker_npc(1, 5);
    worker.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData {
            order: StrategyWorkerOrder::Train {
                storage_item: ItemId(7),
            },
            max_level: 60,
            platin: 10,
            trainspeed: 10,
            exp: 0,
            ..StrategyWorkerDriverData::default()
        },
    ));
    worker.level = 45;
    assert!(world.spawn_character(worker, 100, 100));
    place_item(&mut world, 7, IDR_STR_STORAGE, ItemFlags::USE, 105, 100);

    let acted = world.process_strategy_worker_actions(1);

    assert_eq!(acted, 1);
    // C `TRAINPRICE(cn) == (level-45)*10 == 0` at level 45, so the very
    // first training tick's `TRAINMULTI`-scaled exp (`10*3==30`) already
    // clears it and levels the worker up once.
    assert_eq!(world.characters[&CharacterId(1)].level, 46);
    assert_eq!(worker_state(&world, CharacterId(1)).platin, 0);
}

#[test]
fn train_order_reverts_to_guard_once_max_level_reached() {
    let mut world = World::default();
    let mut worker = worker_with_order(
        1,
        5,
        StrategyWorkerOrder::Train {
            storage_item: ItemId(7),
        },
    );
    worker.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData {
            order: StrategyWorkerOrder::Train {
                storage_item: ItemId(7),
            },
            max_level: 60,
            ..StrategyWorkerDriverData::default()
        },
    ));
    worker.level = 60;
    assert!(world.spawn_character(worker, 100, 100));

    world.process_strategy_worker_actions(1);

    assert_eq!(
        worker_state(&world, CharacterId(1)).order,
        StrategyWorkerOrder::Guard { x: 100, y: 100 }
    );
}
