use super::*;

#[test]
fn world_advances_and_resets_character_action_steps() {
    let mut world = World::default();
    let mut character = character(1);
    character.duration = 2;
    character.action = action::WALK;
    world.add_character(character);

    assert_eq!(world.advance_character_action(CharacterId(1)), Some(false));
    assert_eq!(world.advance_character_action(CharacterId(1)), Some(true));
    assert!(world.reset_character_action(CharacterId(1)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
    assert_eq!(character.duration, 0);
    assert_eq!(character.step, 0);
}

#[test]
fn tick_basic_actions_stamps_regen_ticker_for_active_but_not_passive_actions() {
    // C `act()` (act.c:1877): regen_ticker is stamped to `ticker` for every
    // action except AC_IDLE/AC_MAGICSHIELD/AC_BLESS_SELF/AC_HEAL_SELF.
    let mut world = World::default();
    world.tick = Tick(500);

    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    walker.tox = 10;
    walker.toy = 10;
    walker.action = action::WALK;
    walker.duration = 1;
    world.add_character(walker);

    let mut shielder = character(2);
    shielder.x = 12;
    shielder.y = 10;
    shielder.action = action::MAGICSHIELD;
    shielder.duration = 1;
    world.add_character(shielder);

    world.tick_basic_actions();

    assert_eq!(world.characters[&CharacterId(1)].regen_ticker, 500);
    assert_eq!(world.characters[&CharacterId(2)].regen_ticker, 0);
}

#[test]
fn char_swap_exchanges_idle_character_with_visible_playerlike_target() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.dir = Direction::Right as u8;
    let mut target = character(2);
    target.flags |= CharacterFlags::PLAYERLIKE;
    assert!(world.spawn_character(actor, 10, 10));
    assert!(world.spawn_character(target, 11, 10));

    assert!(world.char_swap(CharacterId(1)));

    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (11, 10)
    );
    assert_eq!(
        (
            world.characters[&CharacterId(2)].x,
            world.characters[&CharacterId(2)].y
        ),
        (10, 10)
    );
    assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
    assert_eq!(world.map.tile(10, 10).unwrap().character, 2);
}

#[test]
fn char_swap_rejects_invisible_targets() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.dir = Direction::Right as u8;
    let mut target = character(2);
    target.flags |= CharacterFlags::PLAYER | CharacterFlags::INVISIBLE;
    assert!(world.spawn_character(actor, 10, 10));
    assert!(world.spawn_character(target, 11, 10));

    assert!(!world.char_swap(CharacterId(1)));

    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (10, 10)
    );
    assert_eq!(
        (
            world.characters[&CharacterId(2)].x,
            world.characters[&CharacterId(2)].y
        ),
        (11, 10)
    );
}

#[test]
fn walk_swap_or_use_falls_back_to_use_after_blocked_walk_and_no_swap() {
    let mut world = World::default();
    assert!(world.spawn_character(character(1), 10, 10));
    let mut lever = item(1, ItemFlags::USE | ItemFlags::MOVEBLOCK);
    assert!(world.map.set_item_map(&mut lever, 11, 10));
    world.add_item(lever);

    assert!(world.walk_swap_or_use_driver(CharacterId(1), Direction::Right, 1));

    let actor = &world.characters[&CharacterId(1)];
    assert_eq!(actor.action, action::USE);
    assert_eq!(actor.act1, 1);
}

#[test]
fn world_updates_map_light_when_lit_item_is_taken_and_dropped() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.act1 = 7;
    let mut light_item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    light_item.x = 11;
    light_item.y = 10;
    light_item.modifier_index[0] = CharacterValue::Light as i16;
    light_item.modifier_value[0] = 16;
    world.map.tile_mut(11, 10).unwrap().item = 7;
    world.add_character(character);
    world.add_item(light_item);
    assert_eq!(world.map.tile(11, 10).unwrap().light, 16);

    assert!(world.complete_take(CharacterId(1), ItemId(7), true));
    assert_eq!(world.map.tile(11, 10).unwrap().light, 0);

    assert!(world.complete_drop(CharacterId(1), ItemId(7)));
    assert_eq!(world.map.tile(11, 10).unwrap().light, 16);
}

#[test]
fn world_updates_map_light_when_character_spawns_walks_and_leaves() {
    let mut world = World::default();
    let mut character = character(1);
    character.values[0][CharacterValue::Light as usize] = 16;

    assert!(world.spawn_character(character, 10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 16);

    let character = world.characters.get_mut(&CharacterId(1)).unwrap();
    character.tox = 12;
    character.toy = 10;
    assert!(world.complete_walk(CharacterId(1)));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 3);
    assert_eq!(world.map.tile(12, 10).unwrap().light, 16);
    assert!(world
        .map
        .tile(12, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));

    assert!(world.remove_character(CharacterId(1)).is_some());
    assert_eq!(world.map.tile(12, 10).unwrap().light, 0);
}

#[test]
fn world_completes_walk_against_map_storage() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.tox = 11;
    character.toy = 10;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);

    assert!(world.complete_walk(CharacterId(1)));
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 11);
    assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
}

#[test]
fn complete_walk_notifies_nearby_characters_with_nt_char() {
    // C `act_walk` (act.c:227-229): notify_area(ch[cn].x, ch[cn].y, NT_CHAR,
    // cn, 0, 0) fires right after the position/light/sector update, so every
    // character within the `NOTIFY_SIZE` (32-tile) box gets an NT_CHAR
    // message queued about the mover, regardless of visibility.
    let mut world = World::default();
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    walker.tox = 11;
    walker.toy = 10;
    world.add_character(walker);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut far_away = character(3);
    far_away.x = 200;
    far_away.y = 200;
    world.add_character(far_away);

    assert!(world.complete_walk(CharacterId(1)));

    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(nearby.driver_messages.len(), 1);
    assert_eq!(nearby.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(nearby.driver_messages[0].dat1, 1);

    let far_away = world.characters.get(&CharacterId(3)).unwrap();
    assert!(far_away.driver_messages.is_empty());

    // The mover itself is inside its own notify box, matching C.
    let walker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(walker.driver_messages.len(), 1);
    assert_eq!(walker.driver_messages[0].dat1, 1);
}

#[test]
fn complete_walk_skips_notify_when_cf_nonotify_set() {
    // C `act_walk`: `if (!(ch[cn].flags & CF_NONOTIFY)) notify_area(...)`.
    let mut world = World::default();
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    walker.tox = 11;
    walker.toy = 10;
    walker.flags.insert(CharacterFlags::NONOTIFY);
    world.add_character(walker);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    assert!(world.complete_walk(CharacterId(1)));

    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert!(nearby.driver_messages.is_empty());
}

#[test]
fn complete_walk_does_not_notify_when_walk_fails() {
    let mut world = World::default();
    // `tox`/`toy` left at their default (0, 0), which is out of the map's
    // legacy inner bounds, so `act_walk` reports no movement.
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    world.add_character(walker);

    let mut nearby = character(2);
    nearby.x = 11;
    nearby.y = 10;
    world.add_character(nearby);

    assert!(!world.complete_walk(CharacterId(1)));
    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert!(nearby.driver_messages.is_empty());
}

#[test]
fn world_completes_take_and_drop_against_item_storage() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.act1 = 7;
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_character(character);
    world.add_item(item);

    assert!(world.complete_take(CharacterId(1), ItemId(7), true));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        Some(ItemId(7))
    );

    world.characters.get_mut(&CharacterId(1)).unwrap().act1 = 7;
    assert!(world.complete_drop(CharacterId(1), ItemId(7)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    assert_eq!(world.map.tile(11, 10).unwrap().item, 7);
}

#[test]
fn complete_take_notifies_nearby_characters_with_nt_char() {
    // C `act_take` (act.c:333-335): `if (!(ch[cn].flags & CF_NONOTIFY))
    // notify_area(ch[cn].x, ch[cn].y, NT_CHAR, cn, 0, 0);` fires right after
    // the item lands on the taker's cursor.
    let mut world = World::default();
    let mut taker = character(1);
    taker.x = 10;
    taker.y = 10;
    taker.dir = Direction::Right as u8;
    taker.act1 = 7;
    world.add_character(taker);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    assert!(world.complete_take(CharacterId(1), ItemId(7), true));

    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(nearby.driver_messages.len(), 1);
    assert_eq!(nearby.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(nearby.driver_messages[0].dat1, 1);
}

#[test]
fn complete_take_skips_notify_when_cf_nonotify_set() {
    let mut world = World::default();
    let mut taker = character(1);
    taker.x = 10;
    taker.y = 10;
    taker.dir = Direction::Right as u8;
    taker.act1 = 7;
    taker.flags.insert(CharacterFlags::NONOTIFY);
    world.add_character(taker);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    assert!(world.complete_take(CharacterId(1), ItemId(7), true));
    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert!(nearby.driver_messages.is_empty());
}

#[test]
fn complete_drop_notifies_nt_char_and_unconditional_nt_item() {
    // C `act_drop` (act.c:440-443): `NT_CHAR` gated on `CF_NONOTIFY`, then an
    // unconditional `notify_area(ch[cn].x, ch[cn].y, NT_ITEM, in, 0, 0);`.
    let mut world = World::default();
    let mut dropper = character(1);
    dropper.x = 10;
    dropper.y = 10;
    dropper.dir = Direction::Right as u8;
    dropper.act1 = 7;
    dropper.cursor_item = Some(ItemId(7));
    dropper.flags.insert(CharacterFlags::NONOTIFY);
    world.add_character(dropper);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));

    assert!(world.complete_drop(CharacterId(1), ItemId(7)));

    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    // NT_CHAR was suppressed by CF_NONOTIFY, but NT_ITEM always fires.
    assert_eq!(nearby.driver_messages.len(), 1);
    assert_eq!(nearby.driver_messages[0].message_type, NT_ITEM);
    assert_eq!(nearby.driver_messages[0].dat1, 7);
}

#[test]
fn complete_use_notifies_nt_char_once_validation_passes() {
    // C `act_use` (act.c:376-379): notify fires once target/item validation
    // passes, before the actual `use_item` outcome is known.
    let mut world = World::default();
    let mut user = character(1);
    user.x = 10;
    user.y = 10;
    user.dir = Direction::Right as u8;
    user.act1 = 7;
    world.add_character(user);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    assert!(world.complete_use(CharacterId(1), ItemId(7)).is_some());

    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(nearby.driver_messages.len(), 1);
    assert_eq!(nearby.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(nearby.driver_messages[0].dat1, 1);
}

#[test]
fn complete_use_skips_notify_when_validation_fails() {
    let mut world = World::default();
    let mut user = character(1);
    user.x = 10;
    user.y = 10;
    user.dir = Direction::Right as u8;
    // act1 does not match the item id, so `act_use` returns `None`.
    user.act1 = 99;
    world.add_character(user);

    let mut nearby = character(2);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    assert!(world.complete_use(CharacterId(1), ItemId(7)).is_none());
    let nearby = world.characters.get(&CharacterId(2)).unwrap();
    assert!(nearby.driver_messages.is_empty());
}

#[test]
fn complete_give_notifies_nt_char_after_nt_give() {
    // C `act_give` (act.c:871-875): `notify_char(co, NT_GIVE, cn, in, 0);`
    // fires first (already handled by `transfer_cursor_item`), then
    // `NT_CHAR` broadcasts to the area gated on `CF_NONOTIFY`.
    let mut world = World::default();
    let mut giver = character(1);
    giver.x = 10;
    giver.y = 10;
    giver.dir = Direction::Right as u8;
    giver.cursor_item = Some(ItemId(7));
    world.add_character(giver);

    let mut receiver = character(2);
    receiver.flags.insert(CharacterFlags::PLAYER);
    receiver.x = 11;
    receiver.y = 10;
    world.add_character(receiver);
    world.map.tile_mut(11, 10).unwrap().character = 2;

    let mut nearby = character(3);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    item.carried_by = Some(CharacterId(1));
    world.add_item(item);

    assert!(world.complete_give(CharacterId(1), CharacterId(2)));

    // The receiver is inside the notify box too, so it gets both NT_GIVE
    // (from `transfer_cursor_item`) and the area-wide NT_CHAR.
    let receiver = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(receiver.driver_messages.len(), 2);
    assert_eq!(receiver.driver_messages[0].message_type, NT_GIVE);
    assert_eq!(receiver.driver_messages[1].message_type, NT_CHAR);

    let nearby = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(nearby.driver_messages.len(), 1);
    assert_eq!(nearby.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(nearby.driver_messages[0].dat1, 1);
}

#[test]
fn complete_give_skips_nt_char_when_cf_nonotify_set() {
    let mut world = World::default();
    let mut giver = character(1);
    giver.x = 10;
    giver.y = 10;
    giver.dir = Direction::Right as u8;
    giver.cursor_item = Some(ItemId(7));
    giver.flags.insert(CharacterFlags::NONOTIFY);
    world.add_character(giver);

    let mut receiver = character(2);
    receiver.flags.insert(CharacterFlags::PLAYER);
    receiver.x = 11;
    receiver.y = 10;
    world.add_character(receiver);
    world.map.tile_mut(11, 10).unwrap().character = 2;

    let mut nearby = character(3);
    nearby.x = 15;
    nearby.y = 10;
    world.add_character(nearby);

    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    item.carried_by = Some(CharacterId(1));
    world.add_item(item);

    assert!(world.complete_give(CharacterId(1), CharacterId(2)));

    let nearby = world.characters.get(&CharacterId(3)).unwrap();
    assert!(nearby.driver_messages.is_empty());
}

#[test]
fn world_applies_player_walkdir_setup_or_falls_back_to_idle() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::WalkDir,
        arg1: Direction::Right as i32,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (11, 10));

    world.map.set_flags(12, 10, MapFlags::MOVEBLOCK);
    world.characters.get_mut(&CharacterId(1)).unwrap().x = 11;
    world.characters.get_mut(&CharacterId(1)).unwrap().y = 10;
    world.characters.get_mut(&CharacterId(1)).unwrap().tox = 0;
    world.characters.get_mut(&CharacterId(1)).unwrap().toy = 0;
    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::IDLE);
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn world_applies_player_walkdir_diagonal_wall_slide() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    world.map.set_flags(11, 10, MapFlags::MOVEBLOCK);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::WalkDir,
        arg1: Direction::RightUp as i32,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (10, 9));
    assert_eq!(character.dir, Direction::Up as u8);
}

#[test]
fn world_applies_player_move_setup_with_pathfinder() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Move,
        arg1: 13,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Move);
}

#[test]
fn world_applies_player_drop_setup_from_cursor_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.cursor_item = Some(ItemId(7));
    world.add_character(character);
    world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Drop,
        arg1: 11,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::DROP);
    assert_eq!(character.act1, 7);
}

#[test]
fn world_applies_player_take_setup_from_adjacent_map_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Take,
        arg1: 11,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::TAKE);
    assert_eq!(character.act1, 7);
    assert_eq!(character.dir, Direction::Right as u8);
}

#[test]
fn world_applies_player_take_setup_by_walking_toward_distant_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut item, 13, 10));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Take,
        arg1: 13,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Take);
}

#[test]
fn world_applies_player_drop_setup_by_walking_toward_distant_target() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.cursor_item = Some(ItemId(7));
    world.add_character(character);
    world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Drop,
        arg1: 13,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Drop);
}

#[test]
fn world_applies_player_give_setup_to_adjacent_character() {
    let mut world = World::default();
    let mut giver = character(1);
    giver.x = 10;
    giver.y = 10;
    giver.cursor_item = Some(ItemId(7));
    let mut receiver = character(2);
    receiver.flags.insert(CharacterFlags::PLAYER);
    receiver.x = 11;
    receiver.y = 10;
    world.add_character(giver);
    world.add_character(receiver);
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    item.carried_by = Some(CharacterId(1));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Give,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.action, action::GIVE);
    assert_eq!(giver.act1, 2);
    assert_eq!(giver.dir, Direction::Right as u8);
}

#[test]
fn world_applies_player_give_setup_by_walking_toward_recipient() {
    let mut world = World::default();
    let mut giver = character(1);
    giver.x = 10;
    giver.y = 10;
    giver.cursor_item = Some(ItemId(7));
    let mut receiver = character(2);
    receiver.flags.insert(CharacterFlags::PLAYER);
    receiver.x = 13;
    receiver.y = 10;
    world.add_character(giver);
    world.add_character(receiver);
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    item.carried_by = Some(CharacterId(1));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Give,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.action, action::WALK);
    assert_eq!((giver.tox, giver.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Give);
}

#[test]
fn world_completes_give_to_player_inventory_or_cursor() {
    let mut world = World::default();
    let mut giver = character(1);
    giver.x = 10;
    giver.y = 10;
    giver.dir = Direction::Right as u8;
    giver.action = action::GIVE;
    giver.duration = 1;
    giver.act1 = 2;
    giver.cursor_item = Some(ItemId(7));
    let mut receiver = character(2);
    receiver.flags.insert(CharacterFlags::PLAYER);
    receiver.x = 11;
    receiver.y = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(giver);
    world.add_character(receiver);
    let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    item.carried_by = Some(CharacterId(1));
    world.add_item(item);

    let completed = world.tick_basic_actions();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].action_id, action::GIVE);
    assert!(completed[0].ok);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(7))
    );
    assert_eq!(
        world.items.get(&ItemId(7)).unwrap().carried_by,
        Some(CharacterId(2))
    );
}

#[test]
fn world_applies_player_use_setup_by_walking_toward_frontwall_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::FRONTWALL);
    assert!(world.map.set_item_map(&mut item, 13, 10));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Use,
        arg1: 13,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::WALK);
    assert_eq!((character.tox, character.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Use);
}

#[test]
fn world_applies_player_look_map_as_immediate_request() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::LookMap,
        arg1: 13,
        arg2: 9,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.dir, Direction::RightUp as u8);
    assert_eq!(character.action, action::IDLE);
    let requests = world.drain_look_map_requests();
    assert_eq!(
        requests,
        vec![LookMapRequest {
            character_id: CharacterId(1),
            x: 13,
            y: 9,
            character_level: 1,
            visible: true,
        }]
    );
}

#[test]
fn world_ticks_basic_action_completion_and_resets_state() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.tox = 11;
    character.toy = 10;
    character.action = action::WALK;
    character.duration = 2;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);

    assert!(world.tick_basic_actions().is_empty());
    let completed = world.tick_basic_actions();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].character_id, CharacterId(1));
    assert_eq!(completed[0].action_id, action::WALK);
    assert!(completed[0].ok);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (11, 10));
    assert_eq!(character.action, 0);
    assert_eq!(character.duration, 0);
    assert_eq!(character.step, 0);
}

#[test]
fn tick_basic_actions_runs_tile_specials_before_skipping_idle_players() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.hp = 1_000;
    assert!(world.spawn_character(player, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

    assert!(world.tick_basic_actions().is_empty());

    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(player.hp, 950);
}

#[test]
fn queued_self_spell_executes_from_player_queue_when_idle() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.x = 10;
    caster.y = 10;
    caster.values[0][CharacterValue::MagicShield as usize] = 10;
    caster.mana = 10 * POWERSCALE;
    world.add_character(caster);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.driver_selfspell(PlayerActionCode::MagicShield);

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        caster.action,
        action::MAGICSHIELD,
        "C run_queue starts the queued spell before the idle action"
    );
    assert!(player.queue.is_empty(), "started tasks leave the queue");
}

#[test]
fn queued_bless_waits_for_mana_like_c_error_state_mana() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.x = 10;
    caster.y = 10;
    caster.values[0][CharacterValue::Bless as usize] = 10;
    caster.mana = 0;
    world.add_character(caster);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.driver_charspell(PlayerActionCode::Bless, CharacterId(1), 1);

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE, "no mana: the idle action runs");
    assert_eq!(player.queue.len(), 1, "mana-low bless stays queued");

    if let Some(caster) = world.characters.get_mut(&CharacterId(1)) {
        caster.mana = 10 * POWERSCALE;
        caster.action = 0;
        caster.duration = 0;
        caster.step = 0;
    }
    assert!(world.apply_player_action_setup(&mut player, 1));
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert!(
        caster.action == action::BLESS_SELF || caster.action == action::BLESS1,
        "bless runs once mana is available"
    );
    assert!(player.queue.is_empty());
}

#[test]
fn queued_invalid_spell_is_discarded_and_idle_continues() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.x = 10;
    caster.y = 10;
    world.add_character(caster);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    // No fireball skill: C error_state discards the queued task.
    player.driver_mapspell(PlayerActionCode::Fireball, 12, 10);

    assert!(world.apply_player_action_setup(&mut player, 1));

    assert!(player.queue.is_empty(), "failed spells drop from the queue");
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE);
}

#[test]
fn player_move_paths_through_closed_door_and_opens_it() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    // Wall with a closed usable door at (11,10).
    for y in 0..MAX_MAP {
        if y != 10 {
            world.map.set_flags(11, y, MapFlags::MOVEBLOCK);
        }
    }
    let mut door = item(900, ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.x = 11;
    door.y = 10;
    world.map.tile_mut(11, 10).unwrap().item = 900;
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::DOOR | MapFlags::TMOVEBLOCK);
    world.items.insert(ItemId(900), door);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Move,
        arg1: 13,
        arg2: 10,
    };

    assert!(
        world.apply_player_action_setup(&mut player, 1),
        "clicking beyond a closed door still starts an action"
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        character.action,
        action::USE,
        "bumping into the closed door on the path uses (opens) it like C"
    );
    assert_eq!(character.act1, 900, "the door item is the use target");
}

#[test]
fn earthmud_extra_movement_cost_sums_own_tile_effects_scaled_by_edemon_reduction() {
    let mut world = World::default();
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    walker.values[0][CharacterValue::Demon as usize] = 4;
    world.add_character(walker);

    // Only the walker's own tile (10,10) should count - a second earthmud
    // effect on an adjacent tile (11,10) must not contribute.
    world.create_earthmud_effect(10, 10, 30);
    world.create_earthmud_effect(11, 10, 30);

    // C `edemon_reduction(cn, 30) = max(0, 30 - 4) = 26`, doubled per C
    // `do_walk`'s `edemon_reduction(cn, ef[fn].strength) * 2`.
    assert_eq!(world.earthmud_extra_movement_cost(CharacterId(1)), 52);
}

#[test]
fn earthmud_extra_movement_cost_is_zero_for_earth_demons() {
    let mut world = World::default();
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    walker.flags |= CharacterFlags::EDEMON;
    world.add_character(walker);
    world.create_earthmud_effect(10, 10, 30);

    // C `do_walk`'s `if (!(ch[cn].flags & CF_EDEMON))` gate skips the whole
    // scan for earth demons - they aren't slowed by their own mud spell.
    assert_eq!(world.earthmud_extra_movement_cost(CharacterId(1)), 0);
}

#[test]
fn setup_walk_direction_slows_down_when_standing_in_earthmud() {
    let mut world = World::default();
    let mut walker = character(1);
    walker.x = 10;
    walker.y = 10;
    world.add_character(walker);
    world.create_earthmud_effect(10, 10, 5);

    assert!(world.setup_walk_direction(CharacterId(1), Direction::Right, 1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    // base cost 8 + edemon_reduction(5, 0) * 2 = 8 + 10 = 18.
    assert_eq!(character.duration, speed_ticks(0, SpeedMode::Normal, 18));
}
