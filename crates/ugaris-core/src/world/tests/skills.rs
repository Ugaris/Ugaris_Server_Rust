use super::*;

// C: `raise_cost(v, n, seyan)`, `src/system/skill.c`.
// nr = n - skill[v].start + 1 + 5; cost = max(1, nr^3 * skill[v].cost / 10)
// (non-seyan). For CharacterValue::Sword (start = 1, cost factor = 1) with
// bare value 10: nr = 10 - 1 + 6 = 15; cost = max(1, 15^3 / 10) = 337.
const SWORD_RAISE_COST_AT_10: u32 = 337;

#[test]
fn raise_skill_spends_unused_exp_and_bumps_bare_and_effective_value() {
    let mut world = World::default();
    let mut player = character(1);
    player.values[1][CharacterValue::Sword as usize] = 10;
    player.values[0][CharacterValue::Sword as usize] = 10;
    player.exp = 400;
    player.exp_used = 50;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(
        outcome,
        RaiseSkillOutcome::Raised {
            value: CharacterValue::Sword as usize,
            bare: 11,
            effective: 11,
            exp: 400,
            exp_used: 50 + SWORD_RAISE_COST_AT_10,
        }
    );
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.values[1][CharacterValue::Sword as usize], 11);
    assert_eq!(character.values[0][CharacterValue::Sword as usize], 11);
    // C `raise_value` (unlike `raise_value_exp`) never adds to `exp`.
    assert_eq!(character.exp, 400);
    assert_eq!(character.exp_used, 50 + SWORD_RAISE_COST_AT_10);
    assert!(character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn raise_skill_does_not_lower_effective_value_above_bare() {
    let mut world = World::default();
    let mut player = character(1);
    // Effective value already boosted above bare (e.g. by equipment); after
    // raising bare from 10 to 11, effective (12) must stay untouched since
    // C only bumps `value[0]` up to match `value[1]`, never down.
    player.values[1][CharacterValue::Sword as usize] = 10;
    player.values[0][CharacterValue::Sword as usize] = 12;
    player.exp = 1_000;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(
        outcome,
        RaiseSkillOutcome::Raised {
            value: CharacterValue::Sword as usize,
            bare: 11,
            effective: 12,
            exp: 1_000,
            exp_used: SWORD_RAISE_COST_AT_10,
        }
    );
}

#[test]
fn raise_skill_blocked_when_unused_exp_would_exceed_exp() {
    let mut world = World::default();
    let mut player = character(1);
    player.values[1][CharacterValue::Sword as usize] = 10;
    player.values[0][CharacterValue::Sword as usize] = 10;
    player.exp = 400;
    player.exp_used = 400; // exp_used + cost > exp
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.values[1][CharacterValue::Sword as usize], 10);
    assert_eq!(character.exp_used, 400);
    assert!(!character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn raise_skill_blocked_when_noexp_flag_set() {
    let mut world = World::default();
    let mut player = character(1);
    player.values[1][CharacterValue::Sword as usize] = 10;
    player.exp = 100_000;
    player.flags.insert(CharacterFlags::NOEXP);
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}

#[test]
fn raise_skill_blocked_when_skill_not_present() {
    let mut world = World::default();
    let mut player = character(1);
    // C: `if (!ch[cn].value[1][v]) return 0;` - bare value 0 means the
    // character doesn't have the skill at all.
    player.values[1][CharacterValue::Sword as usize] = 0;
    player.exp = 100_000;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}

#[test]
fn raise_skill_blocked_at_skillmax() {
    let mut world = World::default();
    let mut player = character(1);
    player.values[1][CharacterValue::Sword as usize] = 50; // non-arch skillmax
    player.exp = 100_000_000;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Sword as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}

#[test]
fn raise_skill_blocked_for_unraisable_values() {
    let mut world = World::default();
    let mut player = character(1);
    // C: Armor's `skill[v].cost == 0` - not raisable at all.
    player.values[1][CharacterValue::Armor as usize] = 10;
    player.exp = 100_000;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.raise_skill(CharacterId(1), CharacterValue::Armor as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}

#[test]
fn raise_skill_blocked_for_out_of_range_value_index() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    // Client-controlled u16 index far beyond CHARACTER_VALUE_COUNT must not
    // panic and must be treated as blocked (C's `n > V_MAX` check, off by
    // one in the original but harmless since it never indexes past bounds
    // here - the Rust helper bounds-checks before mutating).
    let outcome = world.raise_skill(CharacterId(1), 9_000);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}

#[test]
fn raise_skill_blocked_for_unknown_character() {
    let mut world = World::default();

    let outcome = world.raise_skill(CharacterId(42), CharacterValue::Sword as u16);

    assert_eq!(outcome, RaiseSkillOutcome::Blocked);
}
