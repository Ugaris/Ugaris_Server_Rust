use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InventoryCommandResult {
    Ignored,
    Changed,
    /// C `swap`'s `IF_MONEY` branch (`src/system/do.c:1276-1287`): the
    /// cursor held a money item, which was destroyed and its `price`
    /// (silver) credited straight to `character.gold` instead of being
    /// placed in the target slot. The caller must still refresh the
    /// inventory (money items never actually occupy a slot) and award
    /// the `achievement_add_gold_earned` wealth-ladder tail.
    MoneyConverted {
        price: u32,
    },
    ContainerOpened {
        account_depot: bool,
    },
    Look(String),
}

pub(crate) fn item_packet_fields(
    world: &World,
    item_id: ugaris_core::ids::ItemId,
) -> Option<(u32, u32)> {
    world.items.get(&item_id).map(|item| {
        let sprite = item.sprite.max(0) as u32;
        let flags = item.flags.bits() as u32;
        (sprite, flags)
    })
}

pub(crate) fn inventory_snapshot_payload(world: &World, character: &Character) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    let (cursor_sprite, cursor_flags) = character
        .cursor_item
        .and_then(|item_id| item_packet_fields(world, item_id))
        .unwrap_or((0, 0));
    builder.set_cursor_item(cursor_sprite, cursor_flags);

    for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
        let (sprite, flags) = character.inventory[slot]
            .and_then(|item_id| item_packet_fields(world, item_id))
            .unwrap_or((0, 0));
        builder.set_item(slot as u8, sprite, flags);
    }
    builder.gold(character.gold);

    builder.into_payload()
}

pub(crate) fn apply_inventory_client_action(
    world: &mut World,
    player: Option<&PlayerRuntime>,
    character_id: CharacterId,
    action: &ClientAction,
    area_id: u16,
) -> InventoryCommandResult {
    match *action {
        ClientAction::Swap { slot } => inventory_swap_slot(world, character_id, usize::from(slot)),
        ClientAction::LookInventory { slot } => {
            inventory_look_slot(world, character_id, usize::from(slot))
        }
        ClientAction::UseInventory { slot } => {
            inventory_use_slot(world, player, character_id, usize::from(slot), area_id)
        }
        ClientAction::LookItem { x, y } => look_map_item_text(world, character_id, x, y),
        _ => InventoryCommandResult::Ignored,
    }
}

pub(crate) fn inventory_swap_slot(
    world: &mut World,
    character_id: CharacterId,
    slot: usize,
) -> InventoryCommandResult {
    if !can_use_inventory_slot(slot) {
        return InventoryCommandResult::Ignored;
    }

    let Some((cursor_id, slot_id)) = world.characters.get(&character_id).map(|character| {
        let cursor_id = character
            .cursor_item
            .filter(|item_id| world.items.contains_key(item_id));
        let slot_id = character
            .inventory
            .get(slot)
            .copied()
            .flatten()
            .filter(|item_id| world.items.contains_key(item_id));
        (cursor_id, slot_id)
    }) else {
        return InventoryCommandResult::Ignored;
    };
    if cursor_id.is_none() && slot_id.is_none() {
        return InventoryCommandResult::Ignored;
    }

    // C `swap` (`src/system/do.c:1235`-1258`): placing a held item into a
    // worn slot (`pos < 12`) requires `can_wear` (slot flag match, level,
    // class, two-handed hand-conflict). Placing into the spell range
    // (`12..30`) is always illegal from a non-empty cursor - callers
    // already exclude that range via `can_use_inventory_slot`, matching
    // the `else` branch that only re-validates when the cursor is empty.
    if let Some(item_id) = cursor_id {
        if slot < 12 && !world.can_wear(character_id, item_id, slot) {
            return InventoryCommandResult::Ignored;
        }
    }

    // C `swap` (`src/system/do.c:1276-1287`): `it[in].flags & IF_MONEY`,
    // checked against the *original* cursor item (`in`) - a money item
    // held on the cursor never actually lands in the target slot; it's
    // destroyed on the spot (`destroy_money_item`) and its value credited
    // straight to `ch[cn].gold` instead.
    let money_price = cursor_id.and_then(|item_id| {
        world
            .items
            .get(&item_id)
            .filter(|item| item.flags.contains(ItemFlags::MONEY))
            .map(|item| item.value)
    });

    if let Some(item_id) = cursor_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = Some(character_id);
            item.contained_in = None;
            item.x = 0;
            item.y = 0;
        }
    }
    if let Some(item_id) = slot_id {
        if let Some(item) = world.items.get_mut(&item_id) {
            item.carried_by = Some(character_id);
            item.contained_in = None;
            item.x = 0;
            item.y = 0;
        }
    }

    if money_price.is_some() {
        if let Some(item_id) = cursor_id {
            world.items.remove(&item_id);
        }
    }

    let Some(character) = world.characters.get_mut(&character_id) else {
        return InventoryCommandResult::Ignored;
    };
    character.cursor_item = slot_id;
    character.inventory[slot] = if money_price.is_some() {
        None
    } else {
        cursor_id
    };
    character.flags.insert(CharacterFlags::ITEMS);
    if let Some(price) = money_price {
        character.gold = character.gold.saturating_add(price);
    }

    // C `swap` (`src/system/do.c:1216`): `if (pos < 12) update_char(cn);`
    // - only worn-slot swaps trigger a stat recompute.
    if slot < 12 {
        world.update_character(character_id);
    }

    if let Some(price) = money_price {
        InventoryCommandResult::MoneyConverted { price }
    } else {
        InventoryCommandResult::Changed
    }
}

pub(crate) fn inventory_look_slot(
    world: &World,
    character_id: CharacterId,
    slot: usize,
) -> InventoryCommandResult {
    if !can_use_inventory_slot(slot) {
        return InventoryCommandResult::Ignored;
    }
    let Some(character) = world.characters.get(&character_id) else {
        return InventoryCommandResult::Ignored;
    };
    character
        .inventory
        .get(slot)
        .copied()
        .flatten()
        .and_then(|item_id| world.items.get(&item_id))
        .map(|item| legacy_item_look_text(item, character))
        .filter(|text| !text.is_empty())
        .map(InventoryCommandResult::Look)
        .unwrap_or(InventoryCommandResult::Ignored)
}

/// C `cl_look_item` (`src/system/player.c:764`): bounds-check the target
/// tile, resolve the item sitting on `map[m].it`, gate visibility via
/// `char_see_item`, then build the same text `look_item(cn, it+in, -1)`
/// sends for inventory items (slot `-1` means "not an inventory slot").
pub(crate) fn look_map_item_text(
    world: &World,
    character_id: CharacterId,
    x: u16,
    y: u16,
) -> InventoryCommandResult {
    let (x, y) = (usize::from(x), usize::from(y));
    if !world.map.legacy_inner_bounds(x, y) {
        return InventoryCommandResult::Ignored;
    }

    let item_id = world
        .map
        .tile(x, y)
        .map(|tile| tile.item)
        .unwrap_or_default();
    if item_id == 0 {
        return InventoryCommandResult::Ignored;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return InventoryCommandResult::Ignored;
    };
    let Some(item) = world.items.get(&ugaris_core::ids::ItemId(item_id)) else {
        return InventoryCommandResult::Ignored;
    };

    if !ugaris_core::see::char_see_item(character, item, &world.map, world.date.daylight) {
        return InventoryCommandResult::Ignored;
    }

    let text = legacy_item_look_text(item, character);
    if text.is_empty() {
        InventoryCommandResult::Ignored
    } else {
        InventoryCommandResult::Look(text)
    }
}

pub(crate) fn inventory_use_slot(
    world: &mut World,
    player: Option<&PlayerRuntime>,
    character_id: CharacterId,
    slot: usize,
    area_id: u16,
) -> InventoryCommandResult {
    if !can_use_inventory_slot(slot) {
        return InventoryCommandResult::Ignored;
    }
    let Some(item_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.inventory.get(slot).copied().flatten())
    else {
        return InventoryCommandResult::Ignored;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| !item.flags.contains(ItemFlags::USE))
    {
        return InventoryCommandResult::Ignored;
    }

    let request = ItemUseRequest {
        character_id,
        item_id,
        spec: 0,
    };
    match world.use_item_request(request, true) {
        Ok(ugaris_core::item_driver::UseItemOutcome::OpenContainer { .. })
        | Ok(ugaris_core::item_driver::UseItemOutcome::OpenDepot { .. }) => {
            InventoryCommandResult::ContainerOpened {
                account_depot: false,
            }
        }
        Ok(ugaris_core::item_driver::UseItemOutcome::OpenAccountDepot { .. }) => {
            InventoryCommandResult::ContainerOpened {
                account_depot: true,
            }
        }
        Ok(ugaris_core::item_driver::UseItemOutcome::Dispatch(request)) => {
            let context = item_driver_context_for_request(world, player, &request);
            let outcome =
                world.execute_item_driver_request_with_context(request, area_id, &context);
            match outcome {
                ugaris_core::item_driver::ItemDriverOutcome::Unsupported { .. } => {
                    InventoryCommandResult::Ignored
                }
                _ => InventoryCommandResult::Changed,
            }
        }
        Err(_) => InventoryCommandResult::Ignored,
    }
}

pub(crate) fn inventory_sort(world: &mut World, character_id: CharacterId) -> bool {
    let Some(inventory) = world
        .characters
        .get(&character_id)
        .map(|character| character.inventory.clone())
    else {
        return false;
    };

    let mut sorted = inventory;
    sorted[INVENTORY_START_INVENTORY..].sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => {
            let left = world.items.get(left);
            let right = world.items.get(right);
            match (left, right) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(left), Some(right)) => right
                    .value
                    .cmp(&left.value)
                    .then_with(|| right.sprite.cmp(&left.sprite))
                    .then_with(|| {
                        left.name[..left.name.len().min(35)]
                            .cmp(&right.name[..right.name.len().min(35)])
                    }),
            }
        }
    });

    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    character.inventory = sorted;
    character.flags.insert(CharacterFlags::ITEMS);
    true
}

pub(crate) const IDR_FLASK: u16 = 32;

pub(crate) const IDR_BEYONDPOTION: u16 = 133;

pub(crate) const IID_HARDKILL: u32 = (1 << 24) | 0x00005D;

pub(crate) fn legacy_item_look_text(item: &Item, character: &Character) -> String {
    if item.name.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("{}:", item.name)];
    if !item.description.is_empty() {
        lines.push(item.description.clone());
    }
    if item.template_id == IID_HARDKILL {
        lines.push(format!(
            "This is a level {} holy weapon.",
            item.driver_data.get(37).copied().unwrap_or_default()
        ));
    }

    let mut showed_modifiers = false;
    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if value == 0 || index < 0 {
            continue;
        }
        if !showed_modifiers {
            lines.push("Modifiers:".to_string());
            showed_modifiers = true;
        }
        let name = value_name(index);
        if item.driver == IDR_DECAYITEM {
            lines.push(format!(
                "{} +{} (active: {:+})",
                name,
                value,
                item.driver_data.get(2).copied().unwrap_or_default() as i8
            ));
        } else if index == CharacterValue::Armor as i16 {
            lines.push(format!("{} {:+.2}", name, f32::from(value) / 20.0));
        } else {
            lines.push(format!("{} {:+}", name, value));
        }
    }

    let mut showed_requirements = false;
    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if value == 0 || index >= 0 {
            continue;
        }
        if !showed_requirements {
            lines.push("Requirements:".to_string());
            showed_requirements = true;
        }
        let required_index = -index;
        let current = character
            .values
            .get(1)
            .and_then(|values| values.get(required_index as usize))
            .copied()
            .unwrap_or_default();
        lines.push(format!(
            "{} {} (you have {})",
            value_name(required_index),
            value,
            current
        ));
    }
    if !showed_requirements && (item.min_level != 0 || item.max_level != 0 || item.needs_class != 0)
    {
        lines.push("Requirements:".to_string());
    }
    if item.min_level != 0 {
        lines.push(format!("Minimum Level: {}", item.min_level));
    }
    if item.max_level != 0 {
        lines.push(format!("Maximum Level: {}", item.max_level));
    }
    if item.needs_class & 1 != 0 {
        lines.push("Only usable by a Warrior.".to_string());
    }
    if item.needs_class & 2 != 0 {
        lines.push("Only usable by a Mage.".to_string());
    }
    if item.needs_class & 4 != 0 {
        lines.push("Only usable by a Seyan'Du.".to_string());
    }
    if item.needs_class & 8 != 0 {
        lines.push("Only usable by an Arch.".to_string());
    }

    if item.flags.contains(ItemFlags::BONDTAKE) {
        let target = if item.owner_id == character.id.0 as i32 {
            ("you", "you")
        } else {
            ("somebody else", "he")
        };
        lines.push(format!(
            "This item is bonded to {}. Only {} can take it.",
            target.0, target.1
        ));
    }
    if item.flags.contains(ItemFlags::BONDWEAR) {
        let target = if item.owner_id == character.id.0 as i32 {
            ("you", "you")
        } else {
            ("somebody else", "he")
        };
        lines.push(format!(
            "This item is bonded to {}. Only {} can wear it.",
            target.0, target.1
        ));
    }
    if item.flags.contains(ItemFlags::QUEST) {
        lines.push("This is a quest item. You cannot drop it or give it away.".to_string());
    }
    if item.flags.contains(ItemFlags::NOENHANCE) {
        lines.push(
            "This item resists magic, so you cannot enhance it using orbs, metals or shrines."
                .to_string(),
        );
    }
    if item.flags.contains(ItemFlags::BEYONDMAXMOD) {
        lines.push("This item goes beyond maximum modifier limits.".to_string());
    }

    if item.driver == IDR_FLASK && item.driver_data.get(2).copied().unwrap_or_default() != 0 {
        lines.push(format!(
            "Duration: {} minutes.",
            item.driver_data.get(3).copied().unwrap_or_default()
        ));
    }
    if item.driver == IDR_BEYONDPOTION {
        lines.push(format!(
            "Duration: {} minutes.",
            item.driver_data.first().copied().unwrap_or_default()
        ));
    }
    if item.driver == IDR_DECAYITEM {
        let used = u16::from_le_bytes([
            item.driver_data.get(3).copied().unwrap_or_default(),
            item.driver_data.get(4).copied().unwrap_or_default(),
        ]) as u32
            * 2;
        let max = u16::from_le_bytes([
            item.driver_data.get(5).copied().unwrap_or_default(),
            item.driver_data.get(6).copied().unwrap_or_default(),
        ]) as u32
            * 2;
        lines.push(format!(
            "Duration: {}:{:02}:{:02} of {}:{:02}:{:02} active time used up.",
            used / 3600,
            (used / 60) % 60,
            used % 60,
            max / 3600,
            (max / 60) % 60,
            max % 60
        ));
    }

    if (59200..59299).contains(&item.sprite) || item.sprite == 59474 {
        lines.push("The item has been gilded.".to_string());
    }
    if (59299..=59390).contains(&item.sprite) || item.sprite == 59473 {
        lines.push("The item has been silvered.".to_string());
    }
    if (53000..=53006).contains(&item.sprite) {
        lines.push("This is part of an earth demon suit.".to_string());
    }
    if (53025..=53030).contains(&item.sprite) {
        lines.push("This is part of an ice demon suit.".to_string());
    }
    if (53031..=53036).contains(&item.sprite) {
        lines.push("This is part of an fire demon suit.".to_string());
    }

    lines.join("\n")
}

pub(crate) fn value_name(index: i16) -> &'static str {
    CHARACTER_VALUE_NAMES
        .get(index as usize)
        .copied()
        .unwrap_or("Unknown")
}

pub(crate) fn next_runtime_item_id(world: &World) -> ItemId {
    let next = world
        .items
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or_default()
        .saturating_add(1)
        .max(1);
    ItemId(next)
}
