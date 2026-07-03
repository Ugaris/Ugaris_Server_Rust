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

/// C `IID_DEMONSKIN1/2/3` (`src/common/item_id.h:227-229`,
/// `MAKE_ITEMID(DEV_ID_DB, 0xA8..0xAA)`): the full-suit demon skin item
/// templates that override normal class/weapon sprite selection in
/// `update_char` when worn on all six armor slots.
const IID_DEMONSKIN1: u32 = (0x01 << 24) | 0x0000A8;
const IID_DEMONSKIN2: u32 = (0x01 << 24) | 0x0000A9;
const IID_DEMONSKIN3: u32 = (0x01 << 24) | 0x0000AA;

/// C `update_char` (`src/system/create.c:1969-2120`): recompute a player's
/// display sprite from class/gender/weapon-in-hand, with a full demon-skin
/// suit (all six of head/arms/legs/body/cloak/feet matching one
/// `IID_DEMONSKIN*` template) overriding the normal selection entirely.
/// Returns `true` when the sprite actually changed, so the caller can mark
/// the tile dirty (C `set_sector`) the same way C does.
///
/// Known gap vs. C: `reset_name(cn)` (clearing a cached colored-name
/// buffer) is not ported - Rust has no equivalent server-side name-color
/// cache to invalidate, so the demon-sprite-transition color refresh is a
/// no-op here by construction, not an oversight.
fn recompute_character_sprite(character: &mut Character, items: &HashMap<ItemId, Item>) -> bool {
    let is_player = character.flags.contains(CharacterFlags::PLAYER);
    let is_god = character.flags.contains(CharacterFlags::GOD);
    let sprite = character.sprite;
    let god_sprite_exempt =
        (60..120).contains(&sprite) || sprite == 27 || sprite == 157 || sprite == 39;
    if !is_player || (is_god && !god_sprite_exempt) {
        return false;
    }

    let worn = |slot: usize| {
        character
            .inventory
            .get(slot)
            .copied()
            .flatten()
            .and_then(|item_id| items.get(&item_id))
    };

    let right_hand = worn(crate::legacy::worn_slot::RIGHT_HAND);
    let left_hand = worn(crate::legacy::worn_slot::LEFT_HAND);
    let left_hand_lit =
        left_hand.is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) != 0);

    let mut off = match right_hand {
        // `IF_WEAPON` is a composite of several single weapon-class bits;
        // must be `intersects`, not `contains` (see the identical note on
        // the Body Control bare-handed check above).
        Some(weapon) if weapon.flags.intersects(ItemFlags::WEAPON) => {
            if left_hand_lit {
                4 // torch and one-hand weapon
            } else if weapon.flags.contains(ItemFlags::WNTWOHANDED) {
                2 // two-handed weapon
            } else {
                1
            }
        }
        _ => {
            if left_hand_lit {
                3 // torch
            } else {
                0 // nothing
            }
        }
    };

    use CharacterFlags as CF;
    let class_bits = character.flags & (CF::WARRIOR | CF::MAGE | CF::MALE | CF::FEMALE | CF::ARCH);
    let mut sbase = if class_bits == (CF::MAGE | CF::MALE) {
        60
    } else if class_bits == (CF::MAGE | CF::MALE | CF::ARCH) {
        65
    } else if class_bits == (CF::MAGE | CF::FEMALE) {
        75
    } else if class_bits == (CF::MAGE | CF::FEMALE | CF::ARCH) {
        70
    } else if class_bits == (CF::WARRIOR | CF::MALE) {
        85
    } else if class_bits == (CF::WARRIOR | CF::MALE | CF::ARCH) {
        80
    } else if class_bits == (CF::WARRIOR | CF::FEMALE) {
        95
    } else if class_bits == (CF::WARRIOR | CF::FEMALE | CF::ARCH) {
        90
    } else if class_bits == (CF::MAGE | CF::WARRIOR | CF::MALE) {
        105
    } else if class_bits == (CF::MAGE | CF::WARRIOR | CF::MALE | CF::ARCH) {
        100
    } else if class_bits == (CF::MAGE | CF::WARRIOR | CF::FEMALE) {
        115
    } else if class_bits == (CF::MAGE | CF::WARRIOR | CF::FEMALE | CF::ARCH) {
        110
    } else {
        0
    };

    let demonskin_slots = [
        crate::legacy::worn_slot::HEAD,
        crate::legacy::worn_slot::ARMS,
        crate::legacy::worn_slot::LEGS,
        crate::legacy::worn_slot::BODY,
        crate::legacy::worn_slot::CLOAK,
        crate::legacy::worn_slot::FEET,
    ];
    let count_demonskin = |template_id: u32| {
        demonskin_slots
            .iter()
            .filter(|&&slot| worn(slot).is_some_and(|item| item.template_id == template_id))
            .count()
    };
    if count_demonskin(IID_DEMONSKIN1) == 6 {
        sbase = 27;
        off = 0;
    }
    if count_demonskin(IID_DEMONSKIN2) == 6 {
        sbase = 157;
        off = 0;
    }
    // C `create.c:2105-2108` (verbatim, including the `sbase == 257` typo
    // below - copied digit-for-digit rather than "fixed").
    if count_demonskin(IID_DEMONSKIN3) == 6 {
        sbase = 39;
        off = 0;
    }

    if sbase == 0 {
        return false;
    }
    let new_sprite = sbase + off;
    if character.sprite == new_sprite {
        return false;
    }
    // C: reset_name(cn) fires here when transitioning to/from a demon
    // sprite (27/157/39, the last compared against the literal `257` typo
    // in `create.c:2112`) - no-op in Rust, see doc comment above.
    character.sprite = new_sprite;
    true
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
/// bonuses, Body Control/Armor Skill armor-weapon bonuses, the
/// character-attached-effect `V_LIGHT` contribution (`effect_light`,
/// computed by the caller since it needs `&self.effects`), and the
/// HP/endurance/mana current-value clamp. Callers that have map access
/// (`World::update_character`) additionally handle the light emission
/// diff, which needs `&mut World`.
///
/// Known gaps vs. the C original (documented, not silently dropped):
/// - `ch.ef[]`'s fixed four-slot cap on character-attached effects is
///   approximated, not exactly modeled: see
///   [`World::character_attached_effect_light`] for the precise
///   deviation (only matters with 5+ simultaneous character-attached
///   effects).
/// - The `player_reset_map_cache` call on infravision toggle is not
///   ported (display-only side effect tracked separately). Sprite
///   reselection (demon suits, weapon-in-hand offsets) *is* ported, as
///   [`recompute_character_sprite`], called separately by
///   `World::update_character` since it needs `set_sector`/`&mut World`.
pub(crate) fn recompute_character_values(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    hour: i64,
    in_clan_area: bool,
    effect_light: i32,
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

    // C `create.c:1788-1797`: `mod[V_LIGHT] += ef[fn].light` for each of
    // the character's up to four attached effects (`ch[cn].ef[0..4]`).
    // `World::update_character` computes `effect_light` as the sum of
    // `.light` across the character's currently attached effects
    // (`Effect::target_character`), matching this contribution before the
    // shared value-recompute loop below applies it like any other
    // uncapped V_LIGHT modifier (V_LIGHT is exempt from the seyan/
    // warrior mod cap since it is outside the `n <= V_STR || n >= V_PULSE`
    // range).
    mod_arr[CharacterValue::Light as usize] += effect_light;

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
                // `IF_WEAPON` is a composite of several single weapon-class
                // bits (`IF_AXE|IF_DAGGER|IF_HAND|...`); C's `flags &
                // IF_WEAPON` is true if *any* one is set, so this must be
                // `intersects`, not `contains` (which would require every
                // bit set at once - never true for a real item).
                item.flags.intersects(ItemFlags::WEAPON) && !item.flags.contains(ItemFlags::HAND)
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
        // C `create.c:1856`: `areaID == 13 || (mmf & MF_CLAN)` - bonus
        // for a clan master in the catacombs (area 13) or any
        // clan-flagged tile.
        let in_clan_area = self.area_id == 13
            || self
                .map
                .tile(usize::from(x), usize::from(y))
                .is_some_and(|tile| tile.flags.contains(MapFlags::CLAN));

        let effect_light = self.character_attached_effect_light(character_id);

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        recompute_character_values(character, &self.items, hour, in_clan_area, effect_light);
        let sprite_changed = recompute_character_sprite(character, &self.items);

        self.refresh_character_light_after_value_change(character_id, old_light);
        if sprite_changed {
            self.mark_dirty_sector(usize::from(x), usize::from(y));
        }
        true
    }

    /// C `create.c:1785-1797`: sums `ef[fn].light` across the character's
    /// attached effect slots (`ch[cn].ef[0..4]`). C caps a character to at
    /// most four simultaneously attached effects (`add_effect_char`,
    /// `src/system/effect.c:209`, returns 0 and silently drops the
    /// attachment - including its light contribution - once all four
    /// slots are full); Rust does not yet model that fixed four-slot
    /// array (`Effect::target_character` allows unlimited attachments per
    /// character), so as an approximation this takes the four
    /// lowest-id (oldest/earliest-attached) effects, which matches C for
    /// the common case and only deviates from C when a character has more
    /// than four character-attached effects at once (a rare edge case:
    /// e.g. bless + curse + warcry/freeze + a fifth combat effect like
    /// magicshield/firering/pulseback/burn/strike/flash landing at the
    /// same time). Documented, not silently dropped.
    pub(crate) fn character_attached_effect_light(&self, character_id: CharacterId) -> i32 {
        let mut attached: Vec<(u32, i32)> = self
            .effects
            .iter()
            .filter(|(_, effect)| effect.target_character == Some(character_id))
            .map(|(&effect_id, effect)| (effect_id, effect.light))
            .collect();
        attached.sort_unstable_by_key(|&(effect_id, _)| effect_id);
        attached.truncate(4);
        attached.iter().map(|&(_, light)| light).sum()
    }
}
