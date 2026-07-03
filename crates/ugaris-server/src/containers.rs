use super::*;

pub(crate) fn generic_container_item_ids(world: &World, container_id: ItemId) -> Vec<ItemId> {
    let mut items = world
        .items
        .values()
        .filter(|item| {
            item.contained_in == Some(container_id) && item.flags.contains(ItemFlags::USED)
        })
        .map(|item| item.id)
        .collect::<Vec<_>>();
    items.sort_unstable_by_key(|id| id.0);
    items.truncate(LEGACY_CONTAINER_SIZE);
    items
}

pub(crate) fn generic_container_payload(world: &World, container_id: ItemId) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    let name = world
        .items
        .get(&container_id)
        .map(|item| {
            if item.description.is_empty() {
                item.name.as_str()
            } else {
                item.description.as_str()
            }
        })
        .unwrap_or("Container");
    builder
        .container_type(1)
        .container_name(name)
        .container_count(LEGACY_CONTAINER_SIZE as u8);

    let item_ids = generic_container_item_ids(world, container_id);
    for slot in 0..LEGACY_CONTAINER_SIZE {
        let sprite = item_ids
            .get(slot)
            .and_then(|item_id| world.items.get(item_id))
            .map(|item| item.sprite.max(0) as u32)
            .unwrap_or(0);
        builder.container_item(slot as u8, sprite);
    }
    builder.into_payload()
}

pub(crate) fn current_container_payload(
    world: &World,
    depot: Option<&AccountDepotState>,
    character_id: CharacterId,
) -> Option<bytes::BytesMut> {
    let container_id = world.characters.get(&character_id)?.current_container?;
    let container = world.items.get(&container_id)?;
    if container.driver == IDR_ACCOUNT_DEPOT {
        depot.map(account_depot_payload)
    } else if container.content_id != 0 {
        Some(generic_container_payload(world, container_id))
    } else {
        None
    }
}

pub(crate) fn check_current_container(world: &mut World, character_id: CharacterId) -> bool {
    let Some(character) = world.characters.get(&character_id) else {
        return false;
    };
    let Some(container_id) = character.current_container else {
        return false;
    };
    if character.action != action::IDLE && character.action != action::BLESS_SELF {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.current_container = None;
        }
        return false;
    }

    let valid = world.items.get(&container_id).is_some_and(|item| {
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        if item.driver != IDR_ACCOUNT_DEPOT
            && item.content_id == 0
            && !item.flags.contains(ItemFlags::DEPOT)
        {
            return false;
        }

        if item.x != 0 || item.y != 0 {
            let Ok(direction) = Direction::try_from(character.dir) else {
                return false;
            };
            let (dx, dy) = direction.delta();
            let x = i32::from(character.x) + i32::from(dx);
            let y = i32::from(character.y) + i32::from(dy);
            if x < 1 || y < 1 {
                return false;
            }
            return world
                .map
                .tile(x as usize, y as usize)
                .is_some_and(|tile| tile.item == container_id.0);
        }

        true
    });

    if !valid {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.current_container = None;
        }
    }
    valid
}

pub(crate) fn apply_item_container_command(
    world: &mut World,
    character_id: CharacterId,
    action: &ClientAction,
) -> AccountDepotCommandResult {
    if !check_current_container(world, character_id) {
        return AccountDepotCommandResult::Ignored;
    }
    let Some(container_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.current_container)
    else {
        return AccountDepotCommandResult::Ignored;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| item.content_id == 0 || item.driver == IDR_ACCOUNT_DEPOT)
    {
        return AccountDepotCommandResult::Ignored;
    }

    match *action {
        ClientAction::Container { slot, fast } => {
            apply_item_container_swap(world, character_id, container_id, usize::from(slot), fast)
        }
        ClientAction::LookContainer { slot } => {
            let Some(character) = world.characters.get(&character_id) else {
                return AccountDepotCommandResult::Ignored;
            };
            generic_container_item_ids(world, container_id)
                .get(usize::from(slot))
                .and_then(|item_id| world.items.get(item_id))
                .map(|item| legacy_item_look_text(item, character))
                .filter(|text| !text.is_empty())
                .map(AccountDepotCommandResult::Look)
                .unwrap_or(AccountDepotCommandResult::Ignored)
        }
        _ => AccountDepotCommandResult::Ignored,
    }
}

pub(crate) fn apply_item_container_swap(
    world: &mut World,
    character_id: CharacterId,
    container_id: ItemId,
    slot: usize,
    fast: bool,
) -> AccountDepotCommandResult {
    if slot >= LEGACY_CONTAINER_SIZE {
        return AccountDepotCommandResult::Ignored;
    }

    let cursor_id = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    if cursor_id.is_some_and(|item_id| {
        world
            .items
            .get(&item_id)
            .is_some_and(|item| item.flags.contains(ItemFlags::QUEST))
    }) {
        return AccountDepotCommandResult::Blocked(
            "You cannot store quest items in a container.".to_string(),
        );
    }

    let withdrawn_id = generic_container_item_ids(world, container_id)
        .get(slot)
        .copied();
    if cursor_id.is_none() && withdrawn_id.is_none() {
        return AccountDepotCommandResult::Ignored;
    }

    if let Some(item_id) = cursor_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = None;
            item.contained_in = Some(container_id);
            item.x = 0;
            item.y = 0;
        }
    }
    if let Some(item_id) = withdrawn_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = Some(character_id);
            item.contained_in = None;
            item.x = 0;
            item.y = 0;
        }
    }

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = withdrawn_id;
        if fast {
            if let Some(item_id) = withdrawn_id {
                if let Some(slot) = character
                    .inventory
                    .iter_mut()
                    .skip(INVENTORY_START_INVENTORY)
                    .find(|slot| slot.is_none())
                {
                    *slot = Some(item_id);
                    character.cursor_item = None;
                }
            }
        }
        character.flags.insert(CharacterFlags::ITEMS);
    }

    AccountDepotCommandResult::Changed
}
