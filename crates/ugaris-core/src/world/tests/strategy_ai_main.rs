//! Tests for [`World::ai_main`]'s assembly of every previously-ported
//! `ai_init`/`ai_main` piece (`world::strategy_ai`/`strategy_ai_tasks`)
//! plus [`World::register_ai_worker`]/[`World::register_ai_eguard`] - see
//! `world::strategy_ai_main`'s own module doc comment for the exact call
//! order and the two documented spawn-plan simplifications.

use super::*;
use crate::character_driver::CharacterDriverState;

fn strategy_item(id: u32, driver: u16, drdata: Vec<u8>) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.driver_data = drdata;
    it
}

/// A spawner+storage pair sharing area slot `slot` (`drdata[8]`), with
/// the storage placed directly north of the spawner (`spawner2storage`'s
/// zone-layout convention) - the minimal setup [`World::ai_init`] needs.
/// Same shape as `world::tests::strategy_ai`'s own private helper of the
/// same name.
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

fn setup_world_with_funded_party(slot: u8, gold: u32) -> (World, u32) {
    let mut world = World::default();
    let (spawner, mut storage) = spawner_and_storage(slot);
    set_str_item_gold(&mut storage, gold);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);
    let code = STR_OWNER_AI_BASE + 1; // "Zakath"
    (world, code)
}

#[test]
fn ai_main_returns_a_default_outcome_and_stores_nothing_for_an_invalid_code() {
    let mut world = World::default();
    let (spawner, storage) = spawner_and_storage(1);
    world.add_item(spawner);
    world.map.tile_mut(10, 9).unwrap().item = 2;
    world.add_item(storage);

    let outcome = world.ai_main(ItemId(1), STR_OWNER_AI_BASE - 1);

    assert!(outcome.worker_plan.is_none());
    assert!(outcome.eguard_plan.is_none());
    assert!(world.ai_parties.is_empty());
}

#[test]
fn ai_main_initializes_and_stores_a_fresh_party_on_first_call() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);

    world.ai_main(ItemId(1), code);

    let ad = world
        .ai_parties
        .get(&code)
        .expect("ai_main should have run ai_init and stored the party");
    assert_eq!(ad.storage_item, ItemId(2));
    assert_eq!(ad.places.len(), 1);
    assert_eq!(ad.places[0].place_type, AiPlaceType::Storage);
}

#[test]
fn ai_main_reuses_the_stored_party_instead_of_reinitializing() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);

    world.ai_main(ItemId(1), code);
    // Mutate a field only a fresh `ai_init` would reset, to prove the
    // second call reused the stored party instead of rebuilding it.
    world.ai_parties.get_mut(&code).unwrap().worklevel = 3;

    world.ai_main(ItemId(1), code);

    assert_eq!(world.ai_parties[&code].worklevel, 3);
}

#[test]
fn ai_main_plans_one_worker_when_storage_can_afford_it_and_the_party_is_below_cap() {
    let (mut world, code) = setup_world_with_funded_party(4, NPCPRICE as u32 * 2);

    let outcome = world.ai_main(ItemId(1), code);

    let plan = outcome
        .worker_plan
        .expect("fresh party, spare gold, below worker cap: should plan a worker");
    assert_eq!(plan.spawner_id, ItemId(1));
    assert_eq!(plan.group, code as u16);
    // `ai_plan_worker_spawn` deducts `NPCPRICE` up front, unconditionally.
    assert_eq!(str_item_gold(&world.items[&ItemId(2)]), NPCPRICE as u32);
}

#[test]
fn ai_main_plans_no_worker_when_storage_has_no_spare_gold() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);

    let outcome = world.ai_main(ItemId(1), code);

    assert!(outcome.worker_plan.is_none());
}

#[test]
fn register_ai_worker_adds_the_new_npc_to_the_stored_party() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);
    world.ai_main(ItemId(1), code);
    assert!(world.ai_parties[&code].npcs.is_empty());

    world.register_ai_worker(code, CharacterId(9), 11, 12);

    let ad = &world.ai_parties[&code];
    assert_eq!(ad.npcs.len(), 1);
    assert_eq!(ad.npcs[0].cn, Some(CharacterId(9)));
    assert_eq!((ad.npcs[0].x, ad.npcs[0].y), (11, 12));
    assert_eq!(ad.npcs[0].task, AiTask::Idle);
}

#[test]
fn register_ai_worker_is_a_no_op_for_an_unknown_party() {
    let mut world = World::default();
    // No `ai_main` call ever ran for this code - nothing to register into.
    world.register_ai_worker(STR_OWNER_AI_BASE + 5, CharacterId(9), 11, 12);
    assert!(world.ai_parties.is_empty());
}

#[test]
fn register_ai_eguard_adds_an_eternal_guard_to_the_stored_party() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);
    world.ai_main(ItemId(1), code);

    world.register_ai_eguard(code, CharacterId(9), 11, 12, 0);

    let ad = &world.ai_parties[&code];
    let guard = ad
        .npcs
        .iter()
        .find(|npc| npc.cn == Some(CharacterId(9)))
        .expect("registered eguard should be in the roster");
    assert_eq!(guard.task, AiTask::Ignore);
    assert_eq!(guard.order, OR_ETERNALGUARD);
    assert_eq!(ad.etguardcnt, 1);
    assert_eq!(
        ad.places[0].eguard,
        ad.npcs
            .iter()
            .position(|n| n.cn == Some(CharacterId(9)))
            .unwrap() as i32
    );
}

#[test]
fn register_ai_eguard_is_a_no_op_for_an_unknown_party() {
    let mut world = World::default();
    world.register_ai_eguard(STR_OWNER_AI_BASE + 5, CharacterId(9), 11, 12, 0);
    assert!(world.ai_parties.is_empty());
}

#[test]
fn ai_main_dispatches_tasks_and_syncs_the_raw_order_onto_the_live_worker() {
    let (mut world, code) = setup_world_with_funded_party(4, 0);
    world.ai_main(ItemId(1), code);
    world.register_ai_worker(code, CharacterId(9), 11, 12);
    // Note: no live `Character` with `driver == CDR_STRATEGY` is added
    // here deliberately - `Character::group` is `u16`-narrowed (see
    // `World::ai_init`'s own doc comment for the pre-existing, documented
    // gap this avoids exercising: any live `CDR_STRATEGY` character can
    // never actually compare equal to a real `STR_OWNER_AI_BASE`-range
    // `code`, so it would always register as an "enemy" in `ai_refresh_
    // places`'s threat scan and force `panic`, unrelated to what this
    // test checks).
    world.add_character(character(9));

    // Second call: this minimal one-place (storage-only, no mine/depot)
    // party can never see `nogoldleft` go false (C's own `ai_main` only
    // clears it from a *non-storage* place having spare gold,
    // `strategy.c:2626-2629` - `World::ai_refresh_places`'s own doc
    // comment), so `assign_tasks_to_workers` converts the freshly-
    // registered idle worker straight into an eternal guard instead of
    // sending it to storage - a real, C-faithful outcome for a party
    // with no economy to run, not a test bug. The resulting raw order
    // should still be synced onto the live character's own driver state
    // either way.
    world.ai_main(ItemId(1), code);

    let ad = &world.ai_parties[&code];
    let worker = ad
        .npcs
        .iter()
        .find(|npc| npc.cn == Some(CharacterId(9)))
        .unwrap();
    assert_eq!(worker.task, AiTask::EGuard);
    assert!(matches!(
        world.characters[&CharacterId(9)].driver_state,
        Some(CharacterDriverState::StrategyWorker(_))
    ));
}
