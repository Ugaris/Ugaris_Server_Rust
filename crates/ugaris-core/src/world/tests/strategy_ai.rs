use super::*;
use crate::character_driver::CDR_STRATEGY;
use crate::player::StrategyPpd;

fn ai_place(place_type: AiPlaceType, item_id: u32, x: u16, y: u16) -> AiPlace {
    AiPlace::new(place_type, ItemId(item_id), x, y)
}

fn strategy_item(id: u32, driver: u16, drdata: Vec<u8>) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.driver_data = drdata;
    it
}

/// A spawner+storage pair sharing area slot `slot` (`drdata[8]`), with
/// the storage placed directly north of the spawner (`spawner2storage`'s
/// zone-layout convention) - the minimal setup [`World::ai_init`] needs.
fn spawner_and_storage(slot: u8) -> (Item, Item) {
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 10;
    spawner.y = 10;
    spawner.driver_data[8] = slot;

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 10;
    storage.y = 9;
    storage.driver_data[8] = slot;

    (spawner, storage)
}

fn ai_npc(cn: u32, x: u16, y: u16, level: i32) -> AiNpc {
    AiNpc::new(CharacterId(cn), x, y, level)
}

fn char_at(id: u32, x: u16, y: u16, level: u32) -> Character {
    Character {
        x,
        y,
        level,
        ..character(id)
    }
}

// --- AiData::new ---

#[test]
fn ai_data_new_matches_c_ai_init_standard_values() {
    let ad = AiData::new(StrategyPpd::default());
    assert_eq!(ad.worklevel, 1);
    assert_eq!(ad.guard, [-1; AI_MAXGUARD]);
    assert_eq!(ad.nagguard, -1);
    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.free_workers, 0);
    assert!(ad.places.is_empty());
    assert!(ad.npcs.is_empty());
}

#[test]
fn ai_npc_new_matches_c_zero_init_plus_used_free_stamp() {
    let npc = AiNpc::new(CharacterId(5), 10, 20, 30);
    assert_eq!(npc.order, OR_NONE);
    assert_eq!(npc.task, AiTask::Idle);
    assert_eq!(npc.target, 0);
    assert_eq!(npc.current, 0);
    assert_eq!(npc.used, -1);
    assert_eq!(npc.ftarget, 0);
    assert_eq!(npc.walktype, None);
}

#[test]
fn ai_place_new_matches_c_zero_init_plus_sentinel_stamps() {
    let place = ai_place(AiPlaceType::Mine, 7, 40, 50);
    assert_eq!(place.dist, -1);
    assert_eq!(place.parent, -1);
    assert_eq!(place.eguard, -1);
    assert_eq!(place.worker, [-1; AI_MAXWORKER]);
    assert_eq!(place.wcnt, 0);
    assert_eq!(place.threatcount, 0.0);
}

// --- update_npc_place ---

#[test]
fn update_npc_place_stays_when_already_close() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.npcs.push(ai_npc(1, 105, 103, 10));
    ad.update_npc_place(0);
    assert_eq!(ad.npcs[0].current, 0);
}

#[test]
fn update_npc_place_finds_new_place_within_range() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 150, 150));
    ad.npcs.push(ai_npc(1, 152, 148, 10));
    ad.update_npc_place(0);
    assert_eq!(ad.npcs[0].current, 1);
}

#[test]
fn update_npc_place_leaves_current_unchanged_when_no_place_matches() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 150, 150));
    // Far from both places.
    ad.npcs.push(ai_npc(1, 200, 200, 10));
    ad.update_npc_place(0);
    assert_eq!(ad.npcs[0].current, 0);
}

// --- assign_npc / add_worker / add_etguard / add_guard / remove_guard / remove_worker ---

#[test]
fn assign_npc_assigns_mine_task_for_a_mine_place() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;

    assert!(ad.assign_npc(0));
    assert_eq!(ad.npcs[0].task, AiTask::Mine);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.places[0].worker[0], 0);
    assert_eq!(ad.places[0].wcnt, 1);
    assert_eq!(ad.free_workers, 0);
}

#[test]
fn assign_npc_assigns_transfer_task_for_storage_or_depot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Depot, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));

    assert!(ad.assign_npc(0));
    assert_eq!(ad.npcs[0].task, AiTask::Transfer);
}

#[test]
fn assign_npc_returns_false_when_no_free_npc() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    let mut npc = ai_npc(1, 10, 10, 10);
    npc.used = 3; // already busy
    ad.npcs.push(npc);

    assert!(!ad.assign_npc(0));
}

#[test]
fn add_worker_places_explicit_worker_and_task() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;

    ad.add_worker(AiTask::EGuard, 0, 0);
    assert_eq!(ad.npcs[0].task, AiTask::EGuard);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.places[0].wcnt, 1);
    assert_eq!(ad.free_workers, 0);
}

#[test]
fn add_etguard_stations_at_current_place() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 150, 150));
    ad.npcs.push(ai_npc(1, 152, 148, 10));

    ad.add_etguard(0);
    assert_eq!(ad.npcs[0].current, 1);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.places[1].eguard, 0);
}

#[test]
fn add_guard_and_remove_guard_round_trip() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;

    assert!(ad.add_guard(0));
    assert_eq!(ad.guard[0], 0);
    assert_eq!(ad.gcnt, 1);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.free_workers, 0);

    assert!(ad.remove_guard(0));
    assert_eq!(ad.guard[0], -1);
    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.npcs[0].used, -1);
    assert_eq!(ad.npcs[0].task, AiTask::Idle);
    assert_eq!(ad.free_workers, 1);
}

#[test]
fn assign_npc_and_remove_worker_round_trip() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;

    assert!(ad.assign_npc(0));
    assert!(ad.remove_worker(0));
    assert_eq!(ad.npcs[0].task, AiTask::Idle);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.places[0].worker[0], -1);
    assert_eq!(ad.places[0].wcnt, 0);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.free_workers, 1);
}

// --- wantguardcnt ---

#[test]
fn wantguardcnt_matches_c_boundary_table() {
    let ad = AiData::new(StrategyPpd::default());
    assert_eq!(ad.wantguardcnt(0), 0);
    assert_eq!(ad.wantguardcnt(3), 0);
    assert_eq!(ad.wantguardcnt(4), 1);
    assert_eq!(ad.wantguardcnt(5), 2);
    assert_eq!(ad.wantguardcnt(6), 2);
    assert_eq!(ad.wantguardcnt(7), 3);
    assert_eq!(ad.wantguardcnt(20), 10);
}

// --- remove_free_guards ---

#[test]
fn remove_free_guards_recalls_idle_non_nagging_guards() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs.push(ai_npc(2, 20, 20, 10));
    ad.npcs.push(ai_npc(3, 30, 30, 10));
    ad.npcs[0].used = 0;
    ad.npcs[0].target = 5;
    ad.npcs[1].used = 0;
    ad.npcs[1].target = 7;
    ad.npcs[2].used = 3; // busy with a real target, not on standby
    ad.npcs[2].target = 9;
    ad.guard[0] = 0;
    ad.guard[1] = 1;
    ad.guard[2] = 2;
    ad.nagguard = 1; // guard 1 is nagging - must not be recalled

    ad.remove_free_guards();

    assert_eq!(ad.npcs[0].target, 0); // recalled
    assert_eq!(ad.npcs[1].target, 7); // nagging, left alone
    assert_eq!(ad.npcs[2].target, 9); // used != 0, left alone
}

// --- ai_subtask_move ---

#[test]
fn subtask_move_is_noop_when_already_within_five_tiles() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 100, 100));
    let mut npc = ai_npc(1, 103, 102, 10);
    npc.order = 42; // sentinel: must stay untouched
    ad.npcs.push(npc);
    ad.npcs[0].target = 0;

    world.ai_subtask_move(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, 42);
    assert_eq!(ad.npcs[0].walktype, None);
}

#[test]
fn subtask_move_walks_direct_when_within_twenty_tiles_and_reachable() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 115, 100));
    ad.npcs.push(ai_npc(1, 100, 100, 10));
    ad.npcs[0].target = 0;

    world.ai_subtask_move(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 115);
    assert_eq!(ad.npcs[0].or2, 100);
    assert_eq!(ad.npcs[0].walktype, Some(AiWalkType::Direct));
}

#[test]
fn subtask_move_walks_down_toward_target_along_parent_chain() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    // storage(0) <- depot(1) <- mine(2); npc sits at storage(0), target is
    // the mine (2), far enough apart that the direct check fails.
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 60, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 3, 110, 10));
    ad.places[1].parent = 0;
    ad.places[2].parent = 1;
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].target = 2;
    ad.npcs[0].current = 0;

    world.ai_subtask_move(&mut ad, 0);
    // C: walks up from target(2)'s parent chain until it finds `current`
    // (0), then goes to the *previous* step in that chain (depot, 1).
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 60);
    assert_eq!(ad.npcs[0].or2, 10);
    assert_eq!(ad.npcs[0].walktype, Some(AiWalkType::Down));
}

#[test]
fn subtask_move_walks_up_toward_storage_when_off_path() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    // storage(0) <- depot(1); a second, unrelated mine(2) with its own
    // parent(1) - npc sits at the mine (2) but its target is the depot's
    // sibling storage graph, i.e. neither the target nor its parent chain
    // ever passes through place 2, so the NPC is "off path".
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 60, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 3, 200, 200));
    ad.places[1].parent = 0;
    ad.places[2].parent = 1;
    ad.npcs.push(ai_npc(1, 200, 200, 10));
    ad.npcs[0].target = 1; // depot
    ad.npcs[0].current = 2; // mine, off path from depot->storage

    world.ai_subtask_move(&mut ad, 0);
    // Depot's parent chain is depot(1) -> storage(0); `current` (2) never
    // appears in it, so C falls through to "go to storage from own
    // current place": `ap[current].parent` = `ap[2].parent` = 1 (depot).
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 60);
    assert_eq!(ad.npcs[0].or2, 10);
    assert_eq!(ad.npcs[0].walktype, Some(AiWalkType::Up));
}

// --- task_* dispatch ---

#[test]
fn task_idle_sends_worker_to_restplace_when_at_target() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 100, 101, 10));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.npcs.push(ai_npc(1, 100, 101, 10));

    world.ai_task_idle(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    // Rest-place resolves to the first `STRATEGY_REST_OFFSETS` entry
    // that's free on an empty test map (no persisted current offset yet,
    // so the search always starts from the very first candidate).
    assert_eq!(ad.npcs[0].or1, 97);
    assert_eq!(ad.npcs[0].or2, 95);

    // The live worker's own persisted restplace was written back too
    // (auto-vivifying a `StrategyWorkerDriverData` since none existed).
    match world.characters[&worker_id].driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(data.restplace, Some((-3, -5)));
        }
        _ => panic!("expected StrategyWorker driver state to be created"),
    }
}

#[test]
fn task_idle_moves_when_not_at_target() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 10, 10, 10));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 200, 200));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].target = 1; // mine, far away

    world.ai_task_idle(&mut ad, 0);
    // Delegated to subtask_move, which walks via the parent chain (no
    // parent set here, so it falls through to the "go to storage" tail).
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_ne!(ad.npcs[0].walktype, None);
}

#[test]
fn task_take_sets_take_order_with_depot_item_when_at_target() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Depot, 42, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));

    world.ai_task_take(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_TAKE);
    assert_eq!(ad.npcs[0].or1, 42);
    assert_eq!(ad.npcs[0].or2, 0);
}

#[test]
fn task_guard_sets_guard_order_at_place_coordinates() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 55, 66));
    ad.npcs.push(ai_npc(1, 55, 66, 10));

    world.ai_task_guard(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 55);
    assert_eq!(ad.npcs[0].or2, 66);
}

#[test]
fn task_mine_sets_mine_and_transfer_target_items_when_at_target() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 15, 10));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 15, 10, 10));
    ad.npcs[0].target = 1;

    world.ai_task_mine(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_MINE);
    assert_eq!(ad.npcs[0].or1, 2);
    assert_eq!(ad.npcs[0].or2, 1);
}

#[test]
fn task_mine_allows_being_at_targets_parent_too() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 15, 10));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].target = 1; // mine
    ad.npcs[0].current = 0; // storage, the mine's parent

    world.ai_task_mine(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_MINE);
}

#[test]
fn task_mine_moves_when_neither_at_target_nor_its_parent() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 200, 200));
    ad.places[1].parent = 0;
    // A third place the NPC actually sits at - neither the mine target
    // (1) nor its parent (storage, 0).
    ad.places.push(ai_place(AiPlaceType::Depot, 3, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].target = 1;
    ad.npcs[0].current = 2;

    world.ai_task_mine(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_GUARD); // subtask_move's own order
}

#[test]
fn task_transfer_sets_transfer_order_with_item_ids() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 15, 10));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 15, 10, 10));
    ad.npcs[0].target = 1;

    world.ai_task_transfer(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_TRANSFER);
    assert_eq!(ad.npcs[0].or1, 2);
    assert_eq!(ad.npcs[0].or2, 1);
}

#[test]
fn task_train_sets_train_order_with_storage_item() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Storage, 9, 15, 10));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 15, 10, 10));
    ad.npcs[0].target = 1;

    world.ai_task_train(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_TRAIN);
    assert_eq!(ad.npcs[0].or1, 9);
    assert_eq!(ad.npcs[0].or2, 0);
}

#[test]
fn task_fight_sets_guard_order_at_place_not_a_fighter_order() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Storage, 9, 33, 44));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 33, 44, 10));
    ad.npcs[0].target = 1;

    world.ai_task_fight(&mut ad, 0);
    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 33);
    assert_eq!(ad.npcs[0].or2, 44);
}

// --- ai_assign_guards ---

#[test]
fn assign_guards_dispatches_free_high_level_guards_when_enough_threat() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 10, 10, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.npcs.push(ai_npc(1, 10, 10, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // on standby, matching `add_guard`'s own stamp

    let attacked = world.ai_assign_guards(&mut ad, 1, 1.0, 5, false);
    assert!(attacked);
    assert_eq!(ad.npcs[0].ftarget, 1);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
}

#[test]
fn assign_guards_ignores_guards_not_hp_ready() {
    let mut world = World::default();
    let mut character = char_at(1, 10, 10, 20);
    character.values[0][CharacterValue::Hp as usize] = 50;
    character.hp = 0; // not ready (hp below max)
    world.characters.insert(CharacterId(1), character);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.npcs.push(ai_npc(1, 10, 10, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // on standby, matching `add_guard`'s own stamp

    let attacked = world.ai_assign_guards(&mut ad, 1, 1.0, 5, false);
    assert!(!attacked);
    assert_eq!(ad.npcs[0].used, 0); // never dispatched, stays on standby
}

#[test]
fn assign_guards_ragnarok_sends_everyone_regardless_of_readiness() {
    let mut world = World::default();
    let mut character = char_at(1, 10, 10, 20);
    character.hp = 0;
    world.characters.insert(CharacterId(1), character);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.npcs.push(ai_npc(1, 10, 10, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // on standby, matching `add_guard`'s own stamp

    let attacked = world.ai_assign_guards(&mut ad, 1, 1_000_000.0, 5, true);
    assert!(attacked);
    assert_eq!(ad.npcs[0].target, 1);
}

#[test]
fn assign_guards_recalls_already_assigned_guard_when_over_level_gap() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 10, 10, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    let mut npc = ai_npc(1, 10, 10, 5); // AiNpc's cached level is stale/low
    npc.ftarget = 1;
    npc.used = 1;
    ad.npcs.push(npc);
    ad.guard[0] = 0;

    // level requirement (100) far exceeds cached level(5)+5, so this
    // guard must be recalled to standby rather than kept.
    let attacked = world.ai_assign_guards(&mut ad, 1, 1.0, 100, false);
    assert!(!attacked);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].ftarget, 0);
    assert_eq!(ad.npcs[0].used, 0);
}

// --- ai_nag_attack ---

#[test]
fn nag_attack_sends_lowest_level_idle_guard_to_closest_threat() {
    let mut world = World::default();
    world.tick = Tick(1_000_000);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.lastnag = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places[0].threatcount = 5.0;
    ad.places[0].dist = 2;
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.places[1].threatcount = 5.0;
    ad.places[1].dist = 1; // closer than place 0

    ad.npcs.push(ai_npc(1, 10, 10, 30));
    ad.npcs.push(ai_npc(2, 10, 10, 10)); // lower level
    ad.guard[0] = 0;
    ad.guard[1] = 1;

    world.ai_nag_attack(&mut ad);
    assert_eq!(ad.nagguard, 1);
    assert_eq!(ad.nagplace, 1);
    assert_eq!(ad.npcs[1].target, 1);
    assert_eq!(ad.npcs[1].used, 1);
    assert_eq!(ad.lastnag, 1_000_000);
}

#[test]
fn nag_attack_does_nothing_within_cooldown() {
    let mut world = World::default();
    world.tick = Tick(1000);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.lastnag = 999; // just ticked, well within the 5-minute cooldown
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places[0].threatcount = 5.0;
    ad.npcs.push(ai_npc(1, 10, 10, 30));
    ad.npcs.push(ai_npc(2, 10, 10, 10));
    ad.guard[0] = 0;
    ad.guard[1] = 1;

    world.ai_nag_attack(&mut ad);
    assert_eq!(ad.nagguard, -1);
}

// --- AiPreset::to_strategy_ppd ---

#[test]
fn ai_preset_to_strategy_ppd_copies_the_nine_upgrade_fields_only() {
    let ppd = AI_PRESETS[1].to_strategy_ppd(); // "Zakath"
    assert_eq!(ppd.max_worker, 4);
    assert_eq!(ppd.max_level, 60);
    assert_eq!(ppd.trainspeed, 1);
    assert_eq!(ppd.income, 0);
    assert_eq!(ppd.endurance, 0);
    assert_eq!(ppd.warcry, 0);
    assert_eq!(ppd.speed, 0);
    assert_eq!(ppd.eguards, 1);
    assert_eq!(ppd.eguardlvl, 55);
    // Every other field stays at its zero default, matching C's own
    // partial-aggregate-initializer semantics.
    assert_eq!(ppd.exp, 0);
    assert_eq!(ppd.won_cnt, 0);
    assert_eq!(ppd.boss_stage, 0);
}

// --- World::ai_init ---

#[test]
fn ai_init_returns_none_for_a_code_outside_ai_presets_range() {
    let world = World::default();
    assert!(world.ai_init(ItemId(1), STR_OWNER_AI_BASE - 1).is_none());
    assert!(world
        .ai_init(ItemId(1), STR_OWNER_AI_BASE + AI_PRESETS.len() as u32)
        .is_none());
}

#[test]
fn ai_init_returns_none_when_spawner_or_storage_is_missing() {
    let mut world = World::default();
    // No items at all: bad item id.
    assert!(world.ai_init(ItemId(1), STR_OWNER_AI_BASE + 1).is_none());

    // Spawner exists, but nothing sits on the tile directly north of it.
    let (spawner, _storage) = spawner_and_storage(3);
    world.add_item(spawner);
    assert!(world.ai_init(ItemId(1), STR_OWNER_AI_BASE + 1).is_none());
}

#[test]
fn ai_init_seeds_ppd_from_the_matching_ai_preset() {
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(3);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    let code = STR_OWNER_AI_BASE + 1; // "Zakath"
    let ad = world
        .ai_init(ItemId(1), code)
        .expect("ai_init should succeed");
    assert_eq!(ad.ppd.max_worker, 4);
    assert_eq!(ad.ppd.eguardlvl, 55);
    assert_eq!(ad.storage_item, ItemId(2));
    assert_eq!(ad.places.len(), 1);
    assert_eq!(ad.places[0].dist, 0);
    assert_eq!(ad.places[0].parent, -1);
    assert_eq!(ad.places[0].place_type, AiPlaceType::Storage);
}

#[test]
fn ai_init_discovers_mine_and_depot_in_the_same_slot_and_connects_them() {
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(5);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    let mut mine = strategy_item(3, IDR_STR_MINE, vec![0; 10]);
    mine.x = 12;
    mine.y = 9;
    mine.driver_data[8] = 5;
    world.add_item(mine);

    let mut depot = strategy_item(4, IDR_STR_DEPOT, vec![0; 10]);
    depot.x = 11;
    depot.y = 9;
    depot.driver_data[8] = 5;
    world.add_item(depot);

    // A mine/depot pair in a *different* slot must be ignored.
    let mut other_mine = strategy_item(5, IDR_STR_MINE, vec![0; 10]);
    other_mine.x = 50;
    other_mine.y = 50;
    other_mine.driver_data[8] = 9;
    world.add_item(other_mine);

    let ad = world
        .ai_init(ItemId(1), STR_OWNER_AI_BASE + 1)
        .expect("ai_init should succeed");

    // storage (place 0) + mine + depot, in item-id ascending order.
    assert_eq!(ad.places.len(), 3);
    assert_eq!(ad.places[1].place_type, AiPlaceType::Mine);
    assert_eq!(ad.places[1].item, ItemId(3));
    assert_eq!(ad.places[2].place_type, AiPlaceType::Depot);
    assert_eq!(ad.places[2].item, ItemId(4));
    // Every place should have been connected back to storage by the BFS
    // (all within range and on an open default map).
    for place in &ad.places {
        assert_ne!(place.dist, -1, "place {:?} should be connected", place.item);
    }
}

#[test]
fn ai_init_marks_enemy_storage_and_propagates_enemy_possible_up_the_chain() {
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(1);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    // Close enough to storage to connect directly (depth 1, parent =
    // storage).
    let mut depot = strategy_item(4, IDR_STR_DEPOT, vec![0; 10]);
    depot.x = 25;
    depot.y = 9;
    depot.driver_data[8] = 1;
    world.add_item(depot);

    // Too far from storage to connect directly (`dx == 30 >= 20`), but
    // close enough to the depot to connect through it one hop out
    // (depth 2, parent = depot).
    let mut enemy_storage = strategy_item(6, IDR_STR_STORAGE, vec![0; 10]);
    enemy_storage.x = 40;
    enemy_storage.y = 9;
    enemy_storage.driver_data[8] = 1;
    world.add_item(enemy_storage);

    let ad = world
        .ai_init(ItemId(1), STR_OWNER_AI_BASE + 1)
        .expect("ai_init should succeed");

    let enemy_place = ad
        .places
        .iter()
        .find(|p| p.item == ItemId(6))
        .expect("enemy storage should be discovered");
    assert!(enemy_place.enemy_possible);

    let depot_place = ad
        .places
        .iter()
        .find(|p| p.item == ItemId(4))
        .expect("depot should be discovered");
    assert!(
        depot_place.enemy_possible,
        "enemy_possible should propagate up the parent chain through the depot"
    );
    assert!(ad.places[0].enemy_possible, "and all the way to storage");
}

#[test]
fn ai_init_treats_a_same_slot_storage_as_a_partner_not_an_enemy() {
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(2);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    // Same drdata[8] slot as the party's own storage => partner, per C's
    // `it[n].drdata[8] == it[ad->storage_in].drdata[8]` check.
    let mut partner_storage = strategy_item(6, IDR_STR_STORAGE, vec![0; 10]);
    partner_storage.x = 11;
    partner_storage.y = 9;
    partner_storage.driver_data[8] = 2;
    world.add_item(partner_storage);

    let ad = world
        .ai_init(ItemId(1), STR_OWNER_AI_BASE + 1)
        .expect("ai_init should succeed");
    assert_eq!(ad.partner, vec![ItemId(6)]);
}

// --- AiData::register_npc ---
//
// `World::ai_init`'s own roster-discovery loop can never actually find a
// live NPC in the current codebase, since it requires `code` to be in
// the AI range (>= `STR_OWNER_AI_BASE`, so `ai_init`'s own
// `AI_PRESETS`-index lookup succeeds) while `Character::group` is
// narrowed to `u16` (see `World::ai_init`'s own doc comment - the same
// pre-existing, documented gap as `World::str_did_party_lose`). These
// tests cover the classification/registration logic
// `World::ai_init`'s loop delegates to directly, independent of that
// unrelated, already-documented limitation.

#[test]
fn register_npc_classifies_a_fresh_low_level_no_exp_worker_as_idle() {
    let mut ad = AiData::new(StrategyPpd::default());
    let m = ad.register_npc(CharacterId(10), 10, 10, 20, OR_NONE, 0, 0, false);
    assert_eq!(ad.npcs[m].task, AiTask::Idle);
    assert_eq!(ad.npcs[m].used, -1);
    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.etguardcnt, 0);
}

#[test]
fn register_npc_classifies_an_experienced_worker_as_eguard() {
    let mut ad = AiData::new(StrategyPpd::default());
    let m = ad.register_npc(CharacterId(11), 10, 10, 30, OR_NONE, 0, 0, true);
    assert_eq!(ad.npcs[m].task, AiTask::EGuard);
    // The unconditional post-classification reset (`:2423`) undoes
    // `add_guard`'s own `used = 0` stamp.
    assert_eq!(ad.npcs[m].used, -1);
    assert_eq!(ad.gcnt, 1);
}

#[test]
fn register_npc_classifies_a_high_level_worker_as_eguard_even_without_exp() {
    let mut ad = AiData::new(StrategyPpd::default());
    let m = ad.register_npc(CharacterId(12), 10, 10, 60, OR_NONE, 0, 0, false);
    assert_eq!(ad.npcs[m].task, AiTask::EGuard);
    assert_eq!(ad.gcnt, 1);
}

#[test]
fn register_npc_classifies_eternal_guard_order_as_ignore_and_counts_it() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    let m = ad.register_npc(CharacterId(20), 10, 10, 40, OR_ETERNALGUARD, 10, 10, false);
    assert_eq!(ad.npcs[m].task, AiTask::Ignore);
    assert_eq!(ad.npcs[m].order, OR_ETERNALGUARD);
    assert_eq!(ad.etguardcnt, 1);
    // add_etguard stations it at whichever place it's currently standing
    // at - place 0 (storage), here.
    assert_eq!(ad.npcs[m].target, 0);
    assert_eq!(ad.places[0].eguard, 0);
    // Not counted as a roving guard.
    assert_eq!(ad.gcnt, 0);
}

#[test]
fn ai_init_discovers_no_live_roster_today_given_the_group_u16_narrowing_gap() {
    // Documents the real, current limitation noted in `World::ai_init`'s
    // own doc comment: even a live `CDR_STRATEGY` character whose
    // `group` was set from the *same* AI `code` (truncated to `u16`,
    // then zero-extended back for the comparison) can never match,
    // since every valid `ai_init` `code` exceeds `u16::MAX`.
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(4);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    let code = STR_OWNER_AI_BASE + 1;
    let mut worker = character(10);
    worker.driver = CDR_STRATEGY;
    worker.group = code as u16;
    worker.level = 20;
    worker.x = 10;
    worker.y = 10;
    world.add_character(worker);

    let ad = world
        .ai_init(ItemId(1), code)
        .expect("ai_init should succeed");
    assert!(ad.npcs.is_empty());
}

#[test]
fn nag_attack_does_nothing_with_fewer_than_two_idle_guards() {
    let mut world = World::default();
    world.tick = Tick(1_000_000);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.lastnag = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places[0].threatcount = 5.0;
    ad.npcs.push(ai_npc(1, 10, 10, 30));
    ad.guard[0] = 0; // only one idle guard

    world.ai_nag_attack(&mut ad);
    assert_eq!(ad.nagguard, -1);
}

#[test]
fn nag_attack_does_nothing_when_no_place_is_threatened() {
    let mut world = World::default();
    world.tick = Tick(1_000_000);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.lastnag = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 30));
    ad.npcs.push(ai_npc(2, 10, 10, 10));
    ad.guard[0] = 0;
    ad.guard[1] = 1;

    world.ai_nag_attack(&mut ad);
    assert_eq!(ad.nagguard, -1);
}

// --- World::ai_refresh_places ---

fn enemy_strategy_char(id: u32, x: u16, y: u16, level: u32, group: u16) -> Character {
    Character {
        driver: CDR_STRATEGY,
        group,
        ..char_at(id, x, y, level)
    }
}

#[test]
fn ai_refresh_places_updates_platin_and_owned_from_the_live_item() {
    let mut world = World::default();
    let mut place_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    place_item.x = 10;
    place_item.y = 10;
    set_str_item_owner(&mut place_item, 42);
    set_str_item_gold(&mut place_item, 100);
    world.add_item(place_item);

    let mut ad = AiData::new(StrategyPpd::default());
    let mut place = ai_place(AiPlaceType::Storage, 1, 10, 10);
    place.platin = 10;
    ad.places.push(place);

    let result = world.ai_refresh_places(&mut ad, 42);
    // C: platin = platin/2 + drdata4 = 10/2 + 100 = 105.
    assert_eq!(ad.places[0].platin, 105);
    assert!(ad.places[0].owned);
    // No threat this tick.
    assert!(!ad.panic);
    assert_eq!(ad.pplace, -1);
    // Storage (place 0) has gold and no threat, but `cantrain` is false
    // (no `T_EGUARD` npc below `max_level`), so ragnarok stays true.
    assert!(result.ragnarok);
    assert!(result.nogoldleft);
}

#[test]
fn ai_refresh_places_owned_is_false_when_the_item_belongs_to_another_code() {
    let mut world = World::default();
    let mut place_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    place_item.x = 10;
    place_item.y = 10;
    set_str_item_owner(&mut place_item, 7);
    world.add_item(place_item);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    world.ai_refresh_places(&mut ad, 42);
    assert!(!ad.places[0].owned);
}

#[test]
fn ai_refresh_places_detects_nearby_enemy_and_triggers_panic() {
    let mut world = World::default();
    let mut place_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    place_item.x = 100;
    place_item.y = 100;
    world.add_item(place_item);
    world.add_character(enemy_strategy_char(1, 105, 102, 20, 999));

    let mut ad = AiData::new(StrategyPpd::default());
    let mut place = ai_place(AiPlaceType::Storage, 1, 100, 100);
    place.dist = 0; // within default pdist (3)
    ad.places.push(place);

    world.ai_refresh_places(&mut ad, 42);

    // THREAT(cn) * 1.25 = 20^3 * 1.25 = 10000.0.
    assert_eq!(ad.places[0].threatcount, 10_000.0);
    assert_eq!(ad.places[0].threatlevel, 20);
    // threat starts at 0 (decayed), then += 100 + threatlevel.
    assert_eq!(ad.places[0].threat, 120);
    assert!(ad.panic);
    assert_eq!(ad.pplace, 0);
}

#[test]
fn ai_refresh_places_ignores_same_group_and_non_strategy_characters() {
    let mut world = World::default();
    let mut place_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    place_item.x = 100;
    place_item.y = 100;
    world.add_item(place_item);
    // Same party (`group == code`): friendly, not a threat.
    world.add_character(enemy_strategy_char(1, 101, 101, 50, 42));
    // Different group but not a `CDR_STRATEGY` character.
    let mut bystander = char_at(2, 101, 100, 50);
    bystander.driver = 0;
    world.add_character(bystander);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));

    world.ai_refresh_places(&mut ad, 42);
    assert_eq!(ad.places[0].threatcount, 0.0);
    assert!(!ad.panic);
}

#[test]
fn ai_refresh_places_seen_set_is_shared_across_places_in_one_call() {
    let mut world = World::default();
    let mut item1 = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    item1.x = 100;
    item1.y = 100;
    world.add_item(item1);
    let mut item2 = strategy_item(2, IDR_STR_DEPOT, vec![0; 8]);
    item2.x = 101;
    item2.y = 101;
    world.add_item(item2);

    // Within 10 tiles of both places.
    world.add_character(enemy_strategy_char(1, 100, 100, 30, 999));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 101, 101));

    world.ai_refresh_places(&mut ad, 42);

    // The enemy only contributes to the first place (n = 0) that sees
    // it; place 1's scan finds it already `seen`.
    assert!(ad.places[0].threatcount > 0.0);
    assert_eq!(ad.places[1].threatcount, 0.0);
}

#[test]
fn ai_refresh_places_commits_the_closest_untreated_gold_place_into_pdist() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 10;
    storage_item.y = 10;
    world.add_item(storage_item);
    let mut mine_item = strategy_item(2, IDR_STR_MINE, vec![0; 8]);
    mine_item.x = 20;
    mine_item.y = 20;
    set_str_item_gold(&mut mine_item, 50);
    world.add_item(mine_item);

    let mut ad = AiData::new(StrategyPpd::default());
    assert_eq!(ad.pdist, 3);
    let mut storage = ai_place(AiPlaceType::Storage, 1, 10, 10);
    storage.dist = 0;
    ad.places.push(storage);
    let mut mine = ai_place(AiPlaceType::Mine, 2, 20, 20);
    mine.dist = 2;
    ad.places.push(mine);

    world.ai_refresh_places(&mut ad, 42);
    // mindist = 2 (the mine's own dist, since it has gold and no
    // threat); `ad.pdist = min(3, 2) = 2`.
    assert_eq!(ad.pdist, 2);
}

#[test]
fn ai_refresh_places_nongoldleft_and_ragnarok_stay_true_with_no_spare_gold() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 10;
    storage_item.y = 10;
    world.add_item(storage_item);
    let mut depot_item = strategy_item(2, IDR_STR_DEPOT, vec![0; 8]);
    depot_item.x = 20;
    depot_item.y = 20;
    world.add_item(depot_item); // no gold

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 20, 20));

    let result = world.ai_refresh_places(&mut ad, 42);
    assert!(result.ragnarok);
    assert!(result.nogoldleft);
}

#[test]
fn ai_refresh_places_nongoldleft_and_ragnarok_go_false_when_a_non_storage_place_has_gold() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 10;
    storage_item.y = 10;
    world.add_item(storage_item);
    let mut depot_item = strategy_item(2, IDR_STR_DEPOT, vec![0; 8]);
    depot_item.x = 20;
    depot_item.y = 20;
    set_str_item_gold(&mut depot_item, 30);
    world.add_item(depot_item);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 20, 20));

    let result = world.ai_refresh_places(&mut ad, 42);
    // n=1 (depot) has platin != 0 and threat == 0: unconditionally
    // clears both flags.
    assert!(!result.ragnarok);
    assert!(!result.nogoldleft);
}

#[test]
fn ai_refresh_places_ragnarok_stays_true_at_storage_without_cantrain() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 10;
    storage_item.y = 10;
    set_str_item_gold(&mut storage_item, 1000); // plenty of spare gold
    world.add_item(storage_item);

    let mut ppd = StrategyPpd::default();
    ppd.max_level = 50;
    let mut ad = AiData::new(ppd);
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    // No `T_EGUARD` npc below max_level: `cantrain` is false.

    let result = world.ai_refresh_places(&mut ad, 42);
    assert!(result.ragnarok);
}

#[test]
fn ai_refresh_places_ragnarok_goes_false_at_storage_with_cantrain_and_spare_gold() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 10;
    storage_item.y = 10;
    set_str_item_gold(&mut storage_item, 1000); // platin ends up 500, > 2*max_level(50)
    world.add_item(storage_item);

    let mut ppd = StrategyPpd::default();
    ppd.max_level = 50;
    let mut ad = AiData::new(ppd);
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    let mut trainee = ai_npc(1, 10, 10, 20); // level 20 < max_level 50
    trainee.task = AiTask::EGuard;
    ad.npcs.push(trainee);

    let result = world.ai_refresh_places(&mut ad, 42);
    assert!(!result.ragnarok);
}

#[test]
fn ai_refresh_places_projects_threat_between_parent_and_child() {
    let mut world = World::default();
    let mut storage_item = strategy_item(1, IDR_STR_STORAGE, vec![0; 8]);
    storage_item.x = 100;
    storage_item.y = 100;
    world.add_item(storage_item);
    let mut depot_item = strategy_item(2, IDR_STR_DEPOT, vec![0; 8]);
    depot_item.x = 140;
    depot_item.y = 140;
    world.add_item(depot_item);

    // One enemy near storage, a different one near the depot (far
    // enough apart that neither scan sees both).
    world.add_character(enemy_strategy_char(1, 100, 100, 30, 999));
    world.add_character(enemy_strategy_char(2, 140, 140, 50, 999));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 100, 100));
    let mut depot = ai_place(AiPlaceType::Depot, 2, 140, 140);
    depot.parent = 0;
    ad.places.push(depot);

    world.ai_refresh_places(&mut ad, 42);

    let storage_threatcount = ad.places[0].threatcount;
    let depot_threatcount = ad.places[1].threatcount;
    assert!(storage_threatcount > 0.0);
    assert!(depot_threatcount > 0.0);
    // Each place's own threat is projected onto the other.
    assert_eq!(ad.places[0].threatncount, depot_threatcount);
    assert_eq!(ad.places[0].threatnlevel, 50);
    assert_eq!(ad.places[1].threatncount, storage_threatcount);
    assert_eq!(ad.places[1].threatnlevel, 30);
}

// --- AiData::update_guard_list ---

#[test]
fn update_guard_list_counts_qualifying_standby_guards_and_stamps_used() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut guard = ai_npc(1, 10, 10, 20);
    guard.task = AiTask::EGuard;
    guard.used = -1;
    ad.npcs.push(guard);
    ad.guard[0] = 0;

    ad.update_guard_list();
    assert_eq!(ad.gcnt, 1);
    assert_eq!(ad.npcs[0].used, 0);
}

#[test]
fn update_guard_list_evicts_a_slot_whose_npc_is_no_longer_an_eguard() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut reassigned = ai_npc(1, 10, 10, 20);
    reassigned.task = AiTask::Mine; // task-assignment switch moved it away
    ad.npcs.push(reassigned);
    ad.guard[0] = 0;

    ad.update_guard_list();
    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.guard[0], -1);
}

#[test]
fn update_guard_list_evicts_a_slot_already_claimed_by_a_place_this_tick() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut claimed = ai_npc(1, 10, 10, 20);
    claimed.task = AiTask::EGuard;
    claimed.used = 3; // already claimed by place 3 this tick
    ad.npcs.push(claimed);
    ad.guard[0] = 0;

    ad.update_guard_list();
    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.guard[0], -1);
}

// --- AiData::update_nag_guard ---

#[test]
fn update_nag_guard_clears_when_the_npc_left_eguard_duty() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.task = AiTask::Mine; // reassigned away
    ad.npcs.push(npc);
    ad.nagguard = 0;
    ad.nagplace = 0;
    ad.lastnag = 0;

    ad.update_nag_guard(100);
    assert_eq!(ad.nagguard, -1);
}

#[test]
fn update_nag_guard_clears_when_target_no_longer_matches_nagplace() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.task = AiTask::EGuard;
    npc.target = 5;
    ad.npcs.push(npc);
    ad.nagguard = 0;
    ad.nagplace = 1; // different place
    ad.lastnag = 0;

    ad.update_nag_guard(100);
    assert_eq!(ad.nagguard, -1);
}

#[test]
fn update_nag_guard_clears_after_ninety_seconds() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.task = AiTask::EGuard;
    npc.target = 1;
    ad.npcs.push(npc);
    ad.nagguard = 0;
    ad.nagplace = 1;
    ad.lastnag = 0;

    let stale_tick = TICKS_PER_SECOND as i64 * 90 + 1;
    ad.update_nag_guard(stale_tick);
    assert_eq!(ad.nagguard, -1);
}

#[test]
fn update_nag_guard_stays_while_still_valid_and_within_cooldown() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.task = AiTask::EGuard;
    npc.target = 1;
    ad.npcs.push(npc);
    ad.nagguard = 0;
    ad.nagplace = 1;
    ad.lastnag = 0;

    ad.update_nag_guard(10);
    assert_eq!(ad.nagguard, 0);
}

#[test]
fn update_nag_guard_is_a_no_op_when_no_guard_is_nagging() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.nagguard = -1;
    ad.update_nag_guard(100);
    assert_eq!(ad.nagguard, -1);
}

// --- AiData::update_place_worker_and_eguard_counts ---

#[test]
fn update_place_worker_and_eguard_counts_keeps_a_qualifying_worker() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.places[0].worker[0] = 0;
    let mut worker = ai_npc(1, 10, 10, 10);
    worker.target = 0;
    worker.used = -1;
    ad.npcs.push(worker);

    ad.update_place_worker_and_eguard_counts();
    assert_eq!(ad.places[0].wcnt, 1);
    assert_eq!(ad.places[0].worker[0], 0);
    assert_eq!(ad.npcs[0].used, 0);
}

#[test]
fn update_place_worker_and_eguard_counts_drops_a_stale_worker_slot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.places[0].worker[0] = 0;
    let mut worker = ai_npc(1, 10, 10, 10);
    worker.target = 1; // reassigned to a different place
    ad.npcs.push(worker);

    ad.update_place_worker_and_eguard_counts();
    assert_eq!(ad.places[0].wcnt, 0);
    assert_eq!(ad.places[0].worker[0], -1);
}

#[test]
fn update_place_worker_and_eguard_counts_drops_a_worker_already_claimed_this_tick() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.places[0].worker[0] = 0;
    let mut worker = ai_npc(1, 10, 10, 10);
    worker.target = 0;
    worker.used = 7; // already claimed by another place this tick
    ad.npcs.push(worker);

    ad.update_place_worker_and_eguard_counts();
    assert_eq!(ad.places[0].wcnt, 0);
    assert_eq!(ad.places[0].worker[0], -1);
}

#[test]
fn update_place_worker_and_eguard_counts_keeps_a_qualifying_eternal_guard() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places[0].eguard = 0;
    let mut guard = ai_npc(1, 10, 10, 20);
    guard.target = 0;
    guard.used = -1;
    ad.npcs.push(guard);

    ad.update_place_worker_and_eguard_counts();
    assert_eq!(ad.places[0].eguard, 0);
    assert_eq!(ad.npcs[0].used, 0);
}

#[test]
fn update_place_worker_and_eguard_counts_drops_a_stale_eguard_slot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places[0].eguard = 0;
    let mut guard = ai_npc(1, 10, 10, 20);
    guard.target = 1; // no longer targeting this place
    ad.npcs.push(guard);

    ad.update_place_worker_and_eguard_counts();
    assert_eq!(ad.places[0].eguard, -1);
}

// --- AiData::update_free_npc_count ---

#[test]
fn update_free_npc_count_counts_non_eternal_guards_and_free_workers() {
    let mut ad = AiData::new(StrategyPpd::default());
    let mut free = ai_npc(1, 10, 10, 10);
    free.used = -1;
    ad.npcs.push(free);
    let mut busy = ai_npc(2, 10, 10, 10);
    busy.used = 3;
    ad.npcs.push(busy);
    let mut eternal = ai_npc(3, 10, 10, 10);
    eternal.task = AiTask::Ignore;
    eternal.used = -1; // excluded regardless of `used`
    ad.npcs.push(eternal);

    ad.update_free_npc_count();
    assert_eq!(ad.npc_cnt, 2);
    assert_eq!(ad.free_workers, 1);
}

#[test]
fn update_free_npc_count_resets_from_previous_values() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npc_cnt = 999;
    ad.free_workers = 999;
    ad.update_free_npc_count();
    assert_eq!(ad.npc_cnt, 0);
    assert_eq!(ad.free_workers, 0);
}
