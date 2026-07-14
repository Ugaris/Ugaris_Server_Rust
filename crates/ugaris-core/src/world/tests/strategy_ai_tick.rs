// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::strategy_ai::*;
use super::*;
use crate::player::StrategyPpd;

// --- World::ai_threat_and_worklevel_tick ---

#[test]
fn threat_tick_forces_at_least_one_threat_slot_when_none_exist() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    world.ai_threat_and_worklevel_tick(&mut ad, 100, 99, true);
    assert_eq!(ad.threats.len(), 1);
    assert_eq!(ad.threats[0].place, 0);
}

#[test]
fn threat_tick_expires_stale_entries_older_than_twenty_seconds() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.threats.push(AiThreat {
        place: 1,
        level: 5,
        count: 3.0,
        ticker: 0,
    });

    let tick = (TICKS_PER_SECOND as i64) * 21;
    world.ai_threat_and_worklevel_tick(&mut ad, tick, 99, true);
    // Expired back to the "empty" sentinel, then reused/truncated by the
    // reduce step - no place-1 entry survives.
    assert!(ad.threats.iter().all(|t| t.place != 1));
}

#[test]
fn threat_tick_records_a_new_threat_entry_for_a_threatened_reachable_place() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.places[1].dist = 3;
    ad.places[1].threatcount = 7.0;
    ad.places[1].threatncount = 1.0;
    ad.places[1].threatlevel = 20;
    ad.places[1].threatnlevel = 10;

    world.ai_threat_and_worklevel_tick(&mut ad, 100, 5, true);
    let entry = ad.threats.iter().find(|t| t.place == 1).unwrap();
    assert_eq!(entry.count, 8.0);
    assert_eq!(entry.level, 20);
}

#[test]
fn threat_tick_ignores_a_threatened_place_beyond_mindist() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.places[1].dist = 10;
    ad.places[1].threatcount = 7.0;

    world.ai_threat_and_worklevel_tick(&mut ad, 100, 5, true);
    assert!(ad.threats.iter().all(|t| t.place != 1));
}

#[test]
fn threat_tick_dispatches_guards_to_an_untraced_threat_and_truncates_the_list() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), char_at(1, 10, 10, 20));
    world.characters.get_mut(&CharacterId(1)).unwrap().hp = 999_999;

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.places[1].dist = 3;
    ad.places[1].parent = 0; // storage, whose own threatcount stays 0
    ad.places[1].threatcount = 7.0;
    ad.places[1].threatlevel = 5;
    ad.npcs.push(ai_npc(1, 10, 10, 20));
    ad.guard[0] = 0;
    ad.npcs[0].used = 0; // on standby

    world.ai_threat_and_worklevel_tick(&mut ad, 100, 5, false);
    assert_eq!(ad.npcs[0].target, 1);
    assert_eq!(ad.npcs[0].used, 1);
    // Only the one real entry (plus, at most, the reduce loop's own
    // trailing bound) should remain.
    assert!(ad.threats.len() <= 2);
}

#[test]
fn threat_tick_shrinks_worklevel_when_missing_and_no_free_workers() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.worklevel = 2;
    ad.free_workers = 0;
    ad.lastchange = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 50, 50));
    ad.places[1].dist = 3;
    ad.places[1].platin = 500;
    ad.places[1].wcnt = 0;

    let tick = (TICKS_PER_SECOND as i64) * 11;
    world.ai_threat_and_worklevel_tick(&mut ad, tick, 5, true);
    assert_eq!(ad.worklevel, 1);
    assert_eq!(ad.lastchange, tick);
}

#[test]
fn threat_tick_grows_worklevel_when_nothing_missing_and_workers_idle() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.worklevel = 2;
    ad.free_workers = 1;
    ad.lastchange = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    let tick = (TICKS_PER_SECOND as i64) * 21;
    world.ai_threat_and_worklevel_tick(&mut ad, tick, 5, true);
    assert_eq!(ad.worklevel, 3);
    assert_eq!(ad.lastchange, tick);
}

#[test]
fn threat_tick_worklevel_capped_at_ai_maxworker() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.worklevel = AI_MAXWORKER as i32;
    ad.free_workers = 1;
    ad.lastchange = 0;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    let tick = (TICKS_PER_SECOND as i64) * 21;
    world.ai_threat_and_worklevel_tick(&mut ad, tick, 5, true);
    assert_eq!(ad.worklevel, AI_MAXWORKER as i32);
}

#[test]
fn threat_tick_leaves_ragnarok_untouched_when_no_guards_dispatched() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));

    let result = world.ai_threat_and_worklevel_tick(&mut ad, 100, 5, true);
    assert!(result);
}

// --- World::ai_dispatch_tasks ---

#[test]
fn ai_dispatch_tasks_take_writes_typed_order_back_onto_the_live_worker() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 10, 10, 10));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Depot, 42, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].task = AiTask::Take;
    ad.npcs[0].target = 0;

    world.ai_dispatch_tasks(&mut ad);

    assert_eq!(ad.npcs[0].order, OR_TAKE);
    match world.characters[&worker_id].driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(
                data.order,
                StrategyWorkerOrder::Take {
                    depot_item: ItemId(42),
                    leader: CharacterId(0),
                }
            );
        }
        _ => panic!("expected StrategyWorker driver state to be created"),
    }
}

#[test]
fn ai_dispatch_tasks_mine_writes_typed_mine_order() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 15, 10, 10));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Mine, 2, 15, 10));
    ad.places[1].parent = 0;
    ad.npcs.push(ai_npc(1, 15, 10, 10));
    ad.npcs[0].task = AiTask::Mine;
    ad.npcs[0].target = 1;

    world.ai_dispatch_tasks(&mut ad);

    match world.characters[&worker_id].driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(
                data.order,
                StrategyWorkerOrder::Mine {
                    mine_item: ItemId(2),
                    depot_item: ItemId(1),
                }
            );
        }
        _ => panic!("expected StrategyWorker driver state to be created"),
    }
}

#[test]
fn ai_dispatch_tasks_ignore_leaves_eternal_guard_order_untouched() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 20, 20, 60));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.npcs.push(ai_npc(1, 20, 20, 60));
    ad.npcs[0].task = AiTask::Ignore;
    ad.npcs[0].order = OR_ETERNALGUARD;
    ad.npcs[0].or1 = 20;
    ad.npcs[0].or2 = 20;

    world.ai_dispatch_tasks(&mut ad);

    // T_IGNORE runs no `task_*` function - `order`/`or1`/`or2` are
    // whatever `create_eguard`/`register_npc` last stamped them as.
    assert_eq!(ad.npcs[0].order, OR_ETERNALGUARD);
    match world.characters[&worker_id].driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(
                data.order,
                StrategyWorkerOrder::EternalGuard { x: 20, y: 20 }
            );
        }
        _ => panic!("expected StrategyWorker driver state to be created"),
    }
}

#[test]
fn ai_dispatch_tasks_eguard_with_no_target_trains_when_economy_can_afford_it() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 10, 10, 10));

    let mut ppd = StrategyPpd::default();
    ppd.max_level = 60;
    let mut ad = AiData::new(ppd);
    ad.places.push(ai_place(AiPlaceType::Storage, 9, 10, 10));
    ad.places[0].platin = NPCPRICE * 3; // > NPCPRICE * 2: can afford training
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].task = AiTask::EGuard;
    ad.npcs[0].target = 0; // "at storage, no specific place assigned"

    world.ai_dispatch_tasks(&mut ad);

    assert_eq!(ad.npcs[0].order, OR_TRAIN);
    match world.characters[&worker_id].driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(
                data.order,
                StrategyWorkerOrder::Train {
                    storage_item: ItemId(9)
                }
            );
        }
        _ => panic!("expected StrategyWorker driver state to be created"),
    }
}

#[test]
fn ai_dispatch_tasks_eguard_with_no_target_idles_when_already_max_level() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 10, 10, 60));

    let mut ppd = StrategyPpd::default();
    ppd.max_level = 60; // level (60) is not < max_level: never trains
    let mut ad = AiData::new(ppd);
    ad.places.push(ai_place(AiPlaceType::Storage, 9, 10, 10));
    ad.places[0].platin = NPCPRICE * 3;
    ad.npcs.push(ai_npc(1, 10, 10, 60));
    ad.npcs[0].task = AiTask::EGuard;
    ad.npcs[0].target = 0;

    world.ai_dispatch_tasks(&mut ad);

    // Falls back to `task_idle`, which delegates to `restplace`/`OR_GUARD`.
    assert_eq!(ad.npcs[0].order, OR_GUARD);
}

#[test]
fn ai_dispatch_tasks_eguard_with_a_target_guards_it() {
    let mut world = World::default();
    let worker_id = CharacterId(1);
    world.characters.insert(worker_id, char_at(1, 55, 66, 10));

    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10));
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 55, 66));
    ad.npcs.push(ai_npc(1, 55, 66, 10));
    ad.npcs[0].task = AiTask::EGuard;
    ad.npcs[0].target = 1; // assigned to defend place 1

    world.ai_dispatch_tasks(&mut ad);

    assert_eq!(ad.npcs[0].order, OR_GUARD);
    assert_eq!(ad.npcs[0].or1, 55);
    assert_eq!(ad.npcs[0].or2, 66);
}

#[test]
fn ai_dispatch_tasks_skips_write_back_for_a_despawned_npc() {
    // `cn == None` (C's "slot emptied" sentinel, `ai_update_npc_list`):
    // no live character to write back onto, but the task dispatch itself
    // must not panic.
    let mut world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Depot, 42, 10, 10));
    ad.npcs.push(ai_npc(1, 10, 10, 10));
    ad.npcs[0].cn = None;
    ad.npcs[0].task = AiTask::Take;
    ad.npcs[0].target = 0;

    world.ai_dispatch_tasks(&mut ad);

    assert_eq!(ad.npcs[0].order, OR_TAKE);
    assert!(world.characters.is_empty());
}

// --- `register_new_worker` (`ai_main`'s "add new npc to list" tail) ---

#[test]
fn register_new_worker_appends_when_no_empty_slot_exists() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npcs.push(ai_npc(1, 5, 5, 10));

    let idx = ad.register_new_worker(CharacterId(2), 20, 21);

    assert_eq!(idx, 1);
    assert_eq!(ad.npcs.len(), 2);
    let fresh = &ad.npcs[1];
    assert_eq!(fresh.cn, Some(CharacterId(2)));
    assert_eq!((fresh.x, fresh.y), (20, 21));
    assert_eq!(fresh.order, OR_NONE);
    assert_eq!(fresh.task, AiTask::Idle);
    assert_eq!(fresh.target, 0);
    assert_eq!(fresh.current, 0);
    assert_eq!(fresh.used, -1);
}

#[test]
fn register_new_worker_reuses_the_first_emptied_slot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.npcs.push(ai_npc(1, 5, 5, 10));
    ad.npcs.push(ai_npc(2, 6, 6, 10));
    ad.npcs[0].cn = None; // emptied by `ai_update_npc_list`

    let idx = ad.register_new_worker(CharacterId(3), 30, 31);

    assert_eq!(idx, 0);
    assert_eq!(ad.npcs.len(), 2); // reused, did not grow
    assert_eq!(ad.npcs[0].cn, Some(CharacterId(3)));
    assert_eq!((ad.npcs[0].x, ad.npcs[0].y), (30, 31));
    assert_eq!(ad.npcs[1].cn, Some(CharacterId(2))); // untouched
}

// --- `ai_wants_more_workers` (`ai_main`'s "create new workers" `while`
// loop condition) ---

#[test]
fn ai_wants_more_workers_false_when_storage_item_is_missing() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.storage_item = ItemId(2);
    ad.panic = true;
    ad.npc_cnt = 0;

    assert!(!world.ai_wants_more_workers(&ad));
}

#[test]
fn ai_wants_more_workers_true_when_panicking_and_under_cap() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    ad_storage_with_gold(&mut world, &storage, 0);

    let mut ppd = StrategyPpd::default();
    ppd.max_worker = 10;
    let mut ad = AiData::new(ppd);
    ad.storage_item = ItemId(2);
    ad.panic = true;
    ad.free_workers = 3; // would otherwise block on "free_workers != 0"
    ad.npc_cnt = 5; // < min(10, 16 + 0/500) == 10

    assert!(world.ai_wants_more_workers(&ad));
}

#[test]
fn ai_wants_more_workers_false_when_not_panicking_and_workers_are_free() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    ad_storage_with_gold(&mut world, &storage, 0);

    let mut ad = AiData::new(StrategyPpd::default());
    ad.storage_item = ItemId(2);
    ad.panic = false;
    ad.free_workers = 1;
    ad.npc_cnt = 0;

    assert!(!world.ai_wants_more_workers(&ad));
}

#[test]
fn ai_wants_more_workers_false_once_npc_cnt_reaches_the_gold_scaled_cap() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    // `16 + gold / 500` with `gold = 500` -> cap 17, still below
    // `max_worker` (default higher), so the gold-derived term is the
    // binding one.
    ad_storage_with_gold(&mut world, &storage, 500);

    let mut ppd = StrategyPpd::default();
    ppd.max_worker = 100;
    let mut ad = AiData::new(ppd);
    ad.storage_item = ItemId(2);
    ad.panic = true;
    ad.npc_cnt = 17; // == cap, not < cap

    assert!(!world.ai_wants_more_workers(&ad));
}

// --- `ai_plan_worker_spawn` (`ai_main`'s "spawn new worker" body up to
// the character-creation call) ---

fn ad_storage_with_gold(world: &mut World, storage: &Item, gold: u32) {
    let mut storage = storage.clone();
    set_str_item_gold(&mut storage, gold);
    world.add_item(storage);
}

#[test]
fn ai_plan_worker_spawn_returns_none_for_a_code_outside_ai_presets_range() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    ad_storage_with_gold(&mut world, &storage, NPCPRICE as u32 * 10);
    let ad = AiData::new(StrategyPpd::default());

    assert!(world
        .ai_plan_worker_spawn(ItemId(1), &ad, STR_OWNER_AI_BASE - 1)
        .is_none());
}

#[test]
fn ai_plan_worker_spawn_returns_none_when_not_enough_gold() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    ad_storage_with_gold(&mut world, &storage, NPCPRICE as u32 - 1);
    let mut ad = AiData::new(AI_PRESETS[1].to_strategy_ppd());
    ad.storage_item = ItemId(2);

    assert!(world
        .ai_plan_worker_spawn(ItemId(1), &ad, STR_OWNER_AI_BASE + 1)
        .is_none());
    // Not enough gold: no deduction happened either.
    assert_eq!(str_item_gold(&world.items[&ItemId(2)]), NPCPRICE as u32 - 1);
}

#[test]
fn ai_plan_worker_spawn_deducts_npcprice_and_returns_the_preset_plan() {
    let mut world = World::default();
    let (_, storage) = spawner_and_storage(1);
    ad_storage_with_gold(&mut world, &storage, NPCPRICE as u32 * 3);
    let preset = &AI_PRESETS[1]; // "Zakath"
    let mut ad = AiData::new(preset.to_strategy_ppd());
    ad.storage_item = ItemId(2);
    let code = STR_OWNER_AI_BASE + 1;

    let plan = world
        .ai_plan_worker_spawn(ItemId(1), &ad, code)
        .expect("enough gold, valid preset code");

    assert_eq!(plan.spawner_id, ItemId(1));
    assert_eq!(plan.group, code as u16);
    assert_eq!(plan.owner_name, preset.name);
    assert_eq!(plan.warcry, preset.to_strategy_ppd().warcry);
    assert_eq!(plan.endurance, preset.to_strategy_ppd().endurance);
    assert_eq!(plan.speed, preset.to_strategy_ppd().speed);
    assert_eq!(plan.trainspeed, preset.to_strategy_ppd().trainspeed);
    assert_eq!(plan.max_level, preset.to_strategy_ppd().max_level);
    assert_eq!(plan.npc_color, preset.to_strategy_ppd().npc_color);

    assert_eq!(str_item_gold(&world.items[&ItemId(2)]), NPCPRICE as u32 * 2);
}

// --- `ai_wants_more_eguards`/`ai_eguard_spawn_candidates`/
// `ai_plan_eguard_spawn` ("place eternal guards" tail) ---

#[test]
fn ai_wants_more_eguards_false_once_eguard_cap_is_reached() {
    let world = World::default();
    let mut ppd = StrategyPpd::default();
    ppd.eguards = 1;
    let mut ad = AiData::new(ppd);
    ad.etguardcnt = 1;

    assert!(!world.ai_wants_more_eguards(&ad));

    ad.etguardcnt = 0;
    assert!(world.ai_wants_more_eguards(&ad));
}

#[test]
fn ai_eguard_spawn_candidates_requires_pdist_enemy_possible_unguarded_and_owned() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.pdist = 2;
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0

    // Eligible: dist == pdist, enemy_possible, unguarded, owned.
    let mut eligible = ai_place(AiPlaceType::Depot, 2, 20, 20);
    eligible.dist = 2;
    eligible.enemy_possible = true;
    eligible.eguard = -1;
    eligible.owned = true;
    ad.places.push(eligible); // place 1

    // Wrong distance.
    let mut wrong_dist = ai_place(AiPlaceType::Depot, 3, 30, 30);
    wrong_dist.dist = 1;
    wrong_dist.enemy_possible = true;
    wrong_dist.owned = true;
    ad.places.push(wrong_dist); // place 2

    // Not enemy-reachable.
    let mut not_reachable = ai_place(AiPlaceType::Depot, 4, 40, 40);
    not_reachable.dist = 2;
    not_reachable.enemy_possible = false;
    not_reachable.owned = true;
    ad.places.push(not_reachable); // place 3

    // Already guarded.
    let mut already_guarded = ai_place(AiPlaceType::Depot, 5, 50, 50);
    already_guarded.dist = 2;
    already_guarded.enemy_possible = true;
    already_guarded.eguard = 0;
    already_guarded.owned = true;
    ad.places.push(already_guarded); // place 4

    // Not owned by this party.
    let mut not_owned = ai_place(AiPlaceType::Depot, 6, 60, 60);
    not_owned.dist = 2;
    not_owned.enemy_possible = true;
    not_owned.owned = false;
    ad.places.push(not_owned); // place 5

    assert_eq!(world.ai_eguard_spawn_candidates(&ad), vec![1]);
}

#[test]
fn ai_plan_eguard_spawn_returns_none_for_a_code_outside_ai_presets_range() {
    let world = World::default();
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 20, 20));

    assert!(world
        .ai_plan_eguard_spawn(&ad, 0, STR_OWNER_AI_BASE - 1)
        .is_none());
}

#[test]
fn ai_plan_eguard_spawn_returns_none_for_an_out_of_range_place() {
    let world = World::default();
    let ad = AiData::new(AI_PRESETS[1].to_strategy_ppd());

    assert!(world
        .ai_plan_eguard_spawn(&ad, 0, STR_OWNER_AI_BASE + 1)
        .is_none());
}

#[test]
fn ai_plan_eguard_spawn_offsets_the_place_by_two_and_returns_the_preset_plan() {
    let world = World::default();
    let preset = &AI_PRESETS[1]; // "Zakath"
    let mut ad = AiData::new(preset.to_strategy_ppd());
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 20, 30));
    let code = STR_OWNER_AI_BASE + 1;

    let plan = world
        .ai_plan_eguard_spawn(&ad, 0, code)
        .expect("valid place and preset code");

    assert_eq!((plan.x, plan.y), (22, 32));
    assert_eq!(plan.group, code as u16);
    assert_eq!(plan.owner_name, preset.name);
    assert_eq!(plan.level, preset.to_strategy_ppd().eguardlvl);
    assert_eq!(plan.warcry, preset.to_strategy_ppd().warcry);
    assert_eq!(plan.endurance, preset.to_strategy_ppd().endurance);
    assert_eq!(plan.speed, preset.to_strategy_ppd().speed);
    assert_eq!(plan.npc_color, preset.to_strategy_ppd().npc_color);
}

// --- `register_new_eguard` (`ai_main`'s "place eternal guards" "add new
// npc to list" tail) ---

#[test]
fn register_new_eguard_appends_when_no_empty_slot_exists() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 22, 32)); // place 1
    ad.npcs.push(ai_npc(1, 5, 5, 10));

    let idx = ad.register_new_eguard(CharacterId(2), 22, 32, 1);

    assert_eq!(idx, 1);
    assert_eq!(ad.npcs.len(), 2);
    let fresh = &ad.npcs[1];
    assert_eq!(fresh.cn, Some(CharacterId(2)));
    assert_eq!((fresh.x, fresh.y), (22, 32));
    assert_eq!(fresh.order, OR_ETERNALGUARD);
    assert_eq!((fresh.or1, fresh.or2), (22, 32));
    assert_eq!(fresh.task, AiTask::Ignore);
    assert_eq!(fresh.target, 1);
    assert_eq!(fresh.current, 1);
    assert_eq!(fresh.used, 1);
    // `add_etguard` stationed it at its own (already-matching) place.
    assert_eq!(ad.places[1].eguard, 1);
    assert_eq!(ad.etguardcnt, 1);
}

#[test]
fn register_new_eguard_reuses_the_first_emptied_slot() {
    let mut ad = AiData::new(StrategyPpd::default());
    ad.places.push(ai_place(AiPlaceType::Storage, 1, 10, 10)); // place 0
    ad.places.push(ai_place(AiPlaceType::Depot, 2, 22, 32)); // place 1
    ad.npcs.push(ai_npc(1, 5, 5, 10));
    ad.npcs.push(ai_npc(2, 6, 6, 10));
    ad.npcs[0].cn = None; // emptied by `ai_update_npc_list`
    ad.etguardcnt = 3; // pre-existing count from other places

    let idx = ad.register_new_eguard(CharacterId(3), 22, 32, 1);

    assert_eq!(idx, 0);
    assert_eq!(ad.npcs.len(), 2); // reused, did not grow
    assert_eq!(ad.npcs[0].cn, Some(CharacterId(3)));
    assert_eq!(ad.npcs[1].cn, Some(CharacterId(2))); // untouched
    assert_eq!(ad.places[1].eguard, 0);
    assert_eq!(ad.etguardcnt, 4);
}
