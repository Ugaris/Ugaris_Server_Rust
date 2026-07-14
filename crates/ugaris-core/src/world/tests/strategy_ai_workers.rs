// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::strategy_ai::*;
use super::*;
use crate::character_driver::CDR_STRATEGY;
use crate::player::StrategyPpd;

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

// --- World::ai_update_npc_list ---

#[test]
fn ai_update_npc_list_refreshes_position_level_and_platin_from_the_live_character() {
    let mut world = World::default();
    let mut worker = char_at(1, 15, 16, 25);
    worker.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData {
            platin: 77,
            ..Default::default()
        },
    ));
    world.add_character(worker);

    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 0, 0, 1);
    npc.used = 5; // stale from a previous tick's commit
    ad.npcs.push(npc);

    let cantrain = world.ai_update_npc_list(&mut ad);
    assert_eq!(ad.npcs[0].x, 15);
    assert_eq!(ad.npcs[0].y, 16);
    assert_eq!(ad.npcs[0].level, 25);
    assert_eq!(ad.npcs[0].platin, 77);
    assert_eq!(ad.npcs[0].used, -1);
    assert!(!cantrain);
    assert_eq!(ad.npcs[0].cn, Some(CharacterId(1)));
}

#[test]
fn ai_update_npc_list_empties_the_slot_when_the_character_no_longer_exists() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npcs.push(ai_npc(1, 10, 10, 20));

    world.ai_update_npc_list(&mut ad);
    // The slot is emptied (C's `an[n].cn = 0`), but every other field is
    // left stale rather than reset - matching C exactly (see
    // `World::ai_update_npc_list`'s own doc comment).
    assert_eq!(ad.npcs[0].cn, None);
    assert_eq!(ad.npcs[0].x, 10);
}

#[test]
fn ai_update_npc_list_reports_cantrain_for_an_under_max_level_eternal_guard() {
    let mut world = World::default();
    world.add_character(char_at(1, 10, 10, 20));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.ppd.max_level = 50;
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.task = AiTask::EGuard;
    ad.npcs.push(npc);

    assert!(world.ai_update_npc_list(&mut ad));
}

#[test]
fn ai_update_npc_list_cantrain_stays_false_for_a_non_eguard_below_max_level() {
    let mut world = World::default();
    world.add_character(char_at(1, 10, 10, 20));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.ppd.max_level = 50;
    ad.npcs.push(ai_npc(1, 10, 10, 20)); // AiTask::Idle by default

    assert!(!world.ai_update_npc_list(&mut ad));
}

#[test]
fn ai_update_npc_list_leaves_an_already_empty_slot_alone() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    let mut npc = ai_npc(1, 10, 10, 20);
    npc.cn = None;
    ad.npcs.push(npc);

    let cantrain = world.ai_update_npc_list(&mut ad);
    assert!(!cantrain);
    assert_eq!(ad.npcs[0].cn, None);
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

// --- AiData::assign_tasks_to_workers ---

#[test]
fn assign_tasks_to_workers_panic_sends_a_free_npc_to_fight_at_pplace() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Storage, 2, 20, 20));
    ad.npcs.push(ai_npc(1, 10, 10, 10)); // npc 0: free/idle already
    ad.npcs.push(ai_npc(2, 20, 20, 10)); // npc 1: EGuard, untouched
    ad.npcs.push(ai_npc(3, 20, 20, 10)); // npc 2: Ignore, untouched
    ad.npcs[1].task = AiTask::EGuard;
    ad.npcs[2].task = AiTask::Ignore;

    ad.panic = true;
    ad.pplace = 1;
    ad.assign_tasks_to_workers(99);

    // Already free (`used == -1`): task really becomes (and stays) Fight.
    assert_eq!(ad.npcs[0].task, AiTask::Fight);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, -1);
    // Both EGuard and Ignore keep their own task, but `target` is still
    // stamped to `pplace` unconditionally (C `:2684`).
    assert_eq!(ad.npcs[1].task, AiTask::EGuard);
    assert_eq!(ad.npcs[1].target, 1);
    assert_eq!(ad.npcs[2].task, AiTask::Ignore);
    assert_eq!(ad.npcs[2].target, 1);
}

#[test]
fn assign_tasks_to_workers_panic_reverts_a_working_npc_to_idle_via_remove_worker() {
    // C quirk (`strategy.c:2676-2681`, kept verbatim): setting `task =
    // T_FIGHT` right before calling `remove_worker` is pointless for any
    // NPC that was actually working a place, since `remove_worker` itself
    // unconditionally stamps `task = T_IDLE` - so a previously-busy NPC
    // ends up idle (with `target` still redirected to `pplace` right
    // after, since that assignment sits outside the `if`), not fighting.
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Mine, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Storage, 2, 20, 20));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;
    ad.add_worker(AiTask::Mine, 0, 0); // currently working place 0

    ad.panic = true;
    ad.pplace = 1;
    ad.assign_tasks_to_workers(99);

    assert_eq!(ad.npcs[0].task, AiTask::Idle);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.places[0].wcnt, 0);
}

#[test]
fn assign_tasks_to_workers_promotes_a_free_worker_to_eguard_when_understaffed() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;
    ad.npc_cnt = 4; // wantguardcnt(4) == 1
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(99);

    assert_eq!(ad.npcs[0].task, AiTask::EGuard);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.gcnt, 1);
}

#[test]
fn assign_tasks_to_workers_demotes_an_excess_eguard_when_economy_allows() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;
    assert!(ad.add_guard(0));
    ad.npcs[0].task = AiTask::EGuard;
    ad.npc_cnt = 3; // wantguardcnt(3) == 0
    ad.gcnt = 1; // 0 < 1: too many guards
    ad.nogoldleft = false;
    ad.ragnarok = false;

    ad.assign_tasks_to_workers(99);

    assert_eq!(ad.gcnt, 0);
    assert_eq!(ad.guard[0], -1);
    assert_eq!(ad.npcs[0].used, -1);
}

#[test]
fn assign_tasks_to_workers_never_demotes_an_eguard_while_ragnarok_or_nogoldleft() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.free_workers = 1;
    assert!(ad.add_guard(0));
    ad.npcs[0].task = AiTask::EGuard;
    ad.npc_cnt = 3; // wantguardcnt(3) == 0 < gcnt
    ad.gcnt = 1;
    ad.nogoldleft = false;
    ad.ragnarok = true; // blocks the demotion

    ad.assign_tasks_to_workers(99);

    assert_eq!(ad.gcnt, 1);
    assert_eq!(ad.guard[0], 0);
    assert_eq!(ad.npcs[0].task, AiTask::EGuard);
}

#[test]
fn assign_tasks_to_workers_keeps_a_productive_transfer_worker_in_place() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    let mut depot = ai_place(AiPlaceType::Depot, 2, 12, 10);
    depot.parent = 0;
    depot.dist = 1;
    depot.wcnt = 1;
    ad.places.push(depot);
    ad.npcs.push(ai_npc(1, 12, 10, 10));
    ad.npcs[0].task = AiTask::Transfer;
    ad.npcs[0].target = 1;
    ad.npcs[0].used = 1;
    ad.npcs[0].platin = 50; // this worker's own cached earnings
    ad.npc_cnt = 3; // wantguardcnt(3) == 0, no eguard pressure
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    // Still productive (wcnt <= worklevel, no threat, in range): unchanged.
    assert_eq!(ad.npcs[0].task, AiTask::Transfer);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
}

#[test]
fn assign_tasks_to_workers_keeps_a_take_worker_while_the_depot_is_still_unowned() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    let mut depot = ai_place(AiPlaceType::Depot, 2, 12, 10);
    depot.parent = 0;
    depot.dist = 1;
    depot.owned = false;
    ad.places.push(depot);
    ad.npcs.push(ai_npc(1, 12, 10, 10));
    ad.npcs[0].task = AiTask::Take;
    ad.npcs[0].target = 1;
    ad.npcs[0].used = 1;
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    assert_eq!(ad.npcs[0].task, AiTask::Take);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
}

#[test]
fn assign_tasks_to_workers_removes_a_worker_from_a_now_threatened_place() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    let mut depot = ai_place(AiPlaceType::Depot, 2, 12, 10);
    depot.parent = 0;
    depot.dist = 1;
    depot.wcnt = 1;
    depot.threat = 50; // now under attack
    ad.places.push(depot);
    ad.npcs.push(ai_npc(1, 12, 10, 10));
    ad.npcs[0].task = AiTask::Transfer;
    ad.npcs[0].target = 1;
    ad.npcs[0].used = 1;
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    // No other place qualifies as a replacement, so it falls all the way
    // through to the idle fallback - but it was genuinely evicted first
    // (C never re-inspects `threat` again before that fallback).
    // `remove_worker` stamps `used = 0` (not `-1`, see its own doc
    // comment), so that's what stays.
    assert_eq!(ad.npcs[0].task, AiTask::Idle);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].used, 0);
    assert_eq!(ad.places[1].wcnt, 0);
}

#[test]
fn assign_tasks_to_workers_redirects_to_an_understaffed_parent() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0
    let mut mid = ai_place(AiPlaceType::Depot, 2, 12, 10); // place 1
    mid.parent = 0;
    mid.dist = 1;
    mid.wcnt = 0;
    mid.platin = 300; // > (0 + 1) * WORKERPLATIN(200)
    ad.places.push(mid);
    let mut leaf = ai_place(AiPlaceType::Mine, 3, 14, 10); // place 2
    leaf.parent = 1;
    leaf.dist = 2;
    leaf.wcnt = 2; // > place 1's wcnt (0)
    ad.places.push(leaf);
    ad.npcs.push(ai_npc(1, 14, 10, 10));
    ad.npcs[0].task = AiTask::Mine;
    ad.npcs[0].target = 2;
    ad.npcs[0].used = 2;
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    assert_eq!(ad.npcs[0].task, AiTask::Transfer);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
    assert_eq!(ad.places[1].wcnt, 1);
}

#[test]
fn assign_tasks_to_workers_assigns_take_to_the_nearest_unowned_empty_depot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0
    let mut depot = ai_place(AiPlaceType::Depot, 2, 12, 10); // place 1
    depot.dist = 2;
    depot.owned = false;
    depot.wcnt = 0;
    ad.places.push(depot);
    ad.npcs.push(ai_npc(1, 12, 10, 10)); // idle, free
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    assert_eq!(ad.npcs[0].task, AiTask::Take);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
    assert_eq!(ad.places[1].wcnt, 1);
}

#[test]
fn assign_tasks_to_workers_assigns_mine_task_to_the_nearest_understaffed_mine() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0
    let mut mine = ai_place(AiPlaceType::Mine, 2, 12, 10); // place 1
    mine.dist = 2;
    mine.platin = 500; // > wcnt(0) * WORKERPLATIN
    mine.wcnt = 0;
    ad.places.push(mine);
    ad.npcs.push(ai_npc(1, 12, 10, 10)); // idle, free
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    assert_eq!(ad.npcs[0].task, AiTask::Mine);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
}

#[test]
fn assign_tasks_to_workers_falls_back_to_idle_when_nothing_qualifies() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npc_cnt = 3;
    ad.gcnt = 0;

    ad.assign_tasks_to_workers(5);

    assert_eq!(ad.npcs[0].task, AiTask::Idle);
    assert_eq!(ad.npcs[0].target, 0);
    assert_eq!(ad.npcs[0].used, -1);
}
