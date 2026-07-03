use super::*;

const REGEN_TIME: i32 = 96; // C default: 4 * TICKS (TICKS = 24).

#[test]
fn idle_regen_restores_hp_endurance_mana_after_gate_elapses() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 1_000 - REGEN_TIME as u32 - 1; // gate satisfied
    npc.last_regen = 1_000; // avoid also triggering `regenerate()` this tick
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 5;
    npc.values[0][CharacterValue::Endurance as usize] = 8;
    npc.hp = 2 * POWERSCALE;
    npc.mana = POWERSCALE;
    npc.endurance = POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    let npc = &world.characters[&CharacterId(1)];
    // No V_REGENERATE/V_MEDITATE skill: default val = 7 for hp/mana.
    assert_eq!(npc.hp, 2 * POWERSCALE + 7 * 15);
    assert_eq!(npc.mana, POWERSCALE + 7 * 15);
    // Endurance always uses the fixed val = 150.
    assert_eq!(npc.endurance, POWERSCALE + 150 * 15);
    assert!(npc.flags.contains(CharacterFlags::SMALLUPDATE));
}

#[test]
fn idle_regen_caps_at_value_times_powerscale() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.hp = 10 * POWERSCALE - 1;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].hp, 10 * POWERSCALE);
}

#[test]
fn idle_regen_blocked_before_regen_time_elapses() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 1_000; // just acted this tick, gate not satisfied
    npc.last_regen = 1_000;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.hp = 2 * POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].hp, 2 * POWERSCALE);
}

#[test]
fn idle_regen_only_applies_while_idle() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::WALK; // not idle
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.hp = 2 * POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].hp, 2 * POWERSCALE);
}

#[test]
fn idle_regen_skipped_on_noregen_tile_for_players_but_not_npcs() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::NOREGEN);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::NOREGEN);

    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.action = action::IDLE;
    player.regen_ticker = 0;
    player.last_regen = 1_000;
    player.values[0][CharacterValue::Hp as usize] = 10;
    player.hp = 2 * POWERSCALE;
    assert!(world.spawn_character(player, 10, 10));

    let mut npc = character(2);
    npc.action = action::IDLE;
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.hp = 2 * POWERSCALE;
    assert!(world.spawn_character(npc, 11, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    // Player on a NOREGEN tile does not regen...
    assert_eq!(world.characters[&CharacterId(1)].hp, 2 * POWERSCALE);
    // ...but an NPC on the same flagged tile still does (C: `|| !(flags & CF_PLAYER)`).
    assert_eq!(
        world.characters[&CharacterId(2)].hp,
        2 * POWERSCALE + 7 * 15
    );
}

#[test]
fn idle_regen_skips_hp_in_area_33_but_keeps_endurance_and_mana() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 5;
    npc.hp = 2 * POWERSCALE;
    npc.mana = POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 33);

    let npc = &world.characters[&CharacterId(1)];
    assert_eq!(npc.hp, 2 * POWERSCALE);
    assert_eq!(npc.mana, POWERSCALE + 7 * 15);
}

#[test]
fn idle_regen_leaks_lifeshield_without_magicshield_skill_scaled_by_warcry() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.lifeshield = 100;
    npc.values[0][CharacterValue::Regenerate as usize] = 5;
    npc.values[0][CharacterValue::Warcry as usize] = 35; // (35-5)/10 + 1 = 4
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].lifeshield, 96);
}

#[test]
fn idle_lifeshield_leak_does_not_apply_with_magicshield_skill() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.action = action::IDLE;
    npc.regen_ticker = 0;
    npc.last_regen = 1_000;
    npc.lifeshield = 100;
    npc.values[1][CharacterValue::MagicShield as usize] = 1;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].lifeshield, 100);
}

#[test]
fn regenerate_endurance_and_lifeshield_gated_by_bare_skill_and_throttled_per_second() {
    let mut world = World::default();
    world.tick = Tick(48); // 2 real seconds at TICKS_PER_SECOND=24.

    let mut npc = character(1);
    npc.last_regen = 0;
    npc.regen_ticker = 1_000_000; // keep the idle-regen gate closed
    npc.values[0][CharacterValue::Regenerate as usize] = 3;
    npc.values[1][CharacterValue::Regenerate as usize] = 5;
    npc.values[0][CharacterValue::Endurance as usize] = 100;
    npc.endurance = 0;
    npc.values[1][CharacterValue::MagicShield as usize] = 2;
    npc.values[0][CharacterValue::MagicShield as usize] = 50;
    npc.values[1][CharacterValue::Meditate as usize] = 4;
    npc.values[0][CharacterValue::Meditate as usize] = 6;
    npc.lifeshield = 0;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    let npc = &world.characters[&CharacterId(1)];
    // diff = 2; endurance += (3+5)*2*5 = 80.
    assert_eq!(npc.endurance, 80);
    // diff = 2; lifeshield += (6+4)*2*4 = 80.
    assert_eq!(npc.lifeshield, 80);
    // last_regen advances by diff * TICKS_PER_SECOND = 2*24 = 48.
    assert_eq!(npc.last_regen, 48);
}

#[test]
fn regenerate_endurance_not_gained_without_bare_regenerate_skill() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.last_regen = 0;
    npc.regen_ticker = 1_000_000;
    npc.values[0][CharacterValue::Regenerate as usize] = 3;
    // values[1] (bare skill) stays 0: C gates on `value[1][V_REGENERATE]`.
    npc.values[0][CharacterValue::Endurance as usize] = 100;
    npc.endurance = 0;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].endurance, 0);
    // The throttle still advances even when nothing was gained.
    assert_eq!(world.characters[&CharacterId(1)].last_regen, 48);
}

#[test]
fn regenerate_endurance_blocked_in_fast_speed_mode() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.last_regen = 0;
    npc.regen_ticker = 1_000_000;
    npc.speed_mode = SpeedMode::Fast;
    npc.values[0][CharacterValue::Regenerate as usize] = 3;
    npc.values[1][CharacterValue::Regenerate as usize] = 5;
    npc.values[0][CharacterValue::Endurance as usize] = 100;
    // C `check_endurance()` reverts `SM_FAST` to `SM_NORMAL` when endurance
    // drops below `POWERSCALE`; keep endurance at exactly `POWERSCALE` (not
    // below) so the fast-mode block under test stays intact.
    npc.endurance = POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].endurance, POWERSCALE);
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Fast
    );
}

#[test]
fn regenerate_forces_lifeshield_to_zero_in_area_33() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.last_regen = 0;
    npc.regen_ticker = 1_000_000;
    npc.lifeshield = 250;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 33);

    assert_eq!(world.characters[&CharacterId(1)].lifeshield, 0);
}

#[test]
fn check_endurance_reverts_fast_mode_below_powerscale_and_logs_exhausted() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.speed_mode = SpeedMode::Fast;
    npc.endurance = POWERSCALE - 1;
    // Avoid also triggering `regenerate()`'s own effects this tick.
    npc.last_regen = 48;
    npc.regen_ticker = 1_000_000;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Normal
    );
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "You're exhausted.".to_string(),
        }]
    );
}

#[test]
fn check_endurance_keeps_fast_mode_at_or_above_powerscale() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.speed_mode = SpeedMode::Fast;
    npc.endurance = POWERSCALE;
    npc.last_regen = 48;
    npc.regen_ticker = 1_000_000;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Fast
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn check_endurance_ignores_non_fast_speed_modes() {
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.speed_mode = SpeedMode::Stealth;
    npc.endurance = 0;
    npc.last_regen = 48;
    npc.regen_ticker = 1_000_000;
    assert!(world.spawn_character(npc, 10, 10));

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Stealth
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn check_endurance_runs_even_outside_map_bounds() {
    // C `check_endurance()` has no position gate, unlike `regenerate()`.
    let mut world = World::default();
    world.tick = Tick(48);

    let mut npc = character(1);
    npc.x = 0;
    npc.y = 10;
    npc.speed_mode = SpeedMode::Fast;
    npc.endurance = 0;
    npc.last_regen = 48;
    npc.regen_ticker = 1_000_000;
    world.add_character(npc);

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Normal
    );
}

#[test]
fn regenerate_and_idle_regen_skip_characters_outside_map_bounds() {
    let mut world = World::default();
    world.tick = Tick(1_000);

    let mut npc = character(1);
    npc.x = 0; // C: `ch[cn].x < 1` -> return early, no panic either.
    npc.y = 10;
    npc.action = action::IDLE;
    npc.last_regen = 0;
    npc.regen_ticker = 0;
    npc.hp = 0;
    world.add_character(npc);

    world.regenerate_characters(REGEN_TIME, 1);

    assert_eq!(world.characters[&CharacterId(1)].hp, 0);
}
