use super::*;
use crate::character_driver::{FightDriverData, CDR_WARPFIGHTER, NT_CHAR, NT_GOTHIT};
use crate::world::npc::area25::{
    apply_warped_raise, warped_armor_spell_mod_value, warped_equip_mod_value,
    warped_weapon_spell_mod_value, WarpFighterDriverData,
};

fn fighter_npc(id: u32, data: WarpFighterDriverData) -> Character {
    let mut fighter = character(id);
    fighter.name = "Hrus-tak-lan".into();
    fighter.driver = CDR_WARPFIGHTER;
    fighter.group = 5;
    fighter.driver_state = Some(CharacterDriverState::WarpFighter(data));
    // C `fight_driver_set_dist(cn, 40, 0, 40)` (`warped.c:880`), seeded at
    // spawn time by `ugaris-server::spawns::spawn_warp_trial_fighter`
    // rather than a real `NT_CREATE` message - see `warpfighter.rs`'s
    // module doc comment.
    fighter.fight_driver = Some(FightDriverData {
        start_dist: 40,
        stop_dist: 40,
        ..FightDriverData::default()
    });
    fighter
}

fn owned_data(owner: CharacterId, owner_serial: u32) -> WarpFighterDriverData {
    WarpFighterDriverData {
        owner,
        owner_serial,
        tx: 40,
        ty: 41,
        xs: 10,
        xe: 20,
        ys: 10,
        ye: 20,
        creation_time: 0,
        pot_done: 0,
    }
}

fn fighter_state(world: &World, id: CharacterId) -> WarpFighterDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|character| character.driver_state.clone())
    {
        Some(CharacterDriverState::WarpFighter(data)) => data,
        _ => panic!("expected warp fighter driver state"),
    }
}

#[test]
fn self_destructs_when_the_owner_no_longer_exists() {
    let mut world = World::default();
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));

    world.process_warpfighter_actions(25);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn self_destructs_when_the_owner_leaves_the_room_bounds() {
    let mut world = World::default();
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));
    let mut owner = character(2);
    owner.serial = 7;
    owner.x = 99; // outside xs=10..xe=20
    owner.y = 15;
    world.add_character(owner);

    world.process_warpfighter_actions(25);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn self_destructs_when_the_owner_serial_no_longer_matches() {
    let mut world = World::default();
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));
    let mut owner = character(2);
    owner.serial = 99; // a different character now occupies the slot
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    world.process_warpfighter_actions(25);

    assert!(world.characters.get(&CharacterId(1)).is_none());
}

#[test]
fn stays_alive_and_tracks_the_owner_while_inside_the_room_bounds() {
    let mut world = World::default();
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));
    let mut owner = character(2);
    owner.serial = 7;
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    world.process_warpfighter_actions(25);

    assert!(world.characters.get(&CharacterId(1)).is_some());
}

#[test]
fn nt_gothit_adds_the_attacker_as_an_enemy_and_notes_the_hit() {
    let mut world = World::default();
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));
    let mut owner = character(2);
    owner.serial = 7;
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    let mut attacker = character(3);
    attacker.group = 9;
    attacker.x = 16;
    attacker.y = 15;
    world.add_character(attacker);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_GOTHIT, 3, 0, 0);
    world.process_warpfighter_actions(25);

    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    let enemies = fighter
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.clone())
        .unwrap_or_default();
    assert!(enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(3)));
    assert_eq!(fighter.fight_driver.as_ref().unwrap().last_hit, 0);
}

#[test]
fn nt_char_adds_a_visible_different_group_character_as_an_enemy() {
    let mut world = World::default();
    world.map.tile_mut(16, 15).unwrap().light = 255;
    assert!(world.spawn_character(fighter_npc(1, owned_data(CharacterId(2), 7)), 15, 15));
    let mut owner = character(2);
    owner.serial = 7;
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    let mut seen = character(3);
    seen.group = 9;
    seen.x = 16;
    seen.y = 15;
    world.add_character(seen);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_CHAR, 3, 0, 0);
    world.process_warpfighter_actions(25);

    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    let enemies = fighter
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.clone())
        .unwrap_or_default();
    assert!(enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(3)));
}

#[test]
fn potion_delay_gates_the_first_potion_check_until_two_seconds_after_creation() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut data = owned_data(CharacterId(2), 7);
    data.creation_time = 100;
    assert!(world.spawn_character(fighter_npc(1, data), 15, 15));
    let mut owner = character(2);
    owner.serial = 7;
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    // Still inside the two-second delay: `pot_done` stays 0.
    world.process_warpfighter_actions(25);
    assert_eq!(fighter_state(&world, CharacterId(1)).pot_done, 0);

    // Past the delay, but level 0 never rolls the potion (`level > 60`
    // gate) - `pot_done` still advances to 1 since C increments it before
    // checking the level/chance.
    world.tick = Tick(100 + TICKS_PER_SECOND * 3);
    world.process_warpfighter_actions(25);
    assert_eq!(fighter_state(&world, CharacterId(1)).pot_done, 1);
}

#[test]
fn apply_warped_raise_rescales_present_skills_and_recomputes_exp_level() {
    let mut fighter = character(1);
    fighter.values[1][CharacterValue::Hp as usize] = 5;
    fighter.values[1][CharacterValue::Endurance as usize] = 5;
    fighter.values[1][CharacterValue::Mana as usize] = 0; // stays untouched
    fighter.values[1][CharacterValue::MagicShield as usize] = 3;
    fighter.values[1][CharacterValue::Armor as usize] = 7; // not raisable, untouched

    apply_warped_raise(&mut fighter, 40);

    // `max(10, 40 - 40/4) = max(10, 30) = 30`.
    assert_eq!(fighter.values[1][CharacterValue::Hp as usize], 30);
    assert_eq!(fighter.values[1][CharacterValue::Endurance as usize], 30);
    // Skipped: was already zero.
    assert_eq!(fighter.values[1][CharacterValue::Mana as usize], 0);
    // Not cased -> `default: max(1, base - 40) = max(1, 0) = 1`.
    assert_eq!(fighter.values[1][CharacterValue::MagicShield as usize], 1);
    // `V_ARMOR`'s `skill[].cost == 0` guard means it is never touched.
    assert_eq!(fighter.values[1][CharacterValue::Armor as usize], 7);

    assert_eq!(fighter.exp, fighter.exp_used);
    assert_eq!(fighter.level, exp2level(fighter.exp));
}

#[test]
fn apply_warped_raise_sets_light_dark_and_athlete_professions_from_profession_value() {
    let mut fighter = character(1);
    fighter.values[1][CharacterValue::Profession as usize] = 20;

    apply_warped_raise(&mut fighter, 40);

    // `min(60, max(1, 40 - 5)) = 35`.
    assert_eq!(fighter.values[1][CharacterValue::Profession as usize], 35);
    assert_eq!(fighter.professions[crate::legacy::profession::LIGHT], 30);
    assert_eq!(fighter.professions[crate::legacy::profession::DARK], 30);
    assert_eq!(fighter.professions[crate::legacy::profession::ATHLETE], 5);
}

#[test]
fn warped_equip_mod_value_matches_c_formula() {
    // `1 + 40 / 2.75 = 15.545...` truncated to `15`.
    assert_eq!(warped_equip_mod_value(40), 15);
}

#[test]
fn warped_armor_and_weapon_spell_mod_values_clamp_and_scale() {
    let mut fighter = character(1);
    fighter.values[1][CharacterValue::ArmorSkill as usize] = 30;
    fighter.values[1][CharacterValue::Hand as usize] = 200;

    // `max(13, min(123, 30 + 10)) * 20 = 40 * 20 = 800`.
    assert_eq!(warped_armor_spell_mod_value(&fighter), 800);
    // `max(13, min(123, 200 + 10)) = 123` (clamped at the ceiling).
    assert_eq!(warped_weapon_spell_mod_value(&fighter), 123);
}
