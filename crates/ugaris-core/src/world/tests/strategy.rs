// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::character_driver::{CharacterDriverState, CDR_STRATEGY};
use crate::item_driver::{IDR_ENHANCE, IDR_NOSNOW, IDR_STR_TICKER};
use crate::player::StrategyPpd;
use crate::world::npc::area23_24::StrategyWorkerDriverData;

#[test]
fn order_constants_match_c_defines() {
    assert_eq!(OR_NONE, 0);
    assert_eq!(OR_MINE, 1);
    assert_eq!(OR_FOLLOW, 2);
    assert_eq!(OR_GUARD, 3);
    assert_eq!(OR_FIGHTER, 4);
    assert_eq!(OR_TAKE, 5);
    assert_eq!(OR_TRANSFER, 6);
    assert_eq!(OR_TRAIN, 7);
    assert_eq!(OR_ETERNALGUARD, 8);
}

#[test]
fn misc_constants_match_c_defines() {
    assert_eq!(NPCPRICE, 300);
    assert_eq!(TRAINMULTI, 3);
    assert_eq!(MAXMISSIONTRY, 3);
    assert_eq!(STRATEGY_MAXMISSION, 64);
    assert_eq!(MAX_STR_AREA, 16);
    assert_eq!(MAXQUEUE, 4);
}

#[test]
fn train_price_matches_c_macro() {
    // C: #define TRAINPRICE(cn) ((ch[cn].level - 45) * 10)
    assert_eq!(train_price(45), 0);
    assert_eq!(train_price(50), 50);
    assert_eq!(train_price(115), 700);
    assert_eq!(train_price(40), -50);
}

#[test]
fn missions_table_has_all_fourteen_rows_in_c_order() {
    assert_eq!(MISSIONS.len(), 14);
    let names: Vec<&str> = MISSIONS.iter().map(|m| m.name).collect();
    assert_eq!(
        names,
        vec!["A-1", "A-2", "B", "C", "D", "E", "F", "G", "H", "I", "J 2P", "K", "L", "Z"]
    );
}

#[test]
// The tuple layout intentionally mirrors C's `struct mission` columns.
#[allow(clippy::type_complexity)]
fn missions_table_matches_c_literal_values_digit_for_digit() {
    // struct mission mission[] (strategy.c:214-239)
    let expected: [(&str, i32, i32, i32, [i32; 4], i32, i32, i32, i32); 14] = [
        ("A-1", 1, 1000, 600, [1, 0, 0, 0], 1, 0, 0, 10),
        ("A-2", 2, 1000, 600, [2, 0, 0, 0], 1, 0, 0, 10),
        ("B", 5, 1000, 600, [3, 0, 0, 0], 2, 1, 1, 25),
        ("C", 4, 1000, 600, [4, 5, 0, 0], 3, 1, 1, 25),
        ("D", 3, 1500, 600, [6, 7, 0, 0], 4, 2, 3, 25),
        ("E", 5, 2000, 900, [8, 0, 0, 0], 5, 3, 4, 25),
        ("F", 4, 2000, 900, [9, 10, 0, 0], 6, 4, 5, 25),
        ("G", 3, 2000, 900, [11, 12, 0, 0], 7, 5, 6, 25),
        ("H", 7, 2000, 900, [13, 14, 0, 0], 8, 6, 7, 25),
        ("I", 8, 2000, 300, [15, 0, 0, 0], 9, 7, 8, 25),
        ("J 2P", 9, 2000, 300, [16, 0, 0, 0], 0, 0, 0, 0),
        ("K", 10, 2000, 900, [17, 18, 19, 0], 10, 8, 9, 25),
        ("L", 11, 2000, 900, [20, 21, 0, 0], 11, 9, 10, 25),
        ("Z", 3, 2000, 900, [22, 23, 0, 0], 12, 10, 11, 50),
    ];

    for (mission, (name, area, mine, storage, enemy, set_solve, need1, need2, exp)) in
        MISSIONS.iter().zip(expected.iter())
    {
        assert_eq!(mission.name, *name);
        assert_eq!(mission.area, *area, "{name} area");
        assert_eq!(mission.mine_size, *mine, "{name} mine_size");
        assert_eq!(mission.storage_size, *storage, "{name} storage_size");
        assert_eq!(mission.enemy, *enemy, "{name} enemy");
        assert_eq!(mission.set_solve, *set_solve, "{name} set_solve");
        assert_eq!(mission.need_solve, *need1, "{name} need_solve");
        assert_eq!(mission.need_solve2, *need2, "{name} need_solve2");
        assert_eq!(mission.exp, *exp, "{name} exp");
    }
}

#[test]
fn ai_presets_table_has_all_twenty_four_rows_in_c_order() {
    assert_eq!(AI_PRESETS.len(), 24);
    let names: Vec<&str> = AI_PRESETS.iter().map(|p| p.name).collect();
    assert_eq!(
        names,
        vec![
            "",
            "Zakath",
            "Mazian",
            "Durnroth",
            "Saphira",
            "Cleran",
            "Dagdar",
            "Karkarath",
            "Vashini",
            "Kurbatz",
            "Kalim",
            "Sumpfbatz",
            "Umfrag",
            "Sickan",
            "Logasi",
            "Sumso",
            "Karka",
            "Rungan",
            "Kirlo",
            "Surgao",
            "Huwa",
            "Losaki",
            "Death",
            "Despair"
        ]
    );
}

#[test]
// The tuple layout intentionally mirrors C's `struct ai_preset` columns.
#[allow(clippy::type_complexity)]
fn ai_presets_table_matches_c_literal_values_digit_for_digit() {
    // struct ai_preset preset[64] (strategy.c:162-200); columns:
    // worker level trspeed income endur warc speed EGuards EGuardlevel
    let expected: [(&str, i32, i32, i32, i32, i32, i32, i32, i32, i32); 24] = [
        ("", 0, 0, 0, 0, 0, 0, 0, 0, 0),
        ("Zakath", 4, 60, 1, 0, 0, 0, 0, 1, 55),
        ("Mazian", 4, 60, 1, 0, 0, 0, 0, 1, 55),
        ("Durnroth", 4, 60, 1, 0, 0, 5, 0, 1, 55),
        ("Saphira", 8, 60, 1, 0, 0, 15, 5, 1, 65),
        ("Cleran", 4, 60, 1, 0, 0, 5, 5, 1, 55),
        ("Dagdar", 4, 65, 1, 0, 5, 15, 5, 1, 65),
        ("Karkarath", 4, 65, 1, 0, 5, 15, 5, 1, 65),
        ("Vashini", 8, 65, 2, 0, 10, 25, 5, 1, 65),
        ("Kurbatz", 12, 70, 2, 0, 25, 40, 15, 1, 70),
        ("Kalim", 6, 70, 2, 0, 10, 25, 5, 1, 65),
        ("Sumpfbatz", 6, 70, 2, 0, 25, 25, 15, 1, 70),
        ("Umfrag", 6, 70, 2, 0, 25, 40, 15, 1, 70),
        ("Sickan", 8, 75, 3, 0, 30, 45, 20, 1, 75),
        ("Logasi", 8, 75, 3, 0, 30, 45, 20, 1, 75),
        ("Sumso", 20, 80, 4, 0, 35, 60, 30, 1, 90),
        ("Karka", 4, 85, 4, 0, 40, 65, 35, 1, 95),
        ("Rungan", 12, 85, 4, 0, 40, 65, 35, 1, 90),
        ("Kirlo", 12, 85, 4, 0, 40, 65, 35, 1, 90),
        ("Surgao", 12, 85, 4, 0, 40, 65, 35, 1, 90),
        ("Huwa", 12, 90, 5, 0, 45, 70, 45, 1, 95),
        ("Losaki", 12, 90, 5, 0, 45, 70, 45, 1, 95),
        ("Death", 16, 115, 8, 20, 115, 115, 115, 1, 115),
        ("Despair", 16, 115, 8, 20, 115, 115, 115, 1, 115),
    ];

    for (preset, (name, worker, level, trspeed, income, endur, warc, speed, eguards, eguardlvl)) in
        AI_PRESETS.iter().zip(expected.iter())
    {
        assert_eq!(preset.name, *name);
        assert_eq!(preset.max_worker, *worker, "{name} max_worker");
        assert_eq!(preset.max_level, *level, "{name} max_level");
        assert_eq!(preset.trainspeed, *trspeed, "{name} trainspeed");
        assert_eq!(preset.income, *income, "{name} income");
        assert_eq!(preset.endurance, *endur, "{name} endurance");
        assert_eq!(preset.warcry, *warc, "{name} warcry");
        assert_eq!(preset.speed, *speed, "{name} speed");
        assert_eq!(preset.eguards, *eguards, "{name} eguards");
        assert_eq!(preset.eguardlvl, *eguardlvl, "{name} eguardlvl");
    }
}

#[test]
fn str_exp_cost_reports_cap_thresholds_from_c() {
    let mut ppd = StrategyPpd::default();
    // Slot 1 (income): cost 25 until it reaches 20.
    assert_eq!(str_exp_cost(&ppd, 1), 25);
    ppd.income = 20;
    assert_eq!(str_exp_cost(&ppd, 1), 0);

    // Slot 3 (max_worker): cost 10 until it reaches 16.
    let mut ppd = StrategyPpd::default();
    assert_eq!(str_exp_cost(&ppd, 3), 10);
    ppd.max_worker = 16;
    assert_eq!(str_exp_cost(&ppd, 3), 0);

    // Slot 7 (speed): cost 6 until 115.
    let mut ppd = StrategyPpd::default();
    assert_eq!(str_exp_cost(&ppd, 7), 6);
    ppd.speed = 115;
    assert_eq!(str_exp_cost(&ppd, 7), 0);

    // Unknown slot -> 0, same as C's default: branch.
    assert_eq!(str_exp_cost(&StrategyPpd::default(), 9), 0);
    assert_eq!(str_exp_cost(&StrategyPpd::default(), 0), 0);
}

#[test]
fn str_increment_matches_c_table_and_zeroes_out_when_capped() {
    let fresh = StrategyPpd::default();
    assert_eq!(str_increment(&fresh, 1), 1);
    assert_eq!(str_increment(&fresh, 2), 2);
    assert_eq!(str_increment(&fresh, 3), 1);
    assert_eq!(str_increment(&fresh, 4), 1);
    assert_eq!(str_increment(&fresh, 5), 5);
    assert_eq!(str_increment(&fresh, 6), 5);
    assert_eq!(str_increment(&fresh, 7), 5);
    assert_eq!(str_increment(&fresh, 8), 1);
    assert_eq!(str_increment(&fresh, 42), 0);

    let mut capped = StrategyPpd::default();
    capped.income = 20;
    assert_eq!(str_increment(&capped, 1), 0);
}

#[test]
fn str_raise_rejects_when_already_capped() {
    let mut ppd = StrategyPpd::default();
    ppd.income = 20;
    ppd.exp = 1000;
    let outcome = str_raise(&mut ppd, 1);
    assert_eq!(outcome, StrategyRaiseOutcome::CannotRaiseHigher);
    // Nothing changed.
    assert_eq!(ppd.income, 20);
    assert_eq!(ppd.exp, 1000);
}

#[test]
fn str_raise_rejects_when_exp_insufficient() {
    let mut ppd = StrategyPpd::default();
    ppd.exp = 24; // slot 1 costs 25
    let outcome = str_raise(&mut ppd, 1);
    assert_eq!(outcome, StrategyRaiseOutcome::CannotAfford { cost: 25 });
    assert_eq!(ppd.income, 0);
    assert_eq!(ppd.exp, 24);
}

#[test]
fn str_raise_spends_exp_and_applies_increment() {
    let mut ppd = StrategyPpd::default();
    ppd.exp = 100;
    let outcome = str_raise(&mut ppd, 1);
    assert_eq!(outcome, StrategyRaiseOutcome::Raised);
    assert_eq!(ppd.income, 1);
    assert_eq!(ppd.exp, 75);

    // Slot 2 (max_level): +2 per raise, cost 4, capped at 115.
    ppd.max_level = 114;
    ppd.exp = 10;
    let outcome = str_raise(&mut ppd, 2);
    assert_eq!(outcome, StrategyRaiseOutcome::Raised);
    // C: min(114 + 2, 115) == 115 (the raise still costs the full 4 exp
    // even though the increment overshoots the cap).
    assert_eq!(ppd.max_level, 115);
    assert_eq!(ppd.exp, 6);
}

fn strategy_item(id: u32, driver: u16, drdata: Vec<u8>) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.driver_data = drdata;
    it
}

#[test]
fn init_areas_registers_mine_storage_depot_into_their_slot() {
    let mut world = World::default();
    // slot byte lives at drdata[8].
    world.add_item(strategy_item(
        1,
        IDR_STR_MINE,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 2],
    ));
    world.add_item(strategy_item(
        2,
        IDR_STR_STORAGE,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 2],
    ));
    world.add_item(strategy_item(
        3,
        IDR_STR_DEPOT,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 2],
    ));

    world.ensure_strategy_areas_initialized();

    assert_eq!(world.strategy_areas.areas.len(), MAX_STR_AREA);
    let area = &world.strategy_areas.areas[2];
    assert!(area.used);
    assert!(!area.busy);
    assert!(area.spawn.is_empty());
    assert_eq!(
        area.item,
        vec![ItemId(1), ItemId(2), ItemId(3)],
        "ascending item-id discovery order, matching C's `for (n = 1; n < MAXITEM; n++)`"
    );

    // Untouched slots stay unused/default.
    assert!(!world.strategy_areas.areas[0].used);
    assert!(!world.strategy_areas.areas[15].used);
}

#[test]
fn init_areas_spawner_falls_through_into_both_spawn_and_item_arrays() {
    // C `init_areas`'s `switch` has no `break` after `IDR_STR_SPAWNER`, so
    // a spawner item lands in *both* `area[slot].spawn[]` and
    // `area[slot].item[]` (strategy.c:251-260).
    let mut world = World::default();
    world.add_item(strategy_item(
        10,
        IDR_STR_SPAWNER,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 4],
    ));

    world.ensure_strategy_areas_initialized();

    let area = &world.strategy_areas.areas[4];
    assert!(area.used);
    assert_eq!(area.spawn, vec![ItemId(10)]);
    assert_eq!(area.item, vec![ItemId(10)], "spawner also lands in item[]");
}

#[test]
fn init_areas_assigns_ascending_spawn_slot_numbers_into_drdata_ten() {
    // C: `it[n].drdata[10] = area[slot].max_spawn;` before incrementing -
    // the Nth spawner discovered in a slot gets slot number N-1.
    let mut world = World::default();
    world.add_item(strategy_item(
        20,
        IDR_STR_SPAWNER,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 1],
    ));
    world.add_item(strategy_item(
        21,
        IDR_STR_SPAWNER,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 1],
    ));
    world.add_item(strategy_item(
        22,
        IDR_STR_SPAWNER,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 1],
    ));

    world.ensure_strategy_areas_initialized();

    assert_eq!(world.items[&ItemId(20)].driver_data[10], 0);
    assert_eq!(world.items[&ItemId(21)].driver_data[10], 1);
    assert_eq!(world.items[&ItemId(22)].driver_data[10], 2);
    assert_eq!(
        world.strategy_areas.areas[1].spawn,
        vec![ItemId(20), ItemId(21), ItemId(22)]
    );
}

#[test]
fn init_areas_skips_items_with_no_flags_and_unrelated_drivers() {
    // C `if (!it[n].flags) continue;` and the `switch`'s implicit no-op
    // default for any driver other than the four `IDR_STR_*` cases.
    let mut world = World::default();
    let mut unused_mine = strategy_item(1, IDR_STR_MINE, vec![0, 0, 0, 0, 0, 0, 0, 0, 3]);
    unused_mine.flags = ItemFlags::empty();
    world.add_item(unused_mine);
    world.add_item(strategy_item(
        2,
        IDR_STR_TICKER,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 3],
    ));
    world.add_item(strategy_item(
        3,
        IDR_NOSNOW,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 3],
    ));

    world.ensure_strategy_areas_initialized();

    assert!(!world.strategy_areas.areas[3].used);
    assert!(world.strategy_areas.areas[3].item.is_empty());
}

#[test]
fn init_areas_is_idempotent_and_only_scans_once() {
    let mut world = World::default();
    world.add_item(strategy_item(
        1,
        IDR_STR_MINE,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));

    world.ensure_strategy_areas_initialized();
    assert_eq!(world.strategy_areas.areas[0].item.len(), 1);

    // A second call must not re-scan/duplicate entries (mirrors C's
    // `area_init` guard).
    world.add_item(strategy_item(
        2,
        IDR_STR_MINE,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));
    world.ensure_strategy_areas_initialized();
    assert_eq!(world.strategy_areas.areas[0].item.len(), 1);
}

#[test]
fn init_areas_ignores_out_of_range_slot_byte_without_panicking() {
    let mut world = World::default();
    world.add_item(strategy_item(
        1,
        IDR_STR_MINE,
        vec![0, 0, 0, 0, 0, 0, 0, 0, 255],
    ));

    world.ensure_strategy_areas_initialized();

    // No panic, and nothing registered anywhere in-range.
    for area in &world.strategy_areas.areas {
        assert!(area.item.is_empty());
    }
}

#[test]
fn str_raise_max_worker_slot_stops_eligibility_at_sixteen_but_clamp_caps_at_twenty_four() {
    // C: str_exp_cost case 3 gates eligibility on `max_worker < 16`
    // (not the higher `min(..., 24)` clamp inside str_raise's own
    // switch) - so once max_worker reaches 16, slot 3 becomes
    // permanently un-raisable even though its clamp constant is 24.
    let mut ppd = StrategyPpd::default();
    ppd.max_worker = 15;
    ppd.exp = 1000;
    assert_eq!(str_raise(&mut ppd, 3), StrategyRaiseOutcome::Raised);
    assert_eq!(ppd.max_worker, 16);

    assert_eq!(
        str_raise(&mut ppd, 3),
        StrategyRaiseOutcome::CannotRaiseHigher
    );
    assert_eq!(ppd.max_worker, 16);
}

fn strategy_player(id: u32, serial: u32) -> Character {
    let mut c = character(id);
    c.serial = serial;
    c.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    c
}

fn strategy_npc(id: u32, group: u16, name: &str) -> Character {
    let mut c = character(id);
    c.group = group;
    c.name = name.into();
    c
}

fn strategy_worker(id: u32, group: u16, order: StrategyWorkerOrder, platin: i32) -> Character {
    let mut c = character(id);
    c.group = group;
    c.driver = CDR_STRATEGY;
    c.driver_state = Some(CharacterDriverState::StrategyWorker(
        StrategyWorkerDriverData {
            order,
            platin,
            ..Default::default()
        },
    ));
    c
}

#[test]
fn str_item_owner_and_gold_round_trip_through_driver_data() {
    let mut it = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    assert_eq!(str_item_owner(&it), 0);
    set_str_item_owner(&mut it, STR_OWNER_AI_BASE + 5);
    assert_eq!(str_item_owner(&it), STR_OWNER_AI_BASE + 5);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    assert_eq!(str_item_gold(&storage), 0);
    set_str_item_gold(&mut storage, 900);
    assert_eq!(str_item_gold(&storage), 900);
    // Owner bytes untouched by the gold write.
    assert_eq!(str_item_owner(&storage), 0);
}

#[test]
fn str_did_party_lose_false_when_storage_has_enough_gold() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, 42);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, NPCPRICE as u32);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    // C's `noplr` fallback would force `lost = true` for a player-owned
    // party (`code < 0xfffff000`) with no live player found at all,
    // regardless of storage gold - so the owning player must actually be
    // present for the storage-gold check below to matter.
    world.add_character(strategy_player(3, 42));

    assert!(!world.str_did_party_lose(ItemId(1)));
}

#[test]
fn str_did_party_lose_true_when_storage_low_and_no_player() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, 42);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, NPCPRICE as u32 - 1);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    assert!(world.str_did_party_lose(ItemId(1)));
}

#[test]
fn str_did_party_lose_false_when_group_member_alive() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, 42);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, 0);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    // The owning player is present (satisfying the `noplr` fallback), and
    // a live worker with group == 42 keeps the party alive despite empty
    // storage.
    world.add_character(strategy_player(3, 42));
    world.add_character(strategy_npc(4, 42, "Worker"));

    assert!(!world.str_did_party_lose(ItemId(1)));
}

#[test]
fn str_did_party_lose_ai_owned_skips_player_requirement() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, STR_OWNER_AI_BASE + 3);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, 0);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    // AI-owned party: `noplr` starts false, so a low-gold storage with no
    // live worker still isn't forced to "lost" via the no-player fallback
    // - only the storage-gold check matters, which is 0 < NPCPRICE, so
    // this *is* lost (the AI check is about the fallback, not this path).
    assert!(world.str_did_party_lose(ItemId(1)));

    // With enough gold, an AI-owned party with zero live members is not
    // lost (the `noplr` force-lose branch never applies to AI parties).
    let storage = world.items.get_mut(&ItemId(2)).unwrap();
    set_str_item_gold(storage, NPCPRICE as u32);
    assert!(!world.str_did_party_lose(ItemId(1)));
}

#[test]
fn str_remove_party_destroys_grouped_npc_except_cinciac() {
    let mut world = World::default();
    world.add_character(strategy_npc(1, 7, "Worker"));
    world.add_character(strategy_npc(2, 7, "Cinciac"));

    world.str_remove_party(7, None);

    assert!(!world.characters.contains_key(&CharacterId(1)));
    assert!(
        world.characters.contains_key(&CharacterId(2)),
        "Cinciac is never destroyed, matching C's hardcoded safety check"
    );
}

#[test]
fn str_remove_party_never_destroys_a_player_even_if_group_matches() {
    let mut world = World::default();
    let mut player = strategy_player(1, 99);
    player.group = 7;
    world.add_character(player);

    world.str_remove_party(7, None);

    assert!(
        world.characters.contains_key(&CharacterId(1)),
        "C's elog(\"panic...\") branch never destroys a real player"
    );
}

#[test]
fn str_remove_party_teleports_and_messages_the_owning_player() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 555));

    world.str_remove_party(555, Some("You lose. Better luck next time!"));

    let messages = world.drain_pending_system_texts();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].character_id, CharacterId(1));
    assert_eq!(messages[0].message, "You lose. Better luck next time!");
}

#[test]
fn str_remove_party_resets_player_owned_spawner_and_depot_storage() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.driver_data[8] = 3;
    set_str_item_owner(&mut spawner, 555);
    world.add_item(spawner);

    let mut depot = strategy_item(2, IDR_STR_DEPOT, vec![0; 9]);
    depot.driver_data[8] = 3;
    set_str_item_owner(&mut depot, 555);
    world.add_item(depot);

    let mut storage = strategy_item(3, IDR_STR_STORAGE, vec![0; 10]);
    storage.driver_data[8] = 3;
    set_str_item_owner(&mut storage, 555);
    world.add_item(storage);

    world.ensure_strategy_areas_initialized();
    assert!(world.str_remove_party(555, None));

    assert_eq!(str_item_owner(&world.items[&ItemId(1)]), STR_OWNER_NONE);
    assert_eq!(world.items[&ItemId(1)].name, "Spawner (3)");
    assert_eq!(str_item_owner(&world.items[&ItemId(2)]), STR_OWNER_NONE);
    assert_eq!(world.items[&ItemId(2)].name, "Depot (3)");
    assert_eq!(str_item_owner(&world.items[&ItemId(3)]), STR_OWNER_NONE);
    assert_eq!(world.items[&ItemId(3)].name, "Storage (3)");
}

#[test]
fn str_remove_party_resets_ai_owned_spawner_to_unassigned_not_free() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.driver_data[8] = 3;
    let code = STR_OWNER_AI_BASE + 2;
    set_str_item_owner(&mut spawner, code);
    world.add_item(spawner);

    world.ensure_strategy_areas_initialized();
    assert!(world.str_remove_party(code, None));

    assert_eq!(
        str_item_owner(&world.items[&ItemId(1)]),
        STR_OWNER_AI_UNASSIGNED
    );
}

#[test]
fn str_remove_party_returns_false_when_code_owns_no_spawner() {
    let mut world = World::default();
    world.ensure_strategy_areas_initialized();
    assert!(!world.str_remove_party(12345, None));
}

#[test]
fn str_close_area_removes_every_owned_spawn() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.driver_data[8] = 5;
    set_str_item_owner(&mut spawner, 777);
    world.add_item(spawner);
    world.add_character(strategy_player(2, 777));

    world.ensure_strategy_areas_initialized();
    world.str_close_area(5);

    assert_eq!(str_item_owner(&world.items[&ItemId(1)]), STR_OWNER_NONE);
}

#[test]
fn apply_strategy_mission_win_rewards_and_resets_current_mission() {
    let mut ppd = StrategyPpd::default();
    ppd.current_mission = 2; // "B", exp: 25, set_solve: 2
    let outcome = apply_strategy_mission_win(&mut ppd, 2);
    assert_eq!(outcome, StrategyWinOutcome::Rewarded { exp: 25 });
    assert_eq!(ppd.won_cnt, 1);
    assert_eq!(ppd.exp, 25);
    assert_eq!(ppd.boss_exp, 25);
    assert_eq!(ppd.eguards, 1);
    assert_eq!(ppd.solve_count(2), 1);
    assert_eq!(ppd.current_mission, 0);
}

#[test]
fn apply_strategy_mission_win_no_reward_for_zero_exp_mission() {
    let mut ppd = StrategyPpd::default();
    ppd.current_mission = 10; // "J 2P", exp: 0
    let outcome = apply_strategy_mission_win(&mut ppd, 10);
    assert_eq!(outcome, StrategyWinOutcome::NoReward);
    assert_eq!(ppd.won_cnt, 0);
    assert_eq!(ppd.exp, 0);
    assert_eq!(ppd.current_mission, 0, "still reset, matching C");
}

#[test]
fn apply_strategy_mission_win_bad_index_reports_bug() {
    let mut ppd = StrategyPpd::default();
    assert_eq!(
        apply_strategy_mission_win(&mut ppd, 999),
        StrategyWinOutcome::BadMissionIndex
    );
    assert_eq!(
        apply_strategy_mission_win(&mut ppd, -1),
        StrategyWinOutcome::BadMissionIndex
    );
}

#[test]
fn str_reward_winner_queues_event_for_matching_player_serial() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 4242));

    world.str_reward_winner(4242);

    let events = world.drain_pending_strategy_rewards();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].character_id, CharacterId(1));
}

#[test]
fn str_reward_winner_no_match_queues_nothing() {
    let mut world = World::default();
    world.str_reward_winner(999);
    assert!(world.drain_pending_strategy_rewards().is_empty());
}

#[test]
fn str_init_mission_resets_depot_storage_mine_and_assigns_ai_spawner() {
    let mut world = World::default();
    let mission_area = MISSIONS[0].area as usize; // "A-1", area 1

    let mut depot = strategy_item(1, IDR_STR_DEPOT, vec![0; 9]);
    depot.driver_data[8] = mission_area as u8;
    set_str_item_owner(&mut depot, 555);
    set_str_item_gold(&mut depot, 100);
    world.add_item(depot);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.driver_data[8] = mission_area as u8;
    world.add_item(storage);

    let mut mine = strategy_item(3, IDR_STR_MINE, vec![0; 9]);
    mine.driver_data[8] = mission_area as u8;
    world.add_item(mine);

    // Pre-marked AI slot (statically placed in zone data as AI-only).
    let mut ai_spawner = strategy_item(4, IDR_STR_SPAWNER, vec![0; 11]);
    ai_spawner.driver_data[8] = mission_area as u8;
    set_str_item_owner(&mut ai_spawner, STR_OWNER_AI_UNASSIGNED);
    world.add_item(ai_spawner);

    // Pre-marked player slot.
    let mut player_spawner = strategy_item(5, IDR_STR_SPAWNER, vec![0; 11]);
    player_spawner.driver_data[8] = mission_area as u8;
    world.add_item(player_spawner);

    assert!(world.str_init_mission(0));

    assert_eq!(str_item_owner(&world.items[&ItemId(1)]), STR_OWNER_NONE);
    assert_eq!(str_item_gold(&world.items[&ItemId(1)]), 0);
    assert_eq!(
        world.items[&ItemId(1)].name,
        format!("Depot ({mission_area})")
    );

    assert_eq!(str_item_owner(&world.items[&ItemId(2)]), STR_OWNER_NONE);
    assert_eq!(
        str_item_gold(&world.items[&ItemId(2)]),
        MISSIONS[0].storage_size as u32
    );

    assert_eq!(str_item_owner(&world.items[&ItemId(3)]), STR_OWNER_NONE);
    assert_eq!(
        str_item_gold(&world.items[&ItemId(3)]),
        MISSIONS[0].mine_size as u32
    );

    assert_eq!(
        str_item_owner(&world.items[&ItemId(4)]),
        STR_OWNER_AI_BASE + MISSIONS[0].enemy[0] as u32
    );
    assert_eq!(str_item_owner(&world.items[&ItemId(5)]), STR_OWNER_NONE);

    assert!(world.strategy_areas.areas[mission_area].busy);
}

#[test]
fn str_init_mission_rejects_out_of_range_index() {
    let mut world = World::default();
    assert!(!world.str_init_mission(999));
}

#[test]
fn str_ticker_rewards_and_closes_area_on_lone_player_win() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.driver_data[8] = 6;
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, 4242);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.driver_data[8] = 6;
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, NPCPRICE as u32);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    world.add_character(strategy_player(3, 4242));
    world.ensure_strategy_areas_initialized();

    world.str_ticker();

    let events = world.drain_pending_strategy_rewards();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].character_id, CharacterId(3));
    assert!(world.strategy_areas.areas[6].busy);
}

#[test]
fn str_ticker_removes_lost_party_and_closes_when_only_ai_remains() {
    let mut world = World::default();
    // A lone AI-owned spawner in a slot with no player/worker: `pl==0,
    // ai==1` after the scan -> C's `else if (ai && !pl)` branch closes
    // the area without a reward.
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.driver_data[8] = 7;
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, STR_OWNER_AI_BASE + 1);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.driver_data[8] = 7;
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, NPCPRICE as u32);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    world.ensure_strategy_areas_initialized();
    world.str_ticker();

    // No reward event for a pure-AI close.
    assert!(world.drain_pending_strategy_rewards().is_empty());
    assert!(world.strategy_areas.areas[7].busy);
}

#[test]
fn str_ticker_clears_busy_flag_once_a_slot_goes_fully_idle() {
    let mut world = World::default();
    world.ensure_strategy_areas_initialized();
    world.strategy_areas.areas[8].used = true;
    world.strategy_areas.areas[8].busy = true;

    world.str_ticker();

    assert!(!world.strategy_areas.areas[8].busy);
}

#[test]
fn str_ticker_reschedules_itself_via_apply_item_driver_outcome() {
    // Same real bug/fix as `lq_ticker_reschedules_itself_via_apply_item_
    // driver_outcome` (`world/tests/lq.rs`): `str_ticker`'s own C
    // self-reschedule (`strategy.c:462`) used to only be applied by
    // `ugaris-server`'s player-`item_use`-completion dispatcher, a call
    // path a `character_id == 0` timer-fired `StrTicker` outcome never
    // actually reaches. `World::apply_item_driver_outcome` now applies
    // the reschedule directly, so the ticker keeps firing forever
    // instead of going silent after its first call.
    let mut world = World::default();
    let mut ticker = strategy_item(7, IDR_STR_TICKER, vec![]);
    ticker.flags = ItemFlags::USED | ItemFlags::USE;
    world.add_item(ticker);

    assert_eq!(world.timers.used_timers(), 0);
    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_STR_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        23,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert!(matches!(outcome, ItemDriverOutcome::StrTicker { .. }));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn str_spawner_driver_produces_ambient_tick_outcome_for_a_timer_call() {
    // Same shape as `str_ticker_reschedules_itself_via_apply_item_driver_
    // outcome`: exercises the real driver dispatch -> `World::
    // apply_item_driver_outcome` path end to end for `IDR_STR_SPAWNER`'s
    // `cn == 0` branch (`strategy.c:1319-1356`).
    let mut world = World::default();
    let mut spawner = strategy_item(7, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.flags = ItemFlags::USED | ItemFlags::USE;
    set_str_item_owner(&mut spawner, STR_OWNER_AI_UNASSIGNED);
    world.add_item(spawner);

    assert_eq!(world.timers.used_timers(), 0);
    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_STR_SPAWNER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        23,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert!(matches!(
        outcome,
        ItemDriverOutcome::StrSpawnerAmbientTick { .. }
    ));
    // C: `code == 0xfffff000` reschedules only, no other effect.
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn str_spawner_ambient_tick_stops_forever_for_a_player_owned_slot() {
    // C `spawner`'s `cn == 0` branch has no reschedule at all once the
    // owner code is a real player serial (or the unclaimed
    // `STR_OWNER_NONE`) - the ambient chain silently dies.
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    set_str_item_owner(&mut spawner, 42);
    world.add_item(spawner);

    world.str_spawner_ambient_tick(ItemId(1));

    assert_eq!(world.timers.used_timers(), 0);
}

#[test]
fn str_spawner_ambient_tick_first_activation_renames_and_seeds_storage_income() {
    let mut world = World::default();
    // AI_PRESETS[22] == "Death", income 20 (strategy.c's `preset[22]`).
    let code = STR_OWNER_AI_BASE + 22;

    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    spawner.driver_data[8] = 3; // slot
    set_str_item_owner(&mut spawner, code);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    world.str_spawner_ambient_tick(ItemId(1));

    // Rescheduled, but `ai_init`/`ai_main` deferred to the *next* ambient
    // tick (see `World::str_spawner_first_activation`'s own doc comment).
    assert_eq!(world.timers.used_timers(), 1);
    assert!(!world.ai_parties.contains_key(&code));
    assert!(world.drain_pending_ai_worker_spawns().is_empty());

    let spawner = &world.items[&ItemId(1)];
    assert_eq!(spawner.driver_data[9], 1, "init-done flag set");
    assert_eq!(spawner.name, "Death's Spawner (3)");

    let storage = &world.items[&ItemId(2)];
    assert_eq!(storage.driver_data[9], 20, "income seeded from the preset");
    assert_eq!(storage.name, "Death's Storage (3)");
}

#[test]
fn str_spawner_ambient_tick_steady_state_calls_ai_main_and_queues_a_worker_plan() {
    let mut world = World::default();
    let code = STR_OWNER_AI_BASE + 1; // "Zakath"

    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, code);
    spawner.driver_data[9] = 1; // init already done on an earlier tick
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, NPCPRICE as u32 * 2);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    world.str_spawner_ambient_tick(ItemId(1));

    assert_eq!(world.timers.used_timers(), 1);
    assert!(
        world.ai_parties.contains_key(&code),
        "ai_main's own !ad->ai_init fallback should have run ai_init"
    );
    let plans = world.drain_pending_ai_worker_spawns();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].0, code);
    assert_eq!(plans[0].1.owner_name, "Zakath");
}

#[test]
fn queue_mission_appends_to_first_free_slot() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.add_character(strategy_player(2, 222));

    world.queue_mission(CharacterId(1), 3);
    world.queue_mission(CharacterId(2), 3);

    let area = &world.strategy_areas.areas[3];
    assert_eq!(area.q_player_cn[0], Some(CharacterId(1)));
    assert_eq!(area.q_player_id[0], 111);
    assert_eq!(area.q_player_cn[1], Some(CharacterId(2)));
    assert_eq!(area.q_player_id[1], 222);
    assert_eq!(area.q_player_cn[2], None);
}

#[test]
fn queue_mission_is_a_no_op_when_already_queued() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));

    world.queue_mission(CharacterId(1), 3);
    world.queue_mission(CharacterId(1), 3);

    let area = &world.strategy_areas.areas[3];
    assert_eq!(area.q_player_cn[0], Some(CharacterId(1)));
    assert_eq!(area.q_player_cn[1], None);
}

#[test]
fn queue_mission_moves_a_stale_entry_from_another_area() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));

    world.queue_mission(CharacterId(1), 2);
    world.queue_mission(CharacterId(1), 3);

    assert_eq!(world.strategy_areas.areas[2].q_player_cn[0], None);
    assert_eq!(
        world.strategy_areas.areas[3].q_player_cn[0],
        Some(CharacterId(1))
    );
}

#[test]
fn queue_mission_drops_request_when_queue_is_full() {
    let mut world = World::default();
    for id in 1..=5u32 {
        world.add_character(strategy_player(id, id * 10));
    }
    for id in 1..=4u32 {
        world.queue_mission(CharacterId(id), 3);
    }

    world.queue_mission(CharacterId(5), 3);

    let area = &world.strategy_areas.areas[3];
    assert!(area.q_player_cn.iter().all(|c| *c != Some(CharacterId(5))));
}

#[test]
fn queue_validate_drops_and_compacts_entries_for_logged_off_characters() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.add_character(strategy_player(2, 222));
    world.add_character(strategy_player(3, 333));

    world.queue_mission(CharacterId(1), 3);
    world.queue_mission(CharacterId(2), 3);
    world.queue_mission(CharacterId(3), 3);

    // Character 2 logs off.
    world.characters.remove(&CharacterId(2));

    world.queue_validate(3);

    let area = &world.strategy_areas.areas[3];
    assert_eq!(area.q_player_cn[0], Some(CharacterId(1)));
    assert_eq!(area.q_player_cn[1], Some(CharacterId(3)));
    assert_eq!(area.q_player_cn[2], None);
    assert_eq!(area.q_player_id[2], 0);
}

#[test]
fn queue_remove_clears_a_character_from_every_area() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));

    world.queue_mission(CharacterId(1), 2);

    world.queue_remove(CharacterId(1));

    assert_eq!(world.strategy_areas.areas[2].q_player_cn[0], None);
}

#[test]
fn queue_check_true_when_queue_empty_or_character_is_at_the_head() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.add_character(strategy_player(2, 222));

    assert!(world.queue_check(CharacterId(1), 3));

    world.queue_mission(CharacterId(1), 3);
    assert!(world.queue_check(CharacterId(1), 3));

    world.queue_mission(CharacterId(2), 3);
    assert!(!world.queue_check(CharacterId(2), 3));
}

#[test]
fn show_queue_sends_header_before_validating_then_one_line_per_entry() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.add_character(strategy_player(2, 222));
    world.characters.get_mut(&CharacterId(1)).unwrap().name = "Alice".into();
    world.characters.get_mut(&CharacterId(2)).unwrap().name = "Bob".into();

    world.queue_mission(CharacterId(1), 3);
    world.queue_mission(CharacterId(2), 3);

    world.show_queue(CharacterId(1), 3);

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 3);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "Queue:");
    assert_eq!(texts[1].message, "1: Alice");
    assert_eq!(texts[2].message, "2: Bob");
}

// C `mine`/`storage`/`depot`'s `ch[cn].flags & CF_PLAYER` branches
// (`strategy.c:1122-1241`), plus their `DRD_STRATEGYDRIVER` NPC-worker
// branches - see `item_driver::area23_24`'s module doc comment for the
// remaining `cn == 0` ambient-branch gap every one of these three still
// has.

fn mine_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_STR_MINE,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

fn depot_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_STR_DEPOT,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

fn storage_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_STR_STORAGE,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

#[test]
fn str_mine_look_reports_current_platinum_to_the_player() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut mine = strategy_item(7, IDR_STR_MINE, vec![0; 8]);
    set_str_item_gold(&mut mine, 5000);
    world.add_item(mine);

    let outcome = world.execute_item_driver_request(mine_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrMineLook {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            platinum: 5000,
        }
    );
}

#[test]
fn str_mine_ambient_call_remains_a_documented_noop() {
    let mut world = World::default();
    world.add_character(timer_callback_character());
    let mine = strategy_item(7, IDR_STR_MINE, vec![0; 8]);
    world.add_item(mine);

    // C's `cn==0` cosmetic-naming branch: still unported (see
    // `item_driver::area23_24`'s module doc comment).
    assert_eq!(
        world.execute_item_driver_request(mine_request(7, 0), 23),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn str_mine_npc_worker_with_an_empty_mine_or_no_strength_is_a_noop() {
    let mut world = World::default();
    world.add_character(strategy_npc(2, 5, "Worker"));
    let mine = strategy_item(7, IDR_STR_MINE, vec![0; 8]);
    world.add_item(mine);

    assert_eq!(
        world.execute_item_driver_request(mine_request(7, 2), 23),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn str_mine_npc_worker_digs_min_of_strength_and_available_gold() {
    // C `mine`'s `DRD_STRATEGYDRIVER` branch (`strategy.c:1140-1146`):
    // `am = min(ch[cn].value[0][V_STR], gold); gold -= am; dat->platin +=
    // am;`.
    let mut world = World::default();
    let mut worker = strategy_worker(2, 5, StrategyWorkerOrder::None, 10);
    worker.values[0][CharacterValue::Strength as usize] = 40;
    world.add_character(worker);
    let mut mine = strategy_item(7, IDR_STR_MINE, vec![0; 8]);
    set_str_item_gold(&mut mine, 25);
    world.add_item(mine);

    let outcome = world.execute_item_driver_request(mine_request(7, 2), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrMineWorkerDig {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            mined: 25,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 0);
    let CharacterDriverState::StrategyWorker(data) = world.characters[&CharacterId(2)]
        .driver_state
        .clone()
        .unwrap()
    else {
        panic!("expected StrategyWorker driver state");
    };
    assert_eq!(data.platin, 35);
}

#[test]
fn str_depot_look_reports_current_platinum_to_the_player() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut depot = strategy_item(7, IDR_STR_DEPOT, vec![0; 8]);
    set_str_item_gold(&mut depot, 1234);
    world.add_item(depot);

    let outcome = world.execute_item_driver_request(depot_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrDepotLook {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            platinum: 1234,
        }
    );
}

#[test]
fn str_storage_look_with_no_cursor_item_reports_current_platinum_only() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 600);
    world.add_item(storage);

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::None,
            platinum: 600,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 600);
}

fn enhance_stack(item_id: u32, character_id: u32, kind: u8, amount: u32) -> Item {
    let mut it = item(item_id, ItemFlags::USED);
    it.driver = IDR_ENHANCE;
    it.driver_data = vec![0; 5];
    it.driver_data[0] = kind;
    it.driver_data[1..5].copy_from_slice(&amount.to_le_bytes());
    it.carried_by = Some(CharacterId(character_id));
    it
}

#[test]
fn str_storage_converts_a_carried_silver_stack_at_fifty_to_one() {
    // C: `it[in2].drdata[0] == 1` -> `am = *(unsigned int*)(drdata+1) / 50`.
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 600);
    world.add_item(storage);
    world.add_item(enhance_stack(9, 1, 1, 5000));

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::Converted {
                cursor_item_id: ItemId(9),
                added: 100,
            },
            platinum: 700,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 700);
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn str_storage_converts_a_carried_gold_stack_at_five_to_one() {
    // C: `it[in2].drdata[0] == 2` -> `am = *(unsigned int*)(drdata+1) / 5`.
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    world.add_item(storage);
    world.add_item(enhance_stack(9, 1, 2, 1000));

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::Converted {
                cursor_item_id: ItemId(9),
                added: 200,
            },
            platinum: 200,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 200);
    assert!(!world.items.contains_key(&ItemId(9)));
}

#[test]
fn str_storage_rejects_a_non_silver_gold_enhance_kind_without_mutation() {
    // C: `am` stays `0` for any `drdata[0]` other than `1`/`2`, so the
    // warning message prints and nothing is destroyed/credited.
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 600);
    world.add_item(storage);
    world.add_item(enhance_stack(9, 1, 3, 5000));

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::WrongKind,
            platinum: 600,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 600);
    assert!(world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        Some(ItemId(9))
    );
}

#[test]
fn str_storage_treats_a_conversion_that_rounds_down_to_zero_as_wrong_kind() {
    // C: `10 / 50 == 0` in integer division - `am` is `0` even though
    // `drdata[0]` was a valid silver code.
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    world.add_item(storage);
    world.add_item(enhance_stack(9, 1, 1, 10));

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::WrongKind,
            platinum: 0,
        }
    );
    assert!(world.items.contains_key(&ItemId(9)));
}

#[test]
fn str_storage_ignores_a_cursor_item_that_is_not_an_enhance_stack() {
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 600);
    world.add_item(storage);
    world.add_item(item(9, ItemFlags::USED));

    let outcome = world.execute_item_driver_request(storage_request(7, 1), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrStorageInteract {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            conversion: StrStorageConversion::None,
            platinum: 600,
        }
    );
    assert!(world.items.contains_key(&ItemId(9)));
}

#[test]
fn str_storage_ambient_call_remains_a_documented_noop() {
    let mut world = World::default();
    world.add_character(timer_callback_character());
    world.add_item(strategy_item(7, IDR_STR_STORAGE, vec![0; 8]));

    // C's `cn==0` periodic-income-tick branch: still unported (see
    // `item_driver::area23_24`'s module doc comment).
    assert_eq!(
        world.execute_item_driver_request(storage_request(7, 0), 23),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn str_storage_npc_worker_deposits_its_full_carried_platin() {
    // C `storage`'s `DRD_STRATEGYDRIVER` branch, `dat->platin` nonzero
    // case (`strategy.c:1196-1198`).
    let mut world = World::default();
    world.add_character(strategy_worker(2, 5, StrategyWorkerOrder::None, 40));
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 100);
    world.add_item(storage);

    let outcome = world.execute_item_driver_request(storage_request(7, 2), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrBuildingWorkerTransfer {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            deposited: 40,
            withdrawn: 0,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 140);
    let CharacterDriverState::StrategyWorker(data) = world.characters[&CharacterId(2)]
        .driver_state
        .clone()
        .unwrap()
    else {
        panic!("expected StrategyWorker driver state");
    };
    assert_eq!(data.platin, 0);
}

#[test]
fn str_storage_npc_worker_withdraws_min_of_strength_and_available_gold() {
    // C `storage`'s `DRD_STRATEGYDRIVER` branch, `dat->platin == 0` case
    // (`strategy.c:1200-1202`).
    let mut world = World::default();
    let mut worker = strategy_worker(2, 5, StrategyWorkerOrder::None, 0);
    worker.values[0][CharacterValue::Strength as usize] = 15;
    world.add_character(worker);
    let mut storage = strategy_item(7, IDR_STR_STORAGE, vec![0; 8]);
    set_str_item_gold(&mut storage, 100);
    world.add_item(storage);

    let outcome = world.execute_item_driver_request(storage_request(7, 2), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrBuildingWorkerTransfer {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            deposited: 0,
            withdrawn: 15,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 85);
    let CharacterDriverState::StrategyWorker(data) = world.characters[&CharacterId(2)]
        .driver_state
        .clone()
        .unwrap()
    else {
        panic!("expected StrategyWorker driver state");
    };
    assert_eq!(data.platin, 15);
}

#[test]
fn str_depot_npc_worker_claims_ownership_instead_of_transferring() {
    // C `depot`'s ownership-takeover branch (`strategy.c:1224-1229`).
    let mut world = World::default();
    world.add_character(strategy_worker(2, 5, StrategyWorkerOrder::None, 40));
    let mut depot = strategy_item(7, IDR_STR_DEPOT, vec![0; 9]);
    set_str_item_gold(&mut depot, 100);
    world.add_item(depot);

    let outcome = world.execute_item_driver_request(depot_request(7, 2), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrDepotWorkerTakeover {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            owner: 5,
        }
    );
    let depot_item = &world.items[&ItemId(7)];
    assert_eq!(str_item_owner(depot_item), 5);
    // Unchanged: C returns right after claiming, no platin transfer this
    // call.
    assert_eq!(str_item_gold(depot_item), 100);
}

#[test]
fn str_depot_npc_worker_transfers_platin_once_ownership_matches() {
    let mut world = World::default();
    world.add_character(strategy_worker(2, 5, StrategyWorkerOrder::None, 40));
    let mut depot = strategy_item(7, IDR_STR_DEPOT, vec![0; 9]);
    set_str_item_owner(&mut depot, 5);
    set_str_item_gold(&mut depot, 100);
    world.add_item(depot);

    let outcome = world.execute_item_driver_request(depot_request(7, 2), 23);

    assert_eq!(
        outcome,
        ItemDriverOutcome::StrBuildingWorkerTransfer {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            deposited: 40,
            withdrawn: 0,
        }
    );
    assert_eq!(str_item_gold(&world.items[&ItemId(7)]), 140);
}

// --- `spawner`/`spawner_sub` (`strategy.c:1244-1381`) ---

/// Places a spawner/storage pair at the fixed C layout convention
/// (`spawner2storage`: storage sits directly north, `y - 1`), returning
/// `(spawner_id, storage_id)`.
fn spawner_and_storage(world: &mut World, owner: u32, gold: u32) -> (ItemId, ItemId) {
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, owner);
    world.add_item(spawner);

    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0; 10]);
    storage.x = 5;
    storage.y = 4;
    set_str_item_gold(&mut storage, gold);
    world.add_item(storage);
    world.map.tile_mut(5, 4).expect("tile exists").item = 2;

    (ItemId(1), ItemId(2))
}

fn assert_only_system_text(world: &mut World, character_id: CharacterId, expected: &str) {
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, character_id);
    assert_eq!(texts[0].message, expected);
}

#[test]
fn spawner_use_rejects_ownership_mismatch() {
    let mut world = World::default();
    let (spawner_id, _storage_id) = spawner_and_storage(&mut world, 99, NPCPRICE as u32);
    world.add_character(strategy_player(3, 42));
    let ppd = StrategyPpd {
        max_worker: 4,
        ..Default::default()
    };

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), spawner_id, &ppd);

    assert!(matches!(outcome, StrategySpawnerUseOutcome::Rejected));
    assert_only_system_text(
        &mut world,
        CharacterId(3),
        "This spawner belongs to somebody else.",
    );
    // Gold untouched - rejected before the deduction.
    assert_eq!(str_item_gold(&world.items[&ItemId(2)]), NPCPRICE as u32);
}

#[test]
fn spawner_use_rejects_missing_storage() {
    let mut world = World::default();
    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0; 11]);
    spawner.x = 5;
    spawner.y = 5;
    set_str_item_owner(&mut spawner, 42);
    world.add_item(spawner);
    // No item placed on the tile directly north - `spawner2storage` finds
    // nothing.
    world.add_character(strategy_player(3, 42));
    let ppd = StrategyPpd::default();

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), ItemId(1), &ppd);

    assert!(matches!(outcome, StrategySpawnerUseOutcome::Rejected));
    assert_only_system_text(
        &mut world,
        CharacterId(3),
        "Failed. Please report bug #25476e",
    );
}

#[test]
fn spawner_use_rejects_not_enough_gold() {
    let mut world = World::default();
    let (spawner_id, storage_id) = spawner_and_storage(&mut world, 42, NPCPRICE as u32 - 1);
    world.add_character(strategy_player(3, 42));
    let ppd = StrategyPpd {
        max_worker: 4,
        ..Default::default()
    };

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), spawner_id, &ppd);

    assert!(matches!(outcome, StrategySpawnerUseOutcome::Rejected));
    assert_only_system_text(
        &mut world,
        CharacterId(3),
        "Not enough Platinum to create a worker.",
    );
    assert_eq!(
        str_item_gold(&world.items[&storage_id]),
        NPCPRICE as u32 - 1
    );
}

#[test]
fn spawner_use_rejects_when_worker_cap_reached() {
    let mut world = World::default();
    let (spawner_id, storage_id) = spawner_and_storage(&mut world, 42, NPCPRICE as u32);
    world.add_character(strategy_player(3, 42));
    // One existing worker already fills the (max_worker == 1) cap.
    world.add_character(strategy_worker(4, 42, StrategyWorkerOrder::None, 0));
    let ppd = StrategyPpd {
        max_worker: 1,
        ..Default::default()
    };

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), spawner_id, &ppd);

    assert!(matches!(outcome, StrategySpawnerUseOutcome::Rejected));
    assert_only_system_text(
        &mut world,
        CharacterId(3),
        "No space to drop char or max worker reached.",
    );
    // Gold untouched - the cap check runs before the deduction.
    assert_eq!(str_item_gold(&world.items[&storage_id]), NPCPRICE as u32);
}

#[test]
fn spawner_use_ignores_eternal_guards_when_counting_the_cap() {
    let mut world = World::default();
    let (spawner_id, _storage_id) = spawner_and_storage(&mut world, 42, NPCPRICE as u32);
    world.add_character(strategy_player(3, 42));
    // An eternal guard doesn't count toward `max_worker`, so a cap of 1
    // still leaves room for a new recruit.
    world.add_character(strategy_worker(
        4,
        42,
        StrategyWorkerOrder::EternalGuard { x: 1, y: 1 },
        0,
    ));
    let ppd = StrategyPpd {
        max_worker: 1,
        ..Default::default()
    };

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), spawner_id, &ppd);

    assert!(matches!(outcome, StrategySpawnerUseOutcome::Ready(_)));
}

#[test]
fn spawner_use_ready_deducts_npcprice_and_carries_ppd_fields() {
    let mut world = World::default();
    let (spawner_id, storage_id) = spawner_and_storage(&mut world, 42, NPCPRICE as u32 + 50);
    world.add_character(strategy_player(3, 42));
    let ppd = StrategyPpd {
        max_worker: 4,
        warcry: 5,
        endurance: 10,
        speed: 3,
        trainspeed: 2,
        max_level: 90,
        npc_color: 1,
        ..Default::default()
    };

    let outcome = world.try_dispatch_strategy_spawner_use(CharacterId(3), spawner_id, &ppd);

    let StrategySpawnerUseOutcome::Ready(plan) = outcome else {
        panic!("expected Ready, got a Rejected outcome");
    };
    assert_eq!(plan.spawner_id, spawner_id);
    assert_eq!(plan.group, 42);
    assert_eq!(plan.owner_name, world.characters[&CharacterId(3)].name);
    assert_eq!(plan.warcry, 5);
    assert_eq!(plan.endurance, 10);
    assert_eq!(plan.speed, 3);
    assert_eq!(plan.trainspeed, 2);
    assert_eq!(plan.max_level, 90);
    assert_eq!(plan.npc_color, 1);
    // Deducted immediately, before any character is ever created - see
    // `try_dispatch_strategy_spawner_use`'s own doc comment on the C
    // "spend Platinum even if the drop later fails" quirk.
    assert_eq!(str_item_gold(&world.items[&storage_id]), 50);
    // No message queued on success (C's `spawner_sub` logs nothing).
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn finish_strategy_worker_spawn_stamps_driver_and_driver_state() {
    let mut world = World::default();
    world.add_character(strategy_npc(9, 42, "Worker"));

    world.finish_strategy_worker_spawn(CharacterId(9), "Some Player".to_string(), 2, 90);

    let worker = &world.characters[&CharacterId(9)];
    assert_eq!(worker.driver, CDR_STRATEGY);
    match &worker.driver_state {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            assert_eq!(data.owner_name, "Some Player");
            assert_eq!(data.trainspeed, 2);
            assert_eq!(data.max_level, 90);
            assert_eq!(data.order, StrategyWorkerOrder::None);
        }
        other => panic!("expected StrategyWorker driver state, got {other:?}"),
    }
}
