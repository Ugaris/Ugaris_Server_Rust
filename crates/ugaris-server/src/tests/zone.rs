use super::*;

#[test]
fn look_map_payload_visible_tile_reports_coords_and_zone_flags() {
    let mut world = World::default();
    world.map.set_flags(
        12,
        13,
        MapFlags::RESTAREA | MapFlags::CLAN | MapFlags::ARENA | MapFlags::PEACE,
    );

    let payloads = look_map_payloads(
        &world,
        99,
        LookMapRequest {
            character_id: CharacterId(7),
            x: 12,
            y: 13,
            character_level: 0,
            visible: true,
        },
    );

    assert_eq!(
        text_payloads(&payloads),
        vec![
            "(12,13)",
            "This place is a rest area.",
            "This is a clan area.",
            "This place is an arena.",
            "This place is a peaceful zone.",
        ]
    );
}

#[test]
fn load_area_zone_reads_first_area_map_file() {
    let root = unique_temp_zone_root("load_area_zone_reads_first_area_map_file");
    let area = root.join("1");
    std::fs::create_dir_all(&area).unwrap();
    std::fs::write(
        area.join("sample.map"),
        r#"
            field="10,11"
            gsprite=123
            fsprite=456
            flag=MF_MOVEBLOCK
            "#,
    )
    .unwrap();

    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let summary = load_area_zone(&mut world, &mut loader, &root, 1).unwrap();

    let tile = world.map.tile(10, 11).unwrap();
    assert_eq!(tile.ground_sprite, 123);
    assert_eq!(tile.foreground_sprite, 456);
    assert!(tile.flags.contains(MapFlags::MOVEBLOCK));
    assert_eq!(summary.ground_tiles, 1);
    assert_eq!(summary.blocked_tiles, 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn choose_spawn_tile_skips_blocked_default_spawn() {
    let mut world = World::default();
    world
        .map
        .tile_mut(LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    let (x, y) = choose_spawn_tile(&world);

    assert_ne!((x, y), (LOGIN_SPAWN_X, LOGIN_SPAWN_Y));
    assert!(is_spawn_tile_open(&world, x, y));
}
