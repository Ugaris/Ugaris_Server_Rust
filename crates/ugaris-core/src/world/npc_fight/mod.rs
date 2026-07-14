use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum FightDriverTaskKind {
    Freeze,
    Fireball,
    Ball,
    Flash,
    Warcry,
    Attack,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    Regenerate,
    Distance3,
    Distance7,
    Bless,
    EarthRain,
    EarthMud,
    Heal,
    MagicShield,
    Pulse,
    AttackBack,
    Flee,
    FireRing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct FightDriverTask {
    pub(crate) kind: FightDriverTaskKind,
    pub(crate) value: i32,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_LOW_PRIO: i32 = 1;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_MED_PRIO: i32 = 500;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_HIGH_PRIO: i32 = 750;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn order_fight_driver_tasks(
    tasks: &mut [FightDriverTask],
    character_level: i32,
    mut random_below: impl FnMut(i32) -> i32,
) {
    let silliness = character_level / 2 + 5;
    if silliness > 1 {
        for task in tasks.iter_mut() {
            task.value += random_below(silliness).clamp(0, silliness - 1);
        }
    }
    // Kept as an explicit comparator to mirror C's qsort compare callback.
    #[allow(clippy::unnecessary_sort_by)]
    tasks.sort_by(|left, right| right.value.cmp(&left.value));
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn fight_driver_attackback_may_run(tasks: &[FightDriverTask], index: usize) -> bool {
    tasks
        .get(index + 1)
        .is_some_and(|task| task.kind == FightDriverTaskKind::Attack)
}

/// C `fight_driver_attack_enemy`'s 10 positional `no*` suppression
/// parameters (`src/system/drvlib.c:1682`). The NPC-side `CDR_SIMPLEBADDY`/
/// `CDR_DUNGEONFIGHTER` callers always pass all-zero (see
/// `fight_driver_attack_visible`'s `else` branch that calls
/// `fight_driver_attack_enemy(cn, ..., 0,0,0,0,0,0,0,0,0,0)` when the
/// attacker has no `lostcon_ppd`), which `Default`/`From<bool>` (mapping a
/// bare `nomove` bool, matching every existing call site before this type
/// existed) both reproduce. The player-side caller (`ppd->nobless`, etc.)
/// is `process_lostcon_attack_action_with_random`'s `suppressions`
/// argument, built by `ugaris-server` from the lingering `PlayerRuntime`'s
/// `no*` toggles - public so that crate can construct one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FightDriverSuppressions {
    pub nomove: bool,
    pub nobless: bool,
    pub noheal: bool,
    pub noflash: bool,
    pub nofireball: bool,
    pub noball: bool,
    pub noshield: bool,
    pub nowarcry: bool,
    pub nofreeze: bool,
    pub nopulse: bool,
}

impl From<bool> for FightDriverSuppressions {
    /// Every pre-existing call site only ever suppressed movement
    /// (`nomove`); keep those call sites compiling unchanged.
    fn from(nomove: bool) -> Self {
        Self {
            nomove,
            ..Self::default()
        }
    }
}

mod attack;
mod distance;
mod spells;
mod support;
mod tasks;

pub(crate) fn simple_baddy_earth_task_value(good_fields: i32) -> i32 {
    if good_fields < 1 {
        0
    } else if good_fields < 2 {
        FIGHT_DRIVER_LOW_PRIO
    } else if good_fields < 4 {
        FIGHT_DRIVER_MED_PRIO
    } else {
        FIGHT_DRIVER_HIGH_PRIO + good_fields * 20
    }
}

pub(crate) fn simple_baddy_attack_task_value(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let attack = simple_baddy_attack_skill(character, items);
    if character_value_present(character, CharacterValue::Attack) != 0 {
        FIGHT_DRIVER_MED_PRIO + attack * 2 / 7 + 10
    } else {
        FIGHT_DRIVER_LOW_PRIO + attack / 3
    }
}

pub(crate) fn simple_baddy_attack_skill(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let spell_average = spell_average(
        character_value(character, CharacterValue::Bless),
        character_value(character, CharacterValue::Heal),
        character_value(character, CharacterValue::Freeze),
        character_value(character, CharacterValue::MagicShield),
        character_value(character, CharacterValue::Flash),
        character_value(character, CharacterValue::Fireball),
        character_value(character, CharacterValue::Pulse),
    );
    attack_skill(
        character_value_present(character, CharacterValue::Attack) != 0,
        simple_baddy_fight_skill(character, items),
        character_value(character, CharacterValue::Attack),
        character_value(character, CharacterValue::Tactics),
        0,
        character.flags.contains(CharacterFlags::EDEMON),
        character.level as i32,
        spell_average,
    )
}

pub(crate) fn simple_baddy_fight_skill(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let Some(item_id) = character
        .inventory
        .get(worn_slot::RIGHT_HAND)
        .and_then(|slot| *slot)
    else {
        return character_value(character, CharacterValue::Hand);
    };
    let Some(item) = items.get(&item_id) else {
        return character_value(character, CharacterValue::Hand);
    };
    if !item.flags.intersects(ItemFlags::WEAPON) {
        return character_value(character, CharacterValue::Hand);
    }

    let mut value = 0;
    if item.flags.contains(ItemFlags::HAND) {
        value = value.max(character_value(character, CharacterValue::Hand));
    }
    if item.flags.contains(ItemFlags::DAGGER) {
        value = value.max(character_value(character, CharacterValue::Dagger));
    }
    if item.flags.contains(ItemFlags::STAFF) {
        value = value.max(character_value(character, CharacterValue::Staff));
    }
    if item.flags.contains(ItemFlags::SWORD) {
        value = value.max(character_value(character, CharacterValue::Sword));
    }
    if item.flags.contains(ItemFlags::TWOHAND) {
        value = value.max(character_value(character, CharacterValue::TwoHand));
    }
    value
}

pub(crate) fn predicted_fireball_target(caster: &Character, target: &Character) -> (usize, usize) {
    if target.action != action::WALK {
        return (usize::from(target.x), usize::from(target.y));
    }

    let Some(direction) = Direction::try_from(target.dir).ok() else {
        return (usize::from(target.x), usize::from(target.y));
    };
    let (dx, dy) = direction.delta();
    let mut eta = char_dist(caster, target) / 2
        + speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            8,
        );

    eta -= target.duration - target.step;
    if eta <= 0 {
        return (usize::from(target.x), usize::from(target.y));
    }

    for n in 1..6 {
        eta -= target.duration;
        if eta <= 0 {
            let target_x =
                offset_coordinate(usize::from(target.x), dx * n).unwrap_or(usize::from(target.x));
            let target_y =
                offset_coordinate(usize::from(target.y), dy * n).unwrap_or(usize::from(target.y));
            return (target_x, target_y);
        }
    }

    (usize::from(target.x), usize::from(target.y))
}

pub(crate) fn simple_baddy_earth_spell_target(target: &Character) -> (usize, usize) {
    if target.action != action::WALK {
        return (usize::from(target.x), usize::from(target.y));
    }

    let x = i32::from(target.tox) + i32::from(target.tox) - i32::from(target.x);
    let y = i32::from(target.toy) + i32::from(target.toy) - i32::from(target.y);
    (
        usize::try_from(x).unwrap_or(0),
        usize::try_from(y).unwrap_or(0),
    )
}

pub(crate) fn ball_target_damage_multiplier(enemy_count: i32) -> i32 {
    match enemy_count.clamp(1, 10) {
        1 => 100,
        2 => 95,
        3 => 90,
        4 => 85,
        5 => 80,
        6 => 75,
        7 => 70,
        8 => 65,
        9 => 60,
        _ => 55,
    }
}
