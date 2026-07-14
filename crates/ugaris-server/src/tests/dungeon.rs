use super::*;

const WARRIOR_CHR: &str = r#"
    warrior:
      name="Warrior"
      description="Test warrior"
      sprite=50
      flag=CF_WARRIOR
      flag=CF_ALIVE
      V_HP=10
      V_ENDURANCE=10
      V_WIS=10
      V_INT=10
      V_AGI=10
      V_STR=10
      V_HAND=1
      V_ARMORSKILL=1
      V_ATTACK=1
      V_PARRY=1
      V_TACTICS=1
      V_SURROUND=1
      V_BODYCONTROL=1
      V_SPEEDSKILL=1
      V_PERCEPT=1
      V_IMMUNITY=1
      V_WARCRY=1
      V_PROFESSION=1
    ;
"#;

const MAGE_CHR: &str = r#"
    mage:
      name="Mage"
      description="Test mage"
      sprite=51
      flag=CF_MAGE
      flag=CF_ALIVE
      V_HP=10
      V_MANA=10
      V_ENDURANCE=10
      V_WIS=10
      V_INT=10
      V_AGI=10
      V_STR=10
      V_HAND=1
      V_MAGICSHIELD=1
      V_FLASH=1
      V_BLESS=1
      V_IMMUNITY=1
      V_FREEZE=1
      V_HEAL=1
      V_FIREBALL=1
      V_PERCEPT=1
      V_PROFESSION=1
    ;
"#;

const SEYAN_CHR: &str = r#"
    seyan:
      name="Seyan"
      description="Test seyan"
      sprite=50
      flag=CF_MAGE
      flag=CF_WARRIOR
      flag=CF_ALIVE
      V_HP=10
      V_MANA=10
      V_ENDURANCE=10
      V_WIS=10
      V_INT=10
      V_AGI=10
      V_STR=10
      V_HAND=1
      V_ARMORSKILL=1
      V_ATTACK=1
      V_PARRY=1
      V_IMMUNITY=1
      V_BLESS=1
      V_FREEZE=1
      V_TACTICS=1
      V_PERCEPT=1
      V_PROFESSION=1
    ;
"#;

const DUNGEON_ITM: &str = r#"
    equip1:
      name="Equip1"
      mod_index=V_HAND
      mod_value=1
      mod_index=V_ATTACK
      mod_value=1
      mod_index=V_PARRY
      mod_value=1
      mod_index=V_TACTICS
      mod_value=1
      mod_index=V_IMMUNITY
      mod_value=1
    ;
    equip1b:
      name="Equip1"
      mod_index=V_HAND
      mod_value=1
      mod_index=V_MAGICSHIELD
      mod_value=1
      mod_index=V_FLASH
      mod_value=1
      mod_index=V_FREEZE
      mod_value=1
      mod_index=V_IMMUNITY
      mod_value=1
    ;
    equip1c:
      name="Equip1"
      mod_index=V_HAND
      mod_value=1
      mod_index=V_ATTACK
      mod_value=1
      mod_index=V_PARRY
      mod_value=1
      mod_index=V_TACTICS
      mod_value=1
      mod_index=V_IMMUNITY
      mod_value=1
    ;
    equip2:
      name="Equip2"
      mod_index=V_WIS
      mod_value=1
      mod_index=V_INT
      mod_value=1
      mod_index=V_AGI
      mod_value=1
      mod_index=V_STR
      mod_value=1
    ;
    equip2b:
      name="Equip2"
      mod_index=V_MANA
      mod_value=1
      mod_index=V_HP
      mod_value=1
      mod_index=V_BLESS
      mod_value=1
      mod_index=V_INT
      mod_value=1
    ;
    equip2c:
      name="Equip2"
      mod_index=V_WIS
      mod_value=1
      mod_index=V_INT
      mod_value=1
      mod_index=V_AGI
      mod_value=1
      mod_index=V_STR
      mod_value=1
      mod_index=V_FREEZE
      mod_value=1
    ;
    armor_spell:
      name="Armor"
      mod_index=V_ARMOR
      mod_value=1
    ;
    weapon_spell:
      name="Weapon"
      mod_index=V_WEAPON
      mod_value=1
    ;
    teleport_trap:
      name="Teleport Trap"
      flag=IF_STEPACTION
    ;
    fake_wall:
      name="Fake Wall"
      flag=IF_USE
      flag=IF_MOVEBLOCK
      flag=IF_SIGHTBLOCK
      flag=IF_SOUNDBLOCK
      sprite=59172
    ;
    dungeon_door:
      name="Dungeon Door"
      flag=IF_USE
      sprite=19
    ;
    maze_key_spawn:
      name="Key"
      flag=IF_USE
      sprite=50004
    ;
"#;

fn dungeon_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(WARRIOR_CHR).unwrap();
    loader.load_character_templates_str(MAGE_CHR).unwrap();
    loader.load_character_templates_str(SEYAN_CHR).unwrap();
    loader.load_item_templates_str(DUNGEON_ITM).unwrap();
    loader
}

// C `level2maxitem(level) * 1.1 + max(0, level - 63) / 2` (`dungeon.c:334`
// and twins) - the `/ 2` truncates as C integer division before widening.
#[test]
fn dungeon_guard_equip_mod_value_matches_legacy_formula() {
    assert_eq!(dungeon_guard_equip_mod_value(25), 8); // level2maxitem(25)=8, 8*1.1=8.8 -> 8
    assert_eq!(dungeon_guard_equip_mod_value(1), 0); // level2maxitem(1)=0
                                                     // level2maxitem(70)=20, 20*1.1=22.0, (70-63)/2=3 -> 25
    assert_eq!(dungeon_guard_equip_mod_value(70), 25);
}

// C `build_warrior(x, y, level)` (`dungeon.c:217-336`), full happy path at
// a mid-range level: verifies the per-skill switch formulas, the equipment
// item modifier values, the clan-profession assignment, and that
// `update_char`'s hp/endurance/mana tail ran.
#[test]
fn build_warrior_computes_stats_and_equipment_for_level_25() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(300);

    let result = build_warrior(&mut world, &mut loader, &mut runtime, 12, 13, 25, 3, 25);
    assert!(result.is_some());

    let character = world.characters.get(&CharacterId(300)).unwrap();
    assert_eq!(character.name, "Warrior25");
    assert_eq!((character.x, character.y), (12, 13));
    assert_eq!(character.sprite, 269); // 266 + maze_clan(3)
    assert_eq!(character.rest_x, 3);
    assert_eq!(character.rest_y, 0); // (25 - maze_level(25)) / 2

    // WARRIOR_TAB[25] == 33.
    let values = &character.values[1];
    assert_eq!(values[CharacterValue::Hp as usize], 13); // max(10, 33-20)
    assert_eq!(values[CharacterValue::Endurance as usize], 10); // max(10, 33-30)
    assert_eq!(values[CharacterValue::Profession as usize], 26); // max(1, 33-7)
    assert_eq!(values[CharacterValue::Wisdom as usize], 18); // max(10, 33-15)
    assert_eq!(values[CharacterValue::Intelligence as usize], 33);
    assert_eq!(values[CharacterValue::Agility as usize], 28); // max(10, 33-5)
    assert_eq!(values[CharacterValue::Strength as usize], 33);
    assert_eq!(values[CharacterValue::Hand as usize], 33);
    assert_eq!(values[CharacterValue::ArmorSkill as usize], 30); // (33/10)*10
    assert_eq!(values[CharacterValue::Attack as usize], 33);
    assert_eq!(values[CharacterValue::Parry as usize], 33);
    assert_eq!(values[CharacterValue::Immunity as usize], 33);
    assert_eq!(values[CharacterValue::Tactics as usize], 28); // max(1, 33-5)
    assert_eq!(values[CharacterValue::Surround as usize], 1); // max(1, 33-50)
    assert_eq!(values[CharacterValue::BodyControl as usize], 13); // max(1, 33-20)
    assert_eq!(values[CharacterValue::SpeedSkill as usize], 13); // max(1, 33-20)
    assert_eq!(values[CharacterValue::Percept as usize], 23); // max(1, 33-10)
    assert_eq!(values[CharacterValue::Warcry as usize], 1); // default: max(1, 33-50)
    assert_eq!(values[CharacterValue::Rage as usize], 0); // not arch, never forced

    // Clan profession: value[1][Profession]=26 -> CLAN=26, not >30 so no light/dark.
    assert_eq!(character.professions[profession::CLAN], 26);
    assert_eq!(character.professions[profession::LIGHT], 0);
    assert_eq!(character.professions[profession::DARK], 0);

    // Equipment: equip1 (slot 12, 5 modifiers), equip2 (slot 13, 4
    // modifiers), armor_spell (slot 14), weapon_spell (slot 15).
    let equip1_id = character.inventory[12].expect("equip1 attached");
    let equip1 = world.items.get(&equip1_id).unwrap();
    for value in &equip1.modifier_value[..5] {
        assert_eq!(*value, 8); // dungeon_guard_equip_mod_value(25)
    }

    let equip2_id = character.inventory[13].expect("equip2 attached");
    let equip2 = world.items.get(&equip2_id).unwrap();
    for value in &equip2.modifier_value[..4] {
        assert_eq!(*value, 8);
    }

    let armor_id = character.inventory[14].expect("armor_spell attached");
    let armor = world.items.get(&armor_id).unwrap();
    assert_eq!(armor.modifier_value[0], 600); // clamp(13,113,ArmorSkill=30) * 20

    let weapon_id = character.inventory[15].expect("weapon_spell attached");
    let weapon = world.items.get(&weapon_id).unwrap();
    assert_eq!(weapon.modifier_value[0], 33); // clamp(13,113,Hand=33)

    // C `update_char` tail: hp/endurance/mana derived from the recomputed
    // value[0], not left at template defaults.
    assert_eq!(
        character.hp,
        i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE
    );
    assert_eq!(
        character.endurance,
        i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE
    );
    assert_eq!(
        character.mana,
        i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE
    );

    assert_eq!(result, Some(character.level));
}

// C `if (level > 33) { ch[cn].flags |= CF_ARCH; ch[cn].value[1][V_RAGE] =
// 1; }` (`dungeon.c:219-221`) - forces V_RAGE to be raised even though the
// "warrior" template itself never carries it.
#[test]
fn build_warrior_forces_rage_and_arch_flag_above_level_33() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(301);

    build_warrior(&mut world, &mut loader, &mut runtime, 12, 13, 40, 3, 40).unwrap();

    let character = world.characters.get(&CharacterId(301)).unwrap();
    assert!(character.flags.contains(CharacterFlags::ARCH));
    // WARRIOR_TAB[40] == 51; V_RAGE: max(1, 51-20) == 31.
    assert_eq!(character.values[1][CharacterValue::Rage as usize], 31);
}

// C `if (level < 1) level = 1;` (`dungeon.c:223-225`) applies before the
// table lookup and the final `sprintf` name.
#[test]
fn build_warrior_clamps_nonpositive_level_to_one() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(302);

    build_warrior(&mut world, &mut loader, &mut runtime, 12, 13, 0, 3, 0).unwrap();

    let character = world.characters.get(&CharacterId(302)).unwrap();
    assert_eq!(character.name, "Warrior1");
    // WARRIOR_TAB[1] == 1, so V_HP == max(10, 1-20) == 10.
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 10);
}

// C `if (maze_clan < 17) sprite = 266+maze_clan; else sprite = 516 +
// maze_clan - 16;` (`dungeon.c:312-316`).
#[test]
fn build_warrior_uses_high_maze_clan_sprite_branch() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(303);

    build_warrior(&mut world, &mut loader, &mut runtime, 12, 13, 25, 20, 25).unwrap();

    let character = world.characters.get(&CharacterId(303)).unwrap();
    assert_eq!(character.sprite, 520); // 516 + 20 - 16
}

#[test]
fn build_warrior_returns_none_when_template_missing() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();

    assert!(build_warrior(&mut world, &mut loader, &mut runtime, 12, 13, 25, 3, 25).is_none());
    assert!(world.characters.is_empty());
}

// C `build_mage(x, y, level)` (`dungeon.c:389-535`).
#[test]
fn build_mage_computes_stats_and_equipment_for_level_25() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(310);

    let result = build_mage(&mut world, &mut loader, &mut runtime, 12, 13, 25, 3, 25);
    assert!(result.is_some());

    let character = world.characters.get(&CharacterId(310)).unwrap();
    assert_eq!(character.name, "Mage25");
    assert_eq!(character.sprite, 285); // 282 + maze_clan(3)

    // MAGE_TAB[25] == 31.
    let values = &character.values[1];
    assert_eq!(values[CharacterValue::Hp as usize], 10); // max(10, 31-40)
    assert_eq!(values[CharacterValue::Mana as usize], 21); // max(10, 31-10)
    assert_eq!(values[CharacterValue::Endurance as usize], 10); // max(10, 31-30)
    assert_eq!(values[CharacterValue::Profession as usize], 24); // max(1, 31-7)
    assert_eq!(values[CharacterValue::Wisdom as usize], 31);
    assert_eq!(values[CharacterValue::Intelligence as usize], 31);
    assert_eq!(values[CharacterValue::Agility as usize], 31);
    assert_eq!(values[CharacterValue::Strength as usize], 31);
    assert_eq!(values[CharacterValue::Hand as usize], 31);
    assert_eq!(values[CharacterValue::MagicShield as usize], 31);
    assert_eq!(values[CharacterValue::Flash as usize], 31);
    assert_eq!(values[CharacterValue::Bless as usize], 31);
    assert_eq!(values[CharacterValue::Immunity as usize], 31);
    assert_eq!(values[CharacterValue::Freeze as usize], 21); // max(1, 31-10)
    assert_eq!(values[CharacterValue::Heal as usize], 21);
    assert_eq!(values[CharacterValue::Fireball as usize], 21);
    assert_eq!(values[CharacterValue::Percept as usize], 21);
    assert_eq!(values[CharacterValue::Duration as usize], 0); // not arch, never forced

    assert_eq!(character.professions[profession::CLAN], 24);

    // Mage only carries equip1b/equip2b/weapon_spell - no armor_spell.
    let equip1_id = character.inventory[12].expect("equip1b attached");
    let equip1 = world.items.get(&equip1_id).unwrap();
    for value in &equip1.modifier_value[..5] {
        assert_eq!(*value, 8);
    }
    let equip2_id = character.inventory[13].expect("equip2b attached");
    let equip2 = world.items.get(&equip2_id).unwrap();
    for value in &equip2.modifier_value[..4] {
        assert_eq!(*value, 8);
    }
    assert!(character.inventory[14].is_none(), "mage has no armor_spell");
    let weapon_id = character.inventory[15].expect("weapon_spell attached");
    let weapon = world.items.get(&weapon_id).unwrap();
    assert_eq!(weapon.modifier_value[0], 31); // clamp(13,113,Hand=31)
}

// C `if (level > 33) { ch[cn].flags |= CF_ARCH; ch[cn].value[1][V_DURATION]
// = 1; }` (`dungeon.c:392-394`).
#[test]
fn build_mage_forces_duration_and_arch_flag_above_level_33() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(311);

    build_mage(&mut world, &mut loader, &mut runtime, 12, 13, 40, 3, 40).unwrap();

    let character = world.characters.get(&CharacterId(311)).unwrap();
    assert!(character.flags.contains(CharacterFlags::ARCH));
    // MAGE_TAB[40] == 49; V_DURATION: max(1, 49-10) == 39.
    assert_eq!(character.values[1][CharacterValue::Duration as usize], 39);
}

// C `build_seyan(x, y, level)` (`dungeon.c:551-700`).
#[test]
fn build_seyan_computes_stats_and_equipment_for_level_25() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(320);

    let result = build_seyan(&mut world, &mut loader, &mut runtime, 12, 13, 25, 3, 25);
    assert!(result.is_some());

    let character = world.characters.get(&CharacterId(320)).unwrap();
    assert_eq!(character.name, "Seyan25");
    assert_eq!(character.sprite, 269); // 266 + maze_clan(3)

    // SEYAN_TAB[25] == 29.
    let values = &character.values[1];
    assert_eq!(values[CharacterValue::Hp as usize], 10); // max(10, 29-40)
    assert_eq!(values[CharacterValue::Mana as usize], 10); // max(10, 29-30)
    assert_eq!(values[CharacterValue::Endurance as usize], 10); // max(10, 29-50)
    assert_eq!(values[CharacterValue::Profession as usize], 22); // max(1, 29-7)
    assert_eq!(values[CharacterValue::Wisdom as usize], 14); // max(10, 29-15)
    assert_eq!(values[CharacterValue::Intelligence as usize], 29);
    assert_eq!(values[CharacterValue::Agility as usize], 24); // max(10, 29-5)
    assert_eq!(values[CharacterValue::Strength as usize], 29);
    assert_eq!(values[CharacterValue::Hand as usize], 29);
    assert_eq!(values[CharacterValue::ArmorSkill as usize], 20); // (29/10)*10
    assert_eq!(values[CharacterValue::Attack as usize], 29);
    assert_eq!(values[CharacterValue::Parry as usize], 29);
    assert_eq!(values[CharacterValue::Immunity as usize], 29);
    assert_eq!(values[CharacterValue::Bless as usize], 29);
    assert_eq!(values[CharacterValue::Freeze as usize], 29);
    assert_eq!(values[CharacterValue::Tactics as usize], 24); // max(1, 29-5)
    assert_eq!(values[CharacterValue::Percept as usize], 19); // max(1, 29-10)

    assert_eq!(character.professions[profession::CLAN], 22);

    // Seyan carries equip1c (5 mods), equip2c (5 mods), armor_spell,
    // weapon_spell.
    let equip1_id = character.inventory[12].expect("equip1c attached");
    let equip1 = world.items.get(&equip1_id).unwrap();
    for value in &equip1.modifier_value[..5] {
        assert_eq!(*value, 8);
    }
    let equip2_id = character.inventory[13].expect("equip2c attached");
    let equip2 = world.items.get(&equip2_id).unwrap();
    for value in &equip2.modifier_value[..5] {
        assert_eq!(*value, 8);
    }
    let armor_id = character.inventory[14].expect("armor_spell attached");
    let armor = world.items.get(&armor_id).unwrap();
    assert_eq!(armor.modifier_value[0], 400); // clamp(13,113,ArmorSkill=20) * 20
    let weapon_id = character.inventory[15].expect("weapon_spell attached");
    let weapon = world.items.get(&weapon_id).unwrap();
    assert_eq!(weapon.modifier_value[0], 29); // clamp(13,113,Hand=29)
}

// C's per-skill `switch (n)` bodies, tested directly for a couple of
// corner cases that the full-spawn tests above don't exercise: the
// `min(...,125)`-pre-clamp negative-base saturation branches.
#[test]
fn warrior_stat_value_default_and_negative_base_branches() {
    // Surround: max(1, base-50), base=10 -> stays at floor 1.
    assert_eq!(
        warrior_stat_value(CharacterValue::Surround as usize, 10, 25, false),
        1
    );
    // Unlisted index (e.g. Dagger) falls into the default branch.
    assert_eq!(
        warrior_stat_value(CharacterValue::Dagger as usize, 10, 25, false),
        1
    );
    // Rage only applies its formula when arch.
    assert_eq!(
        warrior_stat_value(CharacterValue::Rage as usize, 51, 40, false),
        0
    );
    assert_eq!(
        warrior_stat_value(CharacterValue::Rage as usize, 51, 40, true),
        31
    );
}

// C `build_wall(x, y)` (`dungeon.c:715-723`).
// The sprite asserts intentionally mirror C's `59171 + ((x & 3) + (y & 3)) % 4`
// formula verbatim, even where sub-terms fold away for these coordinates.
#[allow(clippy::identity_op)]
#[test]
fn build_wall_sets_indoor_blocking_flags_and_cycling_sprite() {
    let mut world = World::default();

    build_wall(&mut world, 10, 10);
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(
        tile.flags,
        MapFlags::INDOORS
            | MapFlags::SIGHTBLOCK
            | MapFlags::SOUNDBLOCK
            | MapFlags::SHOUTBLOCK
            | MapFlags::MOVEBLOCK
    );
    assert_eq!(tile.foreground_sprite, 59171 + ((10 & 3) + (10 & 3)) % 4);
    assert_eq!(tile.ground_sprite, 0);

    // Different (x,y) parity picks a different sprite variant.
    build_wall(&mut world, 11, 12);
    let tile = world.map.tile(11, 12).unwrap();
    assert_eq!(tile.foreground_sprite, 59171 + ((11 & 3) + (12 & 3)) % 4);
}

// C `build_teleport(x, y)` (`dungeon.c:786-798`): destination is always
// `xoff+2`, `yoff+78`, never the trap's own placement coordinates.
#[test]
fn build_teleport_places_item_with_fixed_entrance_target() {
    let mut world = World::default();
    let mut loader = dungeon_loader();

    build_teleport(&mut world, &mut loader, 20, 20, 2, 2, 5);

    let tile = world.map.tile(20, 20).unwrap();
    assert_ne!(tile.item, 0);
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    assert_eq!(item.name, "Teleport Trap");
    assert_eq!(
        u16::from_le_bytes([item.driver_data[0], item.driver_data[1]]),
        4
    ); // xoff+2
    assert_eq!(
        u16::from_le_bytes([item.driver_data[2], item.driver_data[3]]),
        80
    ); // yoff+78
    assert_eq!(
        u16::from_le_bytes([item.driver_data[4], item.driver_data[5]]),
        5
    ); // maze_clan
}

// C `build_fake(x, y)` (`dungeon.c:800-813`).
#[test]
fn build_fake_places_wall_like_item_and_stores_clan() {
    let mut world = World::default();
    let mut loader = dungeon_loader();

    build_fake(&mut world, &mut loader, 20, 20, 7);

    let tile = world.map.tile(20, 20).unwrap();
    assert_ne!(tile.item, 0);
    // The fake_wall item's own IF_MOVEBLOCK/IF_SIGHTBLOCK carried through
    // to the tile via the ordinary `set_item_map` propagation.
    assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
    assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    assert_eq!(item.driver_data[0], 7);
}

// C `build_door(x, y, keyid, keys)` (`dungeon.c:814-835`): `keys`
// controls how many `MAKE_ITEMID`-wrapped key slots get populated.
#[test]
fn build_door_wraps_key_ids_by_keys_count() {
    let mut world = World::default();
    let mut loader = dungeon_loader();

    build_door(&mut world, &mut loader, 20, 20, 0xABCD, 2, 9);
    let tile = world.map.tile(20, 20).unwrap();
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    let key1 = u32::from_le_bytes(item.driver_data[0..4].try_into().unwrap());
    let key2 = u32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
    assert_eq!(key1, (0x03 << 24) | 0xABCD); // DEV_ID_MAZE1
    assert_eq!(key2, (0x04 << 24) | 0xABCD); // DEV_ID_MAZE2
    assert_eq!(
        u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]),
        9
    );

    build_door(&mut world, &mut loader, 21, 20, 0xABCD, 0, 9);
    let tile = world.map.tile(21, 20).unwrap();
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    let key1 = u32::from_le_bytes(item.driver_data[0..4].try_into().unwrap());
    let key2 = u32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
    assert_eq!(key1, 0);
    assert_eq!(key2, 0);

    build_door(&mut world, &mut loader, 22, 20, 0xABCD, 1, 9);
    let tile = world.map.tile(22, 20).unwrap();
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    let key1 = u32::from_le_bytes(item.driver_data[0..4].try_into().unwrap());
    let key2 = u32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
    assert_eq!(key1, (0x03 << 24) | 0xABCD);
    assert_eq!(key2, 0);
}

// C `build_key(x, y, nr, keyid)` (`dungeon.c:836-850`).
#[test]
fn build_key_stores_nr_clan_and_raw_keyid() {
    let mut world = World::default();
    let mut loader = dungeon_loader();

    build_key(&mut world, &mut loader, 20, 20, 2, 0x1122_3344, 6);

    let tile = world.map.tile(20, 20).unwrap();
    let item = world.items.get(&ItemId(tile.item)).unwrap();
    assert_eq!(item.driver_data[0], 2); // nr
    assert_eq!(item.driver_data[1], 6); // maze_clan
    assert_eq!(item.driver_data[2], 0); // not yet taken
    let keyid = u32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
    assert_eq!(keyid, 0x1122_3344); // stored raw, not MAKE_ITEMID-wrapped
}

// C `dungeonkey` (`dungeon.c:1913-1937`): the real picked-up key's `ID`
// must be wrapped with the same `MAKE_ITEMID(DEV_ID_MAZE1/2, keyid)` a
// `build_door`-created door checks against (`dungeon.c:820,825`), keyed off
// which of the two `maze_key1`/`maze_key2` templates was granted.
#[test]
fn dungeon_key_item_id_wraps_raw_keyid_by_slot() {
    assert_eq!(
        dungeon_key_item_id("maze_key1", 0xABCD),
        (0x03 << 24) | 0xABCD
    );
    assert_eq!(
        dungeon_key_item_id("maze_key2", 0xABCD),
        (0x04 << 24) | 0xABCD
    );

    // Matches the corresponding door's own wrapped requirement exactly.
    let mut world = World::default();
    let mut loader = dungeon_loader();
    build_door(&mut world, &mut loader, 20, 20, 0xABCD, 2, 9);
    let tile = world.map.tile(20, 20).unwrap();
    let door = world.items.get(&ItemId(tile.item)).unwrap();
    let key1 = u32::from_le_bytes(door.driver_data[0..4].try_into().unwrap());
    let key2 = u32::from_le_bytes(door.driver_data[4..8].try_into().unwrap());
    assert_eq!(dungeon_key_item_id("maze_key1", 0xABCD), key1);
    assert_eq!(dungeon_key_item_id("maze_key2", 0xABCD), key2);
}

// C `build_cell(cx, cy, cell)` (`dungeon.c:851-937`): wall segments plus a
// warrior-tier NPC spawn dispatch.
#[test]
fn build_cell_builds_walls_and_dispatches_warrior_special_code() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(400);

    let cell = MazeCell {
        top_wall: true,
        left_wall: true,
        visited: true,
        special: 8, // warrior, tier 1 (+2 maze_level)
    };

    build_cell(
        &mut world,
        &mut loader,
        &mut runtime,
        1,
        1,
        &cell,
        2,
        2,
        3,
        999,
        20,
    );

    // Walls: cell_x=1*4+2=6, cell_y=6. top_wall builds (7,6)/(8,6)/(9,6);
    // left_wall builds (6,7)/(6,8)/(6,9), plus the corner (6,6).
    for (x, y) in [(6, 6), (7, 6), (8, 6), (9, 6), (6, 7), (6, 8), (6, 9)] {
        assert!(
            world
                .map
                .tile(x, y)
                .unwrap()
                .flags
                .contains(MapFlags::MOVEBLOCK),
            "expected wall at ({x},{y})"
        );
    }

    // special=8 => warrior at maze_level(20)+2=22, spawned at the cell
    // center (6+2, 6+2) = (8,8).
    let character = world.characters.get(&CharacterId(400)).unwrap();
    assert_eq!((character.x, character.y), (8, 8));
    assert_eq!(character.name, "Warrior22");
}

// `build_cell` dispatch for the door/key/teleport special codes (28-30,
// 3-4, 23-27) - just confirms routing, since the individual builders are
// already unit-tested above.
#[test]
fn build_cell_dispatches_door_key_and_teleport_special_codes() {
    let mut world = World::default();
    let mut loader = dungeon_loader();
    let mut runtime = ServerRuntime::default();

    let door_cell = MazeCell {
        special: 29,
        ..Default::default()
    };
    build_cell(
        &mut world,
        &mut loader,
        &mut runtime,
        2,
        2,
        &door_cell,
        2,
        2,
        3,
        555,
        20,
    );
    let center = world.map.tile(2 * 4 + 2 + 2, 2 * 4 + 2 + 2).unwrap();
    assert_ne!(center.item, 0);
    assert_eq!(
        world.items.get(&ItemId(center.item)).unwrap().name,
        "Dungeon Door"
    );

    let key_cell = MazeCell {
        special: 3,
        ..Default::default()
    };
    build_cell(
        &mut world,
        &mut loader,
        &mut runtime,
        3,
        2,
        &key_cell,
        2,
        2,
        3,
        555,
        20,
    );
    let center = world.map.tile(3 * 4 + 2 + 2, 2 * 4 + 2 + 2).unwrap();
    assert_eq!(world.items.get(&ItemId(center.item)).unwrap().name, "Key");

    let teleport_cell = MazeCell {
        special: 23,
        ..Default::default()
    };
    build_cell(
        &mut world,
        &mut loader,
        &mut runtime,
        4,
        2,
        &teleport_cell,
        2,
        2,
        3,
        555,
        20,
    );
    let center = world.map.tile(4 * 4 + 2 + 2, 2 * 4 + 2 + 2).unwrap();
    assert_eq!(
        world.items.get(&ItemId(center.item)).unwrap().name,
        "Teleport Trap"
    );
}
