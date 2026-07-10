use super::*;
use crate::player::StrategyPpd;

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
