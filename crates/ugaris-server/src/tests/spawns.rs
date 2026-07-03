use super::*;

#[test]
fn lq_npc_spawn_request_instantiates_template_and_records_slot_identity() {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                lq_guard:
                  name="Template Guard"
                  description="Template description"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                  V_DAGGER=10
                  V_ATTACK=8
                  V_WARCRY=7
                  V_BLESS=9
                  V_FIREBALL=6
                  V_MAGICSHIELD=5
                ;
            "#,
        )
        .unwrap();
    loader
        .load_item_templates_str(
            r#"
                lqx_spell:
                  name="LQX Spell"
                ;
                dagger3q1:
                  name="Quest Dagger"
                ;
            "#,
        )
        .unwrap();
    let mut world = World::default();
    assert!(world.configure_lq_npc(ugaris_core::world::LqNpcState {
        slot: 2,
        basename: "guard".to_string(),
        x: 12,
        y: 13,
        dir: ugaris_core::direction::Direction::Left as u8,
        level: 17,
        mode: b'n',
        respawn_seconds: 60,
        name: "Quest Guard".to_string(),
        description: "A live quest guard.".to_string(),
        nick: [String::new(), String::new()],
        character_id: None,
        character_serial: 0,
    }));
    let request = ugaris_core::world::LqNpcSpawnRequest {
        slot: 2,
        basename: "guard".to_string(),
        x: 12,
        y: 13,
        dir: ugaris_core::direction::Direction::Left as u8,
        level: 17,
        mode: b'n',
        name: "Quest Guard".to_string(),
        description: "A live quest guard.".to_string(),
        nick: [String::new(), String::new()],
    };
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(200);

    assert!(spawn_lq_npc_character(
        &mut world,
        &mut loader,
        &mut runtime,
        &request,
    ));

    let character = world.characters.get(&CharacterId(200)).unwrap();
    assert_eq!(character.name, "Quest Guard");
    assert_eq!(character.description, "A live quest guard.");
    assert_eq!(character.driver, CDR_LQNPC);
    assert_eq!((character.x, character.y), (12, 13));
    assert_eq!((character.rest_x, character.rest_y), (12, 13));
    assert_eq!(character.level, 17);
    assert_eq!(character.exp, 102_690);
    assert_eq!(character.exp_used, 102_690);
    assert_eq!(character.hp, 25 * POWERSCALE);
    assert_eq!(character.endurance, 24 * POWERSCALE);
    assert_eq!(character.mana, 23 * POWERSCALE);
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 25);
    assert_eq!(character.values[1][CharacterValue::Endurance as usize], 24);
    assert_eq!(character.values[1][CharacterValue::Mana as usize], 23);
    assert_eq!(character.values[1][CharacterValue::Dagger as usize], 24);
    assert_eq!(character.values[1][CharacterValue::Attack as usize], 22);
    assert_eq!(character.values[1][CharacterValue::Warcry as usize], 21);
    assert_eq!(character.values[1][CharacterValue::Bless as usize], 23);
    assert_eq!(character.values[1][CharacterValue::Fireball as usize], 20);
    assert_eq!(
        character.values[1][CharacterValue::MagicShield as usize],
        19
    );
    assert_eq!(character.values[0][CharacterValue::Dagger as usize], 28);
    assert_eq!(character.values[0][CharacterValue::Attack as usize], 26);
    assert_eq!(character.values[0][CharacterValue::Parry as usize], 4);
    assert_eq!(character.values[0][CharacterValue::Tactics as usize], 4);
    assert_eq!(character.values[0][CharacterValue::Warcry as usize], 25);
    assert_eq!(character.values[0][CharacterValue::Bless as usize], 27);
    assert_eq!(character.values[0][CharacterValue::Fireball as usize], 24);
    assert_eq!(
        character.values[0][CharacterValue::MagicShield as usize],
        23
    );
    assert_eq!(character.values[0][CharacterValue::Immunity as usize], 4);
    assert_eq!(character.values[0][CharacterValue::Wisdom as usize], 6);
    assert_eq!(
        character.values[0][CharacterValue::Intelligence as usize],
        6
    );
    assert_eq!(character.inventory[12], Some(ItemId(1)));
    assert_eq!(character.inventory[13], Some(ItemId(2)));
    assert_eq!(character.inventory[14], Some(ItemId(3)));
    assert_eq!(character.inventory[worn_slot::RIGHT_HAND], Some(ItemId(4)));
    assert!(character
        .flags
        .contains(CharacterFlags::IMMORTAL | CharacterFlags::NOATTACK));
    let warrior_spell = world.items.get(&ItemId(1)).unwrap();
    assert_eq!(warrior_spell.name, "LQX Spell");
    assert_eq!(warrior_spell.carried_by, Some(CharacterId(200)));
    assert_eq!(warrior_spell.modifier_index, [12, 18, 19, 21, 20]);
    assert_eq!(warrior_spell.modifier_value, [4, 4, 4, 4, 4]);
    let mage_spell = world.items.get(&ItemId(2)).unwrap();
    assert_eq!(mage_spell.modifier_index[0..3], [28, 33, 31]);
    assert_eq!(mage_spell.modifier_value[0..3], [4, 4, 4]);
    let misc_spell = world.items.get(&ItemId(3)).unwrap();
    assert_eq!(misc_spell.modifier_index, [37, 3, 4, 5, 6]);
    assert_eq!(misc_spell.modifier_value, [4, 6, 6, 6, 6]);
    let weapon = world.items.get(&ItemId(4)).unwrap();
    assert_eq!(weapon.name, "Quest Dagger");
    assert_eq!(weapon.carried_by, Some(CharacterId(200)));
    let npc = world.lq_npcs.iter().find(|npc| npc.slot == 2).unwrap();
    assert_eq!(npc.character_id, Some(CharacterId(200)));
    assert_eq!(npc.character_serial, character.serial);
}

#[test]
fn lq_equipment_creates_legacy_weapon_and_armor_slots() {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                lq_equipment_test:
                  name="LQ Equipment Test"
                ;
            "#,
        )
        .unwrap();
    loader
        .load_item_templates_str(
            r#"
                sword4q1: name="Sword Four" ;
                twohand4q1: name="Twohand Four" ;
                helmet3q1: name="Helmet Three" ;
                armor3q1: name="Armor Three" ;
                leggings3q1: name="Leggings Three" ;
                sleeves3q1: name="Sleeves Three" ;
            "#,
        )
        .unwrap();
    let (mut character, mut inventory_items) = loader
        .instantiate_character_template("lq_equipment_test", CharacterId(300))
        .unwrap();
    character.values[1][CharacterValue::Sword as usize] = 31;
    character.values[1][CharacterValue::TwoHand as usize] = 34;
    character.values[1][CharacterValue::ArmorSkill as usize] = 29;

    add_lq_equipment_items(&mut character, &mut loader, &mut inventory_items);

    assert_eq!(character.inventory[worn_slot::RIGHT_HAND], Some(ItemId(1)));
    assert_eq!(character.inventory[worn_slot::HEAD], Some(ItemId(2)));
    assert_eq!(character.inventory[worn_slot::BODY], Some(ItemId(3)));
    assert_eq!(character.inventory[worn_slot::LEGS], Some(ItemId(4)));
    assert_eq!(character.inventory[worn_slot::ARMS], Some(ItemId(5)));
    assert_eq!(inventory_items[0].name, "Twohand Four");
    assert_eq!(inventory_items[1].name, "Helmet Three");
    assert_eq!(inventory_items[2].name, "Armor Three");
    assert_eq!(inventory_items[3].name, "Leggings Three");
    assert_eq!(inventory_items[4].name, "Sleeves Three");
    assert!(inventory_items
        .iter()
        .all(|item| item.carried_by == Some(CharacterId(300))));
}

#[test]
fn teufel_ratnest_spawn_result_stores_slot_serial_and_increases_wave() {
    let mut world = World::default();
    let mut nest = test_item(
        ItemId(10),
        15281,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK,
    );
    nest.driver_data = vec![5, 0];
    world.add_item(nest);

    assert!(apply_teufel_ratnest_spawn_result(
        &mut world,
        ItemId(10),
        2,
        CharacterId(77),
        0x1122_3344,
        true,
    ));

    let nest = &world.items[&ItemId(10)];
    assert_eq!(
        u16::from_le_bytes([nest.driver_data[0], nest.driver_data[1]]),
        15
    );
    assert_eq!(
        u16::from_le_bytes([nest.driver_data[14], nest.driver_data[15]]),
        77
    );
    assert_eq!(
        u32::from_le_bytes([
            nest.driver_data[28],
            nest.driver_data[29],
            nest.driver_data[30],
            nest.driver_data[31],
        ]),
        0x1122_3344
    );
}

#[test]
fn teufel_ratnest_spawn_uses_item_drop_char_order_and_actual_rest_tile() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    for (x, y) in [(10, 10), (11, 10), (10, 11), (11, 11)] {
        world.map.set_flags(x, y, MapFlags::MOVEBLOCK);
    }
    let mut nest = test_item(
        ItemId(10),
        15281,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK,
    );
    nest.x = 10;
    nest.y = 10;
    nest.driver_data = vec![0; 40];
    world.add_item(nest);

    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                rat70:
                  name="Ice Rat"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                ;
            "#,
        )
        .unwrap();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(70);

    assert!(spawn_teufel_ratnest_character(
        &mut world,
        &mut loader,
        &mut runtime,
        ItemId(10),
        45,
        "rat70",
    ));

    let rat = world.characters.get(&CharacterId(70)).unwrap();
    assert_eq!((rat.x, rat.y), (9, 10));
    assert_eq!((rat.rest_x, rat.rest_y), (9, 10));
}

#[test]
fn teufel_ratnest_random_suffix_adds_legacy_stat_and_text() {
    let cases = [
        (
            0,
            CharacterValue::Attack,
            "Ice Rat *A",
            " Increased Attack.",
        ),
        (1, CharacterValue::Parry, "Ice Rat *P", " Increased Parry."),
        (
            2,
            CharacterValue::Freeze,
            "Ice Rat *R",
            " Increased Freeze.",
        ),
        (3, CharacterValue::Flash, "Ice Rat *F", " Increased Flash."),
        (
            4,
            CharacterValue::Immunity,
            "Ice Rat *I",
            " Increased Immunity.",
        ),
    ];

    for (roll, value, name, description) in cases {
        let mut rat = login_character(CharacterId(70), &login_block("Ice Rat"), 34, 10, 10);
        rat.flags.remove(CharacterFlags::UPDATE);
        let mut rolls = [roll, 9].into_iter();

        apply_teufel_ratnest_random_suffix(&mut rat, |_| rolls.next().unwrap());

        assert_eq!(rat.values[1][value as usize], 16);
        assert_eq!(rat.name, name);
        assert_eq!(rat.description, description);
        assert!(rat.flags.contains(CharacterFlags::UPDATE));
    }
}

#[test]
fn teufel_ratnest_random_suffix_noops_for_default_rolls() {
    let mut rat = login_character(CharacterId(70), &login_block("Ice Rat"), 34, 10, 10);
    rat.flags.remove(CharacterFlags::UPDATE);

    apply_teufel_ratnest_random_suffix(&mut rat, |_| 5);

    assert_eq!(rat.name, "Ice Rat");
    assert!(rat.description.is_empty());
    assert!(!rat.flags.contains(CharacterFlags::UPDATE));
}
