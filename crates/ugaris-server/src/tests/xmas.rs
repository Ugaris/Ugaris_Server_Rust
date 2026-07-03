use super::*;

#[test]
fn apply_xmasmaker_silently_grants_xmaspop_like_c() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    let mut world = World::default();
    world.add_character(character);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
        .unwrap();

    assert!(apply_xmasmaker(&mut world, &mut loader, character_id));

    let character = world.characters.get(&character_id).unwrap();
    let item_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Christmas Pop");
    assert_eq!(item.carried_by, Some(character_id));
}

#[test]
fn apply_xmastree_consumes_holiday_treat_and_marks_area() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut treat = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    treat.driver = IDR_FOOD;
    treat.driver_data = vec![3];
    treat.carried_by = Some(character_id);
    world.add_item(treat);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"ad_bracelet1: name="Holiday Bracelet" flag=IF_TAKE ;"#)
        .unwrap();
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        apply_xmastree(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            1,
            true,
            2025,
            0
        ),
        XmasTreeApplyResult::GiftGranted("Holiday Bracelet".to_string())
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&ItemId(20)));
    let gift_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
    assert_eq!(world.items.get(&gift_id).unwrap().name, "Holiday Bracelet");
    let gift = world.items.get(&gift_id).unwrap();
    assert!(gift
        .description
        .starts_with("To Tester, with holiday blessings from "));
    assert!(gift.description.ends_with(".\nMerry Christmas!"));
    assert_eq!(
        player.touch_xmas_tree(1, 2025, true, true),
        XmasTreeResult::AlreadyGranted
    );
}

#[test]
fn apply_xmastree_rolls_back_area_mark_when_gift_cannot_be_created() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut treat = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    treat.driver = IDR_FOOD;
    treat.driver_data = vec![3];
    treat.carried_by = Some(character_id);
    world.add_item(treat);
    let mut loader = ZoneLoader::new();
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        apply_xmastree(
            &mut world,
            &mut loader,
            &mut player,
            character_id,
            1,
            true,
            2025,
            0
        ),
        XmasTreeApplyResult::NoSpace
    );
    assert_eq!(
        player.touch_xmas_tree(1, 2025, true, false),
        XmasTreeResult::NeedsHolidayTreat
    );
    assert!(world.items.contains_key(&ItemId(20)));
}

#[test]
fn xmas_event_window_matches_legacy_december_to_january_span() {
    assert_eq!(xmas_event_from_ymd(2025, 12, 20), (true, 2025));
    assert_eq!(xmas_event_from_ymd(2026, 1, 7), (true, 2025));
    assert_eq!(xmas_event_from_ymd(2026, 1, 8), (false, 2026));
}
