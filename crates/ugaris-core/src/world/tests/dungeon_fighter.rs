use super::*;
use crate::character_driver::{CDR_DUNGEONFIGHTER, NT_DIDHIT, NT_GOTHIT};
use crate::spell::IDR_POTION_SP;

fn fighter(id: u32, cnr: u16, warrior: bool) -> Character {
    let mut fighter = character(id);
    fighter.name = "Warrior1".into();
    fighter.driver = CDR_DUNGEONFIGHTER;
    fighter.dungeonfighter = Some(crate::character_driver::DungeonfighterDriverData::default());
    fighter.driver_state = Some(CharacterDriverState::SimpleBaddy(
        crate::character_driver::SimpleBaddyDriverData::default(),
    ));
    fighter.rest_x = cnr;
    if warrior {
        fighter.flags |= CharacterFlags::WARRIOR;
    }
    fighter.values[0][CharacterValue::Hp as usize] = 100;
    fighter.values[0][CharacterValue::Mana as usize] = 100;
    fighter.values[0][CharacterValue::Endurance as usize] = 100;
    fighter.hp = 100 * POWERSCALE;
    fighter.mana = 100 * POWERSCALE;
    fighter.endurance = 100 * POWERSCALE;
    fighter
}

fn dungeonfighter_data(
    world: &World,
    id: CharacterId,
) -> crate::character_driver::DungeonfighterDriverData {
    world
        .characters
        .get(&id)
        .and_then(|c| c.dungeonfighter)
        .expect("expected Character::dungeonfighter to be populated")
}

fn found_clan(world: &mut World, name: &str) -> u16 {
    world.clan_registry.found_clan(name, 0).unwrap()
}

/// `may_add_spell` picks the *last* free slot in the spell-slot range (no
/// early `break` once a free slot is found, matching C's own forward-
/// scanning `may_add_spell`), so tests look up the granted potion by
/// scanning for an `IDR_POTION_SP` item rather than assuming a fixed slot.
fn find_potion_spell_item(world: &World, character_id: CharacterId) -> Option<&Item> {
    let character = world.characters.get(&character_id)?;
    character.inventory[crate::spell::SPELL_SLOT_START..crate::spell::SPELL_SLOT_END]
        .iter()
        .flatten()
        .find_map(|item_id| world.items.get(item_id))
        .filter(|item| item.driver == IDR_POTION_SP)
}

// --- dungeon_potion -------------------------------------------------

#[test]
fn dungeon_potion_grants_warrior_stat_boost_and_consumes_stock() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[1][CharacterValue::Intelligence as usize] = 30;
    assert!(world.spawn_character(warrior, 10, 10));

    // Stock tier 2 (str = 12) via 3x attack/parry/immunity mod (mod_value = (tier+1)*4 = 12).
    let mod_index = [
        CharacterValue::Attack as i16,
        CharacterValue::Parry as i16,
        CharacterValue::Immunity as i16,
        0,
        0,
    ];
    let mod_value = [12, 12, 12, 0, 0];
    assert!(world
        .clan_registry
        .add_alc_potion(cnr, mod_index, mod_value));

    assert!(world.dungeon_potion(CharacterId(1)));

    // Stock consumed.
    assert_eq!(
        world.clan_registry.identity(cnr).unwrap().economy.alc_pot[0][2],
        0
    );

    let item = find_potion_spell_item(&world, CharacterId(1)).unwrap();
    assert_eq!(item.driver, IDR_POTION_SP);
    assert_eq!(item.modifier_index[0], CharacterValue::Attack as i16);
    assert_eq!(item.modifier_index[1], CharacterValue::Parry as i16);
    assert_eq!(item.modifier_index[2], CharacterValue::Immunity as i16);
    assert_eq!(item.modifier_value[0], 12);
    assert_eq!(item.modifier_value[1], 12);
    assert_eq!(item.modifier_value[2], 12);
}

#[test]
fn dungeon_potion_grants_non_warrior_flash_boost() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut mage = fighter(1, cnr, false);
    mage.values[1][CharacterValue::Intelligence as usize] = 30;
    assert!(world.spawn_character(mage, 10, 10));

    let mod_index = [
        CharacterValue::Flash as i16,
        CharacterValue::MagicShield as i16,
        CharacterValue::Immunity as i16,
        0,
        0,
    ];
    let mod_value = [8, 8, 8, 0, 0];
    assert!(world
        .clan_registry
        .add_alc_potion(cnr, mod_index, mod_value));

    assert!(world.dungeon_potion(CharacterId(1)));

    let item = find_potion_spell_item(&world, CharacterId(1)).unwrap();
    assert_eq!(item.modifier_index[0], CharacterValue::Flash as i16);
    assert_eq!(item.modifier_index[1], CharacterValue::MagicShield as i16);
}

#[test]
fn dungeon_potion_picks_highest_qualifying_stocked_tier() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    // V_INT = 25 disqualifies tier 3 (needs int >= 30) but allows tier 2 (needs int >= 20).
    warrior.values[1][CharacterValue::Intelligence as usize] = 25;
    assert!(world.spawn_character(warrior, 10, 10));

    // Stock every tier 0..=5 with 1 potion each.
    for tier in 0..=5i16 {
        let value = tier * 4 + 4;
        let mod_index = [
            CharacterValue::Attack as i16,
            CharacterValue::Parry as i16,
            CharacterValue::Immunity as i16,
            0,
            0,
        ];
        let mod_value = [value, value, value, 0, 0];
        assert!(world
            .clan_registry
            .add_alc_potion(cnr, mod_index, mod_value));
    }

    assert!(world.dungeon_potion(CharacterId(1)));

    // Tier 2 (str=12) should have been picked and consumed, tier 3+ untouched.
    let economy = &world.clan_registry.identity(cnr).unwrap().economy;
    assert_eq!(economy.alc_pot[0][2], 0);
    assert_eq!(economy.alc_pot[0][3], 1);

    let item = find_potion_spell_item(&world, CharacterId(1)).unwrap();
    assert_eq!(item.modifier_value[0], 12);
}

#[test]
fn dungeon_potion_fails_without_stock() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[1][CharacterValue::Intelligence as usize] = 30;
    assert!(world.spawn_character(warrior, 10, 10));

    assert!(!world.dungeon_potion(CharacterId(1)));
}

#[test]
fn dungeon_potion_fails_without_free_spell_slot() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[1][CharacterValue::Intelligence as usize] = 30;
    // Fill every spell slot with an unrelated spell item.
    for slot in crate::legacy::INVENTORY_START_SPELLS..crate::spell::SPELL_SLOT_END {
        let item_id = ItemId(1000 + slot as u32);
        warrior.inventory[slot] = Some(item_id);
        world
            .items
            .insert(item_id, item(item_id.0, ItemFlags::USED));
    }
    assert!(world.spawn_character(warrior, 10, 10));

    let mod_index = [
        CharacterValue::Attack as i16,
        CharacterValue::Parry as i16,
        CharacterValue::Immunity as i16,
        0,
        0,
    ];
    let mod_value = [12, 12, 12, 0, 0];
    assert!(world
        .clan_registry
        .add_alc_potion(cnr, mod_index, mod_value));

    assert!(!world.dungeon_potion(CharacterId(1)));
    // Stock untouched since the free-slot check runs first, matching C.
    assert_eq!(
        world.clan_registry.identity(cnr).unwrap().economy.alc_pot[0][2],
        1
    );
}

// --- dungeonfighter message loop ------------------------------------

#[test]
fn dungeonfighter_accumulates_damage_from_messages() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let warrior = fighter(1, cnr, true);
    assert!(world.spawn_character(warrior, 10, 10));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 15, 0);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_GOTHIT, 3, 40, 0);

    world.process_dungeonfighter_actions();

    let dat = dungeonfighter_data(&world, id);
    assert_eq!(dat.damage_done, 15);
    assert_eq!(dat.damage_taken, 40);
}

#[test]
fn dungeonfighter_ignores_other_drivers() {
    let mut world = World::default();
    let mut bystander = character(1);
    bystander.driver = 0;
    assert!(world.spawn_character(bystander, 10, 10));
    // Should not panic and should not touch anything.
    world.process_dungeonfighter_actions();
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .driver_state
        .is_none());
}

#[test]
fn dungeonfighter_drinks_mana_potion_when_low_and_qualifying() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.mana = 10 * POWERSCALE; // 10 < 100/2=50, definitely low.
    assert!(world.spawn_character(warrior, 10, 10));
    // Stock the small (tier 0, add=8) mana potion.
    assert!(world.clan_registry.bump_simple_pot(cnr, 1, 0));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.mana, (10 + 8) * POWERSCALE);
    let dat = dungeonfighter_data(&world, id);
    assert_eq!(dat.simple_pots_taken, 1);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[1][0],
        0
    );
}

#[test]
fn dungeonfighter_drinks_big_mana_potion_for_large_need() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[0][CharacterValue::Mana as usize] = 200;
    warrior.mana = 0; // need = 200, > 24 => big potion (add=24).
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.clan_registry.bump_simple_pot(cnr, 1, 2));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.mana, 24 * POWERSCALE);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[1][2],
        0
    );
}

#[test]
fn dungeonfighter_drinks_hp_potion_when_low_and_qualifying() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.hp = 10 * POWERSCALE;
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.clan_registry.bump_simple_pot(cnr, 0, 0));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.hp, (10 + 8) * POWERSCALE);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[0][0],
        0
    );
}

#[test]
fn dungeonfighter_skips_potions_without_recent_good_damage() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.hp = 10 * POWERSCALE;
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.clan_registry.bump_simple_pot(cnr, 0, 0));

    // No NT_DIDHIT message at all this tick => damage_done stays 0,
    // failing C's `damage_done > 10` gate.
    world.process_dungeonfighter_actions();

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, 10 * POWERSCALE);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[0][0],
        1
    );
}

#[test]
fn dungeonfighter_drinks_combo_potion_only_when_specific_kind_unavailable() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.hp = 10 * POWERSCALE;
    warrior.mana = 10 * POWERSCALE;
    assert!(world.spawn_character(warrior, 10, 10));
    // Neither hp nor mana specific pots stocked, but combo (kind=2) is.
    assert!(world.clan_registry.bump_simple_pot(cnr, 2, 0));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.hp, (10 + 8) * POWERSCALE);
    assert_eq!(character.mana, (10 + 8) * POWERSCALE);
    // `fighter()` starts endurance at its own max (100), so C's own
    // `min(value[0][V_ENDURANCE]*POWERSCALE, endurance+add*POWERSCALE)`
    // clamp caps it right back at 100 - not a bug, just an already-full
    // stat (this test only cares about the hp/mana combo-potion trigger).
    assert_eq!(character.endurance, 100 * POWERSCALE);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[2][0],
        0
    );
}

#[test]
fn dungeonfighter_does_not_drink_combo_potion_when_hp_potion_already_taken() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.hp = 10 * POWERSCALE;
    warrior.mana = 10 * POWERSCALE;
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.clan_registry.bump_simple_pot(cnr, 0, 0)); // hp small
    assert!(world.clan_registry.bump_simple_pot(cnr, 2, 0)); // combo small

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    // The hp-specific potion satisfied the tick (flag=true), so combo
    // stock must remain untouched even though mana is still low.
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[2][0],
        1
    );
    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.hp, (10 + 8) * POWERSCALE);
    assert_eq!(character.mana, 10 * POWERSCALE);
}

#[test]
fn dungeonfighter_respects_simple_pot_budget_of_five() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.hp = 10 * POWERSCALE;
    warrior.dungeonfighter = Some(crate::character_driver::DungeonfighterDriverData {
        simple_pots_taken: 5,
        ..Default::default()
    });
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.clan_registry.bump_simple_pot(cnr, 0, 0));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let character = world.characters.get(&id).unwrap();
    assert_eq!(character.hp, 10 * POWERSCALE);
    assert_eq!(
        world
            .clan_registry
            .identity(cnr)
            .unwrap()
            .economy
            .simple_pot[0][0],
        1
    );
}

#[test]
fn dungeonfighter_drinks_alchemy_potion_after_didhit_when_healthy() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[1][CharacterValue::Intelligence as usize] = 30;
    // Healthy (hp > half of max) so the alchemy-potion gate is open.
    warrior.hp = 90 * POWERSCALE;
    assert!(world.spawn_character(warrior, 10, 10));
    let mod_index = [
        CharacterValue::Attack as i16,
        CharacterValue::Parry as i16,
        CharacterValue::Immunity as i16,
        0,
        0,
    ];
    let mod_value = [8, 8, 8, 0, 0];
    assert!(world
        .clan_registry
        .add_alc_potion(cnr, mod_index, mod_value));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let dat = dungeonfighter_data(&world, id);
    assert_eq!(dat.alc_pots_taken, 1);
    assert_eq!(
        world.clan_registry.identity(cnr).unwrap().economy.alc_pot[0][1],
        0
    );
    assert!(find_potion_spell_item(&world, id).is_some());
}

#[test]
fn dungeonfighter_skips_alchemy_potion_when_hp_below_half() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[1][CharacterValue::Intelligence as usize] = 30;
    warrior.hp = 10 * POWERSCALE; // below half of max (50).
    assert!(world.spawn_character(warrior, 10, 10));
    let mod_index = [
        CharacterValue::Attack as i16,
        CharacterValue::Parry as i16,
        CharacterValue::Immunity as i16,
        0,
        0,
    ];
    let mod_value = [8, 8, 8, 0, 0];
    assert!(world
        .clan_registry
        .add_alc_potion(cnr, mod_index, mod_value));

    let id = CharacterId(1);
    world
        .characters
        .get_mut(&id)
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 20, 0);

    world.process_dungeonfighter_actions();

    let dat = dungeonfighter_data(&world, id);
    assert_eq!(dat.alc_pots_taken, 0);
    assert_eq!(
        world.clan_registry.identity(cnr).unwrap().economy.alc_pot[0][1],
        1
    );
}

// --- reused SimpleBaddy combat AI (`char_driver(CDR_SIMPLEBADDY, ...)`
// tail call, `dungeon.c:2161`) -----------------------------------------

#[test]
fn dungeonfighter_npc_is_spawned_with_simple_baddy_driver_state_and_own_data_field() {
    // The `fighter()` test helper mirrors `zone.rs`'s real
    // `CDR_DUNGEONFIGHTER` template-instantiation branch: `driver_state`
    // holds `SimpleBaddy` (so it can attack/wander like any other
    // SimpleBaddy) while `Character::dungeonfighter` holds this driver's
    // own potion-budget counters independently.
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let warrior = fighter(1, cnr, true);
    assert!(world.spawn_character(warrior, 10, 10));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver, CDR_DUNGEONFIGHTER);
    assert!(matches!(
        character.driver_state,
        Some(CharacterDriverState::SimpleBaddy(_))
    ));
    assert!(character.dungeonfighter.is_some());
}

#[test]
fn dungeonfighter_npc_attacks_visible_enemy_via_reused_simple_baddy_dispatch() {
    // C's `dungeonfighter` ends every tick with `char_driver(CDR_
    // SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`, reusing the SimpleBaddy
    // driver's full attack logic on a `CDR_DUNGEONFIGHTER` character.
    // `process_simple_baddy_attack_action`'s own driver gate must accept
    // `CDR_DUNGEONFIGHTER` (not just `CDR_SIMPLEBADDY`) for this to fire.
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[0][CharacterValue::Fireball as usize] = 20;
    warrior.values[0][CharacterValue::Speed as usize] = 50;
    if let Some(CharacterDriverState::SimpleBaddy(data)) = warrior.driver_state.as_mut() {
        data.enemies.push(SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        });
    }
    let target = character(2);
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, 0);
    // The dungeonfighter's own potion-budget data must survive untouched.
    assert!(npc.dungeonfighter.is_some());
}

#[test]
fn dungeonfighter_aggregate_attack_dispatch_includes_dungeonfighter_driver() {
    let mut world = World::default();
    let cnr = found_clan(&mut world, "TestClan");
    let mut warrior = fighter(1, cnr, true);
    warrior.values[0][CharacterValue::Fireball as usize] = 20;
    warrior.values[0][CharacterValue::Speed as usize] = 50;
    if let Some(CharacterDriverState::SimpleBaddy(data)) = warrior.driver_state.as_mut() {
        data.enemies.push(SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        });
    }
    let target = character(2);
    assert!(world.spawn_character(warrior, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;

    let attacks = world.process_simple_baddy_attack_actions_with_random(1, |_| 0);
    assert_eq!(attacks, 1);
}
