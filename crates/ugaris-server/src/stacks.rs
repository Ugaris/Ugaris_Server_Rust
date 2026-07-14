use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NomadStackApplyResult {
    Split {
        left: u32,
        right: u32,
        unit: &'static str,
    },
    Merged {
        count: u32,
        unit: &'static str,
    },
    CannotSplitOne {
        unit: &'static str,
    },
    CannotMix,
    EnhanceNeedsSilver,
    EnhanceNeedsGold,
    EnhanceNotEnough {
        material: String,
        need: u32,
    },
    EnhanceConfirmUnusable,
    Enhanced {
        used: u32,
        target_name: String,
    },
    Bug(&'static str),
    MissingPlayer,
    MissingItem,
}

pub(crate) fn enhance_xmas_item(item: &mut Item, rng: &mut XmasTreeRng) {
    item.modifier_index.fill(0);
    item.modifier_value.fill(0);

    let mut available = XMAS_ENHANCE_SKILLS.to_vec();
    let num_skills = (rng.random(XMAS_MAX_SKILLS) + 1).min(item.modifier_index.len());
    let mut immunity_selected = false;

    for slot in 0..num_skills.min(XMAS_MAX_SKILLS) {
        if available.is_empty() {
            break;
        }
        let selected = rng.random(available.len());
        let skill = available.swap_remove(selected);
        if skill == CharacterValue::Immunity {
            immunity_selected = true;
        }
        let value = random_xmas_skill_value(rng);
        if value > 0 {
            item.modifier_index[slot] = skill as i16;
            item.modifier_value[slot] = value;
        }
    }

    if !immunity_selected && num_skills < item.modifier_index.len() && num_skills < XMAS_MAX_SKILLS
    {
        let special = random_xmas_special_value(rng);
        if special > 0 {
            item.modifier_index[num_skills] = CharacterValue::Immunity as i16;
            item.modifier_value[num_skills] = special;
        }
    }
}

pub(crate) fn apply_nomad_stack(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
) -> NomadStackApplyResult {
    let Some((kind, unit, template)) = world.items.get(&item_id).and_then(|item| {
        stack_kind(item).map(|kind| (kind, stack_unit(kind), stack_template(kind)))
    }) else {
        return NomadStackApplyResult::Bug(
            if world
                .items
                .get(&item_id)
                .is_some_and(|item| item.driver == IDR_DEMONCHIP)
            {
                "Bug #1445y"
            } else {
                "Bug #1442y"
            },
        );
    };
    let Some(character) = world.characters.get(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
    {
        return NomadStackApplyResult::MissingItem;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return split_nomad_stack(world, loader, item_id, character_id, kind, unit, template);
    };
    if cursor_item_id == item_id {
        return NomadStackApplyResult::MissingItem;
    }
    let Some(cursor_kind) = world.items.get(&cursor_item_id).and_then(stack_kind) else {
        if matches!(kind, StackKind::SilverUnit | StackKind::GoldUnit) {
            return apply_enhance_material(world, item_id, cursor_item_id, character_id, kind);
        }
        return NomadStackApplyResult::CannotMix;
    };
    if cursor_kind != kind {
        if matches!(kind, StackKind::SilverUnit | StackKind::GoldUnit) {
            return apply_enhance_material(world, item_id, cursor_item_id, character_id, kind);
        }
        return NomadStackApplyResult::CannotMix;
    }
    let cursor_value = world
        .items
        .get(&cursor_item_id)
        .map(|item| item.value)
        .unwrap_or_default();
    let cursor_count = world
        .items
        .get(&cursor_item_id)
        .map(stack_count)
        .unwrap_or_default();
    let Some(item) = world.items.get_mut(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    item.value = item.value.saturating_add(cursor_value);
    let count = stack_count(item).saturating_add(cursor_count);
    set_stack_count(item, count, kind);
    world.items.remove(&cursor_item_id);

    let Some(character) = world.characters.get_mut(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    NomadStackApplyResult::Merged { count, unit }
}

pub(crate) fn apply_enhance_material(
    world: &mut World,
    material_id: ItemId,
    target_id: ItemId,
    character_id: CharacterId,
    material_kind: StackKind,
) -> NomadStackApplyResult {
    let Some(target) = world.items.get(&target_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    if target.flags.contains(ItemFlags::NOENHANCE) {
        return NomadStackApplyResult::CannotMix;
    }
    let Some(enhanced_sprite) = enhance_item_sprite(target.sprite) else {
        return NomadStackApplyResult::CannotMix;
    };
    let needs_gold = (59200..59299).contains(&enhanced_sprite) || enhanced_sprite == 59474;
    if needs_gold && material_kind != StackKind::GoldUnit {
        return NomadStackApplyResult::EnhanceNeedsGold;
    }
    if !needs_gold && material_kind != StackKind::SilverUnit {
        return NomadStackApplyResult::EnhanceNeedsSilver;
    }

    let need = enhance_item_price(target);
    let material_count = world
        .items
        .get(&material_id)
        .map(stack_count)
        .unwrap_or_default();
    let material_name = world
        .items
        .get(&material_id)
        .map(|item| item.name.clone())
        .unwrap_or_else(|| "material".to_string());
    if need > material_count {
        return NomadStackApplyResult::EnhanceNotEnough {
            material: material_name,
            need,
        };
    }

    let Some(character) = world.characters.get(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    if enhance_would_make_unusable(target, character) {
        let now = current_realtime_seconds();
        let confirmed = world.items.get(&material_id).is_some_and(|material| {
            read_driver_data_u32(material, 8) == target_id.0
                && now.saturating_sub(read_driver_data_u32(material, 12)) <= 15
        });
        if !confirmed {
            if let Some(material) = world.items.get_mut(&material_id) {
                write_driver_data_u32(material, 8, target_id.0);
                write_driver_data_u32(material, 12, now);
            }
            return NomadStackApplyResult::EnhanceConfirmUnusable;
        }
    }

    let price = world
        .items
        .get(&material_id)
        .map(|material| material.value.saturating_mul(need) / material_count.max(1))
        .unwrap_or_default();
    let remaining = material_count.saturating_sub(need);
    if remaining < 1 {
        world.items.remove(&material_id);
        if let Some(character) = world.characters.get_mut(&character_id) {
            for slot in character.inventory.iter_mut() {
                if *slot == Some(material_id) {
                    *slot = None;
                }
            }
        }
    } else if let Some(material) = world.items.get_mut(&material_id) {
        material.value = material.value.saturating_sub(price);
        set_stack_count(material, remaining, material_kind);
    }

    let Some(target) = world.items.get_mut(&target_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    target.sprite = enhanced_sprite;
    target.value = target.value.saturating_add(price);
    for slot in 0..ugaris_core::entity::MAX_MODIFIERS {
        if target.modifier_value[slot] == 0 {
            continue;
        }
        match target.modifier_index[slot] {
            index
                if index == -(CharacterValue::ArmorSkill as i16)
                    || index == -(CharacterValue::Dagger as i16)
                    || index == -(CharacterValue::Staff as i16)
                    || index == -(CharacterValue::Sword as i16)
                    || index == -(CharacterValue::TwoHand as i16) =>
            {
                target.modifier_value[slot] = target.modifier_value[slot].saturating_add(10);
            }
            index if index == CharacterValue::Armor as i16 => {
                target.modifier_value[slot] =
                    target.modifier_value[slot].saturating_add(armor_bonus(target));
            }
            index if index == CharacterValue::Weapon as i16 => {
                target.modifier_value[slot] = target.modifier_value[slot].saturating_add(10);
            }
            index if index >= 0 && target.modifier_value[slot] < 20 => {
                target.modifier_value[slot] += 1;
            }
            _ => {}
        }
    }
    let target_name = target.name.clone();

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.flags.insert(CharacterFlags::ITEMS);
    }
    NomadStackApplyResult::Enhanced {
        used: need,
        target_name,
    }
}

pub(crate) fn enhance_item_sprite(sprite: i32) -> Option<i32> {
    let sprite = if (10120..=10159).contains(&sprite) {
        sprite - 10120 + 59300
    } else if (10190..=10199).contains(&sprite) {
        sprite - 10190 + 59340
    } else if (10220..=10229).contains(&sprite) {
        sprite - 10220 + 59350
    } else if (10250..=10259).contains(&sprite) {
        sprite - 10250 + 59360
    } else if (10280..=10289).contains(&sprite) {
        sprite - 10280 + 59370
    } else if (59300..=59399).contains(&sprite) {
        sprite - 59300 + 59200
    } else {
        match sprite {
            50510 => 59388,
            50025 => 59380,
            50026 => 59381,
            50122 => 59382,
            50123 => 59383,
            50124 => 59384,
            50125 => 59385,
            50126 => 59386,
            50141 => 59387,
            50512 => 59389,
            50513 => 59390,
            51084 => 59473,
            51617 => 59299,
            59299 => 59291,
            59473 => 59474,
            _ => return None,
        }
    };
    Some(sprite)
}

pub(crate) fn enhance_item_price(item: &Item) -> u32 {
    enhance_item_max_modifier(item).saturating_mul(100) + 100
}

pub(crate) fn enhance_item_max_modifier(item: &Item) -> u32 {
    let mut max_modifier = 0_u32;
    for slot in 0..ugaris_core::entity::MAX_MODIFIERS {
        match item.modifier_index[slot] {
            index
                if index == CharacterValue::Weapon as i16
                    || index == CharacterValue::Armor as i16
                    || index == CharacterValue::Speed as i16
                    || index == CharacterValue::Demon as i16
                    || index == CharacterValue::Light as i16 => {}
            index if index >= 0 => {
                max_modifier = max_modifier.max(item.modifier_value[slot].max(0) as u32);
            }
            _ => {}
        }
    }
    max_modifier
}

pub(crate) fn enhance_would_make_unusable(item: &Item, character: &Character) -> bool {
    for slot in 0..ugaris_core::entity::MAX_MODIFIERS {
        let value = item.modifier_value[slot];
        if value == 0 {
            continue;
        }
        let required_skill = match item.modifier_index[slot] {
            index if index == -(CharacterValue::ArmorSkill as i16) => CharacterValue::ArmorSkill,
            index if index == -(CharacterValue::Dagger as i16) => CharacterValue::Dagger,
            index if index == -(CharacterValue::Staff as i16) => CharacterValue::Staff,
            index if index == -(CharacterValue::Sword as i16) => CharacterValue::Sword,
            index if index == -(CharacterValue::TwoHand as i16) => CharacterValue::TwoHand,
            _ => continue,
        };
        let effective = character
            .values
            .get(1)
            .and_then(|values| values.get(required_skill as usize))
            .copied()
            .unwrap_or_default();
        if effective < value + 10 {
            return true;
        }
    }
    false
}

// The WNARMS/WNLEGS branches are identical on purpose, mirroring the C
// original's separate per-slot cases.
#[allow(clippy::if_same_then_else)]
pub(crate) fn armor_bonus(item: &Item) -> i16 {
    if item.flags.contains(ItemFlags::WNHEAD) {
        40
    } else if item.flags.contains(ItemFlags::WNARMS) {
        30
    } else if item.flags.contains(ItemFlags::WNLEGS) {
        30
    } else if item.flags.contains(ItemFlags::WNBODY) {
        100
    } else {
        0
    }
}

pub(crate) fn read_driver_data_u32(item: &Item, offset: usize) -> u32 {
    let mut bytes = [0_u8; 4];
    for (idx, byte) in item.driver_data.iter().skip(offset).take(4).enumerate() {
        bytes[idx] = *byte;
    }
    u32::from_le_bytes(bytes)
}

pub(crate) fn write_driver_data_u32(item: &mut Item, offset: usize, value: u32) {
    if item.driver_data.len() < offset + 4 {
        item.driver_data.resize(offset + 4, 0);
    }
    item.driver_data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn current_realtime_seconds() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(u64::from(u32::MAX)) as u32)
        .unwrap_or_default()
}

pub(crate) fn split_nomad_stack(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    stack_kind: StackKind,
    unit: &'static str,
    template: &'static str,
) -> NomadStackApplyResult {
    let Some(item) = world.items.get(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    let old_count = stack_count(item);
    if old_count < 2 {
        return NomadStackApplyResult::CannotSplitOne { unit };
    }
    let right = stack_split_amount(old_count / 2);
    let left = old_count - right;
    let old_value = item.value;

    let Ok(mut split_item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return NomadStackApplyResult::Bug("Bug #3199i");
    };
    split_item.value = old_value.saturating_mul(right) / old_count;
    set_stack_count(&mut split_item, right, stack_kind);
    let split_item_id = split_item.id;

    let Some(item) = world.items.get_mut(&item_id) else {
        return NomadStackApplyResult::MissingItem;
    };
    item.value = old_value.saturating_mul(left) / old_count;
    set_stack_count(item, left, stack_kind);

    let Some(character) = world.characters.get_mut(&character_id) else {
        return NomadStackApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return NomadStackApplyResult::MissingItem;
    }
    character.cursor_item = Some(split_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(split_item);
    NomadStackApplyResult::Split { left, right, unit }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackKind {
    Salt,
    Skin1,
    Skin2,
    BronzeChip,
    SilverChip,
    GoldChip,
    SilverUnit,
    GoldUnit,
}

pub(crate) fn stack_kind(item: &Item) -> Option<StackKind> {
    if item.driver == IDR_ENHANCE {
        return match item.driver_data.first().copied() {
            Some(1) => Some(StackKind::SilverUnit),
            Some(2) => Some(StackKind::GoldUnit),
            _ => None,
        };
    }

    match item.template_id {
        IID_AREA19_SALT => Some(StackKind::Salt),
        IID_AREA19_WOLFSSKIN => Some(StackKind::Skin1),
        IID_AREA19_WOLFSSKIN2 => Some(StackKind::Skin2),
        IID_BRONZECHIP => Some(StackKind::BronzeChip),
        IID_SILVERCHIP => Some(StackKind::SilverChip),
        IID_GOLDCHIP => Some(StackKind::GoldChip),
        _ => None,
    }
}

pub(crate) fn stack_template(kind: StackKind) -> &'static str {
    match kind {
        StackKind::Salt => "salt",
        StackKind::Skin1 => "skin1",
        StackKind::Skin2 => "skin2",
        StackKind::BronzeChip => "bronzechip",
        StackKind::SilverChip => "silverchip",
        StackKind::GoldChip => "goldchip",
        StackKind::SilverUnit => "silver",
        StackKind::GoldUnit => "gold",
    }
}

pub(crate) fn stack_unit(kind: StackKind) -> &'static str {
    match kind {
        StackKind::Salt => "ounce",
        StackKind::Skin1 | StackKind::Skin2 => "skin",
        StackKind::BronzeChip | StackKind::SilverChip | StackKind::GoldChip => "chip",
        StackKind::SilverUnit | StackKind::GoldUnit => "unit",
    }
}

pub(crate) fn stack_split_amount(mut amount: u32) -> u32 {
    for step in [10000, 5000, 2500, 1000, 500, 250, 100, 50, 25, 10] {
        if amount >= step {
            amount = step;
            break;
        }
    }
    amount
}

pub(crate) fn stack_count(item: &Item) -> u32 {
    let offset = stack_kind(item).map(stack_count_offset).unwrap_or_default();
    let mut bytes = [0_u8; 4];
    for (idx, byte) in item.driver_data.iter().skip(offset).take(4).enumerate() {
        bytes[idx] = *byte;
    }
    u32::from_le_bytes(bytes)
}

pub(crate) fn stack_count_offset(kind: StackKind) -> usize {
    match kind {
        StackKind::SilverUnit | StackKind::GoldUnit => 1,
        _ => 0,
    }
}

pub(crate) fn set_stack_count(item: &mut Item, count: u32, kind: StackKind) {
    let offset = stack_count_offset(kind);
    if item.driver_data.len() < offset + 4 {
        item.driver_data.resize(offset + 4, 0);
    }
    item.driver_data[offset..offset + 4].copy_from_slice(&count.to_le_bytes());
    match kind {
        StackKind::Salt => {
            item.sprite = if count >= 10000 {
                13212
            } else if count >= 1000 {
                13211
            } else if count >= 100 {
                13210
            } else if count >= 10 {
                13209
            } else {
                13208
            };
            item.description = format!("{count} ounces of {}.", item.name);
        }
        StackKind::Skin1 => {
            item.sprite = skin_stack_sprite(count, 59655);
            item.description = format!("{count} {}s.", item.name);
        }
        StackKind::Skin2 => {
            item.sprite = skin_stack_sprite(count, 59660);
            item.description = format!("{count} {}s.", item.name);
        }
        StackKind::BronzeChip => set_chip_stack_data(item, count, 0),
        StackKind::SilverChip => set_chip_stack_data(item, count, 12),
        StackKind::GoldChip => set_chip_stack_data(item, count, 6),
        StackKind::SilverUnit | StackKind::GoldUnit => {
            item.description = format!("{count} units of {}.", item.name);
        }
    }
}

pub(crate) fn set_chip_stack_data(item: &mut Item, count: u32, sprite_offset: i32) {
    item.sprite = if count > 5 {
        53012 + sprite_offset
    } else if count == 5 {
        53011 + sprite_offset
    } else if count == 4 {
        53010 + sprite_offset
    } else if count == 3 {
        53009 + sprite_offset
    } else if count == 2 {
        53008 + sprite_offset
    } else {
        53007 + sprite_offset
    };
    item.description = if count > 1 {
        format!("{count} {}s.", item.name)
    } else {
        format!("{count} {}.", item.name)
    };
}

pub(crate) fn skin_stack_sprite(count: u32, base: i32) -> i32 {
    if count >= 5 {
        base + 4
    } else if count >= 4 {
        base + 3
    } else if count >= 3 {
        base + 2
    } else if count >= 2 {
        base + 1
    } else {
        base
    }
}
