use super::*;

pub(crate) const P_ALCHEMIST: usize = 1;

pub(crate) fn alchemist_profession(character: &Character) -> i16 {
    character
        .professions
        .get(P_ALCHEMIST)
        .copied()
        .unwrap_or_default()
}

pub(crate) fn flask_power(item: &Item, character: &Character, context: &ItemDriverContext) -> i32 {
    let powers = [
        drdata(item, 18),
        drdata(item, 19),
        drdata(item, 20),
        drdata(item, 21),
        drdata(item, 22),
        drdata(item, 23),
        drdata(item, 24),
        drdata(item, 25),
        drdata(item, 26),
    ];
    let count: u8 = powers.iter().copied().sum();
    let stone = drdata(item, 31) + drdata(item, 32) + drdata(item, 33) + drdata(item, 34);
    let alchemist = alchemist_profession(character);

    const PAIR_POWER: [(i32, i32, i32); 8] = [
        (16, 12, 10),
        (24, 20, 16),
        (32, 26, 20),
        (40, 32, 24),
        (48, 38, 28),
        (56, 44, 32),
        (64, 50, 36),
        (72, 56, 40),
    ];
    if count == 2 {
        for pair in 0..8 {
            if powers[pair] == 1 && powers[pair + 1] == 1 {
                let (best, mid, low) = PAIR_POWER[pair];
                if context.solstice || (context.fullmoon && alchemist >= 30) || alchemist >= 50 {
                    return best;
                }
                if context.equinox || (context.fullmoon && alchemist >= 20) || alchemist >= 40 {
                    return mid;
                }
                if context.fullmoon || stone != 0 || alchemist >= 10 {
                    return low;
                }
            }
        }
    }

    let good = if context.solstice || (context.fullmoon && alchemist >= 30) || alchemist >= 50 {
        8
    } else if context.equinox || (context.fullmoon && alchemist >= 20) || alchemist >= 40 {
        4
    } else if context.fullmoon || alchemist >= 10 {
        2
    } else if context.hour == 12 {
        1
    } else {
        0
    };
    let bad = if context.newmoon {
        2
    } else if context.hour == 0 {
        1
    } else {
        0
    };

    for (idx, present) in powers.iter().enumerate().rev() {
        if *present != 0 {
            let base = match idx {
                8 => 36,
                7 => 32,
                6 => 28,
                5 => 24,
                4 => 20,
                3 => 16,
                2 => 12,
                1 => 8,
                _ => 6,
            };
            return if idx <= 1 {
                (base + good - bad).max(2)
            } else {
                base + good - bad
            };
        }
    }

    -1
}

pub(crate) fn flask_duration(item: &Item) -> Option<(u8, f64)> {
    if drdata(item, 27) != 0 {
        Some((60, 1.75))
    } else if drdata(item, 30) != 0 {
        Some((30, 1.5))
    } else if drdata(item, 29) != 0 {
        Some((20, 1.25))
    } else if drdata(item, 28) != 0 {
        Some((10, 1.0))
    } else {
        None
    }
}

pub(crate) fn flask_ingredient_counts(item: &Item) -> [u8; 29] {
    let mut counts = [0; 29];
    for (idx, count) in counts.iter_mut().enumerate() {
        *count = drdata(item, idx + 11);
    }
    counts
}

pub(crate) fn c_div(power: i32, divi: f64, divisor: f64) -> i16 {
    (f64::from(power) / divi / divisor) as i16
}

pub(crate) fn c_scaled(power: i32, amount: u8, divi: f64, count: u8, divisor: f64) -> i16 {
    (f64::from(power) * f64::from(amount) / divi / f64::from(count) / divisor) as i16
}

pub(crate) fn flask_skill_mix(
    item: &Item,
    character: &Character,
    context: &ItemDriverContext,
) -> Option<([i16; MAX_MODIFIERS], [i16; MAX_MODIFIERS], u8, i32, u8)> {
    let mut power = flask_power(item, character, context);
    let (duration, divi) = flask_duration(item)?;
    if power <= 0 {
        return None;
    }

    let mut wis = drdata(item, 11);
    let mut inu = drdata(item, 12);
    let mut agi = drdata(item, 13);
    let mut strn = drdata(item, 14);
    let mut lfe = drdata(item, 15);
    let mut spr = drdata(item, 16);
    let mut end = drdata(item, 17);
    let count = wis + inu + agi + strn + lfe + spr + end;
    let fire = drdata(item, 31);
    let ice = drdata(item, 32);
    let hell = drdata(item, 34);

    power += i32::from(fire) * 4 + i32::from(ice) * 8 + i32::from(hell) * 12;
    let alchemist = alchemist_profession(character);
    for threshold in [20, 30, 40, 50] {
        if alchemist >= threshold {
            power += 4;
        }
    }

    let c_empty_modifier_index = || {
        let mut idx = [0; MAX_MODIFIERS];
        idx[0] = -1;
        idx[1] = -1;
        idx[2] = -1;
        idx
    };
    let single = |skill: CharacterValue, divisor: f64, value: i32| {
        let mut idx = c_empty_modifier_index();
        let mut val = [0; MAX_MODIFIERS];
        idx[0] = skill as i16;
        val[0] = c_div(power, divi, divisor);
        (idx, val, value)
    };
    let double = |a: CharacterValue, b: CharacterValue, divisor: f64, value: i32| {
        let mut idx = c_empty_modifier_index();
        let mut val = [0; MAX_MODIFIERS];
        idx[0] = a as i16;
        idx[1] = b as i16;
        val[0] = c_div(power, divi, divisor);
        val[1] = c_div(power, divi, divisor);
        (idx, val, value)
    };
    let triple =
        |a: CharacterValue, b: CharacterValue, c: CharacterValue, divisor: f64, value: i32| {
            let mut idx = c_empty_modifier_index();
            let mut val = [0; MAX_MODIFIERS];
            idx[0] = a as i16;
            idx[1] = b as i16;
            idx[2] = c as i16;
            val[0] = c_div(power, divi, divisor);
            val[1] = c_div(power, divi, divisor);
            val[2] = c_div(power, divi, divisor);
            (idx, val, value)
        };

    let (modifier_index, modifier_value, value_factor) =
        if count == 5 && wis == 1 && inu == 1 && agi == 2 && strn == 1 {
            triple(
                CharacterValue::Sword,
                CharacterValue::Attack,
                CharacterValue::Parry,
                4.0,
                10,
            )
        } else if count == 5 && wis == 1 && inu == 1 && agi == 1 && strn == 2 {
            triple(
                CharacterValue::TwoHand,
                CharacterValue::Attack,
                CharacterValue::Parry,
                4.0,
                10,
            )
        } else if count == 5 && agi == 1 && strn == 2 && lfe == 1 && spr == 1 {
            triple(
                CharacterValue::Attack,
                CharacterValue::Parry,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && strn == 1 && lfe == 2 && spr == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && strn == 2 && lfe == 2 && spr == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && lfe == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::MagicShield,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && strn == 1 && lfe == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::MagicShield,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && strn == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::Immunity,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && strn == 3 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::Immunity,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 4 && wis == 1 && inu == 1 && agi == 1 && strn == 1 {
            double(CharacterValue::Attack, CharacterValue::Parry, 3.0, 8)
        } else if count == 4 && inu == 1 && strn == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Flash, CharacterValue::Immunity, 3.0, 8)
        } else if count == 4 && strn == 2 && lfe == 1 && spr == 1 {
            double(CharacterValue::Fireball, CharacterValue::Immunity, 3.0, 8)
        } else if count == 4 && strn == 1 && lfe == 2 && spr == 1 {
            double(
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                3.0,
                10,
            )
        } else if count == 4 && agi == 1 && end == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Dagger, CharacterValue::Flash, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 1 && end == 1 && spr == 1 {
            double(CharacterValue::Dagger, CharacterValue::Fireball, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Staff, CharacterValue::Flash, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 2 && spr == 1 {
            double(CharacterValue::Staff, CharacterValue::Fireball, 3.0, 8)
        } else if count == 3 && strn == 2 && end == 1 {
            single(CharacterValue::Pulse, 2.0, 3)
        } else if count == 3 && agi == 2 && end == 1 {
            single(CharacterValue::Dagger, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 1 && end == 1 {
            single(CharacterValue::Staff, 2.0, 3)
        } else if count == 3 && agi == 2 && strn == 1 {
            single(CharacterValue::Sword, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 2 {
            single(CharacterValue::TwoHand, 2.0, 3)
        } else if count == 3 && inu == 1 && agi == 1 && strn == 1 {
            single(CharacterValue::Attack, 2.0, 3)
        } else if count == 3 && wis == 1 && agi == 1 && strn == 1 {
            single(CharacterValue::Parry, 2.0, 3)
        } else if count == 3 && inu == 2 && end == 1 {
            single(CharacterValue::Percept, 2.0, 3)
        } else if count == 3 && inu == 2 && agi == 1 {
            single(CharacterValue::Stealth, 2.0, 3)
        } else if count == 3 && agi == 2 && lfe == 1 {
            single(CharacterValue::BodyControl, 2.0, 3)
        } else if count == 3 && agi == 1 && end == 1 && spr == 1 {
            single(CharacterValue::Freeze, 2.0, 3)
        } else if count == 3 && lfe == 2 && spr == 1 {
            single(CharacterValue::MagicShield, 2.0, 3)
        } else if count == 3 && inu == 1 && lfe == 1 && spr == 1 {
            single(CharacterValue::Flash, 2.0, 3)
        } else if count == 3 && strn == 1 && lfe == 1 && spr == 1 {
            single(CharacterValue::Fireball, 2.0, 3)
        } else if count == 3 && strn == 2 && spr == 1 {
            single(CharacterValue::Immunity, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 1 && lfe == 1 {
            single(CharacterValue::Hand, 2.0, 3)
        } else if count == 3 && inu == 1 && strn == 1 && end == 1 {
            single(CharacterValue::Warcry, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && agi == 1 {
            single(CharacterValue::Tactics, 2.0, 3)
        } else if count == 3 && inu == 1 && agi == 2 {
            single(CharacterValue::Surround, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 2 {
            single(CharacterValue::Barter, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && spr == 1 {
            single(CharacterValue::Bless, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && lfe == 1 {
            single(CharacterValue::Heal, 2.0, 3)
        } else if count == 3 && lfe == 1 && spr == 2 {
            single(CharacterValue::Duration, 2.0, 3)
        } else if count == 3 && strn == 2 && lfe == 1 {
            single(CharacterValue::Rage, 2.0, 3)
        } else if count != 0 {
            let mut idx = c_empty_modifier_index();
            let mut val = [0; MAX_MODIFIERS];
            for slot in 0..3 {
                if wis != 0 {
                    idx[slot] = CharacterValue::Wisdom as i16;
                    val[slot] = c_scaled(power, wis, divi, count, 4.0);
                    wis = 0;
                } else if inu != 0 {
                    idx[slot] = CharacterValue::Intelligence as i16;
                    val[slot] = c_scaled(power, inu, divi, count, 4.0);
                    inu = 0;
                } else if agi != 0 {
                    idx[slot] = CharacterValue::Agility as i16;
                    val[slot] = c_scaled(power, agi, divi, count, 4.0);
                    agi = 0;
                } else if strn != 0 {
                    idx[slot] = CharacterValue::Strength as i16;
                    val[slot] = c_scaled(power, strn, divi, count, 4.0);
                    strn = 0;
                } else if lfe != 0 {
                    idx[slot] = CharacterValue::Hp as i16;
                    val[slot] = c_scaled(power, lfe, divi, count, 2.0);
                    lfe = 0;
                } else if spr != 0 {
                    idx[slot] = CharacterValue::Mana as i16;
                    val[slot] = c_scaled(power, spr, divi, count, 2.0);
                    spr = 0;
                } else if end != 0 {
                    idx[slot] = CharacterValue::Endurance as i16;
                    val[slot] = c_scaled(power, end, divi, count, 1.0);
                    end = 0;
                }
            }
            (idx, val, 1)
        } else {
            return None;
        };

    if !modifier_value.iter().any(|value| *value != 0) {
        return None;
    }

    let value = value_factor * power * 13 + 50;
    let needs_class = if fire != 0 || ice != 0 || hell != 0 {
        8
    } else {
        0
    };
    Some((modifier_index, modifier_value, duration, value, needs_class))
}

pub(crate) fn finish_flask_mix(
    item: &mut Item,
    character: &Character,
    context: &ItemDriverContext,
) -> Option<()> {
    let (modifier_index, modifier_value, duration, value, needs_class) =
        flask_skill_mix(item, character, context)?;
    item.modifier_index = modifier_index;
    item.modifier_value = modifier_value;
    set_drdata(item, 2, 1);
    set_drdata(item, 3, duration);
    item.value = value.max(0) as u32;
    item.needs_class = needs_class;
    set_flask_magical_state(item);
    Some(())
}

pub fn reset_flask_empty_state(item: &mut Item) {
    let size = drdata(item, 0);
    item.name = "Empty Potion".to_string();
    match size {
        1 => {
            item.sprite = 10290;
            item.description = "A small flask made of glass.".to_string();
        }
        2 => {
            item.sprite = 10294;
            item.description = "A flask made of glass.".to_string();
        }
        3 => {
            item.sprite = 10302;
            item.description = "A big flask made of glass.".to_string();
        }
        _ => {}
    }
    item.driver_data.clear();
    item.driver_data.push(size);
    item.modifier_index = [0; MAX_MODIFIERS];
    item.modifier_value = [0; MAX_MODIFIERS];
    item.value = 10;
    item.needs_class = 0;
}

pub(crate) fn set_flask_magical_state(item: &mut Item) {
    item.name = "Magical Potion".to_string();
    match drdata(item, 0) {
        1 => {
            item.sprite = 50213;
            item.description = "A small flask containing a magical liquid.".to_string();
        }
        2 => {
            item.sprite = 50214;
            item.description = "A flask containing a magical liquid.".to_string();
        }
        3 => {
            item.sprite = 50253;
            item.description = "A big flask containing a magical liquid.".to_string();
        }
        _ => {}
    }
}

pub(crate) fn flask_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let size = drdata(item, 0);
    let used = drdata(item, 1);
    let shaken = drdata(item, 2) != 0;

    if shaken && character.cursor_item.is_some() {
        return ItemDriverOutcome::FlaskFinishedNoMoreIngredients {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some(cursor_item_id) = character.cursor_item else {
        if shaken {
            if !check_item_requirements(character, item) {
                return ItemDriverOutcome::BlockedByRequirements {
                    item_id: item.id,
                    character_id: character.id,
                };
            }
            return ItemDriverOutcome::AlchemyFlaskPotion {
                item_id: item.id,
                character_id: character.id,
                duration_minutes: drdata(item, 3),
                modifier_index: item.modifier_index,
                modifier_value: item.modifier_value,
            };
        }
        if used != 0 {
            let ingredient_counts = flask_ingredient_counts(item);
            if finish_flask_mix(item, character, context).is_some() {
                return ItemDriverOutcome::FlaskMixed {
                    item_id: item.id,
                    character_id: character.id,
                    ingredient_counts,
                };
            }
            reset_flask_empty_state(item);
            return ItemDriverOutcome::FlaskRuined {
                item_id: item.id,
                character_id: character.id,
                ingredient_counts,
            };
        }
        return ItemDriverOutcome::FlaskEmptyShaken {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_template_id != Some(IID_ALCHEMY_INGREDIENT) {
        return ItemDriverOutcome::FlaskWrongCursor {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if used >= size.saturating_mul(3) {
        return ItemDriverOutcome::FlaskFull {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let ingredient_kind = context.cursor_drdata0.unwrap_or_default();
    if !(1..=29).contains(&ingredient_kind) {
        return ItemDriverOutcome::FlaskIngredientBug {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::FlaskIngredientAdded {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        ingredient_kind,
    }
}
