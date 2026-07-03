use super::*;

#[test]
fn world_applies_player_use_setup_from_adjacent_map_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Use,
        arg1: 11,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::USE);
    assert_eq!(character.act1, 7);
    assert_eq!(character.dir, Direction::Right as u8);
}

#[test]
fn world_applies_completed_item_use_request_to_container_state() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.content_id = 22;
    world.add_item(item);

    let outcome = world
        .use_item_request(
            ItemUseRequest {
                character_id: CharacterId(1),
                item_id: ItemId(7),
                spec: 0,
            },
            false,
        )
        .unwrap();

    assert_eq!(
        outcome,
        UseItemOutcome::OpenContainer { item_id: ItemId(7) }
    );
    assert_eq!(
        world
            .characters
            .get(&CharacterId(1))
            .unwrap()
            .current_container,
        Some(ItemId(7))
    );
}

#[test]
fn world_applies_clanjewel_expiry_to_carried_inventory_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(8));
    world.add_character(character);

    let mut jewel = item(8, ItemFlags::USED);
    jewel.name = "Clan Jewel".into();
    jewel.driver = crate::item_driver::IDR_CLANJEWEL;
    jewel.carried_by = Some(CharacterId(1));
    world.add_item(jewel);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::ClanJewelExpired {
            item_id: ItemId(8),
            character_id: Some(CharacterId(1)),
            item_name: crate::item_driver::outcome_item_name("Clan Jewel"),
        },
        30,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::ClanJewelExpired { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(8)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}
