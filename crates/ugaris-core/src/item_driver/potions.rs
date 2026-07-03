use super::*;

pub(crate) fn special_potion_driver(
    character: &mut Character,
    item: &mut Item,
    area_id: u16,
    in_arena: bool,
    current_tick: u32,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if item.min_level != 0 && character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 33 || (area_id == 34 && in_arena) {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let kind = drdata(item, 0);
    let max_hp = character
        .values
        .first()
        .and_then(|values| values.get(CharacterValue::Hp as usize))
        .copied()
        .unwrap_or(0)
        .max(0) as i32
        * POWERSCALE;
    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;

    match kind {
        0..=4 => {
            consume_item(character, item);
            return ItemDriverOutcome::SpecialPotionAntidote {
                item_id: item.id,
                character_id: character.id,
                kind,
                poison_removed: false,
            };
        }
        5 => {
            if character.saves < 10 && !character.flags.contains(CharacterFlags::HARDCORE) {
                character.saves += 1;
                consume_item(character, item);
                return ItemDriverOutcome::SpecialPotionSecurity {
                    item_id: item.id,
                    character_id: character.id,
                    used: true,
                };
            }
            return ItemDriverOutcome::SpecialPotionSecurity {
                item_id: item.id,
                character_id: character.id,
                used: false,
            };
        }
        6 => {
            return ItemDriverOutcome::SpecialPotionInfravision {
                item_id: item.id,
                character_id: character.id,
                installed: false,
            };
        }
        7 => {
            if character.exp < character.exp_used {
                return ItemDriverOutcome::SpecialPotionProfessionReset {
                    item_id: item.id,
                    character_id: character.id,
                    used: false,
                    professions_reset: 0,
                    profession_points_lowered: 0,
                    exp_refunded: 0,
                };
            }

            let professions_reset = character
                .professions
                .iter()
                .fold(0_u16, |sum, &value| sum.saturating_add(value.max(0) as u16));
            if professions_reset == 0 {
                return ItemDriverOutcome::SpecialPotionProfessionReset {
                    item_id: item.id,
                    character_id: character.id,
                    used: false,
                    professions_reset: 0,
                    profession_points_lowered: 0,
                    exp_refunded: 0,
                };
            }

            for profession in &mut character.professions {
                *profession = 0;
            }
            let old_exp_used = character.exp_used;
            let mut profession_points_lowered = 0_u16;
            for _ in 0..(professions_reset / 3) {
                if lower_value(character, CharacterValue::Profession as usize).is_some() {
                    profession_points_lowered = profession_points_lowered.saturating_add(1);
                }
            }
            let exp_refunded = old_exp_used.saturating_sub(character.exp_used);
            character.exp = character.exp.saturating_sub(exp_refunded);
            character
                .flags
                .insert(CharacterFlags::PROF | CharacterFlags::UPDATE);
            consume_item(character, item);
            return ItemDriverOutcome::SpecialPotionProfessionReset {
                item_id: item.id,
                character_id: character.id,
                used: true,
                professions_reset,
                profession_points_lowered,
                exp_refunded,
            };
        }
        8 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        9 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.regen_ticker = current_tick;
        }
        10 => {
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        11 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        12 => {
            if area_id != 33 {
                character.hp = (character.hp + 3 * POWERSCALE).min(max_hp);
            }
        }
        13 => {
            if area_id != 33 {
                character.hp = (character.hp + 4 * POWERSCALE).min(max_hp);
            }
        }
        14 => {
            if area_id != 33 {
                character.hp = (character.hp + 5 * POWERSCALE).min(max_hp);
            }
        }
        15 => {
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        _ => {
            return ItemDriverOutcome::SpecialPotionBug {
                item_id: item.id,
                character_id: character.id,
            };
        }
    }

    consume_item(character, item);
    character.flags.insert(CharacterFlags::UPDATE);
    ItemDriverOutcome::SpecialPotionDrunk {
        item_id: item.id,
        character_id: character.id,
        kind,
        hp_delta: character.hp - old_hp,
        mana_delta: character.mana - old_mana,
        endurance_delta: character.endurance - old_endurance,
    }
}

pub(crate) fn decaying_item_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    if context.timer_call {
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }

        let age = drdata_u16(item, 3).saturating_add(1);
        set_drdata_u16(item, 3, age);
        if age > drdata_u16(item, 5) {
            return ItemDriverOutcome::DecayItemExpired {
                item_id: item.id,
                character_id: item.carried_by.unwrap_or(character.id),
                item_name: outcome_item_name(&item.name),
            };
        }

        return ItemDriverOutcome::DecayItemToggled {
            item_id: item.id,
            character_id: item.carried_by.unwrap_or(character.id),
            active: true,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        };
    }

    if item.x != 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let activating = item.driver_data[0] == 0;
    item.driver_data[0] = u8::from(activating);
    let target_value = i16::from(if activating {
        item.driver_data[2]
    } else {
        item.driver_data[1]
    });
    for value in &mut item.modifier_value {
        if *value != 0 {
            *value = target_value;
        }
    }
    if activating {
        item.sprite += 1;
    } else {
        item.sprite -= 1;
    }
    character.flags.insert(CharacterFlags::ITEMS);

    ItemDriverOutcome::DecayItemToggled {
        item_id: item.id,
        character_id: character.id,
        active: activating,
        schedule_after_ticks: activating.then_some(TICKS_PER_SECOND * 2),
    }
}

pub(crate) fn potion_driver(
    character: &mut Character,
    item: &mut Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 33 || (area_id == 34 && in_arena) {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let empty_kind = drdata(item, 0);
    if empty_kind != 0 {
        return ItemDriverOutcome::EmptyPotionTemplateNeeded {
            item_id: item.id,
            character_id: character.id,
            empty_kind,
        };
    }

    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;
    character.hp = capped_resource(
        character.hp,
        drdata(item, 1),
        max_value(character, CharacterValue::Hp),
    );
    character.mana = capped_resource(
        character.mana,
        drdata(item, 2),
        max_value(character, CharacterValue::Mana),
    );
    character.endurance = capped_resource(
        character.endurance,
        drdata(item, 3),
        max_value(character, CharacterValue::Endurance),
    );
    consume_item(character, item);

    ItemDriverOutcome::PotionDrunk {
        item_id: item.id,
        character_id: character.id,
        hp_added: character.hp - old_hp,
        mana_added: character.mana - old_mana,
        endurance_added: character.endurance - old_endurance,
    }
}

pub(crate) fn beyond_potion_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if !check_item_requirements(character, item) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BeyondPotion {
        item_id: item.id,
        character_id: character.id,
        duration_minutes: drdata(item, 0),
        modifier_index: item.modifier_index,
        modifier_value: item.modifier_value,
        beyond_max_mod: item.flags.contains(ItemFlags::BEYONDMAXMOD),
    }
}
