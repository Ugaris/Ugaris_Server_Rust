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

    fn set_merchant_last_special_add(&mut self, merchant_id: CharacterId, tick: u64) {
        if let Some(CharacterDriverState::Merchant(data)) = self
            .characters
            .get_mut(&merchant_id)
            .and_then(|merchant| merchant.driver_state.as_mut())
        {
            data.last_special_add = tick;
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
        let merchant_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MERCHANT && character.flags.contains(CharacterFlags::USED)
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
