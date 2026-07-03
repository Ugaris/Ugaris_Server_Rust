use super::*;

pub(crate) const RANDCHEST_COOLDOWN_SECONDS: u64 = 60 * 60 * 24;

pub(crate) const RATCHEST_COOLDOWN_SECONDS: u64 = 60 * 60 * 23;

pub(crate) const RATCHEST_TREASURE_RESPAWN_SECONDS: u64 = 60 * 60 * 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChestTreasureApplyResult {
    Granted {
        item_name: String,
        key_name: Option<String>,
    },
    Empty,
    KeyRequired,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RandomChestApplyResult {
    Money { amount: u32 },
    Item { item_name: String },
    Empty,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RatChestApplyResult {
    Money { amount: u32 },
    Treasure { item_name: String },
    Empty,
    CursorOccupied,
    MissingPlayer,
}

pub(crate) fn grant_chest_treasure(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    treasure_index: u8,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }

    let key = format!("treasure_{treasure_index}");
    let Ok(item) = loader.instantiate_item_template(&key, Some(character_id)) else {
        return None;
    };
    let item_id = item.id;
    let item_name = item.name.clone();

    let Some(character) = world.characters.get_mut(&character_id) else {
        return None;
    };
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn apply_chest_treasure(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    chest_item_id: ItemId,
    character_id: CharacterId,
    treasure_index: u8,
    realtime_seconds: u64,
) -> ChestTreasureApplyResult {
    let required_key_id = world
        .items
        .get(&chest_item_id)
        .map(chest_required_key_id)
        .unwrap_or_default();
    let key_name = if required_key_id == 0 {
        None
    } else {
        match chest_key_name(world, character_id, required_key_id).or_else(|| {
            player
                .as_deref()
                .and_then(|player| player.keyring_key_name(required_key_id))
                .map(str::to_string)
        }) {
            Some(name) => Some(name),
            None => return ChestTreasureApplyResult::KeyRequired,
        }
    };

    let Some(character) = world.characters.get(&character_id) else {
        return ChestTreasureApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return ChestTreasureApplyResult::CursorOccupied;
    }

    let Some(player) = player else {
        return ChestTreasureApplyResult::MissingPlayer;
    };

    let timeout_seconds = world
        .items
        .get(&chest_item_id)
        .map(chest_timeout_seconds)
        .unwrap_or_default();
    let last_access = player.chest_last_access_seconds(treasure_index);
    if last_access != 0
        && timeout_seconds != 0
        && last_access.saturating_add(timeout_seconds) > realtime_seconds
    {
        return ChestTreasureApplyResult::Empty;
    }

    let required_deaths = world
        .items
        .get(&chest_item_id)
        .map(chest_required_deaths)
        .unwrap_or_default();
    if required_deaths != 0
        && world
            .characters
            .get(&character_id)
            .is_none_or(|character| character.deaths < u32::from(required_deaths))
    {
        return ChestTreasureApplyResult::Empty;
    }

    match grant_chest_treasure(world, loader, character_id, treasure_index) {
        Some(item_name) => {
            player.mark_chest_access(treasure_index, realtime_seconds);
            player.record_chest_opened(treasure_index);
            ChestTreasureApplyResult::Granted {
                item_name,
                key_name,
            }
        }
        None => ChestTreasureApplyResult::Empty,
    }
}

pub(crate) fn chest_timeout_seconds(item: &ugaris_core::entity::Item) -> u64 {
    let low = item.driver_data.get(5).copied().unwrap_or_default();
    let high = item.driver_data.get(6).copied().unwrap_or_default();
    u64::from(u16::from_le_bytes([low, high])) * 60 * 60
}

pub(crate) fn chest_required_deaths(item: &ugaris_core::entity::Item) -> u8 {
    item.driver_data.get(7).copied().unwrap_or_default()
}

pub(crate) fn chest_blocked_message(
    world: &World,
    item_id: ItemId,
    character_id: CharacterId,
) -> &'static str {
    if world
        .items
        .get(&item_id)
        .is_some_and(|item| chest_required_key_id(item) != 0)
    {
        return CHEST_KEY_REQUIRED_MESSAGE;
    }
    if world
        .characters
        .get(&character_id)
        .is_some_and(|character| character.cursor_item.is_some())
    {
        return CHEST_CURSOR_OCCUPIED_MESSAGE;
    }
    CHEST_EMPTY_MESSAGE
}

pub(crate) fn chest_required_key_id(item: &ugaris_core::entity::Item) -> u32 {
    let b1 = item.driver_data.get(1).copied().unwrap_or_default();
    let b2 = item.driver_data.get(2).copied().unwrap_or_default();
    let b3 = item.driver_data.get(3).copied().unwrap_or_default();
    let b4 = item.driver_data.get(4).copied().unwrap_or_default();
    u32::from_le_bytes([b1, b2, b3, b4])
}

pub(crate) fn chest_key_name(
    world: &World,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<String> {
    let character = world.characters.get(&character_id)?;
    if let Some(item_id) = character.cursor_item {
        if let Some(name) = chest_key_item_name(world, item_id, required_key_id) {
            return Some(name);
        }
    }
    for item_id in character.inventory.iter().skip(30).flatten() {
        if let Some(name) = chest_key_item_name(world, *item_id, required_key_id) {
            return Some(name);
        }
    }
    None
}

pub(crate) fn chest_key_item_name(
    world: &World,
    item_id: ItemId,
    required_key_id: u32,
) -> Option<String> {
    let item = world.items.get(&item_id)?;
    (item.template_id == required_key_id || item.template_id == IID_SKELETON_KEY)
        .then(|| item.name.clone())
}

pub(crate) fn infinite_chest_key_access(
    world: &World,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let character = world.characters.get(&character_id)?;
    let inventory_items = character.inventory.iter().skip(30).flatten().copied();
    for item_id in inventory_items {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    None
}

pub(crate) fn apply_random_chest(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    item_id: ItemId,
    character_id: CharacterId,
    area_id: u16,
    realtime_seconds: u64,
    random_seed: u64,
) -> RandomChestApplyResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return RandomChestApplyResult::CursorOccupied;
    }

    let Some(player) = player else {
        return RandomChestApplyResult::MissingPlayer;
    };
    let Some(chest) = world.items.get(&item_id) else {
        return RandomChestApplyResult::MissingPlayer;
    };
    let location_id = random_chest_location_id(chest.x, chest.y, area_id);
    if player
        .random_chest_last_used_seconds(location_id)
        .is_some_and(|last_used| {
            last_used.saturating_add(RANDCHEST_COOLDOWN_SECONDS) > realtime_seconds
        })
    {
        return RandomChestApplyResult::Empty;
    }

    let money_level = chest.driver_data.first().copied().unwrap_or_default();
    let loot_tier = chest.driver_data.get(1).copied().unwrap_or_default();
    player.mark_random_chest_used(location_id, realtime_seconds);

    if loot_tier == 0 && legacy_random(random_seed, 4) != 0 {
        return RandomChestApplyResult::Empty;
    }

    if let Some(template) = random_chest_loot_template(loot_tier, legacy_random(random_seed, 28)) {
        if let Some(item_name) =
            grant_template_item_to_cursor(world, loader, character_id, template)
        {
            player.record_chest_opened(0);
            return RandomChestApplyResult::Item { item_name };
        }
    }

    let amount = random_chest_money_amount(money_level, random_seed);
    if amount == 0 || !grant_money_to_cursor(world, loader, character_id, amount) {
        return RandomChestApplyResult::Empty;
    }
    player.record_chest_opened(0);
    RandomChestApplyResult::Money { amount }
}

pub(crate) fn apply_rat_chest(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    item_id: ItemId,
    character_id: CharacterId,
    area_id: u16,
    realtime_seconds: u64,
    random_seed: u64,
) -> RatChestApplyResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return RatChestApplyResult::CursorOccupied;
    }

    let Some(player) = player else {
        return RatChestApplyResult::MissingPlayer;
    };

    ensure_rat_chest_treasure(world, player, area_id, realtime_seconds, random_seed);

    let Some(chest) = world.items.get(&item_id) else {
        return RatChestApplyResult::MissingPlayer;
    };
    if chest.x == player.rat_chest_treasure_x && chest.y == player.rat_chest_treasure_y {
        player.rat_chest_treasure_x = 0;
        player.rat_chest_treasure_y = 0;
        player.rat_chest_last_treasure_seconds = realtime_seconds;
        if let Some(item_name) = grant_rat_chest_treasure(world, loader, character_id, random_seed)
        {
            player.record_chest_opened(0);
            return RatChestApplyResult::Treasure { item_name };
        }
        return RatChestApplyResult::Empty;
    }

    let location_id = random_chest_location_id(chest.x, chest.y, area_id);
    if player
        .rat_chest_last_used_seconds(location_id)
        .is_some_and(|last_used| {
            last_used.saturating_add(RATCHEST_COOLDOWN_SECONDS) > realtime_seconds
        })
    {
        return RatChestApplyResult::Empty;
    }
    player.mark_rat_chest_used(location_id, realtime_seconds);

    if legacy_random(random_seed, 4) != 0 {
        return RatChestApplyResult::Empty;
    }

    let money_level = chest.driver_data.first().copied().unwrap_or_default();
    let amount = random_chest_money_amount(money_level, random_seed);
    if amount == 0 || !grant_money_to_cursor(world, loader, character_id, amount) {
        return RatChestApplyResult::Empty;
    }
    player.record_chest_opened(0);
    RatChestApplyResult::Money { amount }
}

pub(crate) fn ensure_rat_chest_treasure(
    world: &World,
    player: &mut PlayerRuntime,
    area_id: u16,
    realtime_seconds: u64,
    random_seed: u64,
) {
    if player.rat_chest_treasure_x != 0
        || realtime_seconds.saturating_sub(player.rat_chest_last_treasure_seconds)
            < RATCHEST_TREASURE_RESPAWN_SECONDS
    {
        return;
    }
    let Some(character_id) = player.character_id else {
        return;
    };
    let Some(character) = world.characters.get(&character_id) else {
        return;
    };
    let chest_number = match character.level {
        37.. => 40,
        29.. => 30,
        21.. => 20,
        13.. => 10,
        _ => return,
    };
    let mut candidates = world
        .items
        .values()
        .filter(|item| {
            item.driver == IDR_RATCHEST
                && item.driver_data.first().copied() == Some(chest_number)
                && random_chest_location_id(item.x, item.y, area_id) >> 16 == u32::from(area_id)
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }
    candidates.sort_by_key(|item| item.id.0);
    let index = legacy_random(random_seed.wrapping_add(3), candidates.len() as u32) as usize;
    let selected = candidates[index];
    player.rat_chest_treasure_x = selected.x;
    player.rat_chest_treasure_y = selected.y;
}

pub(crate) fn grant_rat_chest_treasure(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    random_seed: u64,
) -> Option<String> {
    let character = world.characters.get(&character_id)?.clone();
    let (template, modifier) = match legacy_random(random_seed.wrapping_add(4), 3) {
        0 => (
            "sewer_ring",
            if character.flags.contains(CharacterFlags::MAGE)
                && !character.flags.contains(CharacterFlags::WARRIOR)
            {
                CharacterValue::MagicShield
            } else {
                CharacterValue::Parry
            },
        ),
        1 => (
            "sewer_ring",
            if character.flags.contains(CharacterFlags::MAGE)
                && !character.flags.contains(CharacterFlags::WARRIOR)
            {
                let fireball = character
                    .values
                    .get(1)
                    .and_then(|values| values.get(CharacterValue::Fireball as usize))
                    .copied()
                    .unwrap_or_default();
                let flash = character
                    .values
                    .get(1)
                    .and_then(|values| values.get(CharacterValue::Flash as usize))
                    .copied()
                    .unwrap_or_default();
                if fireball > flash {
                    CharacterValue::Fireball
                } else {
                    CharacterValue::Flash
                }
            } else {
                CharacterValue::Attack
            },
        ),
        _ => ("sewer_amulet", CharacterValue::Immunity),
    };
    let mut item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    item.modifier_index[0] = modifier as i16;
    item.modifier_value[0] = rat_chest_skill_value(&character);
    item.value = u32::from(item.modifier_value[0] as u16) * 300;
    let item_id = item.id;
    let item_name = item.name.clone();
    let character = world.characters.get_mut(&character_id)?;
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name.to_lowercase())
}

pub(crate) fn rat_chest_skill_value(character: &Character) -> i16 {
    let value = match character.level {
        0..=14 => 4,
        15..=16 => 5,
        17..=19 => 6,
        20..=22 => 7,
        23..=25 => 8,
        26..=29 => 9,
        30..=32 => 10,
        33..=35 => 11,
        _ => 12,
    };
    if character.flags.contains(CharacterFlags::ARCH) {
        value
    } else {
        value.min(9)
    }
}

pub(crate) fn random_chest_location_id(x: u16, y: u16, area_id: u16) -> u32 {
    u32::from(x) + (u32::from(y) << 8) + (u32::from(area_id) << 16)
}

pub(crate) fn random_chest_money_amount(level: u8, seed: u64) -> u32 {
    let level = u32::from(level);
    if level == 0 {
        return 0;
    }
    let first = legacy_random(seed.wrapping_add(1), level) + 1;
    let second = legacy_random(seed.wrapping_add(2), level) + 1;
    first.saturating_mul(second)
}

pub(crate) fn random_chest_loot_template(tier: u8, roll: u32) -> Option<&'static str> {
    if !(1..=10).contains(&tier) || !(21..=27).contains(&roll) {
        return None;
    }
    let potion_level = match tier {
        1 => match roll {
            21 => return Some("healing_potion1"),
            22 => return Some("mana_potion1"),
            23 => return Some("combo_potion1"),
            _ => 4,
        },
        2 | 3 => match roll {
            21 => return Some("healing_potion2"),
            22 => return Some("mana_potion2"),
            23 => return Some("combo_potion2"),
            _ => tier * 4,
        },
        4..=10 => match roll {
            21 => return Some("healing_potion3"),
            22 => return Some("mana_potion3"),
            23 => return Some("combo_potion3"),
            _ => tier * 4,
        },
        _ => return None,
    };

    match roll {
        24 => Some(match potion_level {
            4 => "sword4_potion",
            8 => "sword8_potion",
            12 => "sword12_potion",
            16 => "sword16_potion",
            20 => "sword20_potion",
            24 => "sword24_potion",
            28 => "sword28_potion",
            32 => "sword32_potion",
            36 => "sword36_potion",
            _ => "sword40_potion",
        }),
        25 => Some(match potion_level {
            4 => "twohand4_potion",
            8 => "twohand8_potion",
            12 => "twohand12_potion",
            16 => "twohand16_potion",
            20 => "twohand20_potion",
            24 => "twohand24_potion",
            28 => "twohand28_potion",
            32 => "twohand32_potion",
            36 => "twohand36_potion",
            _ => "twohand40_potion",
        }),
        26 => Some(match potion_level {
            4 => "flash4_potion",
            8 => "flash8_potion",
            12 => "flash12_potion",
            16 => "flash16_potion",
            20 => "flash20_potion",
            24 => "flash24_potion",
            28 => "flash28_potion",
            32 => "flash32_potion",
            36 => "flash36_potion",
            _ => "flash40_potion",
        }),
        27 => Some(match potion_level {
            4 => "immunity4_potion",
            8 => "immunity8_potion",
            12 => "immunity12_potion",
            16 => "immunity16_potion",
            20 => "immunity20_potion",
            24 => "immunity24_potion",
            28 => "immunity28_potion",
            32 => "immunity32_potion",
            36 => "immunity36_potion",
            _ => "immunity40_potion",
        }),
        _ => None,
    }
}
