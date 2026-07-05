//! Clan-dungeon raid-guard NPC generation, ported from
//! `src/area/13/dungeon.c`'s `build_warrior`/`build_mage`/`build_seyan`
//! (`dungeon.c:217-700`), together with the per-level base-stat tables from
//! `src/area/13/dungeon_tab.c` (`warrior_tab`/`mage_tab`/`seyan_tab`).
//!
//! These three functions instantiate a "warrior"/"mage"/"seyan" character
//! template (`ugaris_data/zones/13/dungeon.chr`), scale every skill the
//! template already carries to a per-level `base` value from the matching
//! table via a per-skill formula, then attach four "spell of equipment"
//! items (`equip1`/`equip2`/`armor_spell`/`weapon_spell`, carried in
//! non-worn inventory slots 12-15 exactly like C's `ch[cn].item[12..15]`)
//! whose modifier values are computed from `level2maxitem`/the character's
//! own raised skills, matching the average player's gear power at that
//! level.
//!
//! Deliberately **not** wired here (a separate, larger future slice per
//! `PORTING_TODO.md`'s Clan system task): `build_cell`'s dispatch of a
//! generated `dungeon_maze::MazeCell`'s `special` code into calls to these
//! functions (`dungeon.c:849-937`), and the `dungeonmaster`/
//! `dungeonfighter` NPC drivers that orchestrate a full raid. This module
//! only provides the three NPC-generation functions themselves, each
//! independently unit-tested against the C formulas.

// Not yet wired into any call site: `build_cell`'s dispatch of a generated
// `dungeon_maze::MazeCell`'s `special` code into `build_warrior`/
// `build_mage`/`build_seyan` calls (`dungeon.c:849-937`) is a separate,
// larger future slice per `PORTING_TODO.md`'s Clan system task. Exercised
// directly by `crates/ugaris-server/src/tests/dungeon.rs` in the meantime.
#![allow(dead_code)]

use super::*;

/// C `warrior_tab[]` (`dungeon_tab.c:15-138`), indexed by level (0-121).
/// `build_warrior` only ever reads up to index 118 (`min(level, 118)`), but
/// the full table is kept for fidelity with the C source.
const WARRIOR_TAB: [i32; 122] = [
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 19, 20, 21, 22, 24, 25, 27, 28, 29, 30, 31, 33,
    34, 35, 37, 38, 39, 40, 41, 43, 44, 45, 46, 48, 49, 50, 51, 52, 53, 55, 56, 57, 58, 60, 61, 62,
    63, 64, 65, 66, 68, 69, 70, 71, 72, 73, 74, 76, 77, 78, 79, 80, 81, 82, 83, 84, 86, 87, 88, 89,
    90, 91, 92, 93, 94, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 108, 109, 110, 111, 112,
    113, 114, 115, 116, 117, 118, 120, 121, 122, 123, 124, 125, 126, 128, 129, 131, 133, 135, 138,
    140, 143, 146, 151, 155, 161, 167, 171,
];

/// C `mage_tab[]` (`dungeon_tab.c:140-263`), indexed by level (0-121).
const MAGE_TAB: [i32; 122] = [
    0, 1, 2, 3, 4, 5, 6, 8, 10, 11, 13, 14, 15, 17, 18, 20, 21, 22, 23, 24, 25, 27, 28, 29, 30, 31,
    33, 34, 35, 36, 37, 38, 40, 41, 42, 43, 44, 45, 47, 48, 49, 50, 51, 52, 53, 55, 56, 57, 58, 59,
    60, 61, 63, 64, 65, 66, 67, 68, 69, 70, 72, 73, 74, 75, 76, 77, 78, 79, 80, 82, 83, 84, 85, 86,
    87, 88, 89, 90, 91, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 106, 107, 108, 109,
    110, 111, 112, 113, 114, 115, 116, 117, 118, 120, 121, 122, 123, 124, 125, 126, 129, 131, 133,
    135, 141, 146, 151, 155, 161, 167, 174,
];

/// C `seyan_tab[]` (`dungeon_tab.c:265-386`), indexed by level (0-119).
const SEYAN_TAB: [i32; 120] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 15, 16, 18, 19, 20, 21, 22, 23, 24, 26, 27, 28, 29,
    30, 31, 32, 34, 35, 36, 37, 39, 40, 41, 42, 43, 44, 45, 46, 48, 49, 50, 51, 52, 53, 54, 55, 57,
    58, 59, 60, 61, 62, 63, 64, 65, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82,
    83, 84, 85, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105,
    106, 107, 108, 110, 111, 113, 115, 118, 121, 124, 126, 129, 132, 134, 136, 139, 141, 144, 146,
    148, 150, 153, 155, 157,
];

/// C `it[in].mod_value[n] = level2maxitem(level) * 1.1 + max(0, level - 63)
/// / 2;` (`dungeon.c:334-335` and its `build_mage`/`build_seyan` twins) -
/// the shared "spell of equipment" modifier-value formula. The `/ 2` runs
/// as C integer division on `max(0, level - 63)` before being widened to
/// `double` and added to the `level2maxitem` term, so it is computed here
/// as `i32` division first, matching that evaluation order exactly.
pub(crate) fn dungeon_guard_equip_mod_value(level: i32) -> i16 {
    let overlevel_bonus = (level - 63).max(0) / 2;
    (f64::from(level2maxitem(level)) * 1.1 + f64::from(overlevel_bonus)) as i16
}

/// Instantiates one of the `equip1`/`equip1b`/`equip1c`/`equip2`/`equip2b`/
/// `equip2c` "spell of equipment" item templates, overwrites its first
/// `modifier_count` `mod_value` slots (the template itself already sets
/// `mod_index` for each) with [`dungeon_guard_equip_mod_value`], and
/// carries it in the given non-worn inventory slot (C `ch[cn].item[12]`/
/// `item[13]`).
pub(crate) fn set_dungeon_guard_equip_item(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
    slot: usize,
    template: &str,
    modifier_count: usize,
    level: i32,
) -> bool {
    let Ok(mut item) = loader.instantiate_item_template(template, Some(character.id)) else {
        return false;
    };
    let value = dungeon_guard_equip_mod_value(level);
    for slot_value in item.modifier_value.iter_mut().take(modifier_count) {
        *slot_value = value;
    }
    character.inventory[slot] = Some(item.id);
    inventory_items.push(item);
    true
}

/// Instantiates the shared `armor_spell`/`weapon_spell` item template
/// (single `mod_index`/`mod_value` pair) and sets its lone `mod_value`
/// slot to `value`, carried in the given non-worn inventory slot (C
/// `ch[cn].item[14]`/`item[15]`).
pub(crate) fn set_dungeon_guard_spell_item(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
    slot: usize,
    template: &str,
    value: i16,
) -> bool {
    let Ok(mut item) = loader.instantiate_item_template(template, Some(character.id)) else {
        return false;
    };
    item.modifier_value[0] = value;
    character.inventory[slot] = Some(item.id);
    inventory_items.push(item);
    true
}

/// C `if (ch[cn].value[1][V_PROFESSION] > 0) ch[cn].prof[P_CLAN] =
/// min(30, ch[cn].value[1][V_PROFESSION]); if (ch[cn].value[1]
/// [V_PROFESSION] > 30) ch[cn].prof[RANDOM(2) ? P_LIGHT : P_DARK] =
/// min(30, ch[cn].value[1][V_PROFESSION] - 30);` - shared by all three
/// `build_*` functions verbatim (`dungeon.c:325-331` and its two twins).
pub(crate) fn apply_dungeon_guard_profession(character: &mut Character) {
    let profession_value = i32::from(character.values[1][CharacterValue::Profession as usize]);
    if profession_value > 0 {
        character.professions[profession::CLAN] = profession_value.min(30) as i16;
    }
    if profession_value > 30 {
        let index = if runtime_random_below(2) != 0 {
            profession::LIGHT
        } else {
            profession::DARK
        };
        character.professions[index] = (profession_value - 30).min(30) as i16;
    }
}

/// Shared spawn-finalization tail for all three `build_*` functions: place
/// the character on the map (C `drop_char(cn, x, y, 0)`), insert its
/// inventory items, run the full `update_char` recompute so item modifier
/// bonuses take effect, then set `hp`/`endurance`/`mana` from the
/// recomputed `value[0]` (C `dungeon.c:349-352` and its two twins).
pub(crate) fn finish_dungeon_guard_spawn(
    world: &mut World,
    character: Character,
    inventory_items: Vec<Item>,
    x: u16,
    y: u16,
) -> bool {
    let character_id = character.id;
    if !world.spawn_character(character, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.update_character(character_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    }
    true
}

/// C `build_warrior(x, y, level)` (`dungeon.c:217-336`): instantiates the
/// "warrior" template and scales its skills to `level`. Returns the
/// spawned character's final level (matching C's `return ch[cn].level;`),
/// or `None` if the template/item lookups or the map placement failed.
pub(crate) fn build_warrior(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    x: u16,
    y: u16,
    level: i32,
    maze_clan: i32,
    maze_level: i32,
) -> Option<u32> {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, mut inventory_items)) =
        loader.instantiate_character_template("warrior", character_id)
    else {
        return None;
    };

    let is_arch = level > 33;
    if is_arch {
        character.flags.insert(CharacterFlags::ARCH);
        character.values[1][CharacterValue::Rage as usize] = 1;
    }

    let level = level.max(1);
    let base = WARRIOR_TAB[level.min(118) as usize];

    for index in 0..CHARACTER_VALUE_NAMES.len() {
        if legacy_skill_cost_factor(index) == 0 {
            continue;
        }
        if character.values[1][index] == 0 {
            continue;
        }
        let value = warrior_stat_value(index, base, level, is_arch).min(125);
        character.values[1][index] = value as i16;
    }

    character.sprite = if maze_clan < 17 {
        266 + maze_clan
    } else {
        516 + maze_clan - 16
    };
    character.dir = Direction::RightDown as u8;
    character.rest_x = maze_clan as u16;
    character.rest_y = ((level - maze_level) / 2) as u16;

    let exp = legacy_calc_exp_used(&character);
    character.exp = exp;
    character.exp_used = exp;
    character.level = exp2level(exp);

    apply_dungeon_guard_profession(&mut character);

    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        12,
        "equip1",
        5,
        level,
    );
    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        13,
        "equip2",
        4,
        level,
    );
    let armor_skill =
        i32::from(character.values[1][CharacterValue::ArmorSkill as usize]).clamp(13, 113);
    set_dungeon_guard_spell_item(
        &mut character,
        loader,
        &mut inventory_items,
        14,
        "armor_spell",
        (armor_skill * 20) as i16,
    );
    let hand_skill = i32::from(character.values[1][CharacterValue::Hand as usize]).clamp(13, 113);
    set_dungeon_guard_spell_item(
        &mut character,
        loader,
        &mut inventory_items,
        15,
        "weapon_spell",
        hand_skill as i16,
    );

    character.name = format!("Warrior{}", level);
    let final_level = character.level;

    finish_dungeon_guard_spawn(world, character, inventory_items, x, y).then_some(final_level)
}

/// C `build_mage(x, y, level)` (`dungeon.c:389-535`): instantiates the
/// "mage" template and scales its skills to `level`.
pub(crate) fn build_mage(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    x: u16,
    y: u16,
    level: i32,
    maze_clan: i32,
    maze_level: i32,
) -> Option<u32> {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, mut inventory_items)) =
        loader.instantiate_character_template("mage", character_id)
    else {
        return None;
    };

    let is_arch = level > 33;
    if is_arch {
        character.flags.insert(CharacterFlags::ARCH);
        character.values[1][CharacterValue::Duration as usize] = 1;
    }

    let level = level.max(1);
    let base = MAGE_TAB[level.min(118) as usize];

    for index in 0..CHARACTER_VALUE_NAMES.len() {
        if legacy_skill_cost_factor(index) == 0 {
            continue;
        }
        if character.values[1][index] == 0 {
            continue;
        }
        let value = mage_stat_value(index, base, level).min(125);
        character.values[1][index] = value as i16;
    }

    character.sprite = if maze_clan < 17 {
        282 + maze_clan
    } else {
        532 + maze_clan - 16
    };
    character.dir = Direction::RightDown as u8;
    character.rest_x = maze_clan as u16;
    character.rest_y = ((level - maze_level) / 2) as u16;

    let exp = legacy_calc_exp_used(&character);
    character.exp = exp;
    character.exp_used = exp;
    character.level = exp2level(exp);

    apply_dungeon_guard_profession(&mut character);

    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        12,
        "equip1b",
        5,
        level,
    );
    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        13,
        "equip2b",
        4,
        level,
    );
    let hand_skill = i32::from(character.values[1][CharacterValue::Hand as usize]).clamp(13, 113);
    set_dungeon_guard_spell_item(
        &mut character,
        loader,
        &mut inventory_items,
        15,
        "weapon_spell",
        hand_skill as i16,
    );

    character.name = format!("Mage{}", level);
    let final_level = character.level;

    finish_dungeon_guard_spawn(world, character, inventory_items, x, y).then_some(final_level)
}

/// C `build_seyan(x, y, level)` (`dungeon.c:551-700`): instantiates the
/// "seyan" template and scales its skills to `level`.
pub(crate) fn build_seyan(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    x: u16,
    y: u16,
    level: i32,
    maze_clan: i32,
    maze_level: i32,
) -> Option<u32> {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, mut inventory_items)) =
        loader.instantiate_character_template("seyan", character_id)
    else {
        return None;
    };

    if level > 33 {
        character.flags.insert(CharacterFlags::ARCH);
    }

    let level = level.max(1);
    let base = SEYAN_TAB[level.min(118) as usize];

    for index in 0..CHARACTER_VALUE_NAMES.len() {
        if legacy_skill_cost_factor(index) == 0 {
            continue;
        }
        if character.values[1][index] == 0 {
            continue;
        }
        let value = seyan_stat_value(index, base, level).min(107);
        character.values[1][index] = value as i16;
    }

    character.sprite = if maze_clan < 17 {
        266 + maze_clan
    } else {
        516 + maze_clan - 16
    };
    character.dir = Direction::RightDown as u8;
    character.rest_x = maze_clan as u16;
    character.rest_y = ((level - maze_level) / 2) as u16;

    let exp = legacy_calc_exp_used(&character);
    character.exp = exp;
    character.exp_used = exp;
    character.level = exp2level(exp);

    apply_dungeon_guard_profession(&mut character);

    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        12,
        "equip1c",
        5,
        level,
    );
    set_dungeon_guard_equip_item(
        &mut character,
        loader,
        &mut inventory_items,
        13,
        "equip2c",
        5,
        level,
    );
    let armor_skill =
        i32::from(character.values[1][CharacterValue::ArmorSkill as usize]).clamp(13, 113);
    set_dungeon_guard_spell_item(
        &mut character,
        loader,
        &mut inventory_items,
        14,
        "armor_spell",
        (armor_skill * 20) as i16,
    );
    let hand_skill = i32::from(character.values[1][CharacterValue::Hand as usize]).clamp(13, 113);
    set_dungeon_guard_spell_item(
        &mut character,
        loader,
        &mut inventory_items,
        15,
        "weapon_spell",
        hand_skill as i16,
    );

    character.name = format!("Seyan{}", level);
    let final_level = character.level;

    finish_dungeon_guard_spawn(world, character, inventory_items, x, y).then_some(final_level)
}

/// The per-skill `switch (n)` formula body of `build_warrior`
/// (`dungeon.c:238-306`), applied before the shared `min(val, 125)` clamp.
pub(crate) fn warrior_stat_value(index: usize, base: i32, level: i32, is_arch: bool) -> i32 {
    use CharacterValue::*;
    match index {
        i if i == Hp as usize => (base - 20).max(10),
        i if i == Endurance as usize => (base - 30).max(10),
        i if i == Profession as usize => {
            if level > 19 {
                (base - 7).max(1)
            } else {
                0
            }
        }
        i if i == Wisdom as usize => (base - 15).max(10),
        i if i == Intelligence as usize => base.max(10),
        i if i == Agility as usize => (base - 5).max(10),
        i if i == Strength as usize => base.max(10),
        i if i == Hand as usize => base.max(1),
        i if i == ArmorSkill as usize => ((base / 10) * 10).max(1),
        i if i == Attack as usize => base.max(1),
        i if i == Parry as usize => base.max(1),
        i if i == Immunity as usize => base.max(1),
        i if i == Tactics as usize => (base - 5).max(1),
        i if i == Surround as usize => (base - 50).max(1),
        i if i == BodyControl as usize => (base - 20).max(1),
        i if i == SpeedSkill as usize => (base - 20).max(1),
        i if i == Percept as usize => (base - 10).max(1),
        i if i == Rage as usize => {
            if is_arch {
                (base - 20).max(1)
            } else {
                0
            }
        }
        _ => (base - 50).max(1),
    }
}

/// The per-skill `switch (n)` formula body of `build_mage`
/// (`dungeon.c:410-475`), applied before the shared `min(val, 125)` clamp.
pub(crate) fn mage_stat_value(index: usize, base: i32, level: i32) -> i32 {
    use CharacterValue::*;
    match index {
        i if i == Hp as usize => (base - 40).max(10),
        i if i == Mana as usize => (base - 10).max(10),
        i if i == Endurance as usize => (base - 30).max(10),
        i if i == Profession as usize => {
            if level > 19 {
                (base - 7).max(1)
            } else {
                0
            }
        }
        i if i == Wisdom as usize => base.max(10),
        i if i == Intelligence as usize => base.max(10),
        i if i == Agility as usize => base.max(10),
        i if i == Strength as usize => base.max(10),
        i if i == Hand as usize => base.max(1),
        i if i == MagicShield as usize => base.max(1),
        i if i == Flash as usize => base.max(1),
        i if i == Bless as usize => base.max(1),
        i if i == Immunity as usize => base.max(1),
        i if i == Freeze as usize => (base - 10).max(1),
        i if i == Heal as usize => (base - 10).max(1),
        i if i == Fireball as usize => (base - 10).max(1),
        i if i == Percept as usize => (base - 10).max(1),
        i if i == Duration as usize => (base - 10).max(1),
        _ => (base - 50).max(1),
    }
}

/// The per-skill `switch (n)` formula body of `build_seyan`
/// (`dungeon.c:564-632`), applied before the shared `min(val, 107)` clamp.
pub(crate) fn seyan_stat_value(index: usize, base: i32, level: i32) -> i32 {
    use CharacterValue::*;
    match index {
        i if i == Hp as usize => (base - 40).max(10),
        i if i == Mana as usize => (base - 30).max(10),
        i if i == Endurance as usize => (base - 50).max(10),
        i if i == Profession as usize => {
            if level > 19 {
                (base - 7).max(1)
            } else {
                0
            }
        }
        i if i == Wisdom as usize => (base - 15).max(10),
        i if i == Intelligence as usize => base.max(10),
        i if i == Agility as usize => (base - 5).max(10),
        i if i == Strength as usize => base.max(10),
        i if i == Hand as usize => base.max(1),
        i if i == ArmorSkill as usize => ((base / 10) * 10).max(1),
        i if i == Attack as usize => base.max(1),
        i if i == Parry as usize => base.max(1),
        i if i == Immunity as usize => base.max(1),
        i if i == Bless as usize => base.max(1),
        i if i == Freeze as usize => base.max(1),
        i if i == Tactics as usize => (base - 5).max(1),
        i if i == Percept as usize => (base - 10).max(1),
        _ => (base - 50).max(1),
    }
}
