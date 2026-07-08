//! Randomly-enchanted "special" items.
//!
//! Ports `create_special_item` (`src/system/tool.c:2620-2789`) and
//! `add_special_store` (`src/module/merchants/store.c:229-323`). Per
//! `create_special_item`'s own C doc comment it is also meant to back
//! chest/loot drops (`src/system/create.c:1102`'s `special_prob`/
//! `special_str`/`special_base` character-template fields), but that call
//! site is not wired yet - only the merchant "special store" path
//! (`add_special_store`) uses it so far.

use super::*;

/// C `struct special_item`'s modifier-slot count (`mod_index[3]`,
/// `tool.c:2288`) - distinct from [`MAX_MODIFIERS`], which is the *item's*
/// modifier-slot count (C `MAXMOD`, 5).
const SPECIAL_ITEM_MOD_SLOTS: usize = 3;

/// C `struct special_item` (`tool.c:2286-2293`).
struct SpecialItemEntry {
    name: &'static str,
    mod_index: [i16; SPECIAL_ITEM_MOD_SLOTS],
    chance: i32,
    needflag: ItemFlags,
    the: bool,
    pricemulti: i32,
}

macro_rules! mods {
    ($a:ident) => {
        [CharacterValue::$a as i16, -1, -1]
    };
    ($a:ident, $b:ident) => {
        [CharacterValue::$a as i16, CharacterValue::$b as i16, -1]
    };
    ($a:ident, $b:ident, $c:ident) => {
        [
            CharacterValue::$a as i16,
            CharacterValue::$b as i16,
            CharacterValue::$c as i16,
        ]
    };
}

/// C `special_item[]` (`tool.c:2295-2390`), transcribed verbatim (name,
/// up to three modifiers, roll weight, required weapon-class flag when the
/// rolled base item happens to be a weapon, "the" name prefix, price
/// multiplier).
const SPECIAL_ITEM_TABLE: [SpecialItemEntry; 76] = [
    SpecialItemEntry {
        name: "Wisdom",
        mod_index: mods!(Wisdom),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Intuition",
        mod_index: mods!(Intelligence),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Agility",
        mod_index: mods!(Agility),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Strength",
        mod_index: mods!(Strength),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Hitpoints",
        mod_index: mods!(Hp),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Mana",
        mod_index: mods!(Mana),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Endurance",
        mod_index: mods!(Endurance),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Dagger",
        mod_index: mods!(Dagger),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::DAGGER,
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Hand to Hand",
        mod_index: mods!(Hand),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Staff",
        mod_index: mods!(Staff),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::STAFF,
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Sword",
        mod_index: mods!(Sword),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::SWORD,
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Two-Handed",
        mod_index: mods!(TwoHand),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::TWOHAND,
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Attack",
        mod_index: mods!(Attack),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Parry",
        mod_index: mods!(Parry),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Warcry",
        mod_index: mods!(Warcry),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Tactics",
        mod_index: mods!(Tactics),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Surround Hit",
        mod_index: mods!(Surround),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Body Control",
        mod_index: mods!(BodyControl),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Speed",
        mod_index: mods!(SpeedSkill),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Bartering",
        mod_index: mods!(Barter),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Perception",
        mod_index: mods!(Percept),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Stealth",
        mod_index: mods!(Stealth),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Bless",
        mod_index: mods!(Bless),
        chance: SP_MANY_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Heal",
        mod_index: mods!(Heal),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Freeze",
        mod_index: mods!(Freeze),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Magic Shield",
        mod_index: mods!(MagicShield),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Lightning",
        mod_index: mods!(Flash),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Fireball",
        mod_index: mods!(Fireball),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Pulse",
        mod_index: mods!(Pulse),
        chance: SP_SOME_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Regenerate",
        mod_index: mods!(Regenerate),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Meditate",
        mod_index: mods!(Meditate),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 1,
    },
    SpecialItemEntry {
        name: "Immunity",
        mod_index: mods!(Immunity),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Duration",
        mod_index: mods!(Duration),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Rage",
        mod_index: mods!(Rage),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 2,
    },
    SpecialItemEntry {
        name: "Weird",
        mod_index: mods!(Immunity, Regenerate, Percept),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Odd",
        mod_index: mods!(Immunity, Meditate, Stealth),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Strange",
        mod_index: mods!(Flash, Duration, Percept),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Peculiar",
        mod_index: mods!(Attack, Rage, Percept),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Unusual",
        mod_index: mods!(Parry, Rage, Stealth),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Bizarre",
        mod_index: mods!(MagicShield, Heal, Stealth),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Offbeat",
        mod_index: mods!(Immunity, Heal, Meditate),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Aberrant",
        mod_index: mods!(Immunity, Rage, Regenerate),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Thief",
        mod_index: mods!(Stealth, Endurance),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Vision",
        mod_index: mods!(Percept, Intelligence),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Wounded",
        mod_index: mods!(Heal, Regenerate),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Eccentric",
        mod_index: mods!(BodyControl, Bless, SpeedSkill),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 4,
    },
    SpecialItemEntry {
        name: "Sorcery",
        mod_index: mods!(Mana, Duration),
        chance: SP_FEW_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Fighting",
        mod_index: mods!(Attack, Parry),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Magic",
        mod_index: mods!(Flash, MagicShield),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Berserk",
        mod_index: mods!(Attack, Rage),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Strategic Reflex",
        mod_index: mods!(Tactics, BodyControl, SpeedSkill),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Sword Mastery",
        mod_index: mods!(Sword, BodyControl, SpeedSkill),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Two Handed Reflex",
        mod_index: mods!(TwoHand, BodyControl, SpeedSkill),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Frozen Strike",
        mod_index: mods!(Pulse, Freeze, Dagger),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Arcane Chill",
        mod_index: mods!(Staff, Freeze, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Divine Surge",
        mod_index: mods!(Staff, Bless, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Frostbite Sting",
        mod_index: mods!(Dagger, Freeze, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Sacred Slash",
        mod_index: mods!(Dagger, Bless, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Mystic Benediction",
        mod_index: mods!(Mana, Bless, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Holy Freeze",
        mod_index: mods!(Pulse, Bless, Freeze),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "One Handed Offense",
        mod_index: mods!(Attack, Sword),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::SWORD,
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "One Handed Defense",
        mod_index: mods!(Parry, Sword),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::SWORD,
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Two Handed Offense",
        mod_index: mods!(Attack, TwoHand),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::TWOHAND,
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Two Handed Defense",
        mod_index: mods!(Parry, TwoHand),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::TWOHAND,
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Tactical Offense",
        mod_index: mods!(Attack, Tactics),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Tactical Defense",
        mod_index: mods!(Parry, Tactics),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Magical Offense",
        mod_index: mods!(Flash, Fireball, Pulse),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Magical Defense",
        mod_index: mods!(MagicShield, Freeze, Heal),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 8,
    },
    SpecialItemEntry {
        name: "Sagacious Might",
        mod_index: mods!(Intelligence, Wisdom, Strength),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Cunning Force",
        mod_index: mods!(Intelligence, Agility, Strength),
        chance: SP_RARE_CONST,
        needflag: ItemFlags::empty(),
        the: false,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Warrior",
        mod_index: mods!(Attack, Parry, Immunity),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Mage",
        mod_index: mods!(Flash, MagicShield, Immunity),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Tactician",
        mod_index: mods!(Attack, Parry, Tactics),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Seyan'Du",
        mod_index: mods!(Attack, Parry, Bless),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Arch-Warrior",
        mod_index: mods!(Attack, Parry, Rage),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
    SpecialItemEntry {
        name: "Arch-Mage",
        mod_index: mods!(Flash, MagicShield, Duration),
        chance: SP_ULTRA_CONST,
        needflag: ItemFlags::empty(),
        the: true,
        pricemulti: 16,
    },
];

/// C `STRENGTH_DESCRIPTIONS[]` (`tool.c:2598-2617`): `(description,
/// price_add)` indexed by `strength - 1`.
const STRENGTH_DESCRIPTIONS: [(&str, i32); 20] = [
    ("Extremely Weak ", 200),
    ("Very Weak ", 300),
    ("Weak ", 400),
    ("Fairly Weak ", 500),
    ("Somewhat Weak ", 600),
    ("", 700),
    ("Somewhat Strong ", 800),
    ("Fairly Strong ", 1000),
    ("Strong ", 1200),
    ("Very Strong ", 1400),
    ("Extremely Strong ", 1600),
    ("Somewhat Powerful ", 2000),
    ("Fairly Powerful ", 2400),
    ("Powerful ", 2800),
    ("Very Powerful ", 3200),
    ("Extremely Powerful ", 4000),
    ("Superhuman ", 4800),
    ("Demi-Godly ", 5600),
    ("Godly ", 10000),
    ("Ultimate ", 20000),
];

/// C `ITEM_TYPES[]` (`tool.c:2623-2626`): the first 8 templates take the
/// (post `BASE_MAPPING`-remapped) `base` as a quality-tier suffix
/// (`"armor%dq3"` etc, i.e. `armor1q3..armor10q3`); the rest are fixed
/// template keys that ignore `base` entirely. `plain_gold_ring` appears
/// twice on purpose (double roll weight vs. every other fixed entry).
const ITEM_TYPE_TEMPLATES: [&str; 21] = [
    "armor{}q3",
    "helmet{}q3",
    "sleeves{}q3",
    "leggings{}q3",
    "sword{}q3",
    "twohanded{}q3",
    "dagger{}q3",
    "staff{}q3",
    "plain_gold_ring",
    "plain_gold_ring",
    "green_hat",
    "brown_hat",
    "blue_cape",
    "brown_cape",
    "red_belt",
    "amulet",
    "boots",
    "vest",
    "trousers",
    "bracelet",
    "gloves",
];

/// C `lowhi_random` (`tool.c:2791-2799`): non-gaussian random value from 1
/// to `val` inclusive, low numbers much more likely. Only ever called here
/// with `val` in `1..=7`, so `(val + 1)^4 - 1` fits comfortably in `u32`.
fn lowhi_random(seed: &mut u32, val: i32) -> i32 {
    let range = (i64::from(val) + 1).pow(4) - 1;
    let rolled = f64::from(legacy_random_below_from_seed(seed, range as u32)) + 1.0;
    // C assigns the `double` `sqrt(sqrt(...))` result into an `int`,
    // truncating toward zero - identical to `as i32` here since the value
    // is never negative.
    let p = rolled.sqrt().sqrt() as i32;
    val - p + 1
}

/// C `set_item_requirements_sub` (`tool.c:2392-2514`). The commented-out
/// `lvl += sum - high;` line in C is dead code and intentionally not
/// ported.
fn set_item_requirements_sub(item: &mut Item, maxlvl: u8) {
    if item.flags.contains(ItemFlags::BEYONDBOUNDS) {
        return;
    }
    let mut high: i32 = 0;
    for n in 0..MAX_MODIFIERS {
        let idx = item.modifier_index[n];
        if idx == CharacterValue::Weapon as i16
            || idx == CharacterValue::Armor as i16
            || idx == CharacterValue::Speed as i16
            || idx == CharacterValue::Demon as i16
            || idx == CharacterValue::Light as i16
        {
            continue;
        }
        if idx >= 0 {
            high = high.max(i32::from(item.modifier_value[n]));
        }
    }
    let (lvl, needs_arch): (u8, bool) = match high {
        0 => (0, false),
        1 => (2, false),
        2 => (3, false),
        3 => (5, false),
        4 => (10, false),
        5 => (15, false),
        6 => (17, false),
        7 => (20, false),
        8 => (23, false),
        9 => (26, false),
        10 => (30, true),
        11 => (33, true),
        12 => (36, true),
        13 => (40, true),
        14 => (43, true),
        15 => (46, true),
        16 => (50, true),
        17 => (53, true),
        18 => (56, true),
        19 => (60, true),
        20 => (63, true),
        21 => (66, true),
        22 => (70, true),
        23 => (73, true),
        _ => (76, true),
    };
    if needs_arch {
        item.needs_class |= 8;
    }
    item.min_level = item.min_level.max(lvl.min(maxlvl));
}

/// C `set_item_requirements` (`tool.c:2581-2583`).
fn set_item_requirements(item: &mut Item) {
    set_item_requirements_sub(item, 120);
}

/// C: the shared "no special items" eligibility prelude duplicated at the
/// top of both `can_receive_mod` and `can_give_mod` (`tool.c:1845-1928`) -
/// no item driver, template id is generic/hardkill/none, and not
/// `IF_NOENHANCE`.
fn shrine_item_eligible(item: &Item) -> bool {
    if item.driver != 0 {
        return false;
    }
    if item.template_id != IID_GENERIC_SPECIAL
        && item.template_id != IID_HARDKILL
        && item.template_id != 0
    {
        return false;
    }
    !item.flags.contains(ItemFlags::NOENHANCE)
}

/// C `can_give_mod(in, slot)` (`tool.c:1893-1928`): true if `item`'s
/// `slot`-th modifier is a nonzero, non-requirement, non-weapon/armor/
/// speed/demon/light modifier that can be donated to another item.
fn shrine_can_give_mod(item: &Item, slot: usize) -> bool {
    if !shrine_item_eligible(item) {
        return false;
    }
    if item.modifier_value[slot] == 0 {
        return false;
    }
    let idx = item.modifier_index[slot];
    if idx < 0 {
        return false;
    }
    !(idx == CharacterValue::Weapon as i16
        || idx == CharacterValue::Armor as i16
        || idx == CharacterValue::Speed as i16
        || idx == CharacterValue::Demon as i16
        || idx == CharacterValue::Light as i16)
}

/// C `can_receive_mod(in, pslot, v)` (`tool.c:1845-1891`): true (returning
/// the first free modifier slot) if `item` doesn't already carry modifier
/// `v`, has at most two other "generic" modifiers already (weapon/armor/
/// demon/light modifiers don't count toward that cap - note `V_SPEED` is
/// *not* in this exclusion list, matching C's switch exactly even though
/// `can_give_mod`'s exclusion list does include it), and has a free
/// modifier slot to put it in.
fn shrine_can_receive_mod(item: &Item, target_mod_index: i16) -> Option<usize> {
    if !shrine_item_eligible(item) {
        return None;
    }
    let mut generic_mod_count = 0;
    for n in 0..MAX_MODIFIERS {
        if item.modifier_value[n] != 0 && item.modifier_index[n] == target_mod_index {
            return None;
        }
        let idx = item.modifier_index[n];
        if idx == CharacterValue::Weapon as i16
            || idx == CharacterValue::Armor as i16
            || idx == CharacterValue::Demon as i16
            || idx == CharacterValue::Light as i16
        {
            continue;
        }
        if item.modifier_value[n] > 0 && idx >= 0 {
            generic_mod_count += 1;
        }
    }
    if generic_mod_count > 2 {
        return None;
    }
    (0..MAX_MODIFIERS).find(|&n| item.modifier_value[n] == 0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RandomShrineWeldingResult {
    /// C `random.c:1932-1935`: "You are not powerful enough to use this
    /// shrine."
    NotPowerfulEnough,
    /// C `random.c:1937-1940`: "Only paying players can use this shrine."
    NotPaying,
    /// C `random.c:1953-1957`: no worn item has any donatable modifier -
    /// "...leaves with a laugh of contempt."
    Contempt,
    /// C `random.c:1985-1989`: a donor was found but no *other* worn item
    /// can receive its modifier - "...leaves with a laugh of regret."
    Regret,
    /// C `random.c:1979-1982`: "You found bug #337 (...)" - the picked
    /// `RANDOM(cnt)` index somehow wasn't found on the second pass. Should
    /// be unreachable given the counting pass always agrees with the
    /// picking pass, kept only because C keeps the safety net.
    Bug,
    Used {
        item1_name: String,
        item2_name: String,
    },
}

impl World {
    /// C `create_special_item(strength, base, potionprob, maxchance)`
    /// (`src/system/tool.c:2620-2789`). Builds a fresh, randomly enchanted
    /// item instance from `loader`'s templates and returns it un-inserted
    /// into `self.items` - matching C, where every current caller
    /// (`add_special_store`) immediately copies the result elsewhere and
    /// destroys the throwaway `it[]` slot. `None` mirrors every C `return
    /// 0` (unknown item type, failed template creation, out-of-range
    /// strength, no eligible special entry, or an overlong description).
    pub fn create_special_item(
        &mut self,
        loader: &mut ZoneLoader,
        strength: i32,
        base: i32,
        potionprob: i32,
        maxchance: i32,
    ) -> Option<Item> {
        // C: `if (RANDOM(potionprob))` - true with probability
        // `(potionprob - 1) / potionprob` (0% at potionprob=1, the value
        // every current caller passes).
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, potionprob.max(0) as u32)
            != 0
        {
            let strength =
                strength + legacy_random_below_from_seed(&mut self.legacy_random_seed, 4) as i32;
            let level = if strength < 6 {
                1
            } else if strength < 9 {
                2
            } else {
                3
            };
            let key = match legacy_random_below_from_seed(&mut self.legacy_random_seed, 3) {
                0 => format!("healing_potion{level}"),
                1 => format!("mana_potion{level}"),
                _ => format!("combo_potion{level}"),
            };
            return loader.instantiate_item_template(&key, None).ok();
        }

        // C: `BASE_MAPPING[base / 10]` for a `base` in `1..=100` that's a
        // multiple of ten is the identity function (`BASE_MAPPING[n] ==
        // n`), so the lookup table collapses to plain integer division.
        let base = if (1..=100).contains(&base) && base % 10 == 0 {
            base / 10
        } else {
            base
        };

        let type_index = legacy_random_below_from_seed(
            &mut self.legacy_random_seed,
            ITEM_TYPE_TEMPLATES.len() as u32,
        ) as usize;
        let template = ITEM_TYPE_TEMPLATES[type_index];
        let key = if template.contains("{}") {
            template.replacen("{}", &base.to_string(), 1)
        } else {
            template.to_string()
        };
        let mut item = loader.instantiate_item_template(&key, None).ok()?;

        let strength = if (1..=7).contains(&strength) {
            lowhi_random(&mut self.legacy_random_seed, strength)
        } else if strength > 7 {
            strength - 7 + lowhi_random(&mut self.legacy_random_seed, 7)
        } else {
            strength
        };
        if !(1..=20).contains(&strength) {
            return None;
        }
        let (str_desc, priceadd) = STRENGTH_DESCRIPTIONS[(strength - 1) as usize];

        // C: two-pass weighted roll over `special_item[]`, filtered by
        // `maxchance` and (only when the rolled base item is itself a
        // weapon) the entry's required weapon-class flag.
        let item_flags = item.flags;
        let is_weapon = item_flags.intersects(ItemFlags::WEAPON);
        let eligible = |entry: &SpecialItemEntry| -> bool {
            if !entry.needflag.is_empty() && is_weapon && !item_flags.intersects(entry.needflag) {
                return false;
            }
            maxchance >= entry.chance
        };
        let total: i32 = SPECIAL_ITEM_TABLE
            .iter()
            .filter(|entry| eligible(entry))
            .map(|entry| entry.chance)
            .sum();
        if total <= 0 {
            return None;
        }
        let roll = legacy_random_below_from_seed(&mut self.legacy_random_seed, total as u32) as i32;
        let mut running = 0;
        let entry = SPECIAL_ITEM_TABLE
            .iter()
            .filter(|entry| eligible(entry))
            .find(|entry| {
                running += entry.chance;
                running > roll
            })?;

        let description = format!(
            "{} of {}{}{}.",
            item.name,
            if entry.the { "the " } else { "" },
            str_desc,
            entry.name
        );
        if description.len() < 3 || description.len() > 79 {
            return None;
        }

        let mut m = 0;
        for n in 0..MAX_MODIFIERS {
            if m >= SPECIAL_ITEM_MOD_SLOTS || entry.mod_index[m] == -1 {
                break;
            }
            if item.modifier_index[n] == 0 {
                item.modifier_index[n] = entry.mod_index[m];
                item.modifier_value[n] = strength as i16;
                m += 1;
            }
        }

        item.description = description;
        item.value = item
            .value
            .saturating_add((priceadd * entry.pricemulti) as u32);
        set_item_requirements(&mut item);
        item.template_id = IID_GENERIC_SPECIAL;

        Some(item)
    }

    /// C `add_special_store(cn)` (`src/module/merchants/store.c:229-323`):
    /// rolls a random strength (1-22, reused directly as
    /// `create_special_item`'s `strength` argument) and derived `base`
    /// tier, then adds one no-junk-tier special item to the merchant's
    /// store. Returns `false` if the merchant has no store yet or item
    /// creation failed (matches C's silent `return 0`).
    pub fn add_special_store(&mut self, merchant_id: CharacterId, loader: &mut ZoneLoader) -> bool {
        if !self.merchant_stores.contains_key(&merchant_id) {
            return false;
        }
        let roll = legacy_random_below_from_seed(&mut self.legacy_random_seed, 22) as i32 + 1;
        let base = match roll {
            1..=3 => 1,
            4 | 5 => 10,
            6 => 20,
            7 | 8 => 30,
            9 => 40,
            10 | 11 => 50,
            12 | 13 => 60,
            14 | 15 => 70,
            16 | 17 => 80,
            18..=20 => 90,
            _ => 100,
        };
        // C: `create_special_item(str, base, 1, 1000)` - `potionprob=1`
        // (never a potion) and `maxchance=1000` (`SP_SOME_CONST` tier -
        // "no junk").
        let Some(item) = self.create_special_item(loader, roll, base, 1, 1000) else {
            return false;
        };
        self.add_item_to_merchant_store(merchant_id, item);
        true
    }

    /// C `set_item_requirements(in)` (`tool.c:2581-2583`) exposed as a
    /// public `World` method: recomputes `min_level`/`needs_class` from an
    /// item's current modifiers after something (e.g. [`Self::
    /// apply_random_shrine_welding`]) rewrites `modifier_index`/
    /// `modifier_value` directly. No-op if `item_id` doesn't exist.
    pub fn recompute_item_requirements(&mut self, item_id: ItemId) {
        if let Some(item) = self.items.get_mut(&item_id) {
            set_item_requirements(item);
        }
    }

    /// C `shrine_welding(in, cn, nr, level, ppd)` (`src/area/14/random.c:
    /// 1929-2013`): picks one random enhanceable-mod slot off any of the
    /// player's 12 worn items and welds it onto another random worn item
    /// that has room for it, wiping the mod off the donor. Eligibility
    /// mirrors C's `can_give_mod`/`can_receive_mod` exactly (see
    /// [`shrine_can_give_mod`]/[`shrine_can_receive_mod`]). Does not call
    /// `shrine_set`/`sendquestlog` itself - matching every other
    /// `apply_random_shrine_*` helper, the caller resends the questlog and
    /// marks the shrine used only on [`RandomShrineWeldingResult::Used`].
    pub fn apply_random_shrine_welding(
        &mut self,
        character_id: CharacterId,
        level: u8,
    ) -> RandomShrineWeldingResult {
        let Some(character) = self.characters.get(&character_id) else {
            return RandomShrineWeldingResult::Bug;
        };
        if character.level + character.level / 4 + 2 < u32::from(level) {
            return RandomShrineWeldingResult::NotPowerfulEnough;
        }
        if !character.flags.contains(CharacterFlags::PAID) {
            return RandomShrineWeldingResult::NotPaying;
        }

        let worn: Vec<ItemId> = LEGACY_EQUIPMENT_SLOTS
            .clone()
            .filter_map(|slot| character.inventory.get(slot).copied().flatten())
            .collect();

        // Phase 1: pick a random (item, mod-slot) willing to give up its
        // modifier (C's first `n`/`m` double loop + `RANDOM(cnt)`).
        let mut total_give = 0i32;
        for item_id in &worn {
            let Some(item) = self.items.get(item_id) else {
                continue;
            };
            for slot in 0..MAX_MODIFIERS {
                if shrine_can_give_mod(item, slot) {
                    total_give += 1;
                }
            }
        }
        if total_give == 0 {
            return RandomShrineWeldingResult::Contempt;
        }
        let mut pick =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, total_give as u32) as i32;
        let mut give: Option<(ItemId, usize)> = None;
        'give: for item_id in &worn {
            let Some(item) = self.items.get(item_id) else {
                continue;
            };
            for slot in 0..MAX_MODIFIERS {
                if shrine_can_give_mod(item, slot) {
                    if pick == 0 {
                        give = Some((*item_id, slot));
                        break 'give;
                    }
                    pick -= 1;
                }
            }
        }
        let Some((in2, slot2)) = give else {
            return RandomShrineWeldingResult::Bug;
        };
        let target_mod_index = self.items[&in2].modifier_index[slot2];

        // Phase 2: pick a random *other* worn item with room to receive
        // that same modifier (C's second `n` loop + `RANDOM(cnt)`).
        let mut total_receive = 0i32;
        for item_id in &worn {
            if *item_id == in2 {
                continue;
            }
            if let Some(item) = self.items.get(item_id) {
                if shrine_can_receive_mod(item, target_mod_index).is_some() {
                    total_receive += 1;
                }
            }
        }
        if total_receive == 0 {
            return RandomShrineWeldingResult::Regret;
        }
        let mut pick2 =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, total_receive as u32)
                as i32;
        let mut receive: Option<(ItemId, usize)> = None;
        for item_id in &worn {
            if *item_id == in2 {
                continue;
            }
            let Some(item) = self.items.get(item_id) else {
                continue;
            };
            if let Some(slot1) = shrine_can_receive_mod(item, target_mod_index) {
                if pick2 == 0 {
                    receive = Some((*item_id, slot1));
                    break;
                }
                pick2 -= 1;
            }
        }
        let Some((in1, slot1)) = receive else {
            return RandomShrineWeldingResult::Bug;
        };

        let (mod_index, mod_value) = {
            let donor = &self.items[&in2];
            (donor.modifier_index[slot2], donor.modifier_value[slot2])
        };

        let item1_name = self.items[&in1].name.clone();
        let item2_name = self.items[&in2].name.clone();

        if let Some(item1) = self.items.get_mut(&in1) {
            item1.modifier_index[slot1] = mod_index;
            item1.modifier_value[slot1] = mod_value;
            // C: `if (!strstr(it[in1].description, "Christmas"))
            // snprintf(it[in1].description, ..., "%s of Welding.",
            // it[in1].name);` - `name` (max 40 bytes in C) plus the fixed
            // suffix never overflows the 80-byte description buffer, so no
            // truncation guard is needed here.
            if !item1.description.contains("Christmas") {
                item1.description = format!("{} of Welding.", item1.name);
            }
        }
        self.recompute_item_requirements(in1);

        if let Some(item2) = self.items.get_mut(&in2) {
            item2.modifier_index[slot2] = 0;
            item2.modifier_value[slot2] = 0;
            if !item2.description.contains("Christmas") {
                item2.description = format!("{} of Unwelding.", item2.name);
            }
        }
        self.recompute_item_requirements(in2);

        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }

        RandomShrineWeldingResult::Used {
            item1_name,
            item2_name,
        }
    }

    fn set_merchant_last_special_add(&mut self, merchant_id: CharacterId, tick: u64) {
        if let Some(driver_state) = self
            .characters
            .get_mut(&merchant_id)
            .and_then(|merchant| merchant.driver_state.as_mut())
        {
            match driver_state {
                CharacterDriverState::Merchant(data) => data.last_special_add = tick,
                // C `aclerk_driver`'s special-store timer block is
                // identical to `merchant_driver`'s.
                CharacterDriverState::Aclerk(data) => data.last_special_add = tick,
                _ => {}
            }
        }
    }

    /// C `merchant_driver`'s special-store handling
    /// (`src/module/merchants/merchant.c:337-347` initial seeding,
    /// `:546-548` the 12h refresh): a `special`-flagged merchant seeds
    /// five special wares the first time its store is created, then adds
    /// one more every 12 real-time hours (`dat->lastadd`, ported as
    /// `MerchantDriverData::last_special_add`). Not folded into
    /// [`World::process_merchant_actions`] itself since that would give
    /// every existing non-special-store caller a new mandatory
    /// `&mut ZoneLoader` parameter; call this once per tick alongside it
    /// instead. Returns the merchants whose store actually changed this
    /// tick, so the caller can persist them (C: each successful
    /// `add_special_store` call ends with its own
    /// `queue_merchant_full_save(cn)`).
    pub fn refresh_special_stores(&mut self, loader: &mut ZoneLoader) -> Vec<CharacterId> {
        // C: `aclerk_driver`'s special-store block (`merchant.c`) is
        // identical to `merchant_driver`'s, so `CDR_ACLERK` shares this
        // refresh path with `CDR_MERCHANT`.
        let merchant_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                (character.driver == CDR_MERCHANT || character.driver == CDR_ACLERK)
                    && character.flags.contains(CharacterFlags::USED)
            })
            .map(|character| character.id)
            .collect();
        let tick = self.tick.0;
        let mut changed = Vec::new();

        for merchant_id in merchant_ids {
            if !self.merchant_stores.contains_key(&merchant_id) {
                continue;
            }
            let Some((special, last_special_add)) =
                self.characters.get(&merchant_id).and_then(|merchant| {
                    match merchant.driver_state.as_ref() {
                        Some(CharacterDriverState::Merchant(data)) => {
                            Some((data.special, data.last_special_add))
                        }
                        Some(CharacterDriverState::Aclerk(data)) => {
                            Some((data.special, data.last_special_add))
                        }
                        _ => None,
                    }
                })
            else {
                continue;
            };
            if special == 0 {
                continue;
            }
            if last_special_add == 0 {
                for _ in 0..5 {
                    if self.add_special_store(merchant_id, loader) {
                        changed.push(merchant_id);
                    }
                }
                // C stamps `dat->lastadd = ticker` unconditionally once the
                // store exists; `.max(1)` just keeps our "never seeded yet"
                // sentinel (`0`) from re-triggering forever if this runs on
                // tick 0 itself.
                self.set_merchant_last_special_add(merchant_id, tick.max(1));
            } else if tick > last_special_add + MERCHANT_SPECIAL_REFRESH_TICKS {
                if self.add_special_store(merchant_id, loader) {
                    changed.push(merchant_id);
                }
                self.set_merchant_last_special_add(merchant_id, tick);
            }
        }
        changed
    }
}

/// C `TICKS * 60 * 60 * 12` (`merchant.c:546`/`:846`) - the special-store
/// refresh period, matching the sibling `MERCHANT_MEMORY_CLEAR_TICKS`
/// pattern already used for the greeting-memory clear. `pub(crate)` so
/// `world/tests/special_item.rs` can assert the exact refresh boundary.
pub(crate) const MERCHANT_SPECIAL_REFRESH_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
