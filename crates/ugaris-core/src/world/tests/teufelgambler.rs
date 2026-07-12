use super::*;
use crate::character_driver::{CDR_TEUFELGAMBLER, NT_CHAR};
use crate::world::npc::area34::teufelgambler::{IID_BRONZECHIP, IID_GOLDCHIP, IID_SILVERCHIP};

const AREA_ID: u16 = 34;

fn teufelgambler_npc(id: u32, nr: i32) -> Character {
    let mut gambler = character(id);
    gambler.name = "Demon Gambler".into();
    gambler.driver = CDR_TEUFELGAMBLER;
    gambler.sprite = 27;
    gambler.driver_state = Some(CharacterDriverState::TeufelGambler(
        crate::character_driver::TeufelGambleDriverData {
            nr,
            ..Default::default()
        },
    ));
    gambler
}

fn player(id: u32, name: &str, sprite: i32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.sprite = sprite;
    player
}

fn chip_item(id: u32, template_id: u32, count: u32) -> Item {
    let mut chip = item(id, ItemFlags::TAKE);
    chip.name = "Bronze Chip".into();
    chip.template_id = template_id;
    chip.driver_data = count.to_le_bytes().to_vec();
    chip
}

fn lit_world() -> World {
    let mut world = World::default();
    for x in 0..20 {
        for y in 0..20 {
            world.map.tile_mut(x, y).unwrap().light = 255;
        }
    }
    world
}

#[test]
fn teufelgambler_greets_demon_disguised_player_with_matching_play_word() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 2), 10, 10));
    // sprite 157 == fire-demon-suit, matches `is_demon`.
    assert!(world.spawn_character(player(2, "Godmode", 157), 12, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelgambler_actions(&mut loader);

    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(|text| {
        let s = String::from_utf8_lossy(&text.message);
        s.contains("play2") && s.contains("Godmode")
    }));
}

#[test]
fn teufelgambler_greets_undisguised_human_with_species_specific_line() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    // sprite 0 (plain human) fails `is_demon`.
    assert!(world.spawn_character(player(2, "Godmode", 0), 12, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelgambler_actions(&mut loader);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Oh. A human") && text.message.contains("play")));
}

#[test]
fn teufelgambler_no_chips_no_game() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "bet one");
    }
    world.process_teufelgambler_actions(&mut loader);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("No chips, no game")));
}

#[test]
fn teufelgambler_bet_consumes_matching_chip_stack_partially() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    let mut chip = chip_item(900, IID_BRONZECHIP, 3);
    chip.carried_by = Some(CharacterId(2));
    world.items.insert(chip.id, chip);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.inventory[30] = Some(ItemId(900));
    }

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "bet one");
    }
    world.process_teufelgambler_actions(&mut loader);

    // Regardless of the (RNG-dependent) win/lose roll, betting "one"
    // always consumes exactly 1 bronze chip from the 3-chip stack first
    // (`teufel.c:1336-1348`).
    let remaining = world.items.get(&ItemId(900)).expect("stack not destroyed");
    let mut bytes = [0_u8; 4];
    bytes.copy_from_slice(&remaining.driver_data[0..4]);
    assert_eq!(u32::from_le_bytes(bytes), 2);
    assert_eq!(remaining.description, "2 Bronze Chips.");
}

#[test]
fn teufelgambler_bet_destroys_stack_when_fully_consumed() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 2), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    let mut chip = chip_item(900, IID_SILVERCHIP, 1);
    chip.carried_by = Some(CharacterId(2));
    world.items.insert(chip.id, chip);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.inventory[30] = Some(ItemId(900));
    }

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "bet one");
    }
    world.process_teufelgambler_actions(&mut loader);

    // C `if (cnt == have) { destroy_item(in); ch[co].item[n] = 0; }`
    // (`teufel.c:1339-1341`).
    assert!(world.items.get(&ItemId(900)).is_none());
    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player_after.inventory[30], None);
}

#[test]
fn teufelgambler_wrong_chip_color_is_not_consumed() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    // Gold-tier gambler (nr=3) only accepts gold chips.
    assert!(world.spawn_character(teufelgambler_npc(1, 3), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    let mut chip = chip_item(900, IID_BRONZECHIP, 5);
    chip.carried_by = Some(CharacterId(2));
    world.items.insert(chip.id, chip);
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.inventory[30] = Some(ItemId(900));
    }

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "bet one");
    }
    world.process_teufelgambler_actions(&mut loader);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("No chips, no game")));
    let unchanged = world.items.get(&ItemId(900)).unwrap();
    let mut bytes = [0_u8; 4];
    bytes.copy_from_slice(&unchanged.driver_data[0..4]);
    assert_eq!(u32::from_le_bytes(bytes), 5);
}

#[test]
fn teufelgambler_god_reward_cheat_grants_money() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    let mut god = player(2, "Godmode", 157);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        // C `give_reward(co, 4, 5)`: `case 4: give_money(cn, bet *
        // 2000000, ...)` (`teufel.c:660-663`).
        gambler.push_driver_text_message(CharacterId(2), "reward: 4");
    }
    world.process_teufelgambler_actions(&mut loader);

    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player_after.gold, 10_000_000);
}

#[test]
fn teufelgambler_reward_cheat_ignored_for_non_god() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "reward: 4");
    }
    world.process_teufelgambler_actions(&mut loader);

    let player_after = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player_after.gold, 0);
}

#[test]
fn teufelgambler_reward_cheat_with_invalid_roll_reports_bug_1778() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    let mut god = player(2, "Godmode", 157);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        // 21 is in the losing band (21..=42) and has no `give_reward`
        // case, so C's `ptr`/`cnt` stay unset (`teufel.c:802-806`).
        gambler.push_driver_text_message(CharacterId(2), "reward: 21");
    }
    world.process_teufelgambler_actions(&mut loader);

    let system_texts = world.drain_pending_system_texts();
    assert!(system_texts
        .iter()
        .any(|text| text.character_id == CharacterId(2) && text.message == "Bug #1778"));
}

#[test]
fn teufelgambler_greet_once_via_driver_memory() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelgambler_actions(&mut loader);
    assert!(!world.drain_pending_area_text_bytes().is_empty());

    // Second sighting of the same player is suppressed by
    // `mem_check_driver` (`teufel.c:1270-1273`).
    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_teufelgambler_actions(&mut loader);
    assert!(world.drain_pending_area_text_bytes().is_empty());
}

#[test]
fn teufelgambler_asking_own_name_replies_without_playing() {
    let mut world = lit_world();
    let mut loader = ZoneLoader::new();
    assert!(world.spawn_character(teufelgambler_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 157), 11, 10));

    if let Some(gambler) = world.characters.get_mut(&CharacterId(1)) {
        gambler.push_driver_text_message(CharacterId(2), "who are you");
    }
    world.process_teufelgambler_actions(&mut loader);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'm Demon Gambler")));
}

#[test]
fn iid_chip_constants_match_c_header() {
    // `src/common/item_id.h:231-233`.
    assert_eq!(IID_BRONZECHIP, 0x0100_00AC);
    assert_eq!(IID_SILVERCHIP, 0x0100_00AD);
    assert_eq!(IID_GOLDCHIP, 0x0100_00AE);
}
