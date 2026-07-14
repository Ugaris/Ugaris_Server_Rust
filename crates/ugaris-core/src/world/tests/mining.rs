// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

fn mine_wall(id: u32, silver_base: u8, gold_base: u8, tier: u8) -> Item {
    let mut wall = item(id, ItemFlags::USED | ItemFlags::USE);
    wall.driver_data = vec![silver_base, gold_base, tier];
    wall
}

#[test]
fn classify_mining_roll_matches_default_cumulative_bands() {
    let settings = GameSettings::default();
    // Bands (default settings): Silver 0..6667, Gold 6667..8667,
    // Golem 8667..18667, Orb 18667..18672, CaveIn 18672..20672,
    // Artifact 20672..20872, Nothing 20872..100000.
    assert_eq!(classify_mining_roll(0, &settings), MiningEvent::Silver);
    assert_eq!(classify_mining_roll(6666, &settings), MiningEvent::Silver);
    assert_eq!(classify_mining_roll(6667, &settings), MiningEvent::Gold);
    assert_eq!(classify_mining_roll(8666, &settings), MiningEvent::Gold);
    assert_eq!(classify_mining_roll(8667, &settings), MiningEvent::Golem);
    assert_eq!(classify_mining_roll(18666, &settings), MiningEvent::Golem);
    assert_eq!(classify_mining_roll(18667, &settings), MiningEvent::Orb);
    assert_eq!(classify_mining_roll(18671, &settings), MiningEvent::Orb);
    assert_eq!(classify_mining_roll(18672, &settings), MiningEvent::CaveIn);
    assert_eq!(classify_mining_roll(20671, &settings), MiningEvent::CaveIn);
    assert_eq!(
        classify_mining_roll(20672, &settings),
        MiningEvent::Artifact
    );
    assert_eq!(
        classify_mining_roll(20871, &settings),
        MiningEvent::Artifact
    );
    assert_eq!(classify_mining_roll(20872, &settings), MiningEvent::Nothing);
    assert_eq!(classify_mining_roll(99999, &settings), MiningEvent::Nothing);
}

#[test]
fn classify_mining_roll_respects_event_multipliers() {
    let mut settings = GameSettings::default();
    settings.mining_silver_gold_multiplier = 0.0;
    settings.mining_golem_event_multiplier = 0.0;
    settings.mining_cavein_multiplier = 0.0;
    settings.mining_artifact_multiplier = 0.0;
    // Only the orb band (unscaled) survives: 0..5.
    assert_eq!(classify_mining_roll(0, &settings), MiningEvent::Orb);
    assert_eq!(classify_mining_roll(4, &settings), MiningEvent::Orb);
    assert_eq!(classify_mining_roll(5, &settings), MiningEvent::Nothing);
}

#[test]
fn roll_mining_silver_amount_uses_wall_base_and_miner_bonus() {
    let mut world = World::default();
    let mut player = character(1);
    world.add_character(player.clone());
    world.items.insert(ItemId(7), mine_wall(7, 3, 0, 1));

    world.legacy_random_seed = 42;
    let mut seed = 42_u32;
    let expected_roll = legacy_random_below_from_seed(&mut seed, 7) as i32; // base*2+1 = 7
    let expected_no_miner = expected_roll + 3;

    let amount = world
        .roll_mining_silver_amount(ItemId(7), CharacterId(1))
        .unwrap();
    assert_eq!(amount, expected_no_miner);
    // RNG stream advanced exactly once.
    assert_eq!(world.legacy_random_seed, seed);

    // Miner profession scales the result by +prof/10 (integer division).
    world.legacy_random_seed = 42;
    player.professions[profession::MINER] = 20;
    world.characters.insert(CharacterId(1), player);
    let amount = world
        .roll_mining_silver_amount(ItemId(7), CharacterId(1))
        .unwrap();
    assert_eq!(amount, expected_no_miner + expected_no_miner * 20 / 10);
}

#[test]
fn roll_mining_gold_amount_can_be_zero_for_zero_base() {
    let mut world = World::default();
    world.add_character(character(1));
    world.items.insert(ItemId(7), mine_wall(7, 3, 0, 1));

    // gold_base == 0 -> RANDOM(1) + 0 == 0 regardless of seed.
    for seed in [0_u32, 1, 999, 123_456] {
        world.legacy_random_seed = seed;
        assert_eq!(
            world
                .roll_mining_gold_amount(ItemId(7), CharacterId(1))
                .unwrap(),
            0
        );
    }
}

#[test]
fn roll_mining_golem_rare_matches_default_chance_denominator() {
    let mut world = World::default();
    // Default rare_golem_chance == 25; RANDOM(25) == 0 only for seeds
    // whose next draw resolves to remainder 0. Sweep enough seeds to
    // observe both branches actually occur.
    let mut saw_rare = false;
    let mut saw_normal = false;
    for seed in 0..200_u32 {
        world.legacy_random_seed = seed;
        if world.roll_mining_golem_rare() {
            saw_rare = true;
        } else {
            saw_normal = true;
        }
    }
    assert!(saw_rare, "expected at least one rare roll in the sweep");
    assert!(saw_normal, "expected at least one normal roll in the sweep");
}

#[test]
fn calculate_golem_drop_amount_scales_with_level_and_rarity() {
    let mut world = World::default();
    world.legacy_random_seed = 7;
    // level_divisor=10, base_drop_multiplier=8 (defaults): level 40 ->
    // base_min = (40/10)*8 = 32, base_max = 40, span 0..=8.
    let amount = world.calculate_golem_drop_amount(40, false);
    assert!((32..=40).contains(&amount), "amount was {amount}");

    world.legacy_random_seed = 7;
    let rare_amount = world.calculate_golem_drop_amount(40, true);
    // rare_drop_multiplier defaults to 1.2; same roll sequence as above.
    assert_eq!(rare_amount, (amount as f32 * 1.2) as i32);
}

#[test]
fn apply_mining_cave_in_avoids_when_miner_roll_succeeds() {
    let mut world = World::default();
    let mut miner = character(1);
    miner.professions[profession::MINER] = 50; // avoid chance 100% (capped by RANDOM(100))
    miner.endurance = 10_000;
    world.add_character(miner);
    world.items.insert(ItemId(7), mine_wall(7, 3, 0, 8));

    world.legacy_random_seed = 5;
    let result = world
        .apply_mining_cave_in(ItemId(7), CharacterId(1))
        .unwrap();
    assert_eq!(result, CaveInResult::Avoided);
    // Endurance must be untouched when avoided.
    assert_eq!(world.characters[&CharacterId(1)].endurance, 10_000);
}

#[test]
fn apply_mining_cave_in_reduces_endurance_and_flags_exhaustion() {
    let mut world = World::default();
    let mut victim = character(1);
    victim.endurance = POWERSCALE; // exactly at the exhaustion boundary
    world.add_character(victim);
    world.items.insert(ItemId(7), mine_wall(7, 3, 0, 8)); // mine_level = 80

    world.legacy_random_seed = 99;
    let result = world
        .apply_mining_cave_in(ItemId(7), CharacterId(1))
        .unwrap();
    let CaveInResult::Collapsed {
        endurance_loss_units,
        unreduced_loss_units,
        now_exhausted,
    } = result
    else {
        panic!("expected a collapse, got {result:?}");
    };
    assert!(unreduced_loss_units.is_none(), "no athlete profession here");
    assert!(endurance_loss_units >= 0);
    let remaining = world.characters[&CharacterId(1)].endurance;
    assert!(remaining <= POWERSCALE);
    assert_eq!(now_exhausted, remaining < POWERSCALE);
}

#[test]
fn apply_mining_cave_in_athlete_reduction_reports_both_values() {
    let mut world = World::default();
    let mut victim = character(1);
    victim.endurance = 1_000_000;
    victim.professions[profession::ATHLETE] = 20; // 40% reduction
    world.add_character(victim);
    world.items.insert(ItemId(7), mine_wall(7, 3, 0, 8));

    world.legacy_random_seed = 99;
    let result = world
        .apply_mining_cave_in(ItemId(7), CharacterId(1))
        .unwrap();
    let CaveInResult::Collapsed {
        endurance_loss_units,
        unreduced_loss_units,
        ..
    } = result
    else {
        panic!("expected a collapse, got {result:?}");
    };
    let unreduced = unreduced_loss_units.expect("athlete profession should report both values");
    // Reduced loss must be strictly less than the reverse-derived
    // unreduced value (matching C's `1.0f - prof*0.02f` shrink).
    assert!(endurance_loss_units < unreduced);
}
