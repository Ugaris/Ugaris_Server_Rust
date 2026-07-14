use crate::{
    entity::{CharacterFlags, SpeedMode},
    ids::CharacterId,
};

use super::*;

#[test]
fn error_codes_match_c_header() {
    assert_eq!(DoError::None as u8, 0);
    assert_eq!(DoError::Blocked as u8, 2);
    assert_eq!(DoError::Dead as u8, 16);
    assert_eq!(DoError::AlreadyThere as u8, 38);
}

#[test]
fn action_duration_constants_match_do_c() {
    assert_eq!(DUR_COMBAT_ACTION, 12);
    assert_eq!(DUR_MISC_ACTION, 12);
    assert_eq!(DUR_USE_ACTION, 8);
    assert_eq!(DUR_MAGIC_ACTION, 12);
}

#[test]
fn do_idle_clamps_duration_and_sets_action_fields() {
    let mut character = character();

    do_idle(&mut character, 1).unwrap();
    assert_eq!(character.action, action::IDLE);
    assert_eq!(character.duration, 2);
    assert_eq!(character.act1, 2);

    do_idle(&mut character, 1000).unwrap();
    assert_eq!(character.duration, 48);
    assert_eq!(character.act1, 48);
}

#[test]
fn turn_rejects_invalid_direction_and_dead_characters() {
    let mut character = character();
    assert_eq!(turn(&mut character, 0), Err(DoError::IllegalDirection));
    assert_eq!(turn(&mut character, 9), Err(DoError::IllegalDirection));

    character.flags.insert(CharacterFlags::DEAD);
    assert_eq!(turn(&mut character, 1), Err(DoError::Dead));
}

#[test]
fn turn_sets_direction_and_reports_sector_dirty_only_on_change() {
    let mut character = character();

    assert_eq!(turn(&mut character, 3), Ok(true));
    assert_eq!(character.dir, 3);
    assert_eq!(turn(&mut character, 3), Ok(false));
    assert_eq!(character.dir, 3);
}

#[test]
fn speed_ticks_matches_legacy_formula_without_weather_modifier() {
    assert_eq!(speed_ticks(0, SpeedMode::Normal, 8), 10);
    assert_eq!(speed_ticks(40, SpeedMode::Fast, 8), 8);
    assert_eq!(speed_ticks(-40, SpeedMode::Stealth, 8), 15);
    assert_eq!(speed_ticks(1000, SpeedMode::Fast, 8), 4);
}

#[test]
fn speed_ticks_inverse_matches_legacy_formula_without_weather_modifier() {
    assert_eq!(speed_ticks_inverse(0, SpeedMode::Normal, 50), 38);
    assert_eq!(speed_ticks_inverse(-420, SpeedMode::Normal, 50), 10);
    assert_eq!(speed_ticks_inverse(40, SpeedMode::Fast, 8), 8);
    assert_eq!(speed_ticks_inverse(1000, SpeedMode::Fast, 8), 16);
}

#[test]
fn speed_ticks_with_weather_movement_applies_percent_before_final_scale() {
    // 100% is a no-op, identical to the weather-unaware `speed_ticks`.
    assert_eq!(
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, 8, 100),
        speed_ticks(100, SpeedMode::Normal, 8)
    );
    assert_eq!(speed_ticks(100, SpeedMode::Normal, 8), 8);
    // C `weather.c` Storm-heavy `move_mod` (70): the same character
    // takes one extra tick to complete the same action.
    assert_eq!(
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, 8, 70),
        9
    );
}

#[test]
fn endurance_cost_uses_athlete_profession_reduction() {
    let mut character = character();
    assert_eq!(endurance_cost(&character), 250);
    character.professions[profession::ATHLETE] = 9;
    assert_eq!(endurance_cost(&character), 200);
}

#[test]
fn do_walk_reserves_target_and_sets_action_fields() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.flags.insert(CharacterFlags::PLAYER);
    character.x = 10;
    character.y = 10;

    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 100, 0).unwrap();

    assert_eq!(character.action, action::WALK);
    assert_eq!(character.tox, 11);
    assert_eq!(character.toy, 10);
    assert_eq!(character.dir, Direction::Right as u8);
    assert_eq!(character.duration, speed_ticks(0, SpeedMode::Normal, 8));
    assert!(map
        .tile(11, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
}

#[test]
fn do_walk_rejects_blocked_and_corner_cutting_moves() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    map.set_flags(11, 10, MapFlags::MOVEBLOCK);

    assert_eq!(
        do_walk(&mut character, &mut map, Direction::Right as u8, 1, 100, 0),
        Err(DoError::Blocked)
    );

    map.set_flags(11, 10, MapFlags::empty());
    map.set_flags(10, 11, MapFlags::MOVEBLOCK);
    assert_eq!(
        do_walk(
            &mut character,
            &mut map,
            Direction::RightDown as u8,
            1,
            100,
            0
        ),
        Err(DoError::Blocked)
    );
}

#[test]
fn do_walk_applies_terrain_and_fast_endurance_costs() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.flags.insert(CharacterFlags::PLAYER);
    character.speed_mode = SpeedMode::Fast;
    character.endurance = 1000;
    character.x = 10;
    character.y = 10;
    map.tile_mut(10, 10).unwrap().ground_sprite = 59423;

    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 100, 0).unwrap();

    assert_eq!(character.duration, speed_ticks(0, SpeedMode::Fast, 24));
    assert_eq!(character.endurance, 750);
}

#[test]
fn edemon_reduction_matches_c_max_of_zero_and_strength_minus_demon() {
    assert_eq!(edemon_reduction(30, 10), 20);
    assert_eq!(edemon_reduction(10, 30), 0);
    assert_eq!(edemon_reduction(10, 10), 0);
}

#[test]
fn do_walk_applies_earthmud_extra_cost_before_terrain_overrides() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;

    // C `do_walk` (`do.c:93-99`): non-earth-demon walkers pay the
    // pre-computed `earthmud_extra_cost` on top of the base cost 8.
    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 100, 6).unwrap();

    assert_eq!(character.duration, speed_ticks(0, SpeedMode::Normal, 14));
}

#[test]
fn do_walk_swamp_sprite_overrides_discard_earthmud_extra_cost_for_players() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.flags.insert(CharacterFlags::PLAYER);
    character.x = 10;
    character.y = 10;
    map.tile_mut(10, 10).unwrap().ground_sprite = 59423;

    // C's swamp-sprite branches assign `cost = 24` outright (not `+=`),
    // so a muddy swamp tile silently loses the earthmud bonus for
    // players - an authentic C quirk, preserved here.
    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 100, 6).unwrap();

    assert_eq!(character.duration, speed_ticks(0, SpeedMode::Normal, 24));
}

#[test]
fn do_walk_slows_down_outdoors_under_a_weather_movement_percent() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.flags.insert(CharacterFlags::PLAYER);
    character.x = 10;
    character.y = 10;
    character.values[0][CharacterValue::Speed as usize] = 100;

    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 70, 0).unwrap();

    assert_eq!(
        character.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, 8, 70)
    );
    assert_ne!(character.duration, speed_ticks(100, SpeedMode::Normal, 8));
}

#[test]
fn do_walk_ignores_weather_movement_percent_indoors() {
    let mut map = MapGrid::new(20, 20);
    map.set_flags(10, 10, MapFlags::INDOORS);
    let mut character = character();
    character.flags.insert(CharacterFlags::PLAYER);
    character.x = 10;
    character.y = 10;
    character.values[0][CharacterValue::Speed as usize] = 100;

    // C `modify_movement_speed` returns `speed` unmodified indoors, even
    // though the weather-slow flag/percent is passed in.
    do_walk(&mut character, &mut map, Direction::Right as u8, 1, 70, 0).unwrap();

    assert_eq!(character.duration, speed_ticks(100, SpeedMode::Normal, 8));
}

#[test]
fn do_take_applies_weather_movement_percent_outdoors_but_not_indoors() {
    // C `take_item` (`system/do.c:290`)'s `speed(cn, ..., DUR_MISC_ACTION)`
    // call folds `modify_movement_speed` in unconditionally, same as
    // `do_walk`/`do_attack` above - not just movement/melee.
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.values[0][CharacterValue::Speed as usize] = 100;
    let item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    map.tile_mut(11, 10).unwrap().item = item.id.0;

    do_take(
        &mut character,
        &map,
        &item,
        Direction::Right as u8,
        true,
        70,
    )
    .unwrap();
    assert_eq!(
        character.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, DUR_MISC_ACTION, 70)
    );
    assert_ne!(
        character.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_MISC_ACTION)
    );

    map.set_flags(10, 10, MapFlags::INDOORS);
    character.cursor_item = None;
    do_take(
        &mut character,
        &map,
        &item,
        Direction::Right as u8,
        true,
        70,
    )
    .unwrap();
    assert_eq!(
        character.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_MISC_ACTION)
    );
}

#[test]
fn do_take_validates_cursor_item_and_takeable_flag() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    let item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    map.tile_mut(11, 10).unwrap().item = item.id.0;

    do_take(
        &mut character,
        &map,
        &item,
        Direction::Right as u8,
        true,
        100,
    )
    .unwrap();
    assert_eq!(character.action, action::TAKE);
    assert_eq!(character.act1, 7);
    assert_eq!(
        character.duration,
        speed_ticks(0, SpeedMode::Normal, DUR_MISC_ACTION)
    );

    character.cursor_item = Some(item.id);
    assert_eq!(
        do_take(
            &mut character,
            &map,
            &item,
            Direction::Right as u8,
            true,
            100
        ),
        Err(DoError::HaveCursorItem)
    );
}

#[test]
fn do_use_sets_spec_and_rejects_non_useable_items() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    map.tile_mut(11, 10).unwrap().item = item.id.0;

    do_use(&mut character, &map, &item, Direction::Right as u8, 42, 100).unwrap();
    assert_eq!(character.action, action::USE);
    assert_eq!((character.act1, character.act2), (7, 42));

    item.flags.remove(ItemFlags::USE);
    assert_eq!(
        do_use(&mut character, &map, &item, Direction::Right as u8, 0, 100),
        Err(DoError::NotUsable)
    );
}

#[test]
fn do_drop_validates_cursor_item_target_and_drop_flags() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    let item = item(7, ItemFlags::USED | ItemFlags::TAKE);

    assert_eq!(
        do_drop(&mut character, &map, &item, Direction::Right as u8, 100),
        Err(DoError::NoCursorItem)
    );

    character.cursor_item = Some(item.id);
    do_drop(&mut character, &map, &item, Direction::Right as u8, 100).unwrap();
    assert_eq!(character.action, action::DROP);
    assert_eq!(character.act1, 7);

    map.tile_mut(11, 10).unwrap().item = 99;
    assert_eq!(
        do_drop(&mut character, &map, &item, Direction::Right as u8, 100),
        Err(DoError::HaveItem)
    );
}

#[test]
fn act_walk_moves_character_from_reserved_target() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.tox = 11;
    character.toy = 10;
    map.tile_mut(10, 10).unwrap().character = character.id.0 as u16;
    map.tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    map.tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);

    assert!(act_walk(&mut character, &mut map));
    assert_eq!((character.x, character.y), (11, 10));
    assert_eq!((character.tox, character.toy), (0, 0));
    assert_eq!(map.tile(10, 10).unwrap().character, 0);
    assert!(!map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(map.tile(11, 10).unwrap().character, character.id.0 as u16);
}

#[test]
fn act_take_removes_map_item_and_places_it_on_cursor() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.act1 = 7;
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(map.set_item_map(&mut item, 11, 10));

    assert!(act_take(&mut character, &mut map, &mut item, true));
    assert_eq!(map.tile(11, 10).unwrap().item, 0);
    assert_eq!(character.cursor_item, Some(item.id));
    assert_eq!(item.carried_by, Some(character.id));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn act_drop_places_cursor_item_on_map() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.act1 = 7;
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    character.cursor_item = Some(item.id);
    item.carried_by = Some(character.id);

    assert!(act_drop(&mut character, &mut map, &mut item));
    assert_eq!(character.cursor_item, None);
    assert_eq!(item.carried_by, None);
    assert_eq!(map.tile(11, 10).unwrap().item, item.id.0);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn act_use_validates_target_and_returns_driver_request() {
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.act1 = 7;
    character.act2 = 42;
    let item = item(7, ItemFlags::USED | ItemFlags::USE);
    map.tile_mut(11, 10).unwrap().item = item.id.0;

    assert_eq!(
        act_use(&mut character, &map, &item),
        Some(ItemUseRequest {
            character_id: character.id,
            item_id: item.id,
            spec: 42,
        })
    );

    map.tile_mut(11, 10).unwrap().item = 0;
    assert_eq!(act_use(&mut character, &map, &item), None);
}

#[test]
fn do_attack_sets_legacy_action_fields_and_fast_endurance_cost() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    attacker.speed_mode = SpeedMode::Fast;
    attacker.endurance = 1000;

    do_attack(
        &mut attacker,
        &map,
        &defender,
        Direction::Right as u8,
        action::ATTACK2,
        100,
    )
    .unwrap();

    assert_eq!(attacker.action, action::ATTACK2);
    assert_eq!(attacker.act1, defender.id.0 as i32);
    assert_eq!(attacker.dir, Direction::Right as u8);
    assert_eq!(
        attacker.duration,
        speed_ticks(0, SpeedMode::Fast, DUR_COMBAT_ACTION)
    );
    assert_eq!(attacker.endurance, 500);
}

#[test]
fn do_attack_rejects_unreachable_dead_and_peace_zone_targets() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 15;
    defender.y = 10;
    map.tile_mut(15, 10).unwrap().character = defender.id.0 as u16;

    assert_eq!(
        do_attack(
            &mut attacker,
            &map,
            &defender,
            Direction::Right as u8,
            action::ATTACK1,
            100,
        ),
        Err(DoError::NoCharacter)
    );

    defender.x = 11;
    defender.y = 10;
    map.tile_mut(15, 10).unwrap().character = 0;
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    defender.flags.insert(CharacterFlags::DEAD);
    assert_eq!(
        do_attack(
            &mut attacker,
            &map,
            &defender,
            Direction::Right as u8,
            action::ATTACK1,
            100,
        ),
        Err(DoError::Dead)
    );

    defender.flags.remove(CharacterFlags::DEAD);
    map.set_flags(10, 10, MapFlags::PEACE);
    assert_eq!(
        do_attack(
            &mut attacker,
            &map,
            &defender,
            Direction::Right as u8,
            action::ATTACK1,
            100,
        ),
        Err(DoError::IllegalAttack)
    );

    map.tile_mut(10, 10).unwrap().flags.remove(MapFlags::PEACE);
    attacker.clan = 42;
    defender.clan = 42;
    assert_eq!(
        do_attack(
            &mut attacker,
            &map,
            &defender,
            Direction::Right as u8,
            action::ATTACK1,
            100,
        ),
        Err(DoError::IllegalAttack)
    );
}

#[test]
fn do_attack_slows_down_outdoors_under_a_weather_movement_percent() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    attacker.values[0][CharacterValue::Speed as usize] = 100;

    do_attack(
        &mut attacker,
        &map,
        &defender,
        Direction::Right as u8,
        action::ATTACK1,
        70,
    )
    .unwrap();

    assert_eq!(
        attacker.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, DUR_COMBAT_ACTION, 70)
    );
    assert_ne!(
        attacker.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_COMBAT_ACTION)
    );
}

#[test]
fn do_attack_ignores_weather_movement_percent_indoors() {
    let mut map = MapGrid::new(20, 20);
    map.set_flags(10, 10, MapFlags::INDOORS);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    attacker.values[0][CharacterValue::Speed as usize] = 100;

    // C `modify_movement_speed` returns `speed` unmodified indoors, even
    // though the weather-slow flag/percent is passed in.
    do_attack(
        &mut attacker,
        &map,
        &defender,
        Direction::Right as u8,
        action::ATTACK1,
        70,
    )
    .unwrap();

    assert_eq!(
        attacker.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_COMBAT_ACTION)
    );
}

#[test]
fn can_attack_in_area_blocks_legacy_area_one_player_vs_player() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.flags.insert(CharacterFlags::PLAYER);
    defender.flags.insert(CharacterFlags::PLAYER);

    assert!(can_attack(&attacker, &defender, &map));
    assert!(!can_attack_in_area(&attacker, &defender, &map, 1));
}

#[test]
fn can_attack_in_area_requires_pk_level_range_and_hate_for_player_vs_player() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    attacker.id = CharacterId(100);
    defender.id = CharacterId(200);
    defender.x = 11;
    defender.y = 10;
    attacker.flags.insert(CharacterFlags::PLAYER);
    defender.flags.insert(CharacterFlags::PLAYER);

    assert!(!can_attack_in_area(&attacker, &defender, &map, 2));

    attacker.flags.insert(CharacterFlags::PK);
    assert!(!can_attack_in_area(&attacker, &defender, &map, 2));

    defender.flags.insert(CharacterFlags::PK);
    assert!(!can_attack_in_area(&attacker, &defender, &map, 2));
    assert!(can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));

    defender.level = attacker.level + 4;
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_allows_legacy_zero_attacker_after_basic_defender_guards() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    attacker.id = CharacterId(0);
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.group = 7;
    defender.group = 7;
    attacker.clan = 42;
    defender.clan = 42;
    attacker.flags.insert(CharacterFlags::NOPLRATT);
    defender.flags.insert(CharacterFlags::PLAYERLIKE);
    map.tile_mut(10, 10).unwrap().flags.insert(MapFlags::PEACE);
    map.tile_mut(11, 10).unwrap().flags.insert(MapFlags::PEACE);

    assert!(can_attack_in_area(&attacker, &defender, &map, 1));

    defender.flags.insert(CharacterFlags::DEAD);
    assert!(!can_attack_in_area(&attacker, &defender, &map, 1));
}

#[test]
fn can_attack_requires_same_connected_arena() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    attacker.x = 5;
    attacker.y = 5;
    defender.x = 6;
    defender.y = 5;

    map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::ARENA);
    assert!(!can_attack(&attacker, &defender, &map));

    map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::ARENA);
    assert!(can_attack(&attacker, &defender, &map));

    defender.x = 12;
    defender.y = 12;
    map.tile_mut(12, 12).unwrap().flags.insert(MapFlags::ARENA);
    assert!(!can_attack(&attacker, &defender, &map));

    for x in 5..=12 {
        map.tile_mut(x, 5).unwrap().flags.insert(MapFlags::ARENA);
    }
    for y in 5..=12 {
        map.tile_mut(12, y).unwrap().flags.insert(MapFlags::ARENA);
    }
    assert!(can_attack(&attacker, &defender, &map));
}

#[test]
fn can_attack_blocks_same_group_npcs_outside_arena() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.group = 7;
    defender.group = 7;

    assert!(!can_attack(&attacker, &defender, &map));
}

#[test]
fn can_attack_admitted_player_vs_player_returns_before_same_group_check() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    attacker.id = CharacterId(100);
    defender.id = CharacterId(200);
    defender.x = 11;
    defender.y = 10;
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    attacker.group = 7;
    defender.group = 7;

    assert!(can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_arena_allows_same_clan_unless_clan_area() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    attacker.x = 5;
    attacker.y = 5;
    defender.x = 6;
    defender.y = 5;
    attacker.clan = 42;
    defender.clan = 42;
    map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::ARENA);
    map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::ARENA);

    assert!(can_attack(&attacker, &defender, &map));

    map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::CLAN);
    map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::CLAN);

    assert!(!can_attack(&attacker, &defender, &map));
}

struct TestClanPolicy;

impl ClanAttackPolicy for TestClanPolicy {
    fn are_allied(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        (attacker_clan, defender_clan) == (10, 11)
    }

    fn can_attack_inside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        (attacker_clan, defender_clan) == (20, 21)
    }

    fn can_attack_outside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        (attacker_clan, defender_clan) == (30, 31)
    }

    fn has_pk_hate(&self, attacker: &Character, defender: &Character) -> bool {
        (attacker.id, defender.id) == (CharacterId(100), CharacterId(200))
    }
}

#[test]
fn can_attack_clan_area_alliance_blocks_arena_attack() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    attacker.x = 5;
    attacker.y = 5;
    defender.x = 6;
    defender.y = 5;
    attacker.clan = 10;
    defender.clan = 11;
    map.tile_mut(5, 5)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA | MapFlags::CLAN);
    map.tile_mut(6, 5)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA | MapFlags::CLAN);

    assert!(can_attack(&attacker, &defender, &map));
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_clan_war_inside_clan_area_before_pvp_pk_gate() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.clan = 20;
    defender.clan = 21;
    attacker.flags.insert(CharacterFlags::PLAYER);
    defender.flags.insert(CharacterFlags::PLAYER);
    map.tile_mut(10, 10).unwrap().flags.insert(MapFlags::CLAN);
    map.tile_mut(11, 10).unwrap().flags.insert(MapFlags::CLAN);

    assert!(!can_attack_in_area(&attacker, &defender, &map, 2));
    assert!(can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_clan_feud_outside_clan_area_uses_level_range_and_area_one_gate() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.clan = 30;
    defender.clan = 31;
    attacker.flags.insert(CharacterFlags::PLAYER);
    defender.flags.insert(CharacterFlags::PLAYER);

    assert!(can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        1,
        &TestClanPolicy
    ));

    defender.level = attacker.level + 4;
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_player_vs_player_uses_policy_hate_list_after_pk_checks() {
    let map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    attacker.id = CharacterId(100);
    defender.id = CharacterId(200);
    defender.x = 11;
    defender.y = 10;
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);

    assert!(can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));

    defender.id = CharacterId(201);
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker,
        &defender,
        &map,
        2,
        &TestClanPolicy
    ));
}

#[test]
fn can_attack_wired_against_real_clan_relations_registry() {
    // End-to-end check that `crate::clan::ClanRelations` (the real
    // `clan.c` relation state machine) satisfies `ClanAttackPolicy` and
    // produces the same war/feud/alliance gating as the hand-written
    // `TestClanPolicy` above, once its clans are set up with matching
    // relations.
    use crate::clan::{ClanRelation, ClanRelations};

    let mut relations = ClanRelations::new();
    relations.found_clan(20, 0);
    relations.found_clan(21, 0);
    relations
        .set_relation(20, 21, ClanRelation::War, 0)
        .unwrap();
    relations
        .set_relation(21, 20, ClanRelation::War, 0)
        .unwrap();
    relations.update(0);

    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    attacker.clan = 20;
    defender.clan = 21;
    attacker.flags.insert(CharacterFlags::PLAYER);
    defender.flags.insert(CharacterFlags::PLAYER);
    map.tile_mut(10, 10).unwrap().flags.insert(MapFlags::CLAN);
    map.tile_mut(11, 10).unwrap().flags.insert(MapFlags::CLAN);

    assert!(can_attack_in_area_with_clan_policy(
        &attacker, &defender, &map, 2, &relations
    ));
    // Area 1 always blocks player-vs-player attacks, clan war or not.
    assert!(!can_attack_in_area_with_clan_policy(
        &attacker, &defender, &map, 1, &relations
    ));
}

#[test]
fn act_attack_uses_strict_hit_roll_and_applies_damage() {
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    defender.dir = Direction::Left as u8;
    defender.hp = 10_000;
    attacker.dir = Direction::Right as u8;
    attacker.act1 = defender.id.0 as i32;
    // `has_attack_base`/`has_parry_base` (C `ch[].value[1][V_ATTACK]`/
    // `[V_PARRY]`, the "present" flag `get_attack_skill`/
    // `get_parry_skill` branch on) must be set, or the fallback
    // spellcaster formula (`get_fight_skill(cn) + get_spell_average(cn)
    // * 2 - level`) applies instead of the raised-Attack/Parry-stat one.
    attacker.values[1][CharacterValue::Attack as usize] = 1;
    attacker.values[0][CharacterValue::Attack as usize] = 10;
    attacker.values[0][CharacterValue::Weapon as usize] = 10;
    defender.values[1][CharacterValue::Parry as usize] = 1;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    let items = HashMap::new();

    assert_eq!(
        act_attack(&mut attacker, &mut defender, &map, &items, 50, 6),
        Some(AttackResolution {
            hit: false,
            // C `get_attack_skill`/`get_parry_skill`: no weapon worn ->
            // `get_fight_skill` falls back to `V_HAND` (0 here), so each
            // side's skill is `0 + 10 * 2 = 20`, not the raw stat.
            attack_skill: 20,
            parry_skill: 20,
            hit_chance: 50,
            raw_damage: 0,
            armor_divisor: ATTACK_DIV,
            armor_percent: 90,
            shield_percent: 97,
            hp_damage: 0,
            shield_absorbed: 0,
        })
    );

    let result = act_attack(&mut attacker, &mut defender, &map, &items, 49, 6).unwrap();
    assert!(result.hit);
    assert_eq!(result.raw_damage, 3200);
    assert_eq!(result.hp_damage, 3200);
    assert_eq!(defender.hp, 10_000);
}

#[test]
fn act_attack_uses_get_attack_skill_get_parry_skill_not_raw_stat() {
    // C `act_attack` (act.c:747-748) always calls `get_attack_skill`/
    // `get_parry_skill`, never reads `ch[].value[0][V_ATTACK]`/
    // `[V_PARRY]` directly. A pure spellcaster (no `V_ATTACK`/`V_PARRY`
    // base, i.e. `value[1]` unset) still gets a nonzero effective skill
    // from the `get_fight_skill(cn) + get_spell_average(cn) * 2 -
    // level` fallback branch, which the old "just read the raw stat"
    // code (which would have zeroed both sides) could never produce.
    let mut map = MapGrid::new(20, 20);
    let mut attacker = character();
    let mut defender = character();
    defender.id = CharacterId(2);
    defender.x = 11;
    defender.y = 10;
    defender.dir = Direction::Left as u8;
    attacker.dir = Direction::Right as u8;
    attacker.act1 = defender.id.0 as i32;
    attacker.level = 5;
    attacker.values[0][CharacterValue::Fireball as usize] = 40; // spell_average = 5.0
    map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
    let items = HashMap::new();

    let resolution = act_attack(&mut attacker, &mut defender, &map, &items, 0, 6).unwrap();
    // attack_skill = fight_skill(0) + spell_average(5.0) * 2 - level(5) = 5
    assert_eq!(resolution.attack_skill, 5);
    // parry_skill: no V_PARRY/V_MAGICSHIELD base -> spell_average
    // fallback too, but defender's spell_average is 0.
    assert_eq!(resolution.parry_skill, 0);
}

#[test]
fn do_fireball_sets_targeted_legacy_action() {
    let map = MapGrid::new(20, 20);
    let items = HashMap::new();
    let mut character = character();
    character.values[0][CharacterValue::Fireball as usize] = 50;
    character.mana = FIREBALL_COST;

    do_fireball(&mut character, &items, 15, 10, 0, &map, 100).unwrap();

    assert_eq!(character.action, action::FIREBALL1);
    assert_eq!(character.act1, 15);
    assert_eq!(character.act2, 10);
    assert_eq!(character.dir, Direction::Right as u8);
    assert_eq!(
        character.duration,
        speed_ticks(0, SpeedMode::Normal, DUR_MAGIC_ACTION / 2)
    );
    assert_eq!(character.mana, 0);
}

#[test]
fn do_fireball_same_tile_sets_firering_action() {
    let map = MapGrid::new(20, 20);
    let items = HashMap::new();
    let mut character = character();
    character.values[0][CharacterValue::Fireball as usize] = 50;
    character.mana = FIREBALL_COST;
    character.dir = Direction::RightUp as u8;

    do_fireball(&mut character, &items, 10, 10, 0, &map, 100).unwrap();

    assert_eq!(character.action, action::FIRERING);
    assert_eq!(character.dir, Direction::Right as u8);
    assert_eq!(
        character.duration,
        speed_ticks(0, SpeedMode::Normal, DUR_MAGIC_ACTION)
    );
    assert_eq!(character.mana, 0);
}

#[test]
fn do_earthrain_sets_legacy_action_and_hp_cost() {
    let map = MapGrid::new(20, 20);
    let mut character = character();
    character.hp = 10 * POWERSCALE;
    character.speed_mode = SpeedMode::Fast;

    do_earthrain(&mut character, &map, 12, 10, 15, 100).unwrap();

    assert_eq!(character.action, action::EARTHRAIN);
    assert_eq!(character.act1, (12 + 10 * MAX_MAP) as i32);
    assert_eq!(character.act2, 15);
    assert_eq!(character.dir, Direction::Right as u8);
    assert_eq!(character.hp, 10 * POWERSCALE - 1500);
    assert_eq!(
        character.duration,
        speed_ticks(0, SpeedMode::Fast, DUR_MAGIC_ACTION)
    );
    assert_eq!(character.endurance, -endurance_cost(&character));
}

#[test]
fn do_earthmud_rejects_self_and_low_hp_like_c() {
    let map = MapGrid::new(20, 20);
    let mut character = character();
    character.hp = 2 * POWERSCALE;

    assert_eq!(
        do_earthmud(&mut character, &map, 10, 10, 1, 100),
        Err(DoError::SelfTarget)
    );
    assert_eq!(
        do_earthmud(&mut character, &map, 11, 10, 11, 100),
        Err(DoError::ManaLow)
    );
}

#[test]
fn do_magicshield_applies_weather_movement_percent_outdoors_but_not_indoors() {
    // C `magicshield_spell` (`system/do.c:630`)'s `speed(cn, ...)` call
    // folds weather in unconditionally, same as every other `speed(cn,`
    // call site (`tool.c:118-160`).
    let mut map = MapGrid::new(20, 20);
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.values[0][CharacterValue::Speed as usize] = 100;
    character.values[0][CharacterValue::MagicShield as usize] = 50;
    character.mana = POWERSCALE * 10;

    do_magicshield(&mut character, &map, 70).unwrap();
    assert_eq!(
        character.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, DUR_MAGIC_ACTION, 70)
    );
    assert_ne!(
        character.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_MAGIC_ACTION)
    );

    map.set_flags(10, 10, MapFlags::INDOORS);
    character.mana = POWERSCALE * 10;
    character.lifeshield = 0;
    do_magicshield(&mut character, &map, 70).unwrap();
    assert_eq!(
        character.duration,
        speed_ticks(100, SpeedMode::Normal, DUR_MAGIC_ACTION)
    );
}

#[test]
fn do_heal_applies_weather_movement_percent_to_self_and_other_target_durations() {
    // C `heal_spell` (`system/do.c:816,825`)'s two `speed(cn, ...)` call
    // sites (self vs. other target, halved duration) both fold weather
    // in identically.
    let map = MapGrid::new(20, 20);
    let mut caster = character();
    caster.x = 10;
    caster.y = 10;
    caster.values[0][CharacterValue::Speed as usize] = 100;
    caster.values[0][CharacterValue::Heal as usize] = 50;
    caster.values[0][CharacterValue::Hp as usize] = 10;
    caster.mana = POWERSCALE * 10;
    caster.hp = POWERSCALE;

    let self_target = caster.clone();
    do_heal(&mut caster, &self_target, None, &map, 70).unwrap();
    assert_eq!(
        caster.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, DUR_MAGIC_ACTION, 70)
    );

    caster.mana = POWERSCALE * 10;
    caster.hp = POWERSCALE;
    let mut other_target = caster.clone();
    other_target.id = CharacterId(2);
    do_heal(&mut caster, &other_target, None, &map, 70).unwrap();
    assert_eq!(
        caster.duration,
        speed_ticks_with_weather_movement(100, SpeedMode::Normal, DUR_MAGIC_ACTION / 2, 70)
    );
}

#[test]
fn do_fireball_applies_weather_movement_percent() {
    let map = MapGrid::new(20, 20);
    let items = HashMap::new();
    let mut character = character();
    character.x = 10;
    character.y = 10;
    character.values[0][CharacterValue::Speed as usize] = 200;
    character.values[0][CharacterValue::Fireball as usize] = 50;
    character.mana = FIREBALL_COST;

    do_fireball(&mut character, &items, 15, 10, 0, &map, 70).unwrap();

    assert_eq!(
        character.duration,
        speed_ticks_with_weather_movement(200, SpeedMode::Normal, DUR_MAGIC_ACTION / 2, 70)
    );
    assert_ne!(
        character.duration,
        speed_ticks(200, SpeedMode::Normal, DUR_MAGIC_ACTION / 2)
    );
}

#[test]
fn action_step_matches_tick_char_readiness_rule() {
    let mut character = character();
    character.duration = 2;

    assert!(!advance_action_step(&mut character));
    assert_eq!(character.step, 1);
    assert!(advance_action_step(&mut character));
    assert_eq!(character.step, 2);

    character.action = action::WALK;
    reset_action_after_act(&mut character);
    assert_eq!(character.action, 0);
    assert_eq!(character.duration, 0);
    assert_eq!(character.step, 0);
}

fn character() -> Character {
    Character {
        merchant: None,
        template_key: String::new(),
        respawn_ticks: 0,
        id: CharacterId(1),
        serial: 1,
        name: "Character".into(),
        description: String::new(),
        flags: CharacterFlags::USED,
        sprite: 0,
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
        staff_code: String::new(),
        speed_mode: SpeedMode::Normal,
        x: 10,
        y: 10,
        rest_area: 0,
        rest_x: 0,
        rest_y: 0,
        tox: 0,
        toy: 0,
        dir: 0,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 0,
        mana: 0,
        endurance: 0,
        lifeshield: 0,
        level: 1,
        exp: 0,
        exp_used: 0,
        military_points: 0,
        military_normal_exp: 0,
        gold: 0,
        karma: 0,
        creation_time: 0,
        saves: 0,
        got_saved: 0,
        deaths: 0,
        regen_ticker: 0,
        last_regen: 0,
        cursor_item: None,
        current_container: None,
        values: Character::empty_values(),
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
        driver_memory: crate::character_driver::DriverMemory::default(),
        class: 0,
        dungeonfighter: None,
        fight_driver: None,
        lq_usurp: None,
    }
}

fn item(id: u32, flags: ItemFlags) -> Item {
    Item {
        id: crate::ids::ItemId(id),
        name: "Item".into(),
        description: String::new(),
        flags,
        sprite: 0,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; crate::entity::MAX_MODIFIERS],
        modifier_value: [0; crate::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: Vec::new(),
        serial: 0,
    }
}
