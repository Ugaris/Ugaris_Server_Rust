// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::player::StrategyPpd;

#[test]
fn strategy_ppd_defaults_to_zeroed_c_struct() {
    let player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.strategy, StrategyPpd::default());
    assert_eq!(player.strategy.max_worker, 0);
    assert_eq!(player.strategy.exp, 0);
    assert_eq!(player.strategy.current_mission, 0);
    assert!(player.strategy.solve_cnt.is_empty());
}

#[test]
fn solve_count_reads_zero_for_never_written_indices() {
    let ppd = StrategyPpd::default();
    assert_eq!(ppd.solve_count(0), 0);
    assert_eq!(ppd.solve_count(63), 0);
    // Out-of-range index (beyond C's 64-entry array) also reads as 0
    // rather than panicking.
    assert_eq!(ppd.solve_count(1000), 0);
}

#[test]
fn increment_solve_count_grows_the_backing_vec_lazily_and_bumps_once() {
    let mut ppd = StrategyPpd::default();
    ppd.increment_solve_count(1); // mission "A-1"/"A-2"'s set_solve slot
    assert_eq!(ppd.solve_count(1), 1);
    // Earlier/later slots are untouched.
    assert_eq!(ppd.solve_count(0), 0);
    assert_eq!(ppd.solve_count(2), 0);

    ppd.increment_solve_count(1);
    ppd.increment_solve_count(1);
    assert_eq!(ppd.solve_count(1), 3);
}

#[test]
fn increment_solve_count_clamps_out_of_range_index_into_c_array_bounds() {
    let mut ppd = StrategyPpd::default();
    ppd.increment_solve_count(9999);
    assert_eq!(ppd.solve_count(crate::world::STRATEGY_MAXMISSION - 1), 1);
    assert_eq!(ppd.solve_cnt.len(), crate::world::STRATEGY_MAXMISSION);
}

#[test]
fn strategy_ppd_round_trips_through_json_like_migration_0020_storage() {
    let mut ppd = StrategyPpd::default();
    ppd.income = 10;
    ppd.max_worker = 8;
    ppd.increment_solve_count(3);
    ppd.increment_solve_count(3);

    let json = serde_json::to_string(&ppd).expect("serialize");
    let round_tripped: StrategyPpd = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(round_tripped, ppd);
}

#[test]
fn strategy_ppd_deserializes_from_json_missing_the_field_entirely() {
    // Simulates a pre-existing player_state_json document saved before
    // this field existed - `#[serde(default)]` on PlayerRuntime::strategy
    // must fill it in rather than failing to deserialize.
    #[derive(serde::Deserialize)]
    struct Probe {
        #[serde(default)]
        strategy: StrategyPpd,
    }
    let probe: Probe = serde_json::from_str("{}").expect("deserialize");
    assert_eq!(probe.strategy, StrategyPpd::default());
}
