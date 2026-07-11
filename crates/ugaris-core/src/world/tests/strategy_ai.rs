use super::*;
use crate::player::StrategyPpd;

fn ai_place(place_type: AiPlaceType, item_id: u32, x: u16, y: u16) -> AiPlace {
    AiPlace::new(place_type, ItemId(item_id), x, y)
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
    assert_eq!(ad.lastnag, 999);
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
