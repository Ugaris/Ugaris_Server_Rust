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

fn gatekeeper_templates() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                gatekeeper_w:
                  name="Gatekeeper"
                  V_HP=50
                  V_ENDURANCE=50
                  V_MANA=0
                ;
                gatekeeper_m:
                  name="Gatekeeper"
                  V_HP=40
                  V_ENDURANCE=40
                  V_MANA=60
                ;
                gatekeeper_s:
                  name="Gatekeeper"
                  V_HP=45
                  V_ENDURANCE=45
                  V_MANA=30
                ;
            "#,
        )
        .unwrap();
    loader
}

fn gate_test_player(character_id: CharacterId) -> Character {
    // Spawned well clear of every `GATE_TEST_ROOM_STARTS` candidate room so
    // the player's own tile never fails `gate_room_is_clear`.
    let mut player = login_character(character_id, &login_block("Godmode"), 3, 100, 100);
    player.gold = 20000;
    for slot in 12..30 {
        player.inventory[slot] = Some(ItemId(999));
    }
    player
}

/// C `enter_test`/`enter_room` (`gatekeeper.c:227-407`)'s success path:
/// the first empty room spawns the class-appropriate opponent, teleports
/// and resets the player, and takes the 100G fee.
#[test]
fn gate_enter_test_spawn_room_success_spawns_opponent_and_resets_player() {
    let mut world = World::default();
    let mut loader = gatekeeper_templates();
    world
        .items
        .insert(ItemId(999), test_item(ItemId(999), 1, ItemFlags::empty()));
    let player = gate_test_player(CharacterId(2));
    assert!(world.spawn_character(player, 100, 100));

    let mut runtime = ServerRuntime::default();
    let mut player_runtime = PlayerRuntime::connected(1, 0);
    player_runtime.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player_runtime);
    runtime.set_next_character_id(50);

    assert!(gate_enter_test_spawn_room(
        &mut world,
        &mut loader,
        &mut runtime,
        CharacterId(2),
        5,
    ));

    // First candidate room (186, 196): opponent at (190, 209), player
    // door tile at (190, 200) (`gatekeeper.c:271,285`).
    let opponent = world.characters.get(&CharacterId(50)).unwrap();
    assert_eq!(opponent.name, "Gatekeeper");
    assert_eq!((opponent.x, opponent.y), (190, 209));
    assert_eq!((opponent.rest_x, opponent.rest_y), (190, 209));
    assert_eq!(opponent.dir, Direction::RightDown as u8);
    assert_eq!(opponent.hp, 50 * POWERSCALE);
    assert_eq!(opponent.driver_messages.len(), 1);
    assert_eq!(opponent.driver_messages[0].message_type, NT_NPC);
    assert_eq!(opponent.driver_messages[0].dat1, NTID_GATEKEEPER);
    assert_eq!(opponent.driver_messages[0].dat2, 2);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((player.x, player.y), (190, 200));
    assert_eq!(player.gold, 10000);
    assert_eq!(player.hp, POWERSCALE);
    assert_eq!(player.endurance, POWERSCALE);
    assert_eq!(player.mana, POWERSCALE);
    assert!(player.inventory[12..30].iter().all(Option::is_none));
    assert!(world.items.get(&ItemId(999)).is_none());

    let system_texts = world.drain_pending_system_texts();
    assert!(system_texts
        .iter()
        .any(|entry| entry.message.contains("All your spells have been removed.")));
    assert!(system_texts
        .iter()
        .any(|entry| entry.message.contains("ten minutes from now on")));

    let stored = runtime.player_for_character_mut(CharacterId(2)).unwrap();
    assert_eq!(stored.gate_target_class, 5);
    assert_eq!(stored.gate_step, 1);
}

/// C `enter_test`'s room-busy refund path (`gatekeeper.c:400-405`): when
/// every candidate room is occupied, the fee is refunded and no opponent
/// is spawned.
#[test]
fn gate_enter_test_spawn_room_refunds_when_every_room_is_busy() {
    let mut world = World::default();
    let mut loader = gatekeeper_templates();
    let player = gate_test_player(CharacterId(2));
    assert!(world.spawn_character(player, 100, 100));

    let mut blocker_id = 900_u32;
    for (xs, ys) in GATE_TEST_ROOM_STARTS {
        let blocker = login_character(
            CharacterId(blocker_id),
            &login_block("Blocker"),
            3,
            usize::from(xs),
            usize::from(ys),
        );
        assert!(world.spawn_character(blocker, usize::from(xs), usize::from(ys)));
        blocker_id += 1;
    }

    let mut runtime = ServerRuntime::default();
    let mut player_runtime = PlayerRuntime::connected(1, 0);
    player_runtime.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player_runtime);
    runtime.set_next_character_id(50);

    assert!(!gate_enter_test_spawn_room(
        &mut world,
        &mut loader,
        &mut runtime,
        CharacterId(2),
        5,
    ));

    assert!(world.characters.get(&CharacterId(50)).is_none());
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.gold, 20000);
    let system_texts = world.drain_pending_system_texts();
    assert!(system_texts.iter().any(|entry| entry
        .message
        .contains("Sorry, the gatekeeper is busy at the moment. Please come back later.")));
}

/// C `enter_test`'s `take_money` guard (`gatekeeper.c:392-395`): an
/// underfunded player is rejected before any room is searched.
#[test]
fn gate_enter_test_spawn_room_rejects_when_underfunded() {
    let mut world = World::default();
    let mut loader = gatekeeper_templates();
    let mut player = gate_test_player(CharacterId(2));
    player.gold = 9999;
    assert!(world.spawn_character(player, 100, 100));

    let mut runtime = ServerRuntime::default();
    let mut player_runtime = PlayerRuntime::connected(1, 0);
    player_runtime.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player_runtime);
    runtime.set_next_character_id(50);

    assert!(!gate_enter_test_spawn_room(
        &mut world,
        &mut loader,
        &mut runtime,
        CharacterId(2),
        5,
    ));

    assert!(world.characters.get(&CharacterId(50)).is_none());
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.gold, 9999);
    let system_texts = world.drain_pending_system_texts();
    assert!(system_texts
        .iter()
        .any(|entry| entry.message.contains("Thou canst pay the price of 100G.")));
}

/// C `create.c:1121-1125`'s `loot_apply_to_npc` runs inside `create_char_nr`
/// for every character creation, including `respawn_callback`'s recreate-
/// from-template path - not just the original zone-load `pop_create_char`.
#[test]
fn respawn_npc_character_rolls_its_templates_spawn_mode_loot_table() {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                looter:
                  name="Looter"
                  loot_table="looter_spawn_loot"
                  V_HP=10
                ;
            "#,
        )
        .unwrap();
    loader
        .load_item_templates_str(
            r#"
                bronzechip:
                  name="Bronze Chip"
                  flag=IF_TAKE
                ;
            "#,
        )
        .unwrap();

    let mut world = World::default();
    world.loot_registry.load_str(
        r#"{
            "id": "looter_spawn_loot",
            "rolls": 1,
            "entries": [{"weight": 1, "item": "bronzechip"}]
        }"#,
    );
    world.legacy_random_seed = 0;

    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(300);
    let request = ugaris_core::world::NpcRespawnRequest {
        slot: 0,
        template_key: "looter".to_string(),
        x: 20,
        y: 20,
    };

    assert!(respawn_npc_character(
        &mut world,
        &mut loader,
        &mut runtime,
        &request,
    ));

    let npc = world.characters.get(&CharacterId(300)).unwrap();
    assert_eq!(npc.name, "Looter");
    let carried_id = npc.inventory[30].expect("loot item placed at first carried slot");
    let carried_item = world.items.get(&carried_id).unwrap();
    assert_eq!(carried_item.name, "Bronze Chip");
    assert_eq!(carried_item.carried_by, Some(CharacterId(300)));
}

fn lampghost_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                lampghost:
                  name="Lamp Ghost"
                  driver=25
                  V_HP=10
                ;
            "#,
        )
        .unwrap();
    loader
}

/// C `ch_respawn_driver`'s `CDR_LAMPGHOST` case -> `lampghost_respawn`
/// (`area3.c:2729-2739`): `if (map[m].light > 4) return 2;` blocks the
/// respawn (and, via `respawn_callback`'s `== 1` check, gets retried) while
/// the target tile is still lit.
#[test]
fn respawn_npc_character_refuses_lampghost_while_palace_is_lit() {
    let mut loader = lampghost_loader();
    let mut world = World::default();
    world.map.tile_mut(20, 20).unwrap().light = 5;
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(300);
    let request = ugaris_core::world::NpcRespawnRequest {
        slot: 0,
        template_key: "lampghost".to_string(),
        x: 20,
        y: 20,
    };

    assert!(!respawn_npc_character(
        &mut world,
        &mut loader,
        &mut runtime,
        &request,
    ));
    assert!(world.characters.get(&CharacterId(300)).is_none());
}

#[test]
fn respawn_npc_character_allows_lampghost_once_palace_is_dark() {
    let mut loader = lampghost_loader();
    let mut world = World::default();
    world.map.tile_mut(20, 20).unwrap().light = 4;
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(300);
    let request = ugaris_core::world::NpcRespawnRequest {
        slot: 0,
        template_key: "lampghost".to_string(),
        x: 20,
        y: 20,
    };

    assert!(respawn_npc_character(
        &mut world,
        &mut loader,
        &mut runtime,
        &request,
    ));
    let npc = world.characters.get(&CharacterId(300)).unwrap();
    assert_eq!(npc.driver, CDR_LAMPGHOST);
}
