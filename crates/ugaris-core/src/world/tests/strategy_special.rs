use super::*;
use crate::player::StrategyPpd;

fn strategy_item(id: u32, driver: u16, drdata: Vec<u8>) -> Item {
    let mut it = item(id, ItemFlags::USED);
    it.driver = driver;
    it.driver_data = drdata;
    it
}

fn strategy_player(id: u32, serial: u32) -> Character {
    let mut c = character(id);
    c.serial = serial;
    c.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    c
}

fn boss_ready_ppd() -> StrategyPpd {
    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 9;
    ppd
}

#[test]
fn jump_points_are_discovered_in_ascending_id_order_with_description_text() {
    let mut world = World::default();
    world.add_item(strategy_item(5, IDR_STR_DEPOT, vec![0; 9]));
    world.add_item(strategy_item(2, IDR_STR_STORAGE, vec![0; 10]));
    world.add_item(strategy_item(9, IDR_STR_MINE, vec![0; 9])); // not a jump point

    world.ensure_strategy_jump_points_initialized();

    // index 0 is the unused placeholder; ascending item-id order means
    // item 2 (storage) is jp 1, item 5 (depot) is jp 2.
    assert_eq!(world.strategy_jump_points.points.len(), 3);
    assert_eq!(world.strategy_jump_points.points[1].item_id, ItemId(2));
    assert_eq!(world.strategy_jump_points.points[2].item_id, ItemId(5));
    assert_eq!(
        world.items[&ItemId(2)].description,
        "JP 1. The storage contains all Platinum collected so far."
    );
    assert_eq!(
        world.items[&ItemId(5)].description,
        "JP 2. A depot is used to store Platinum temporarily."
    );
}

#[test]
fn jump_points_init_is_idempotent() {
    let mut world = World::default();
    world.add_item(strategy_item(1, IDR_STR_DEPOT, vec![0; 9]));
    world.ensure_strategy_jump_points_initialized();
    world.add_item(strategy_item(2, IDR_STR_DEPOT, vec![0; 9]));
    world.ensure_strategy_jump_points_initialized();
    assert_eq!(world.strategy_jump_points.points.len(), 2);
}

#[test]
fn command_ignored_outside_areas_23_and_24() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    let matched = world.apply_strategy_special_command(CharacterId(1), 25, &mut ppd, "#info");
    assert!(!matched);
    assert!(world.drain_pending_system_text_bytes().is_empty());
}

#[test]
fn every_command_gates_on_boss_stage_except_reset_and_queue() {
    let mut world = World::default();
    let mut player = strategy_player(1, 111);
    player.flags.insert(CharacterFlags::GOD);
    world.add_character(player);
    let mut ppd = StrategyPpd::default(); // boss_stage == 0

    for command in [
        "#jp 1",
        "#list",
        "#info",
        "#raise 1",
        "#mission",
        "#enter 1",
        "#surrender",
    ] {
        let matched = world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, command);
        assert!(matched, "{command} should be recognized");
        let texts = world.drain_pending_system_texts();
        assert_eq!(
            texts.last().map(|t| t.message.as_str()),
            Some("You have to talk to Cinciac first."),
            "{command} should be gated"
        );
    }
}

#[test]
fn reset_and_queue_are_god_only_and_bypass_the_boss_gate() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111)); // not GOD
    let mut ppd = StrategyPpd::default();
    ppd.init_done = 1;

    assert!(!world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#reset"));
    assert_eq!(
        ppd.init_done, 1,
        "non-god reset is left unmatched, untouched"
    );

    let mut player = strategy_player(2, 222);
    player.flags.insert(CharacterFlags::GOD);
    world.add_character(player);
    assert!(world.apply_strategy_special_command(CharacterId(2), 23, &mut ppd, "#reset"));
    assert_eq!(ppd.init_done, 0);
}

#[test]
fn jp_teleports_only_the_controlling_player() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut depot = strategy_item(1, IDR_STR_DEPOT, vec![0; 9]);
    depot.x = 30;
    depot.y = 40;
    set_str_item_owner(&mut depot, 111);
    world.add_item(depot);
    world.ensure_strategy_jump_points_initialized();

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#jp 1"));
    let character = &world.characters[&CharacterId(1)];
    assert_eq!((character.x, character.y), (30, 40));
}

#[test]
fn jp_rejects_a_point_the_player_does_not_control() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let depot = strategy_item(1, IDR_STR_DEPOT, vec![0; 9]); // owner 0
    world.add_item(depot);
    world.ensure_strategy_jump_points_initialized();

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#jp 1"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts.last().unwrap().message,
        "You can only jump to points you control."
    );
}

#[test]
fn jp_out_of_bounds_reports_error() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#jp 99"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.last().unwrap().message, "Jump point out of bounds.");
}

#[test]
fn list_reports_only_owned_jump_points_with_gold_and_a_trailing_hint() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));

    let mut owned = strategy_item(1, IDR_STR_STORAGE, vec![0; 10]);
    owned.name = "Storage".into();
    set_str_item_owner(&mut owned, 111);
    set_str_item_gold(&mut owned, 400);
    world.add_item(owned);

    let unowned = strategy_item(2, IDR_STR_DEPOT, vec![0; 9]);
    world.add_item(unowned);

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#list"));

    let bytes = world.drain_pending_system_text_bytes();
    assert_eq!(bytes.len(), 1, "only the owned jump point is listed");
    let mut expected = b"JP 1: ".to_vec();
    expected.push(0x03);
    expected.extend_from_slice(b"Storage ");
    expected.push(0x10);
    expected.extend_from_slice(b"400 Platinum.");
    assert_eq!(bytes[0].message, expected);

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.last().unwrap().message, "Use /jp <nr> to teleport.");
}

#[test]
fn list_sends_no_hint_line_when_player_owns_nothing() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.add_item(strategy_item(1, IDR_STR_DEPOT, vec![0; 9]));

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#list"));
    assert!(world.drain_pending_system_text_bytes().is_empty());
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn info_renders_header_and_income_row_byte_for_byte() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();

    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#info"));
    let rows = world.drain_pending_system_text_bytes();
    // header + 7 numbered rows + eguards + guard-level + 3 totals = 13
    assert_eq!(rows.len(), 13);

    let mut header = vec![0x01u8];
    header.extend_from_slice(b"Name ");
    header.push(0x08);
    header.extend_from_slice(b"Value ");
    header.push(0x0D);
    header.extend_from_slice(b"Exp Cost");
    header.push(0x11);
    header.extend_from_slice(b"Increment");
    assert_eq!(rows[0].message, header);

    let mut income_row = b"1 ".to_vec();
    income_row.push(0x01);
    income_row.extend_from_slice(b"Base Income: ");
    income_row.push(0x09);
    income_row.extend_from_slice(b"0 ");
    income_row.push(0x0E);
    income_row.extend_from_slice(b"25 ");
    income_row.push(0x12);
    income_row.extend_from_slice(b"1");
    assert_eq!(rows[1].message, income_row);

    let mut missions_row = b"- ".to_vec();
    missions_row.push(0x01);
    missions_row.extend_from_slice(b"Missions: ");
    missions_row.push(0x09);
    missions_row.extend_from_slice(b"0 ");
    missions_row.push(0x0E);
    missions_row.extend_from_slice(b"-");
    missions_row.push(0x12);
    missions_row.extend_from_slice(b"-");
    assert_eq!(rows[10].message, missions_row);
}

#[test]
fn raise_dispatches_through_str_raise_and_reports_c_text() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    ppd.exp = 100;

    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#raise 1"));
    assert_eq!(ppd.income, 1);
    assert_eq!(ppd.exp, 75);
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.last().unwrap().message, "Done.");
}

#[test]
fn raise_number_out_of_bounds_reports_error_without_touching_ppd() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#raise 10"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.last().unwrap().message, "Number is out of bounds.");
    assert_eq!(ppd.income, 0);
}

#[test]
fn mission_list_skips_missions_with_no_spawners_registered() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();

    // No IDR_STR_SPAWNER items registered anywhere, so every mission's
    // `area[mission.area].max_spawn` stays zero and the whole table is
    // skipped - only the header and trailing hint are sent.
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#mission"));
    let bytes = world.drain_pending_system_text_bytes();
    assert_eq!(bytes.len(), 1, "header only, no mission rows");
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts.last().unwrap().message,
        "Use /enter <nr> to start a mission. If that mission is busy your request will be queued."
    );
}

#[test]
fn mission_list_renders_a_row_for_an_available_mission() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    // MISSIONS[0] ("A-1") is area 1 with no prerequisites.
    let spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0, 0, 0, 0, 0, 0, 0, 0, 1]);
    world.add_item(spawner);

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#mission"));
    let bytes = world.drain_pending_system_text_bytes();
    assert_eq!(bytes.len(), 2, "header + one row for A-1");

    let mut expected_row = crate::text::COL_LIGHT_GREEN.to_vec();
    expected_row.extend_from_slice(b" 1 \x02A-1 \x060   \x080 \x0BZakath   ");
    assert_eq!(bytes[1].message, expected_row);
}

#[test]
fn enter_claims_a_free_spawner_and_teleports_the_player() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    world.characters.get_mut(&CharacterId(1)).unwrap().name = "Alice".into();

    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
    spawner.x = 50;
    spawner.y = 60;
    world.add_item(spawner);
    let mut storage = strategy_item(2, IDR_STR_STORAGE, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
    storage.x = 50;
    storage.y = 59; // directly north of the spawner
    world.add_item(storage);
    world.map.tile_mut(50, 59).expect("tile exists").item = 2;

    let mut ppd = boss_ready_ppd();
    ppd.income = 5;
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#enter 1"));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!((character.x, character.y), (50, 60));
    assert_eq!(str_item_owner(&world.items[&ItemId(1)]), 111);
    assert_eq!(world.items[&ItemId(1)].name, "Alice's Spawner (1)");
    assert_eq!(str_item_owner(&world.items[&ItemId(2)]), 111);
    assert_eq!(world.items[&ItemId(2)].name, "Alice's Storage (1)");
    assert_eq!(world.items[&ItemId(2)].driver_data[9], 5);
    assert_eq!(ppd.mis_cnt, 1);
    assert_eq!(ppd.current_mission, 0);

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|t| t.message == "You take control of this spawner. Use it again to create workers."));
}

#[test]
fn enter_re_enters_a_spawner_the_player_already_owns() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));

    let mut spawner = strategy_item(1, IDR_STR_SPAWNER, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
    spawner.x = 70;
    spawner.y = 80;
    set_str_item_owner(&mut spawner, 111);
    world.add_item(spawner);

    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#enter 1"));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!((character.x, character.y), (70, 80));
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.last().unwrap().message, "Re-entering mission.");
}

#[test]
fn enter_out_of_bounds_mission_number_reports_error() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#enter 999"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts.last().unwrap().message,
        "Mission number is out of bounds."
    );
}

#[test]
fn surrender_removes_the_party_and_reports_when_none_exists() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();

    assert!(world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#surrender"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts.last().unwrap().message,
        "You are not doing any mission."
    );
}

#[test]
fn queue_debug_is_god_only() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111)); // not GOD
    let mut ppd = StrategyPpd::default();
    world.queue_mission(CharacterId(1), 3);

    assert!(!world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#queue"));

    let mut god = strategy_player(2, 222);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    assert!(world.apply_strategy_special_command(CharacterId(2), 23, &mut ppd, "#queue"));
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Area 3, queue 0, cn=1, ID=111");
}

#[test]
fn eguard_is_not_yet_recognized_and_falls_through_as_plain_text() {
    let mut world = World::default();
    world.add_character(strategy_player(1, 111));
    let mut ppd = boss_ready_ppd();
    assert!(!world.apply_strategy_special_command(CharacterId(1), 23, &mut ppd, "#eguard 1"));
}
