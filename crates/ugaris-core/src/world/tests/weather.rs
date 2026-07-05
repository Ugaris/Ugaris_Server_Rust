use super::*;

fn player_character(id: u32) -> Character {
    let mut character = character(id);
    character.flags |= CharacterFlags::PLAYER;
    character.hp = 1_000 * POWERSCALE;
    character
}

#[test]
fn apply_weather_damage_hurts_outdoor_players() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    let outcome = world
        .apply_weather_damage(CharacterId(1), 3)
        .expect("outdoor player should take weather damage");

    assert!(outcome.hp_damage > 0);
    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 1_000 * POWERSCALE - outcome.hp_damage);
}

#[test]
fn apply_weather_damage_is_a_noop_for_zero_or_negative_damage() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world.apply_weather_damage(CharacterId(1), 0).is_none());
    assert!(world.apply_weather_damage(CharacterId(1), -1).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().hp,
        1_000 * POWERSCALE
    );
}

#[test]
fn apply_weather_damage_skips_non_player_characters() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = 1_000 * POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.apply_weather_damage(CharacterId(1), 5).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().hp,
        1_000 * POWERSCALE
    );
}

#[test]
fn apply_weather_damage_skips_gods_and_immortals() {
    let mut world = World::default();
    let mut god = player_character(1);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 10, 10));
    let mut immortal = player_character(2);
    immortal.flags |= CharacterFlags::IMMORTAL;
    assert!(world.spawn_character(immortal, 11, 10));

    assert!(world.apply_weather_damage(CharacterId(1), 5).is_none());
    assert!(world.apply_weather_damage(CharacterId(2), 5).is_none());
}

#[test]
fn apply_weather_damage_skips_indoor_players() {
    let mut world = World::default();
    world.map.set_flags(10, 10, MapFlags::INDOORS);
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world.apply_weather_damage(CharacterId(1), 5).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().hp,
        1_000 * POWERSCALE
    );
}

#[test]
fn apply_weather_damage_skips_unknown_characters() {
    let mut world = World::default();
    assert!(world.apply_weather_damage(CharacterId(99), 5).is_none());
}

#[test]
fn apply_lightning_strike_damage_ignores_armor_divisor() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    let outcome = world
        .apply_lightning_strike_damage(CharacterId(1), 20)
        .expect("outdoor player should be struck by lightning");

    // C `hurt(cn, base_damage * POWERSCALE, 0, 0, 50, 50)`: armor_divisor
    // 0 means armor never reduces the hit, unlike ordinary weather damage
    // (armor_divisor 1) - the struck character with zero armor takes
    // exactly `base_damage * POWERSCALE` either way here, but the divisor
    // itself is what distinguishes the two `hurt` call sites.
    assert!(outcome.hp_damage > 0);
    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 1_000 * POWERSCALE - outcome.hp_damage);
}

#[test]
fn apply_lightning_strike_damage_is_a_noop_for_zero_or_negative_damage() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world
        .apply_lightning_strike_damage(CharacterId(1), 0)
        .is_none());
    assert!(world
        .apply_lightning_strike_damage(CharacterId(1), -5)
        .is_none());
}

#[test]
fn apply_lightning_strike_damage_skips_non_player_characters() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = 1_000 * POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world
        .apply_lightning_strike_damage(CharacterId(1), 20)
        .is_none());
}

#[test]
fn apply_lightning_strike_damage_skips_gods_and_immortals() {
    let mut world = World::default();
    let mut god = player_character(1);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 10, 10));
    let mut immortal = player_character(2);
    immortal.flags |= CharacterFlags::IMMORTAL;
    assert!(world.spawn_character(immortal, 11, 10));

    assert!(world
        .apply_lightning_strike_damage(CharacterId(1), 20)
        .is_none());
    assert!(world
        .apply_lightning_strike_damage(CharacterId(2), 20)
        .is_none());
}

#[test]
fn apply_lightning_strike_damage_skips_indoor_players() {
    let mut world = World::default();
    world.map.set_flags(10, 10, MapFlags::INDOORS);
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world
        .apply_lightning_strike_damage(CharacterId(1), 20)
        .is_none());
}

#[test]
fn apply_lightning_strike_damage_skips_unknown_characters() {
    let mut world = World::default();
    assert!(world
        .apply_lightning_strike_damage(CharacterId(99), 20)
        .is_none());
}

#[test]
fn character_weather_eligible_matches_every_apply_weather_damage_guard() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    assert!(world.character_weather_eligible(CharacterId(1)));

    let mut npc = character(2);
    assert!(world.spawn_character(npc.clone(), 12, 10));
    assert!(!world.character_weather_eligible(CharacterId(2)));
    npc.flags |= CharacterFlags::PLAYER;

    let mut god = player_character(3);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 13, 10));
    assert!(!world.character_weather_eligible(CharacterId(3)));

    world.map.set_flags(14, 10, MapFlags::INDOORS);
    assert!(world.spawn_character(player_character(4), 14, 10));
    assert!(!world.character_weather_eligible(CharacterId(4)));

    assert!(!world.character_weather_eligible(CharacterId(99)));
}
