use super::*;
use ugaris_core::character_driver::{CDR_MISSIONFIGHT, CDR_MISSIONGIVE};
use ugaris_core::entity::CharacterValue;
use ugaris_core::item_driver::IDR_MISSIONCHEST;
use ugaris_core::player::MissionPpd;
use ugaris_core::world::npc::area32::governor::{
    MissionGiveOutcomeEvent, MissionGiverDriverData, SPECIAL_OFFER_PERIOD_TICKS, SPECIAL_OFFER_SLOT,
};
use ugaris_core::world::npc::area32::mission_start::FighterSpawnSpec;
use ugaris_core::world::LegacyHurtOutcome;

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
    mis_documents: name="Documents" sprite=88 flag=IF_TAKE ;
    mis_potionbase: name="Custom Potion" sprite=10002 flag=IF_USE flag=IF_TAKE ;
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
    assert_eq!(fighter.driver, CDR_MISSIONFIGHT);
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

fn mission_fighter_npc(id: CharacterId, fighter_kind: u8) -> Character {
    let mut fighter = login_character(id, &login_block("Thief Apprentice"), 32, 5, 5);
    fighter.flags.remove(CharacterFlags::PLAYER);
    fighter.driver = CDR_MISSIONFIGHT;
    fighter.deaths = u32::from(fighter_kind);
    fighter
}

// C `mission_fighter_dead(cn, co)` (`missions.c:1852-1881`): a fighter
// kill that doesn't yet complete every objective just re-prints
// `mission_status`'s HUD lines and bumps the matching kill counter -
// `ppd->solved` stays `0`.
#[test]
fn mission_fighter_death_bumps_kill_counter_without_solving_an_unfinished_job() {
    let mut world = World::default();
    world.add_character(mission_fighter_npc(CharacterId(1), 1));
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 32, 6, 5);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.governor = MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 2],
        ..Default::default()
    };
    runtime.players.insert(1, player);

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(apply_mission_fighter_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.governor.kill_easy, [1, 2]);
    assert_eq!(player.governor.active, 1);
    assert_eq!(player.governor.solved, 0);

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "#30Stolen Documents"));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Kill 1 Thief Apprentice")));
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("finished the job")));
}

// The kill that completes every remaining objective solves the job:
// `mission_done` promotes `active` to `solved` and clears `active`
// (`missions.c:922-940`), and announces it with the killer's own name.
#[test]
fn mission_fighter_death_solves_the_job_once_every_objective_is_complete() {
    let mut world = World::default();
    world.add_character(mission_fighter_npc(CharacterId(1), 1));
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 32, 6, 5);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.governor = MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 1],
        ..Default::default()
    };
    runtime.players.insert(1, player);

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(apply_mission_fighter_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.governor.kill_easy, [1, 1]);
    assert_eq!(player.governor.active, 0);
    assert_eq!(player.governor.solved, 1);

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message
        == "You've finished the job. Good work, Godmode. Now talk to Mr. Jones for your reward."));
}

// C has no explicit "no killer" guard in `mission_fighter_dead` (unlike
// `missionchest_driver`), but a kill by a non-player NPC never reaches
// this hook at all in this port.
#[test]
fn mission_fighter_death_ignores_a_kill_by_a_non_player() {
    let mut world = World::default();
    world.add_character(mission_fighter_npc(CharacterId(1), 1));
    let mut other_npc = login_character(CharacterId(2), &login_block("Other"), 32, 6, 5);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    world.add_character(other_npc);

    let mut runtime = ServerRuntime::default();

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_mission_fighter_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));
}

// A dying character with any other driver (i.e. not a mission fighter)
// is left alone.
#[test]
fn mission_fighter_death_ignores_a_non_mission_fighter_driver() {
    let mut world = World::default();
    let mut not_a_fighter = login_character(CharacterId(1), &login_block("Bystander"), 32, 5, 5);
    not_a_fighter.flags.remove(CharacterFlags::PLAYER);
    world.add_character(not_a_fighter);
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 32, 6, 5);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_mission_fighter_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));
}

fn mission_chest_item(driver_data: Vec<u8>) -> ugaris_core::entity::Item {
    let mut chest = test_item_with_driver(ItemId(200), IDR_MISSIONCHEST);
    chest.driver_data = driver_data;
    chest
}

fn mission_player(governor: MissionPpd) -> PlayerRuntime {
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    player.governor = governor;
    player
}

// C `missionchest_driver`'s empty-chest branch (`missions.c:1806-1809`):
// `mdtab[ppd->md_idx]->itemtemp == NULL` (the beast/ruffian/vampire
// mission templates have no find-item objective).
#[test]
fn mission_chest_open_reports_empty_when_template_has_no_find_item() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        32,
        5,
        5,
    ));
    world.add_item(mission_chest_item(vec![0, 0, 0, 0, 0]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        md_idx: 2, // beast_data: itemtemp == None.
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    assert_eq!(result, MissionChestApplyResult::Empty);
}

// C `missionchest_driver`'s no-key-required happy path
// (`missions.c:1833-1846`): item is created with the template's
// `itemname`/`itemdesc`, placed on the cursor, `find_item[0]` is set, and
// `mission_status`'s HUD lines are re-printed. The job isn't solved yet
// since `kill_easy` still has an outstanding count.
#[test]
fn mission_chest_open_grants_item_and_reports_status_without_solving() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        32,
        5,
        5,
    ));
    world.add_item(mission_chest_item(vec![0, 0, 0, 0, 0]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0, // thief_data: itemtemp = "mis_documents".
        kill_easy: [0, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    match result {
        MissionChestApplyResult::Granted {
            item_name,
            key_name,
            status_lines,
            solved_message,
        } => {
            assert_eq!(item_name, "Documents");
            assert_eq!(key_name, None);
            assert!(status_lines
                .iter()
                .any(|line| line == "#30Stolen Documents"));
            assert!(status_lines
                .iter()
                .any(|line| line.contains("Kill 1 Thief Apprentice")));
            assert_eq!(solved_message, None);
        }
        other => panic!("expected Granted, got {other:?}"),
    }

    let character = world.characters.get(&CharacterId(7)).unwrap();
    let item = world.items.get(&character.cursor_item.unwrap()).unwrap();
    assert_eq!(item.name, "Documents");
    assert_eq!(item.description, "The stolen documents.");
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(player.governor.find_item, [1, 1]);
    // `active` unchanged: `kill_easy` is still outstanding.
    assert_eq!(player.governor.active, 1);
    assert_eq!(player.governor.solved, 0);
}

// C `mission_done`'s auto-solve (`missions.c:922-947`): once
// `missionchest_driver` sets `find_item[0]` and every other objective is
// already complete, `mission_done` promotes `active` to `solved` and
// prints the "finished the job" line.
#[test]
fn mission_chest_open_solves_the_job_once_it_is_the_last_objective() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Godmode"),
        32,
        5,
        5,
    ));
    world.add_item(mission_chest_item(vec![0, 0, 0, 0, 0]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [1, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    match result {
        MissionChestApplyResult::Granted { solved_message, .. } => {
            assert_eq!(
                solved_message,
                Some(
                    "You've finished the job. Good work, Godmode. Now talk to Mr. Jones for your reward."
                        .to_string()
                )
            );
        }
        other => panic!("expected Granted, got {other:?}"),
    }
    assert_eq!(player.governor.active, 0);
    assert_eq!(player.governor.solved, 1);
}

// C's key-search loop (`missions.c:1811-1824`): no matching key anywhere
// in inventory slots 30.. or on the cursor refuses the chest.
#[test]
fn mission_chest_open_requires_the_matching_key() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        32,
        5,
        5,
    ));
    world.add_item(mission_chest_item(vec![0, 0x44, 0x33, 0x22, 0x11]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    assert_eq!(result, MissionChestApplyResult::KeyRequired);
}

// The matching key, carried in inventory slot 30.., unlocks the chest
// (`missions.c:1812-1818`) and the "You use ... to unlock the chest."
// line is reported alongside the granted item.
#[test]
fn mission_chest_open_unlocks_with_a_carried_key() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 32, 5, 5);
    character.inventory[30] = Some(ItemId(30));
    world.add_character(character);
    let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
    key.template_id = 0x1122_3344;
    key.name = "Door Key I".to_string();
    world.add_item(key);
    world.add_item(mission_chest_item(vec![0, 0x44, 0x33, 0x22, 0x11]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    match result {
        MissionChestApplyResult::Granted {
            item_name,
            key_name,
            ..
        } => {
            assert_eq!(item_name, "Documents");
            assert_eq!(key_name, Some("Door Key I".to_string()));
        }
        other => panic!("expected Granted, got {other:?}"),
    }
}

// The real C quirk (`missions.c:1811-1831`): the key-search/unlock
// message runs *before* the cursor-occupied check, so if the only carried
// copy of the required key sits on the cursor itself, C still reports the
// unlock line even though the same non-empty cursor then blocks the
// reward item.
#[test]
fn mission_chest_open_reports_unlock_message_even_when_the_key_itself_blocks_the_cursor() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 32, 5, 5);
    let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
    key.template_id = 0x1122_3344;
    key.name = "Door Key I".to_string();
    character.cursor_item = Some(ItemId(30));
    world.add_character(character);
    world.add_item(key);
    world.add_item(mission_chest_item(vec![0, 0x44, 0x33, 0x22, 0x11]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    assert_eq!(
        result,
        MissionChestApplyResult::CursorOccupied {
            key_name: Some("Door Key I".to_string())
        }
    );
}

// Cursor already holding an unrelated item, no key required: the plain
// "Please empty your hand" branch, no unlock message.
#[test]
fn mission_chest_open_reports_cursor_occupied_without_key_message_when_no_key_needed() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 32, 5, 5);
    character.cursor_item = Some(ItemId(30));
    world.add_character(character);
    world.add_item(test_item(ItemId(30), 1, ItemFlags::TAKE));
    world.add_item(mission_chest_item(vec![0, 0, 0, 0, 0]));
    let mut loader = mission_loader();
    let mut player = mission_player(MissionPpd {
        active: 1,
        md_idx: 0,
        kill_easy: [0, 1],
        find_item: [0, 1],
        ..Default::default()
    });

    let result = apply_mission_chest_open(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(200),
        CharacterId(7),
    );
    assert_eq!(
        result,
        MissionChestApplyResult::CursorOccupied { key_name: None }
    );
}

// ---- special-offer regen (`regenerate_mission_giver_special_offers`) ----

/// Registers every `ITEM_TYPE_TEMPLATES` family `World::create_special_item`
/// can roll, so the regen pre-pass always succeeds regardless of which of
/// the 21 item types/10 quality tiers the RNG picks - mirrors `ugaris-core`'s
/// own `create_special_item`/`add_special_store` test helper (not exported
/// across the crate boundary, hence the duplication here).
fn special_item_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    let mut text = String::new();
    for family in [
        "armor",
        "helmet",
        "sleeves",
        "leggings",
        "sword",
        "twohanded",
        "dagger",
        "staff",
    ] {
        for tier in 1..=10 {
            text.push_str(&format!(
                "{family}{tier}q3:\nname=\"{family}{tier}q3\"\nsprite=1\nvalue=0\nflag=IF_TAKE\n;\n"
            ));
        }
    }
    for flat in [
        "plain_gold_ring",
        "green_hat",
        "brown_hat",
        "blue_cape",
        "brown_cape",
        "red_belt",
        "amulet",
        "boots",
        "vest",
        "trousers",
        "bracelet",
        "gloves",
    ] {
        text.push_str(&format!(
            "{flat}:\nname=\"{flat}\"\nsprite=1\nvalue=0\nflag=IF_TAKE\n;\n"
        ));
    }
    loader.load_item_templates_str(&text).unwrap();
    loader
}

fn governor_npc(id: CharacterId) -> Character {
    let mut giver = login_character(id, &login_block("Mister Jones"), 32, 10, 10);
    giver.flags.remove(CharacterFlags::PLAYER);
    giver.driver = CDR_MISSIONGIVE;
    giver.driver_state = Some(CharacterDriverState::MissionGiver(
        MissionGiverDriverData::default(),
    ));
    giver
}

fn governor_driver_data(world: &World, giver_id: CharacterId) -> MissionGiverDriverData {
    match world
        .characters
        .get(&giver_id)
        .and_then(|giver| giver.driver_state.clone())
    {
        Some(CharacterDriverState::MissionGiver(data)) => data,
        _ => panic!("expected MissionGiver driver state"),
    }
}

// C `if (ticker > dat->next_spec || !ch[cn].item[30])` (`missions.c:1308`):
// a freshly spawned governor (empty slot, `next_spec` defaults to `0`)
// always regenerates on the first call.
#[test]
fn regenerate_special_offers_seeds_a_fresh_governor() {
    let mut world = World::default();
    let mut loader = special_item_loader();
    world.tick.0 = 1000;
    world.add_character(governor_npc(CharacterId(1)));

    regenerate_mission_giver_special_offers(&mut world, &mut loader);

    let giver = world.characters.get(&CharacterId(1)).unwrap();
    let item_id = giver.inventory[SPECIAL_OFFER_SLOT].expect("special offer item expected");
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.carried_by, Some(CharacterId(1)));
    let data = governor_driver_data(&world, CharacterId(1));
    assert!(data.spec_cost > 0);
    assert_eq!(data.next_spec, 1000 + SPECIAL_OFFER_PERIOD_TICKS);
}

// C: `ticker > dat->next_spec || !ch[cn].item[30]` both false - no reroll.
#[test]
fn regenerate_special_offers_skips_when_not_due() {
    let mut world = World::default();
    let mut loader = special_item_loader();
    world.tick.0 = 1000;
    let mut giver = governor_npc(CharacterId(1));
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 5000,
        ..MissionGiverDriverData::default()
    }));
    world.add_character(giver);
    let mut offer_item = test_item(ItemId(900), 1, ItemFlags::empty());
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);

    regenerate_mission_giver_special_offers(&mut world, &mut loader);

    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.inventory[SPECIAL_OFFER_SLOT], Some(ItemId(900)));
    let data = governor_driver_data(&world, CharacterId(1));
    assert_eq!(data.spec_cost, 500);
    assert_eq!(data.next_spec, 5000);
}

// C: `ticker > dat->next_spec` true, old item destroyed then replaced.
#[test]
fn regenerate_special_offers_rerolls_and_destroys_the_old_item_once_the_period_elapses() {
    let mut world = World::default();
    let mut loader = special_item_loader();
    world.tick.0 = 10_000;
    let mut giver = governor_npc(CharacterId(1));
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 1000,
        ..MissionGiverDriverData::default()
    }));
    world.add_character(giver);
    let mut offer_item = test_item(ItemId(900), 1, ItemFlags::empty());
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);

    regenerate_mission_giver_special_offers(&mut world, &mut loader);

    assert!(
        world.items.get(&ItemId(900)).is_none(),
        "the stale item must be destroyed"
    );
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    let new_item_id = giver.inventory[SPECIAL_OFFER_SLOT].expect("new item expected");
    assert_ne!(new_item_id, ItemId(900));
    let data = governor_driver_data(&world, CharacterId(1));
    assert_eq!(data.next_spec, 10_000 + SPECIAL_OFFER_PERIOD_TICKS);
}

// C `case 18:`'s `look_item`/"Price: .../Do you want to buy..." lines
// (`missions.c:1627-1634`), applied via `MissionGiveOutcomeEvent::
// ShowSpecialOffer`.
#[test]
fn apply_show_special_offer_event_previews_item_and_price() {
    let mut world = World::default();
    let mut loader = special_item_loader();
    let mut runtime = ServerRuntime::default();
    let mut giver = governor_npc(CharacterId(1));
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 777,
        next_spec: 5000,
        ..MissionGiverDriverData::default()
    }));
    world.add_character(giver);
    let mut offer_item = test_item(ItemId(900), 1, ItemFlags::empty());
    offer_item.name = "Special Sword".into();
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        32,
        5,
        5,
    ));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.governor = MissionPpd {
        points: 300,
        ..Default::default()
    };
    runtime.players.insert(1, player);

    let applied = apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::ShowSpecialOffer {
            player_id: CharacterId(2),
            npc_id: CharacterId(1),
        }],
    );
    assert_eq!(applied, 1);

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Special Sword")));
    assert!(texts.iter().any(|text| text
        .message
        .contains("Price: 777 points (you have 300 points)")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("buy the special offer")));
}

// ---- `GiveCustomStatPotion` (`missions.c:1702-1734`) ----

fn setup_governor_and_player(
    statowed: i32,
) -> (World, ZoneLoader, ServerRuntime, CharacterId, CharacterId) {
    let mut world = World::default();
    let loader = mission_loader();
    let mut runtime = ServerRuntime::default();
    world.add_character(governor_npc(CharacterId(1)));
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        32,
        5,
        5,
    ));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.governor = MissionPpd {
        statowed,
        ..Default::default()
    };
    runtime.players.insert(1, player);
    (world, loader, runtime, CharacterId(1), CharacterId(2))
}

// C `create_item("mis_potionbase")` + `mod_index[0..3]`/`mod_value[0..3]`
// stamping + `give_char_item` success path (`missions.c:1702-1725`).
#[test]
fn apply_give_custom_stat_potion_creates_item_with_modifiers_and_resets_statowed() {
    let (mut world, mut loader, mut runtime, npc_id, player_id) = setup_governor_and_player(1);

    let applied = apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id,
            npc_id,
            stat: [
                CharacterValue::Attack as i32,
                CharacterValue::Parry as i32,
                0,
            ],
            statcnt: 2,
        }],
    );
    assert_eq!(applied, 1);

    let player_char = world.characters.get(&player_id).unwrap();
    let item_id = player_char.cursor_item.expect("potion expected on cursor");
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Custom Potion");
    assert_eq!(
        item.modifier_index[0..3],
        [
            CharacterValue::Attack as i16,
            CharacterValue::Parry as i16,
            0
        ]
    );
    assert_eq!(item.modifier_value[0..3], [30, 30, 0]);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.governor.statowed, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Very well, Godmode, here you go.")));
}

// C statcnt=1/3 stamp `mod_value` 50/0/0 and 20/20/20 respectively
// (`missions.c:1708-1722`).
#[test]
fn apply_give_custom_stat_potion_uses_correct_modifier_values_per_statcnt() {
    let (mut world, mut loader, mut runtime, npc_id, player_id) = setup_governor_and_player(1);
    apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id,
            npc_id,
            stat: [CharacterValue::Attack as i32, 0, 0],
            statcnt: 1,
        }],
    );
    let item_id = world
        .characters
        .get(&player_id)
        .unwrap()
        .cursor_item
        .unwrap();
    assert_eq!(
        world.items.get(&item_id).unwrap().modifier_value[0..3],
        [50, 0, 0]
    );

    let (mut world, mut loader, mut runtime, npc_id, player_id) = setup_governor_and_player(1);
    apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id,
            npc_id,
            stat: [
                CharacterValue::Attack as i32,
                CharacterValue::Parry as i32,
                CharacterValue::Warcry as i32,
            ],
            statcnt: 3,
        }],
    );
    let item_id = world
        .characters
        .get(&player_id)
        .unwrap()
        .cursor_item
        .unwrap();
    assert_eq!(
        world.items.get(&item_id).unwrap().modifier_value[0..3],
        [20, 20, 20]
    );
}

// C `if (give_char_item(co, in)) { ... } else { quiet_say(cn,"please try
// again"); ppd->stat[0]=ppd->stat[1]=ppd->stat[2]=0; destroy_item(in); }`
// (`missions.c:1723-1730`).
#[test]
fn apply_give_custom_stat_potion_reports_no_room_and_resets_stat_but_not_statowed() {
    let (mut world, mut loader, mut runtime, npc_id, player_id) = setup_governor_and_player(1);
    // Fill cursor + every inventory slot so `give_char_item` fails.
    if let Some(player_char) = world.characters.get_mut(&player_id) {
        player_char.cursor_item = Some(ItemId(9000));
        for slot in player_char
            .inventory
            .iter_mut()
            .skip(ugaris_core::legacy::INVENTORY_START_INVENTORY)
        {
            *slot = Some(ItemId(9000));
        }
    }

    let applied = apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id,
            npc_id,
            stat: [CharacterValue::Attack as i32, 0, 0],
            statcnt: 1,
        }],
    );
    assert_eq!(applied, 0);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(
        player.governor.statowed, 1,
        "statowed is untouched on failure, matching C"
    );
    assert_eq!(player.governor.stat, [0, 0, 0]);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("please try again")));
}

// C `create_item` failing (missing template) reports the same "please try
// again" + `stat[]` reset, without ever calling `give_char_item`
// (`missions.c:1731-1734`).
#[test]
fn apply_give_custom_stat_potion_reports_missing_template() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new(); // no `mis_potionbase` registered
    let mut runtime = ServerRuntime::default();
    world.add_character(governor_npc(CharacterId(1)));
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        32,
        5,
        5,
    ));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.governor = MissionPpd {
        statowed: 1,
        ..Default::default()
    };
    runtime.players.insert(1, player);

    let applied = apply_mission_giver_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id: CharacterId(2),
            npc_id: CharacterId(1),
            stat: [CharacterValue::Attack as i32, 0, 0],
            statcnt: 1,
        }],
    );
    assert_eq!(applied, 0);

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.governor.statowed, 1);
    assert_eq!(player.governor.stat, [0, 0, 0]);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("please try again")));
}
