// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::character_driver::CDR_STRATEGY;
use crate::player::StrategyPpd;

pub(super) fn ai_place(place_type: AiPlaceType, item_id: u32, x: u16, y: u16) -> AiPlace {
    AiPlace::new(place_type, ItemId(item_id), x, y)
}

pub(super) fn strategy_item(id: u32, driver: u16, drdata: Vec<u8>) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.driver_data = drdata;
    it
}

/// A spawner+storage pair sharing area slot `slot` (`drdata[8]`), with
/// the storage placed directly north of the spawner (`spawner2storage`'s
/// zone-layout convention) - the minimal setup [`World::ai_init`] needs.
pub(super) fn spawner_and_storage(slot: u8) -> (Item, Item) {
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

pub(super) fn ai_npc(cn: u32, x: u16, y: u16, level: i32) -> AiNpc {
    AiNpc::new(CharacterId(cn), x, y, level)
}

pub(super) fn char_at(id: u32, x: u16, y: u16, level: u32) -> Character {
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

// C `assign_guards`'s `attarget` staging (`strategy.c:2138-2140,2156-2158,
// 2165-2181`): a dispatched group whose members are not all standing at
// `place` or its parent walks to the parent first.
#[test]
fn assign_guards_stages_dispatch_at_parent_when_group_not_at_place() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 10, 10, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 30, 30));
    ad.places.push(ai_place(AiPlaceType::Mine, 3, 50, 50));
    ad.places[1].parent = 0;
    ad.places[2].parent = 1;
    ad.npcs.push(ai_npc(1, 10, 10, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // free, on standby
    ad.npcs[0].current = 0; // at storage: neither place 2 nor its parent 1

    let attacked = world.ai_assign_guards(&mut ad, 2, 1.0, 5, false);
    assert!(attacked);
    assert_eq!(ad.npcs[0].ftarget, 2, "forced target is the place itself");
    assert_eq!(ad.npcs[0].target, 1, "but the walk target is the parent");
    assert_eq!(ad.npcs[0].used, 1);
}

#[test]
fn assign_guards_dispatches_directly_when_guard_already_at_parent() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 30, 30, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 30, 30));
    ad.places.push(ai_place(AiPlaceType::Mine, 3, 50, 50));
    ad.places[1].parent = 0;
    ad.places[2].parent = 1;
    ad.npcs.push(ai_npc(1, 30, 30, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // free, on standby
    ad.npcs[0].current = 1; // already at place 2's parent

    let attacked = world.ai_assign_guards(&mut ad, 2, 1.0, 5, false);
    assert!(attacked);
    assert_eq!(ad.npcs[0].ftarget, 2);
    assert_eq!(ad.npcs[0].target, 2, "at parent already: walk to the place");
    assert_eq!(ad.npcs[0].used, 2);
}

// C's recall loop only resets `use[n] == 2` (already-assigned) entries
// (`strategy.c:2185`); a free pickup that never got dispatched keeps all
// its state untouched.
#[test]
fn assign_guards_failed_defense_leaves_free_pickups_untouched() {
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
    ad.npcs[0].used = 0; // free, on standby
    ad.npcs[0].target = 3; // stale walk target from an earlier assignment

    // threat 20^3 = 8000 is far below the required count, so the defense
    // fails - but the free pickup must stay exactly as it was.
    let attacked = world.ai_assign_guards(&mut ad, 1, 1_000_000.0, 5, false);
    assert!(!attacked);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.npcs[0].ftarget, 0);
    assert_eq!(ad.npcs[0].target, 3, "stale target must not be cleared");
}

// C's recall loop is gated on `have < count` with `have` never modified
// inside it (`strategy.c:2184`): when the counted threat exactly equals
// `count`, nothing is recalled and the assigned guard keeps its post.
#[test]
fn assign_guards_exact_threat_equality_keeps_assigned_guards() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 10, 10, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.ftarget = 1;
    npc.used = 1;
    ad.npcs.push(npc);
    ad.guard[0] = 0;

    // THREAT(level 20) == 8000.0 == count: `have > count` fails (no
    // attack) but `have < count` fails too (no recall).
    let attacked = world.ai_assign_guards(&mut ad, 1, 8000.0, 5, false);
    assert!(!attacked);
    assert_eq!(ad.npcs[0].ftarget, 1, "guard keeps its assignment");
    assert_eq!(ad.npcs[0].used, 1);
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
fn ai_init_seeds_npc_color_from_the_spawners_own_drdata_slot_10() {
    // C `preset[code - STR_OWNER_AI_BASE].ppd.npc_color = it[in].
    // drdata[10];` (`strategy.c:1349`), applied right before `ai_init`
    // runs (`:1352`) - ported as a direct override on this call's own
    // `ad.ppd` instead of mutating the (immutable, in this port)
    // `AI_PRESETS` table, see `World::ai_init`'s own doc comment.
    let mut world = World::default();
    let (mut spawner, storage) = spawner_and_storage(3);
    spawner.driver_data[10] = 5;
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    let ad = world
        .ai_init(ItemId(1), STR_OWNER_AI_BASE + 1)
        .expect("ai_init should succeed");
    assert_eq!(ad.ppd.npc_color, 5);
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
