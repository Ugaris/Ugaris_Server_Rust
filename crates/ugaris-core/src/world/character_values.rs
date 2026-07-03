use super::*;

pub(crate) fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn bonus_spell_shape(driver: u16) -> Option<(&'static str, CharacterValue)> {
    Some(match driver {
        IDR_ARMOR => ("Armor", CharacterValue::Armor),
        IDR_WEAPON => ("Weapon", CharacterValue::Weapon),
        IDR_MANA => ("Mana", CharacterValue::Mana),
        IDR_HP => ("HP", CharacterValue::Hp),
        _ => return None,
    })
}

pub(crate) fn refresh_driver_spell_flags(character: &mut Character, items: &HashMap<ItemId, Item>) {
    let mut has_infravision_spell = false;
    let mut has_nonomagic_spell = false;
    let mut has_oxygen_spell = false;

    for item_id in character.inventory.iter().take(30).flatten() {
        let Some(item) = items.get(item_id) else {
            continue;
        };
        match item.driver {
            IDR_INFRARED => has_infravision_spell = true,
            IDR_NONOMAGIC => has_nonomagic_spell = true,
            IDR_OXYGEN => has_oxygen_spell = true,
            _ => {}
        }
    }

    let old_flags = character.flags;
    character
        .flags
        .set(CharacterFlags::INFRAVISION, has_infravision_spell);
    character
        .flags
        .set(CharacterFlags::NONOMAGIC, has_nonomagic_spell);
    character
        .flags
        .set(CharacterFlags::OXYGEN, has_oxygen_spell);
    if character.flags != old_flags {
        character.flags.insert(CharacterFlags::UPDATE);
    }
}

pub(crate) fn character_value_from_index(index: usize) -> Option<CharacterValue> {
    Some(match index {
        0 => CharacterValue::Hp,
        1 => CharacterValue::Endurance,
        2 => CharacterValue::Mana,
        3 => CharacterValue::Wisdom,
        4 => CharacterValue::Intelligence,
        5 => CharacterValue::Agility,
        6 => CharacterValue::Strength,
        7 => CharacterValue::Armor,
        8 => CharacterValue::Weapon,
        9 => CharacterValue::Light,
        10 => CharacterValue::Speed,
        11 => CharacterValue::Pulse,
        12 => CharacterValue::Dagger,
        13 => CharacterValue::Hand,
        14 => CharacterValue::Staff,
        15 => CharacterValue::Sword,
        16 => CharacterValue::TwoHand,
        17 => CharacterValue::ArmorSkill,
        18 => CharacterValue::Attack,
        19 => CharacterValue::Parry,
        20 => CharacterValue::Warcry,
        21 => CharacterValue::Tactics,
        22 => CharacterValue::Surround,
        23 => CharacterValue::BodyControl,
        24 => CharacterValue::SpeedSkill,
        25 => CharacterValue::Barter,
        26 => CharacterValue::Percept,
        27 => CharacterValue::Stealth,
        28 => CharacterValue::Bless,
        29 => CharacterValue::Heal,
        30 => CharacterValue::Freeze,
        31 => CharacterValue::MagicShield,
        32 => CharacterValue::Flash,
        33 => CharacterValue::Fireball,
        34 => CharacterValue::Empty,
        35 => CharacterValue::Regenerate,
        36 => CharacterValue::Meditate,
        37 => CharacterValue::Immunity,
        38 => CharacterValue::Demon,
        39 => CharacterValue::Duration,
        40 => CharacterValue::Rage,
        41 => CharacterValue::Cold,
        42 => CharacterValue::Profession,
        _ => return None,
    })
}

pub(crate) fn character_value_present(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn character_value_base(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn character_profession(character: &Character, index: usize) -> i32 {
    character
        .professions
        .get(index)
        .copied()
        .unwrap_or_default() as i32
}

fn set_character_value0(character: &mut Character, index: usize, value: i32) {
    if let Some(slot) = character.values.get_mut(0).and_then(|v| v.get_mut(index)) {
        *slot = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

fn set_character_value1(character: &mut Character, index: usize, value: i32) {
    if let Some(slot) = character.values.get_mut(1).and_then(|v| v.get_mut(index)) {
        *slot = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

fn add_character_value0(character: &mut Character, value: CharacterValue, delta: i32) {
    let index = value as usize;
    let current = character_value(character, value);
    set_character_value0(character, index, current + delta);
}

/// C `armor_skill_req` (`src/system/create.c:1661`): the strongest
/// `-V_ARMORSKILL` requirement modifier carried by a single worn item.
fn armor_skill_req(item: Option<&Item>) -> i32 {
    let Some(item) = item else {
        return 0;
    };
    let mut req = 0;
    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if i32::from(index) == -(CharacterValue::ArmorSkill as i32) {
            req = req.max(i32::from(value));
        }
    }
    req
}

/// C `armor_skill_bonus` (`src/system/create.c:1676`): body/head/legs/arms
/// armor-skill requirements weighted 50/20/15/15, compared against the
/// character's raised Armor Skill value.
fn armor_skill_bonus(character: &Character, items: &HashMap<ItemId, Item>) -> i32 {
    let worn = |slot: usize| {
        character
            .inventory
            .get(slot)
            .copied()
            .flatten()
            .and_then(|item_id| items.get(&item_id))
    };

    let mut req_total = 0;
    let mut used = 0;
    for (slot, weight) in [
        (crate::legacy::worn_slot::BODY, 50),
        (crate::legacy::worn_slot::HEAD, 20),
        (crate::legacy::worn_slot::LEGS, 15),
        (crate::legacy::worn_slot::ARMS, 15),
    ] {
        let tmp = armor_skill_req(worn(slot)) * weight;
        if tmp != 0 {
            req_total += tmp;
            used += weight;
        }
    }
    if used == 0 {
        return 0;
    }
    let req = req_total / used;
    let raised = character_value_present(character, CharacterValue::ArmorSkill);
    (raised - req) * 5 * used / 100
}

/// C `skill[]` table (`src/system/skill.c:27`): the three base attributes
/// averaged (then divided by 5) to seed a raisable value's total. Powers,
/// attributes, Armor/Weapon/Light, Cold and Profession have no base
/// (`-1,-1,-1`) and are handled by their callers instead.
fn skill_base_attributes(
    value: CharacterValue,
) -> Option<(CharacterValue, CharacterValue, CharacterValue)> {
    use CharacterValue::*;
    Some(match value {
        Speed => (Agility, Agility, Strength),
        Pulse => (Intelligence, Intelligence, Wisdom),
        Dagger => (Intelligence, Agility, Strength),
        Hand => (Agility, Strength, Strength),
        Staff => (Intelligence, Agility, Strength),
        Sword => (Intelligence, Agility, Strength),
        TwoHand => (Agility, Strength, Strength),
        ArmorSkill => (Agility, Agility, Strength),
        Attack => (Intelligence, Agility, Strength),
        Parry => (Intelligence, Agility, Strength),
        Warcry => (Intelligence, Agility, Strength),
        Tactics => (Intelligence, Agility, Strength),
        Surround => (Intelligence, Agility, Strength),
        BodyControl => (Intelligence, Agility, Strength),
        SpeedSkill => (Intelligence, Agility, Strength),
        Barter => (Intelligence, Intelligence, Wisdom),
        Percept => (Intelligence, Intelligence, Wisdom),
        Stealth => (Intelligence, Agility, Agility),
        Bless => (Intelligence, Intelligence, Wisdom),
        Heal => (Intelligence, Intelligence, Wisdom),
        Freeze => (Intelligence, Intelligence, Wisdom),
        MagicShield => (Intelligence, Intelligence, Wisdom),
        Flash => (Intelligence, Intelligence, Wisdom),
        Fireball => (Intelligence, Intelligence, Wisdom),
        Empty => (Intelligence, Intelligence, Wisdom),
        Regenerate => (Strength, Strength, Strength),
        Meditate => (Wisdom, Wisdom, Wisdom),
        Immunity => (Intelligence, Wisdom, Strength),
        Duration => (Intelligence, Wisdom, Strength),
        Rage => (Intelligence, Strength, Strength),
        _ => return None,
    })
}

/// C `update_char` (`src/system/create.c:1710`): recompute
/// `value[0]` (the effective total) for every character value from
/// `value[1]` (the raised/base amount) plus equipment/spell modifiers,
/// profession bonuses and race/class caps. This is the pure-data slice:
/// equipment modifier sum + caps, base-attribute averaging, profession
/// bonuses, Body Control/Armor Skill armor-weapon bonuses and the
/// HP/endurance/mana current-value clamp. Callers that have map access
/// (`World::update_character`) additionally handle the light emission
/// diff, which needs `&mut World`.
///
/// Known gaps vs. the C original (documented, not silently dropped):
/// - `ch.ef[]` area-effect light contributions are not modeled (Rust
///   effects are not attached to characters the same way).
/// - The `P_CLAN` night-in-catacombs bonus only checks `MF_CLAN`; the
///   `areaID == 13` special case is not available (`World` has no
///   current-area id).
/// - Sprite reselection (demon suits, weapon-in-hand offsets) and the
///   `player_reset_map_cache` call on infravision toggle are not ported
///   here; they are display-only side effects tracked separately.
pub(crate) fn recompute_character_values(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    hour: i64,
    in_clan_area: bool,
) {
    refresh_driver_spell_flags(character, items);

    let is_warrior = character.flags.contains(CharacterFlags::WARRIOR);
    let is_seyan = character
        .flags
        .contains(CharacterFlags::WARRIOR | CharacterFlags::MAGE);
    let nomagic = character.flags.contains(CharacterFlags::NOMAGIC);
    let nonomagic = character.flags.contains(CharacterFlags::NONOMAGIC);

    let mut mod_arr = [0i32; CHARACTER_VALUE_COUNT];
    let mut beyond_arr = [0i32; CHARACTER_VALUE_COUNT];
    let mut bless_arr = [0i32; CHARACTER_VALUE_COUNT];

    for item_id in character.inventory.iter().take(30).flatten() {
        let Some(item) = items.get(item_id) else {
            continue;
        };
        for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
            if value == 0 {
                continue;
            }
            let v1 = i32::from(index);
            if v1 <= -(CHARACTER_VALUE_COUNT as i32) || v1 >= CHARACTER_VALUE_COUNT as i32 {
                continue;
            }
            if v1 < 0 {
                continue; // negative indices are requirements, not modifiers.
            }
            let idx = v1 as usize;
            let delta = i32::from(value);
            if item.flags.contains(ItemFlags::BEYONDMAXMOD) {
                beyond_arr[idx] += delta;
            } else if !is_warrior && item.driver == IDR_BLESS {
                bless_arr[idx] += delta;
            } else {
                mod_arr[idx] += delta;
            }
        }
    }

    let strength_idx = CharacterValue::Strength as usize;
    let pulse_idx = CharacterValue::Pulse as usize;
    let wisdom_idx = CharacterValue::Wisdom as usize;

    for n in 0..CHARACTER_VALUE_COUNT {
        let Some(value) = character_value_from_index(n) else {
            continue;
        };

        if value == CharacterValue::Demon {
            let capped =
                character_value(character, value).min(character_value_present(character, value));
            set_character_value0(character, n, capped);
            continue;
        }
        if value == CharacterValue::Cold {
            let m = mod_arr[n];
            set_character_value0(character, n, m);
            set_character_value1(character, n, m);
            continue;
        }

        let mut base_n = 0;
        if let Some((b1, b2, b3)) = skill_base_attributes(value) {
            base_n = (character_value(character, b1)
                + character_value(character, b2)
                + character_value(character, b3))
                / 5;
        }

        let v1n = character_value_present(character, value);

        let mut total = if v1n == 0 && n >= pulse_idx {
            0
        } else {
            let mut mod_n = mod_arr[n];
            let mut bless_n = bless_arr[n];
            if n <= strength_idx || n >= pulse_idx {
                let cap_fraction = if is_seyan { 0.725 } else { 0.500 };
                mod_n = mod_n.min((f64::from(v1n) * cap_fraction) as i32);
                bless_n = bless_n.min((f64::from(v1n) * 0.500) as i32);
            }
            if n >= pulse_idx {
                base_n = base_n.min(15.max(v1n * 2));
            }

            let allow_magic = !nomagic
                || nonomagic
                || value == CharacterValue::Weapon
                || value == CharacterValue::Armor
                || value == CharacterValue::Light;
            let mut computed = if allow_magic {
                base_n + v1n + mod_n + bless_n
            } else {
                base_n + v1n
            };

            if n >= wisdom_idx && n <= strength_idx {
                let light_prof = character_profession(character, profession::LIGHT);
                if light_prof != 0 && (6..18).contains(&hour) {
                    computed += light_prof / 2;
                }
                let dark_prof = character_profession(character, profession::DARK);
                if dark_prof != 0 && !(6..18).contains(&hour) {
                    computed += dark_prof / 2;
                }
                let clan_prof = character_profession(character, profession::CLAN);
                if clan_prof != 0 && in_clan_area {
                    computed += clan_prof;
                }
            }
            computed
        };

        total += beyond_arr[n];
        if value == CharacterValue::Hp && total < 0 {
            total = 1;
        } else if n <= strength_idx && total < 0 {
            total = 0;
        }
        set_character_value0(character, n, total);
    }

    if character_value_present(character, CharacterValue::SpeedSkill) != 0 {
        let bonus = character_value(character, CharacterValue::SpeedSkill) / 2;
        add_character_value0(character, CharacterValue::Speed, bonus);
    }
    let athlete = character_profession(character, profession::ATHLETE);
    if athlete != 0 {
        add_character_value0(character, CharacterValue::Speed, athlete * 3);
    }
    let thief = character_profession(character, profession::THIEF);
    if thief != 0 {
        if character.flags.contains(CharacterFlags::THIEFMODE) {
            add_character_value0(character, CharacterValue::Stealth, thief * 2);
        } else {
            add_character_value0(character, CharacterValue::Stealth, thief);
        }
        add_character_value0(character, CharacterValue::Percept, thief / 2);
    }

    let demon_prof = character_profession(character, profession::DEMON);
    if demon_prof != 0 && character.flags.contains(CharacterFlags::DEMON) {
        for value in [
            CharacterValue::Hand,
            CharacterValue::Dagger,
            CharacterValue::Staff,
            CharacterValue::Sword,
            CharacterValue::TwoHand,
            CharacterValue::Attack,
            CharacterValue::Parry,
            CharacterValue::Tactics,
            CharacterValue::Immunity,
            CharacterValue::Flash,
            CharacterValue::Fireball,
            CharacterValue::Freeze,
        ] {
            if character_value_present(character, value) != 0 {
                add_character_value0(character, value, demon_prof);
            }
        }
    }

    if character_value_present(character, CharacterValue::BodyControl) != 0 {
        let body_control = character_value(character, CharacterValue::BodyControl);
        add_character_value0(character, CharacterValue::Armor, body_control * 5);
        add_character_value0(character, CharacterValue::Weapon, body_control / 4);

        let has_real_weapon_in_hand = character
            .inventory
            .get(crate::legacy::worn_slot::RIGHT_HAND)
            .copied()
            .flatten()
            .and_then(|item_id| items.get(&item_id))
            .is_some_and(|item| {
                item.flags.contains(ItemFlags::WEAPON) && !item.flags.contains(ItemFlags::HAND)
            });
        if !has_real_weapon_in_hand && character.flags.contains(CharacterFlags::PLAYER) {
            add_character_value0(
                character,
                CharacterValue::Weapon,
                (body_control / 2).min(90),
            );
        }
    } else {
        let average = spell_average(
            character_value(character, CharacterValue::Bless),
            character_value(character, CharacterValue::Heal),
            character_value(character, CharacterValue::Freeze),
            character_value(character, CharacterValue::MagicShield),
            character_value(character, CharacterValue::Flash),
            character_value(character, CharacterValue::Fireball),
            character_value(character, CharacterValue::Pulse),
        );
        add_character_value0(character, CharacterValue::Armor, (average * 17.5) as i32);
    }

    if character_value_present(character, CharacterValue::ArmorSkill) != 0 {
        let bonus = armor_skill_bonus(character, items);
        add_character_value0(character, CharacterValue::Armor, bonus);
    }

    character.flags.insert(CharacterFlags::UPDATE);

    if character.hp > character_value(character, CharacterValue::Hp) * POWERSCALE {
        character.hp = character_value(character, CharacterValue::Hp) * POWERSCALE;
    }
    if character.endurance > character_value(character, CharacterValue::Endurance) * POWERSCALE {
        character.endurance = character_value(character, CharacterValue::Endurance) * POWERSCALE;
    }
    if character.mana > character_value(character, CharacterValue::Mana) * POWERSCALE {
        character.mana = character_value(character, CharacterValue::Mana) * POWERSCALE;
    }
}

impl World {
    /// C `update_char(cn)` (`src/system/create.c:1710`). See
    /// [`recompute_character_values`] for the pure-data recompute; this
    /// wrapper additionally re-emits map light when Light changes, which
    /// needs `&mut World`. Call this everywhere C calls `update_char`:
    /// equip/unequip, spell install/expiry, level up, login, death
    /// respawn.
    pub fn update_character(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let old_light = character_value(character, CharacterValue::Light) as i16;
        let hour = self.date.hour;
        let (x, y) = (character.x, character.y);
        let in_clan_area = self
            .map
            .tile(usize::from(x), usize::from(y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::CLAN));

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        recompute_character_values(character, &self.items, hour, in_clan_area);

        self.refresh_character_light_after_value_change(character_id, old_light);
        true
    }
}
