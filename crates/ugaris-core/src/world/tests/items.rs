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

#[test]
fn can_wear_rejects_positions_outside_the_worn_slot_range() {
    let mut world = World::default();
    world.add_character(character(1));
    world.add_item(item(9, ItemFlags::USED | ItemFlags::WNHEAD));

    // C `can_wear` (`src/system/tool.c:1007-1010`): `pos < 0 || pos > 11`
    // is an illegal call, always rejected.
    assert!(!world.can_wear(CharacterId(1), ItemId(9), 12));
}

#[test]
fn check_requirements_rejects_above_maximum_level() {
    let mut world = World::default();
    let mut wearer = character(1);
    wearer.level = 40;
    world.add_character(wearer);
    let mut cap = item(9, ItemFlags::USED | ItemFlags::WNHEAD);
    cap.max_level = 20;
    world.add_item(cap);

    // C `check_requirements` (`src/system/tool.c:969-971`): `max_level`.
    assert!(!world.can_wear(CharacterId(1), ItemId(9), worn_slot::HEAD));
}

#[test]
fn check_requirements_seyanddu_gate_needs_both_mage_and_warrior_flags() {
    let mut world = World::default();
    let mut hybrid = character(1);
    hybrid
        .flags
        .insert(CharacterFlags::MAGE | CharacterFlags::WARRIOR);
    world.add_character(hybrid);
    let mut pure_warrior = character(2);
    pure_warrior.flags.insert(CharacterFlags::WARRIOR);
    world.add_character(pure_warrior);

    let mut robe = item(9, ItemFlags::USED | ItemFlags::WNBODY);
    robe.needs_class = 4; // C: "Only usable by a Seyan'Du."
    world.add_item(robe);
    let mut robe2 = item(10, ItemFlags::USED | ItemFlags::WNBODY);
    robe2.needs_class = 4;
    world.add_item(robe2);

    assert!(world.can_wear(CharacterId(1), ItemId(9), worn_slot::BODY));
    assert!(!world.can_wear(CharacterId(2), ItemId(10), worn_slot::BODY));
}

#[test]
fn check_requirements_arch_gate_rejects_non_arch_characters() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut relic = item(9, ItemFlags::USED | ItemFlags::WNNECK);
    relic.needs_class = 8; // C: "Only usable by an Arch."
    world.add_item(relic);

    assert!(!world.can_wear(CharacterId(1), ItemId(9), worn_slot::NECK));

    let mut arch = character(2);
    arch.flags.insert(CharacterFlags::ARCH);
    world.add_character(arch);
    let mut relic2 = item(10, ItemFlags::USED | ItemFlags::WNNECK);
    relic2.needs_class = 8;
    world.add_item(relic2);

    assert!(world.can_wear(CharacterId(2), ItemId(10), worn_slot::NECK));
}

#[test]
fn check_requirements_bondwear_restricts_to_the_bonded_owner() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut ring = item(
        9,
        ItemFlags::USED | ItemFlags::WNLRING | ItemFlags::BONDWEAR,
    );
    ring.owner_id = 999;
    world.add_item(ring);

    // C `check_requirements` (`src/system/tool.c:986-988`):
    // `(it[in].flags & IF_BONDWEAR) && it[in].ownerID != ch[cn].ID`.
    assert!(!world.can_wear(CharacterId(1), ItemId(9), worn_slot::LEFT_RING));

    let mut ring2 = item(
        10,
        ItemFlags::USED | ItemFlags::WNLRING | ItemFlags::BONDWEAR,
    );
    ring2.owner_id = 1;
    world.add_item(ring2);
    assert!(world.can_wear(CharacterId(1), ItemId(10), worn_slot::LEFT_RING));
}

#[test]
fn complete_drop_arms_decay_timer_for_take_items_and_expires_after_item_decay_time() {
    let mut world = World::default();
    let mut dropper = character(1);
    dropper.x = 10;
    dropper.y = 10;
    dropper.dir = Direction::Right as u8;
    dropper.act1 = 7;
    dropper.cursor_item = Some(ItemId(7));
    world.add_character(dropper);
    world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));

    assert!(world.complete_drop(CharacterId(1), ItemId(7)));
    assert!(world.items.contains_key(&ItemId(7)));

    // C `set_item_map` (`map.c:36-85`): `IF_TAKE` items get
    // `set_expire(in, item_decay_time)`; `item_decay_time` defaults to
    // `5 * 60 * TICKS` (`game_settings.c`).
    world.tick = Tick(world.tick.0 + world.settings.item_decay_time as u64 - 1);
    world.process_due_timers(1);
    assert!(
        world.items.contains_key(&ItemId(7)),
        "item must not decay before item_decay_time elapses"
    );

    world.tick = Tick(world.tick.0 + 1);
    world.process_due_timers(1);
    assert!(
        !world.items.contains_key(&ItemId(7)),
        "item must decay exactly at item_decay_time"
    );
}

#[test]
fn complete_drop_does_not_arm_decay_timer_for_nodecay_take_items() {
    let mut world = World::default();
    let mut dropper = character(1);
    dropper.x = 10;
    dropper.y = 10;
    dropper.dir = Direction::Right as u8;
    dropper.act1 = 7;
    dropper.cursor_item = Some(ItemId(7));
    world.add_character(dropper);
    world.add_item(item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::NODECAY,
    ));

    assert!(world.complete_drop(CharacterId(1), ItemId(7)));

    // C `set_expire` (`expire.c`) no-ops for `IF_NODECAY` items; a lit
    // torch dropped on the ground must survive past `item_decay_time`.
    world.tick = Tick(world.tick.0 + world.settings.item_decay_time as u64 + 1);
    world.process_due_timers(1);
    assert!(
        world.items.contains_key(&ItemId(7)),
        "IF_NODECAY items must never be armed with a decay timer"
    );
}

#[test]
fn check_requirements_ignores_out_of_range_modifier_index_without_panicking() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut cursed = item(9, ItemFlags::USED | ItemFlags::WNFEET);
    cursed.modifier_index[0] = i16::MIN;
    cursed.modifier_value[0] = 5;
    world.add_item(cursed);

    // C `check_requirements` bounds-checks `mod_index` and drops illegal
    // entries rather than indexing out of bounds; must not panic and must
    // not spuriously block the wear.
    assert!(world.can_wear(CharacterId(1), ItemId(9), worn_slot::FEET));
}
