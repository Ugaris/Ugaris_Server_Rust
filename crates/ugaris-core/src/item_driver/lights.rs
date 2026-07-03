use super::*;

pub(crate) const V_LIGHT: i16 = 9;

pub(crate) const LIGHT_TIMER_TICKS: u64 = TICKS_PER_SECOND * 30;

pub(crate) fn toylight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(2, 0);
    if item.driver_data[0] != 0 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
    } else {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: None,
    }
}

pub(crate) fn nightlight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(2, 0);
    let was_on = item.driver_data[0] != 0;
    if was_on && context.daylight > 80 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
    } else if !was_on && context.daylight < 80 {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
    }
}

pub(crate) fn onofflight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    if context.timer_call && character.id.0 == 0 {
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }
        if item.driver_data[6] == 0 {
            item.driver_data[6] = 1;
            return ItemDriverOutcome::Noop;
        }
    }

    let now_on = if item.driver_data[0] != 0 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
        false
    } else {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
        true
    };

    ItemDriverOutcome::OnOffLightChanged {
        item_id: item.id,
        character_id: character.id,
        now_on,
        remaining_off: None,
        gates_opened: false,
    }
}

pub(crate) fn torch_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(4, 0);

    if context.timer_call {
        mark_special_modified_torch(item);
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }
        if context.character_underwater {
            extinguish_torch(item);
            character.flags.insert(CharacterFlags::ITEMS);
            return ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id: item.id,
                character_id: character.id,
                schedule_after_ticks: LIGHT_TIMER_TICKS,
            };
        }

        item.driver_data[1] = item.driver_data[1].saturating_add(1);
        if item.driver_data[1] > item.driver_data[2] {
            return ItemDriverOutcome::TorchExpired {
                item_id: item.id,
                character_id: character.id,
                item_name: outcome_item_name(&item.name),
            };
        }
        set_torch_light(item);
        character.flags.insert(CharacterFlags::ITEMS);
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
        };
    }

    if item.x != 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    if let Some((modifier_slot, modifier)) = torch_extractable_modifier(item) {
        return ItemDriverOutcome::TorchExtractOrb {
            item_id: item.id,
            character_id: character.id,
            modifier_slot,
            modifier,
        };
    }

    if item.driver_data[0] != 0 {
        extinguish_torch(item);
    } else {
        if context.character_underwater {
            return ItemDriverOutcome::BlockedByRequirements {
                item_id: item.id,
                character_id: character.id,
            };
        }
        item.driver_data[0] = 1;
        set_torch_light(item);
        item.sprite -= 1;
        item.flags.insert(ItemFlags::NODECAY);
    }
    character.flags.insert(CharacterFlags::ITEMS);

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: (item.driver_data[0] != 0).then_some(LIGHT_TIMER_TICKS),
    }
}

pub(crate) fn mark_special_modified_torch(item: &mut Item) {
    if item.min_level == 200 {
        return;
    }
    if item
        .modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .any(|(&index, &value)| index != V_LIGHT && index >= 0 && value > 0)
    {
        item.min_level = 200;
    }
}

pub(crate) fn torch_extractable_modifier(item: &Item) -> Option<(usize, i16)> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .enumerate()
        .find_map(|(slot, (&index, &value))| {
            (index != V_LIGHT && index >= 0 && value > 0).then_some((slot, index))
        })
}

pub(crate) fn extinguish_torch(item: &mut Item) {
    item.driver_data[0] = 0;
    item.modifier_value[0] = 0;
    item.sprite += 1;
    item.flags.remove(ItemFlags::NODECAY);
}

pub(crate) fn set_torch_light(item: &mut Item) {
    let burn = i32::from(item.driver_data[1]);
    let max_burn = i32::from(item.driver_data[2]);
    let base = i32::from(item.driver_data[3]);
    let light = base.min(base * max_burn / (burn + 1) / 2);
    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light as i16;
}
