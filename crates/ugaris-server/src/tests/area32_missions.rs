use super::*;
use ugaris_core::character_driver::CDR_SIMPLEBADDY;
use ugaris_core::entity::CharacterValue;
use ugaris_core::world::npc::area32::mission_start::FighterSpawnSpec;

const MISSION_CHR: &str = r#"
    mis_warrior:
      name="Replace Me"
      description="Replace Me"
      sprite=299
      flag=CF_INFRARED
      V_HP=10
      V_ENDURANCE=10
      V_MANA=0
      V_HAND=1
      V_ATTACK=1
      V_ARMORSKILL=1
      driver=112
      arg="aggressive=1;helper=0;scavenger=0;startdist=20;chardist=0;stopdist=80;"
    ;
"#;

const MISSION_ITM: &str = r#"
    mis_key: name="Key" ;
    armor_spell: name="Armor Spell" ;
    weapon_spell: name="Weapon Spell" ;
"#;

fn mission_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(MISSION_CHR).unwrap();
    loader.load_item_templates_str(MISSION_ITM).unwrap();
    loader
}

// C `build_fighter` (`missions.c:678-865`): a keyholder fighter (fID=2,
// key_id set) gets the raisable-skill rescale, the `mis_key` item stamped
// with the instance key id, and `armor_spell`/`weapon_spell` items scaled
// from its own `V_ARMORSKILL`/`V_HAND`.
#[test]
fn spawn_mission_fighter_scales_stats_and_attaches_key_and_spell_items() {
    let mut world = World::default();
    let mut loader = mission_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);

    let spec = FighterSpawnSpec {
        x: 20,
        y: 21,
        diff: 42,
        key_id: 0x0900_0003,
        key_name: "Door Key I",
        name: "Thief Apprentice".to_string(),
        temp: "mis_warrior",
        desc: "A thief belonging to the famous gang 'The Pickers'.",
        fighter_kind: 2,
        sprite: 312,
        has_special_item: false,
        extra_flags: CharacterFlags::ALIVE,
    };

    assert!(spawn_mission_fighter(
        &mut world,
        &mut loader,
        &mut runtime,
        &spec
    ));

    let fighter = world.characters.get(&CharacterId(50)).unwrap();
    assert_eq!(fighter.driver, CDR_SIMPLEBADDY);
    assert_eq!(fighter.name, "Thief Apprentice");
    assert_eq!(
        fighter.description,
        "A thief belonging to the famous gang 'The Pickers'."
    );
    assert_eq!((fighter.x, fighter.y), (20, 21));
    assert_eq!(fighter.sprite, 312);
    assert_eq!(fighter.deaths, 2);
    assert!(fighter.flags.contains(CharacterFlags::ALIVE));
    // `V_HAND`/`V_ATTACK`: `max(1, diff)` = 42.
    assert_eq!(fighter.values[1][CharacterValue::Hand as usize], 42);
    assert_eq!(fighter.values[1][CharacterValue::Attack as usize], 42);
    // `V_ARMORSKILL`: `max(1, (diff/10)*10)` = 40.
    assert_eq!(fighter.values[1][CharacterValue::ArmorSkill as usize], 40);
    // `V_HP`/`V_ENDURANCE`: `max(10, diff-15)` = 27.
    assert_eq!(fighter.values[1][CharacterValue::Hp as usize], 27);
    assert!(fighter.hp > 0);

    let key_item_id = fighter.inventory[30].expect("mis_key expected in slot 30");
    let key_item = world.items.get(&key_item_id).unwrap();
    assert_eq!(key_item.template_id, 0x0900_0003);
    assert_eq!(key_item.name, "Door Key I");

    let armor_item_id = fighter.inventory[14].expect("armor_spell expected in slot 14");
    let armor_item = world.items.get(&armor_item_id).unwrap();
    // `max(13, min(113, 40)) * 20` = 800.
    assert_eq!(armor_item.modifier_value[0], 800);

    let weapon_item_id = fighter.inventory[15].expect("weapon_spell expected in slot 15");
    let weapon_item = world.items.get(&weapon_item_id).unwrap();
    // `max(13, min(113, 42))` = 42.
    assert_eq!(weapon_item.modifier_value[0], 42);

    // No special item requested.
    assert!(fighter.inventory[31].is_none());
}

// C `build_fighter`'s no-key branch (`keyID` param `0`): fighters that
// don't carry a key skip the `mis_key` item entirely.
#[test]
fn spawn_mission_fighter_without_key_skips_key_item() {
    let mut world = World::default();
    let mut loader = mission_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(60);

    let spec = FighterSpawnSpec {
        x: 5,
        y: 5,
        diff: 42,
        key_id: 0,
        key_name: "",
        name: "Thief Apprentice".to_string(),
        temp: "mis_warrior",
        desc: "desc",
        fighter_kind: 1,
        sprite: 312,
        has_special_item: false,
        extra_flags: CharacterFlags::ALIVE,
    };

    assert!(spawn_mission_fighter(
        &mut world,
        &mut loader,
        &mut runtime,
        &spec
    ));
    let fighter = world.characters.get(&CharacterId(60)).unwrap();
    assert!(fighter.inventory[30].is_none());
}
