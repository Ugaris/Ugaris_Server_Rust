use super::*;
use crate::map::MapGrid;

#[test]
fn parses_legacy_record_syntax() {
    let records = parse_zone_records(
        r#"
            # comments are ignored
        // C++ style comments are also ignored
            Torch:
              name="Training Torch"
              flag=IF_TAKE
              flag=IF_MOVEBLOCK
              mod_index=V_LIGHT
              mod_value=5
              ID=1A
            ;
            "#,
    )
    .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key, "Torch");
    assert!(records[0]
        .fields
        .contains(&("flag".to_string(), "IF_MOVEBLOCK".to_string())));
}

#[test]
fn parses_map_directives_with_origin_offsets() {
    let directives = parse_map_directives(
        r#"
            origin="10,20"
            field="1,2"
            gsprite=100
            from="3,4"
            to="4,4"
            flag=MF_INDOORS
            "#,
    )
    .unwrap();

    assert!(directives.contains(&MapDirective::Field { x: 11, y: 22 }));
    assert!(directives.contains(&MapDirective::From { x: 13, y: 24 }));
    assert!(directives.contains(&MapDirective::To { x: 14, y: 24 }));
    assert!(directives.contains(&MapDirective::Flag(MapFlags::INDOORS)));
}

#[test]
fn parses_negative_legacy_sprite_values_as_u32_bits() {
    let directives = parse_map_directives(
        r#"
            field="1,1"
            gsprite=-420589820
            fsprite=-1
            "#,
    )
    .unwrap();

    assert!(directives.contains(&MapDirective::GroundSprite((-420589820_i32) as u32)));
    assert!(directives.contains(&MapDirective::ForegroundSprite(u32::MAX)));
}

#[test]
fn map_application_keeps_terrain_when_item_template_is_missing() {
    let mut loader = ZoneLoader::new();
    let mut world = World::default();

    loader
        .apply_map_str(
            &mut world,
            r#"
                field="5,6"
                gsprite=123
                it=missing_item_template
                "#,
        )
        .unwrap();

    let tile = world.map.tile(5, 6).unwrap();
    assert_eq!(tile.ground_sprite, 123);
    assert_eq!(tile.item, 0);
}

#[test]
fn range_copy_does_not_duplicate_dynamic_item_or_character_ids() {
    let items = r#"
            Door:
              name="Door"
              sprite=42
              flag=IF_MOVEBLOCK
              flag=IF_SIGHTBLOCK
              flag=IF_DOOR
            ;
        "#;
    let chars = r#"
            Guard:
              name="Guard"
              V_HP=10
            ;
        "#;
    let map = r#"
            field="5,5"
            gsprite=100
            it=Door
            ch=Guard
            from="5,5"
            to="6,5"
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_item_templates_str(items).unwrap();
    loader.load_character_templates_str(chars).unwrap();
    let mut world = World::default();
    loader.apply_map_str(&mut world, map).unwrap();

    let original = world.map.tile(5, 5).unwrap();
    assert_eq!(original.item, 1);
    assert_eq!(original.character, 1);

    let copied = world.map.tile(6, 5).unwrap();
    assert_eq!(copied.ground_sprite, 100);
    assert_eq!(copied.item, 0);
    assert_eq!(copied.character, 0);
    assert!(!copied
        .flags
        .intersects(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));
}

#[test]
fn applies_tiny_zone_to_world() {
    let items = r#"
            Torch:
              name="Training Torch"
              sprite=42
              flag=IF_MOVEBLOCK
              flag=IF_SIGHTBLOCK
              mod_index=V_LIGHT
              mod_value=7
            ;
        "#;
    let chars = r#"
            Guard:
              name="Practice Guard"
              flag=CF_RESPAWN
              flag=CF_NOBODY
              driver=7
              arg="aggressive=1; startdist=8; drinkinvpots=1;"
              V_HP=10
              P_ATHLETE=3
              WN_RHAND=Torch
              item=Torch
              spell=Torch
            ;
        "#;
    let map = r#"
            origin="10,20"
            field="1,2"
            gsprite=100
            fsprite=101
            flag=MF_INDOORS
            it=Torch
            ch=Guard
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_item_templates_str(items).unwrap();
    loader.load_character_templates_str(chars).unwrap();

    let mut world = World::default();
    world.map = MapGrid::new(32, 32);
    loader.apply_map_str(&mut world, map).unwrap();

    let tile = world.map.tile(11, 22).unwrap();
    assert_eq!(tile.ground_sprite, 100);
    assert_eq!(tile.foreground_sprite, 101);
    assert_eq!(tile.item, 1);
    assert_eq!(tile.character, 1);
    assert!(tile.flags.contains(MapFlags::INDOORS));
    assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
    assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.name, "Practice Guard");
    assert_eq!(character.x, 11);
    assert_eq!(character.y, 22);
    assert_eq!(character.values[1][0], 10);
    assert_eq!(character.professions[0], 3);
    assert_eq!(character.driver, CDR_SIMPLEBADDY);
    assert_eq!(character.inventory[6], Some(ItemId(2)));
    assert_eq!(character.inventory[12], Some(ItemId(3)));
    assert_eq!(character.inventory[30], Some(ItemId(4)));
    assert!(!character.flags.contains(CharacterFlags::NOBODY));
    assert!(character.flags.contains(CharacterFlags::ITEMDEATH));
    assert!(character.driver_messages.is_empty());
    let Some(crate::character_driver::CharacterDriverState::SimpleBaddy(data)) =
        &character.driver_state
    else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.startdist, 8);
    assert_eq!(data.drink_inventory_potions, 1);

    assert_eq!(world.items.get(&ItemId(1)).unwrap().x, 11);
    assert_eq!(
        world.items.get(&ItemId(2)).unwrap().carried_by,
        Some(CharacterId(1))
    );
}

#[test]
fn lab2_undead_template_installs_regenerate_spell_for_undead() {
    let items = r#"
            lab2_regenerate_spell:
              name="lab2_regenerate_spell"
              driver=194
              arg="180800000000000000000000"
            ;
        "#;
    let chars = r#"
            Undead:
              name="Lab Undead"
              driver=198
              arg="undead=1; patrol=1;"
              V_HP=10
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_item_templates_str(items).unwrap();
    loader.load_character_templates_str(chars).unwrap();

    let (character, inventory_items) = loader
        .instantiate_character_template("Undead", CharacterId(7))
        .unwrap();

    assert!(character.flags.contains(CharacterFlags::NODEATH));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    let Some(CharacterDriverState::Lab2Undead(data)) = &character.driver_state else {
        panic!("lab2 undead state missing");
    };
    let regen_item_id = data.regenerate_item_id.expect("regenerate item id");
    assert_eq!(character.inventory[29], Some(regen_item_id));

    let regen = inventory_items
        .iter()
        .find(|item| item.id == regen_item_id)
        .expect("regenerate item");
    assert_eq!(regen.driver, IDR_LAB2_REGENERATE);
    assert_eq!(regen.carried_by, Some(CharacterId(7)));
    assert_eq!(&regen.driver_data[4..8], &7_u32.to_le_bytes());
    assert_eq!(data.undead, 1);
    assert_eq!(data.patstep, 4);
}

#[test]
fn dungeonfighter_template_installs_simple_baddy_state_from_arg_and_own_data_field() {
    // Mirrors `zones/13/dungeon.chr`'s real "warrior"/"mage"/"seyan"
    // entries: `driver=52` (`CDR_DUNGEONFIGHTER`) with a SimpleBaddy-
    // style `arg=` string that `dungeonfighter` itself never reads but
    // its own tail `char_driver(CDR_SIMPLEBADDY, ...)` call does.
    let chars = r#"
            warrior:
              name="Warrior"
              driver=52
              arg="aggressive=1;helper=0;scavenger=0;startdist=40;chardist=0;stopdist=80;"
              V_HP=10
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("warrior", CharacterId(9))
        .unwrap();

    assert_eq!(
        character.driver,
        crate::character_driver::CDR_DUNGEONFIGHTER
    );
    assert!(character.driver_messages.is_empty());
    assert!(character.dungeonfighter.is_some());
    let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.startdist, 40);
    assert_eq!(data.stopdist, 80);
}

#[test]
fn swampmonster_template_installs_simple_baddy_state_from_arg() {
    // Mirrors `zones/15/swamp.chr`'s real `swamp25n`/`swamp27n`/
    // `swamp29n`/`swamp31n` entries: `driver=56` (`CDR_SWAMPMONSTER`)
    // with a SimpleBaddy-style `arg=` string that `swamp_monster`'s
    // own tail `char_driver(CDR_SIMPLEBADDY, ...)` call reads.
    let chars = r#"
            swamp25n:
              name="Swamp Beastling"
              driver=56
              arg="aggressive=1;helper=0;scavenger=0;startdist=40;chardist=0;stopdist=60;"
              V_HP=12
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("swamp25n", CharacterId(9))
        .unwrap();

    assert_eq!(character.driver, crate::character_driver::CDR_SWAMPMONSTER);
    assert!(character.driver_messages.is_empty());
    let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.startdist, 40);
    assert_eq!(data.stopdist, 60);
}

#[test]
fn clara_template_installs_default_clara_driver_state() {
    // C never parses zone-file args into `struct clara_driver_data`
    // (`set_data` zero-initializes it) - no args to read for
    // `CDR_SWAMPCLARA` (`driver=54`).
    let chars = r#"
            clara:
              name="Clara"
              driver=54
              V_HP=100
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("clara", CharacterId(9))
        .unwrap();

    assert_eq!(character.driver, crate::character_driver::CDR_SWAMPCLARA);
    assert_eq!(
        character.driver_state,
        Some(CharacterDriverState::Clara(ClaraDriverData::default()))
    );
}

#[test]
fn fdemon_big1_sprite_190_spawns_as_plain_simplebaddy() {
    // Mirrors `zones/8/fire.chr`'s real `fdemon_big1` entry: `driver=46`
    // (`CDR_FDEMON_DEMON`) but `sprite=190` - C's own `fdemon_demon`
    // unconditionally tail-calls `char_driver(CDR_SIMPLEBADDY, ...)`
    // for this sprite every tick, so this port assigns
    // `CDR_SIMPLEBADDY` directly at spawn (see the `CDR_FDEMON_DEMON`
    // branch above).
    let chars = r#"
            fdemon_big1:
              name="Fire Golem"
              driver=46
              sprite=190
              arg="aggressive=1;helper=0;scavenger=0;startdist=20;chardist=0;stopdist=40;"
              V_HP=35
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("fdemon_big1", CharacterId(10))
        .unwrap();

    assert_eq!(character.driver, CDR_SIMPLEBADDY);
    assert!(character.driver_messages.is_empty());
    let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.startdist, 20);
    assert_eq!(data.stopdist, 40);
}

#[test]
fn fdemon_trash_mob_installs_fixed_distances_ignoring_zone_file_args() {
    // Mirrors `zones/8/fire.chr`'s real `fdemon1s` entry: `driver=46`,
    // `sprite=157` (not 190), with its `arg=` commented out in the real
    // data since C's own `fdemon_demon` `NT_CREATE` handler never
    // parses `ch[cn].arg` and hardcodes `fight_driver_set_dist(cn, 0,
    // 30, 0)` plus `standard_message_driver(cn, msg, 1, 1)` instead.
    let chars = r#"
            fdemon1s:
              name="Fire Demon"
              driver=46
              sprite=157
              arg="aggressive=0;helper=0;scavenger=20;startdist=99;chardist=99;stopdist=99;"
              V_HP=35
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("fdemon1s", CharacterId(11))
        .unwrap();

    assert_eq!(character.driver, crate::character_driver::CDR_FDEMON_DEMON);
    assert!(character.driver_messages.is_empty());
    let fight_driver = character.fight_driver.expect("fight driver data");
    assert_eq!(fight_driver.start_dist, 0);
    assert_eq!(fight_driver.char_dist, 30);
    assert_eq!(fight_driver.stop_dist, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.helper, 1);
    // The zone-file `arg=` string above must be ignored entirely.
    assert_ne!(data.scavenger, 20);
}

#[test]
fn zone_population_rolls_spawn_mode_loot_table_into_new_npcs_own_inventory() {
    // C `create.c:1121-1125`: `if (ch_temp[ctmp].loot_table[0])
    // loot_apply_to_npc(n, ch_temp[ctmp].loot_table);`, run for every
    // NPC `pop_create_char` places while loading a zone's map.
    let items = r#"
            bronzechip:
              name="Bronze Chip"
              flag=IF_TAKE
            ;
        "#;
    let chars = r#"
            Guard:
              name="Practice Guard"
              loot_table="guard_spawn_loot"
              V_HP=10
            ;
        "#;
    let map = r#"
            field="5,5"
            ch=Guard
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_item_templates_str(items).unwrap();
    loader.load_character_templates_str(chars).unwrap();
    let mut world = World::default();
    world.loot_registry.load_str(
        r#"{
                "id": "guard_spawn_loot",
                "rolls": 1,
                "entries": [{"weight": 1, "item": "bronzechip"}]
            }"#,
    );
    world.legacy_random_seed = 0;
    loader.apply_map_str(&mut world, map).unwrap();

    let npc = world
        .characters
        .get(&CharacterId(1))
        .expect("guard spawned");
    assert_eq!(npc.name, "Practice Guard");
    // Slots 0-29 (worn/spells) stay empty; the rolled item lands at the
    // first free carried slot (30).
    assert!(npc.inventory[..30].iter().all(Option::is_none));
    let carried_id = npc.inventory[30].expect("loot item placed at slot 30");
    let carried_item = world.items.get(&carried_id).expect("item exists");
    assert_eq!(carried_item.name, "Bronze Chip");
    assert_eq!(carried_item.carried_by, Some(CharacterId(1)));
}

#[test]
fn teufelgambler_template_parses_nr_from_zone_arg() {
    // Mirrors `zones/34/teufel.chr`'s real `gambler`/`gambler2`/
    // `gambler3` entries (`:750-946`): `driver=115`
    // (`CDR_TEUFELGAMBLER`) with `arg="1"`/`"2"`/`"3"` parsed into
    // `dat->nr` at spawn time (`teufel.c:1248-1251`).
    let chars = r#"
            gambler2:
              name="Demon Gambler"
              driver=115
              arg="2"
              V_HP=120
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("gambler2", CharacterId(9))
        .unwrap();

    assert_eq!(character.driver, crate::character_driver::CDR_TEUFELGAMBLER);
    let Some(CharacterDriverState::TeufelGambler(data)) = &character.driver_state else {
        panic!("teufel gambler state missing");
    };
    assert_eq!(data.nr, 2);
    assert_eq!(data.memcleartimer, 0);
}

#[test]
fn teufelrat_template_installs_simple_baddy_state_from_arg() {
    // Mirrors `zones/34/teufel.chr`'s real `rat80`/`rat90`/`rat70`
    // family (`driver=117`, `CDR_TEUFELRAT`): `teufelrat_driver`'s
    // own `NT_CHAR` case is an empty no-op, so this is a pure
    // unconditional tail call to `char_driver(CDR_SIMPLEBADDY, ...)`
    // (`teufel.c:1610-1626`) - same precedent as `CDR_TEUFELDEMON`/
    // `CDR_TWOROBBER` above.
    let chars = r#"
            rat80:
              name="Baby Ice Rat"
              driver=117
              arg="aggressive=1;helper=1;scavenger=10;startdist=15;chardist=0;stopdist=40;"
              V_HP=60
            ;
        "#;

    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(chars).unwrap();

    let (character, _inventory_items) = loader
        .instantiate_character_template("rat80", CharacterId(9))
        .unwrap();

    assert_eq!(character.driver, crate::character_driver::CDR_TEUFELRAT);
    assert!(character.driver_messages.is_empty());
    let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.helper, 1);
    assert_eq!(data.scavenger, 10);
    assert_eq!(data.startdist, 15);
    assert_eq!(data.stopdist, 40);
}
