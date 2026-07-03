use super::*;

pub(crate) const ORBSPAWN_RESPAWN_SECONDS: u64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssembleApplyResult {
    Assembled,
    MissingPlayer,
    MissingItem,
    TemplateUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ForestSpadeApplyResult {
    Found { item_name: String },
    FoundMoney { amount: u32 },
    AlreadyDug,
    Nothing,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ForestChestApplyResult {
    FoundMoney { amount: u32 },
    Empty,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JunkpileApplyResult {
    Found { item_name: String },
    FoundMoney { amount: u32 },
    Nothing,
    CursorOccupied,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrbSpawnApplyResult {
    Granted { item_name: String, special: bool },
    Cooldown { days_left: String },
    Nothing,
    CursorOccupied,
    MissingPlayer,
}

pub(crate) fn legacy_orb_value_from_seed(seed: u64) -> CharacterValue {
    const VALUES: [CharacterValue; 32] = [
        CharacterValue::Endurance,
        CharacterValue::Hp,
        CharacterValue::Mana,
        CharacterValue::Wisdom,
        CharacterValue::Intelligence,
        CharacterValue::Agility,
        CharacterValue::Strength,
        CharacterValue::Barter,
        CharacterValue::Percept,
        CharacterValue::Stealth,
        CharacterValue::Hand,
        CharacterValue::Warcry,
        CharacterValue::Surround,
        CharacterValue::BodyControl,
        CharacterValue::SpeedSkill,
        CharacterValue::Heal,
        CharacterValue::Fireball,
        CharacterValue::Tactics,
        CharacterValue::Duration,
        CharacterValue::Rage,
        CharacterValue::Bless,
        CharacterValue::Freeze,
        CharacterValue::MagicShield,
        CharacterValue::Flash,
        CharacterValue::Pulse,
        CharacterValue::Dagger,
        CharacterValue::Staff,
        CharacterValue::Sword,
        CharacterValue::TwoHand,
        CharacterValue::Attack,
        CharacterValue::Parry,
        CharacterValue::Immunity,
    ];
    VALUES[(seed as usize) % VALUES.len()]
}

pub(crate) fn grant_orb_spawn_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    anti: bool,
    special: bool,
    seed: u64,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }

    let template = if anti { "empty_anti_orb" } else { "empty_orb" };
    let Ok(mut item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return None;
    };
    let value = legacy_orb_value_from_seed(seed) as u8;
    let value_name = CHARACTER_VALUE_NAMES[usize::from(value)];
    if anti {
        if special {
            item.name = format!("Extracting Anti-Orb of {value_name}");
            item.description =
                format!("A dark orb that extracts {value_name} from items and crystallizes it.");
            ensure_drdata_len(&mut item, 3);
            item.driver_data[2] = 1;
        } else {
            item.name = format!("Anti-Orb of {value_name}");
            item.description = format!("A dark orb that removes {value_name} from items.");
            ensure_drdata_len(&mut item, 3);
            item.driver_data[2] = 0;
        }
    } else {
        item.name = format!("Orb of {value_name}");
        ensure_drdata_len(&mut item, 2);
    }
    item.driver_data[0] = value;
    item.driver_data[1] = 1;

    let item_id = item.id;
    let item_name = item.name.clone();
    let Some(character) = world.characters.get_mut(&character_id) else {
        return None;
    };
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn grant_clan_jewel(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
) -> bool {
    let Ok(mut item) = loader.instantiate_item_template("clan_jewel", Some(character_id)) else {
        return false;
    };
    let item_id = item.id;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    match give_item_to_character(character, &mut item, GiveItemFlags::NONE) {
        GiveItemResult::Ok => {
            world.add_item(item);
            world.schedule_item_driver_timer(
                item_id,
                CharacterId(0),
                ugaris_core::item_driver::CLANJEWEL_CHECK_INTERVAL_TICKS,
            );
            true
        }
        GiveItemResult::Money
        | GiveItemResult::Dropped
        | GiveItemResult::Full
        | GiveItemResult::Failed => false,
    }
}

pub(crate) fn instantiate_orb_with_modifier(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    modifier: i16,
) -> Option<Item> {
    let value = u8::try_from(modifier).ok()?;
    let value_name = CHARACTER_VALUE_NAMES.get(usize::from(value))?;
    let Ok(mut item) = loader.instantiate_item_template("empty_orb", Some(character_id)) else {
        return None;
    };
    item.name = format!("Orb of {value_name}");
    ensure_drdata_len(&mut item, 2);
    item.driver_data[0] = value;
    item.driver_data[1] = 1;
    Some(item)
}

pub(crate) fn ensure_drdata_len(item: &mut Item, len: usize) {
    if item.driver_data.len() < len {
        item.driver_data.resize(len, 0);
    }
}

pub(crate) fn apply_orb_spawn(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    spawn_item_id: ItemId,
    character_id: CharacterId,
    area_id: u16,
    realtime_seconds: u64,
    anti: bool,
    special: bool,
    random_seed: u64,
) -> OrbSpawnApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return OrbSpawnApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return OrbSpawnApplyResult::CursorOccupied;
    }
    let Some(player) = player else {
        return OrbSpawnApplyResult::MissingPlayer;
    };
    let Some(spawner) = world.items.get(&spawn_item_id) else {
        return OrbSpawnApplyResult::Nothing;
    };
    let location_id =
        u32::from(spawner.x) + (u32::from(spawner.y) << 8) + (u32::from(area_id) << 16);
    if let Some(last_used) = player.orb_spawn_last_used_seconds(location_id) {
        if last_used.saturating_add(ORBSPAWN_RESPAWN_SECONDS) > realtime_seconds {
            let remaining = last_used
                .saturating_add(ORBSPAWN_RESPAWN_SECONDS)
                .saturating_sub(realtime_seconds);
            return OrbSpawnApplyResult::Cooldown {
                days_left: format!("{:.2}", remaining as f64 / 60.0 / 60.0 / 24.0),
            };
        }
    }

    player.mark_orb_spawn_used(location_id, realtime_seconds);
    match grant_orb_spawn_item(world, loader, character_id, anti, special, random_seed) {
        Some(item_name) => OrbSpawnApplyResult::Granted { item_name, special },
        None => OrbSpawnApplyResult::Nothing,
    }
}

pub(crate) const FOREST_SPADE_DIG_COOLDOWN_SECONDS: u64 = 365 * 24 * 60 * 60;

pub(crate) fn apply_forest_spade_find(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    find: ForestSpadeFind,
    realtime_seconds: u64,
    random_seed: u64,
) -> ForestSpadeApplyResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return ForestSpadeApplyResult::CursorOccupied;
    }

    match find {
        ForestSpadeFind::ForestNote1 => {
            grant_template_item_to_cursor(world, loader, character_id, "forest_note1")
                .map(|item_name| ForestSpadeApplyResult::Found { item_name })
                .unwrap_or(ForestSpadeApplyResult::Nothing)
        }
        ForestSpadeFind::BranningtonTreasure { dig_index } => {
            let Some(player) = player else {
                return ForestSpadeApplyResult::MissingPlayer;
            };
            let last_dig = player.treasure_dig_last_seconds(dig_index);
            if last_dig != 0
                && realtime_seconds.saturating_sub(last_dig) < FOREST_SPADE_DIG_COOLDOWN_SECONDS
            {
                return ForestSpadeApplyResult::AlreadyDug;
            }
            let amount = 100_000 + legacy_random(random_seed, 100_000);
            if !grant_money_to_cursor(world, loader, character_id, amount) {
                return ForestSpadeApplyResult::Nothing;
            }
            if !player.mark_treasure_dig(dig_index, realtime_seconds) {
                return ForestSpadeApplyResult::MissingPlayer;
            }
            player.set_forestbran_done(dig_index);
            ForestSpadeApplyResult::FoundMoney { amount }
        }
    }
}

pub(crate) fn apply_forest_chest(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    amount: u32,
    imp_flag_mask: u32,
) -> ForestChestApplyResult {
    if world.characters.get(&character_id).is_none() {
        return ForestChestApplyResult::MissingPlayer;
    }
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return ForestChestApplyResult::CursorOccupied;
    }
    let Some(player) = player else {
        return ForestChestApplyResult::MissingPlayer;
    };
    if player.area3_imp_flags() & imp_flag_mask != 0 {
        return ForestChestApplyResult::Empty;
    }
    if !grant_money_to_cursor(world, loader, character_id, amount) {
        return ForestChestApplyResult::Empty;
    }
    if !player.mark_area3_imp_flag(imp_flag_mask) {
        return ForestChestApplyResult::Empty;
    }
    ForestChestApplyResult::FoundMoney { amount }
}

pub(crate) fn apply_junkpile_search(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    level: u8,
    random_seed: u64,
) -> JunkpileApplyResult {
    if world.characters.get(&character_id).is_none() {
        return JunkpileApplyResult::MissingPlayer;
    }
    if world
        .characters
        .get(&character_id)
        .is_some_and(|character| character.cursor_item.is_some())
    {
        return JunkpileApplyResult::CursorOccupied;
    }

    let roll = legacy_random(random_seed, 10);
    let result = match roll {
        1 | 2 | 4 | 5 | 7 | 9 => {
            grant_template_item_to_cursor(world, loader, character_id, "steelbar")
                .map(|item_name| JunkpileApplyResult::Found { item_name })
                .unwrap_or(JunkpileApplyResult::Nothing)
        }
        3 => {
            let max = 100_u32.saturating_mul(u32::from(level));
            let amount =
                legacy_random(random_seed.wrapping_add(1), max).saturating_add(u32::from(level));
            if grant_money_to_cursor(world, loader, character_id, amount) {
                JunkpileApplyResult::FoundMoney { amount }
            } else {
                JunkpileApplyResult::Nothing
            }
        }
        _ => JunkpileApplyResult::Nothing,
    };

    world.destroy_item(item_id);
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PickBerryApplyResult {
    Picked(String),
    NotRipe,
    CursorOccupied,
    MissingPlayer,
    Bug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lab2WaterApplyResult {
    Converted(usize),
    MissingPlayer,
    TemplateMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineSecurityApplyResult {
    Used { saves: u8 },
    SecureAlready,
    Hardcore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineJoblessApplyResult {
    Used,
    AlreadyJobless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineEdgeApplyResult {
    Used { exp: u32 },
    AlreadyOnEdge,
    NoExp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineKindnessApplyResult {
    Used,
    AlreadyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineBravenessApplyResult {
    Used { exp: u32, gold: u32 },
    Coward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineVitalityApplyResult {
    Used {
        value: CharacterValue,
        amount: i16,
        cost: u32,
    },
    NoExp,
    Capped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomShrineContinuityApplyResult {
    Used { exp: u32, opens_gate: bool },
    AlreadyVisited { opens_gate: bool },
    NeedYoungerBrother,
}

pub(crate) fn legacy_level_value(level: u32) -> u32 {
    let level = u64::from(level);
    let next = level.saturating_add(1);
    next.saturating_pow(4)
        .saturating_sub(level.saturating_pow(4))
        .min(u64::from(u32::MAX)) as u32
}

pub(crate) fn legacy_level_exp(level: u32) -> u32 {
    u64::from(level).saturating_pow(4).min(u64::from(u32::MAX)) as u32
}

pub(crate) fn legacy_save_number(saves: u8) -> String {
    match saves {
        0 => "no".to_string(),
        1 => "one".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        9 => "nine".to_string(),
        10 => "ten".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn apply_random_shrine_security(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
) -> RandomShrineSecurityApplyResult {
    if character.saves > 5 {
        return RandomShrineSecurityApplyResult::SecureAlready;
    }
    if character.flags.contains(CharacterFlags::HARDCORE) {
        return RandomShrineSecurityApplyResult::Hardcore;
    }

    character.saves = character.saves.saturating_add(1);
    player.mark_random_shrine_used(shrine_type);
    RandomShrineSecurityApplyResult::Used {
        saves: character.saves,
    }
}

pub(crate) fn apply_random_shrine_jobless(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
) -> RandomShrineJoblessApplyResult {
    if character
        .professions
        .iter()
        .all(|profession| *profession == 0)
    {
        return RandomShrineJoblessApplyResult::AlreadyJobless;
    }

    for profession in &mut character.professions {
        *profession = 0;
    }
    character
        .flags
        .insert(CharacterFlags::PROF | CharacterFlags::UPDATE);
    player.mark_random_shrine_used(shrine_type);
    RandomShrineJoblessApplyResult::Used
}

pub(crate) fn apply_random_shrine_edge(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
    shrine_level: u8,
) -> RandomShrineEdgeApplyResult {
    if character.saves == 0 {
        return RandomShrineEdgeApplyResult::AlreadyOnEdge;
    }
    if character.flags.contains(CharacterFlags::NOEXP) {
        return RandomShrineEdgeApplyResult::NoExp;
    }

    let level = character
        .level
        .saturating_add(5)
        .min(u32::from(shrine_level));
    let level_value = legacy_level_value(level);
    let exp = level_value.saturating_div(3).saturating_add(
        u32::from(character.saves)
            .saturating_mul(level_value)
            .saturating_div(30),
    );
    character.exp = character.exp.saturating_add(exp);
    character.saves = 0;
    character.flags.insert(CharacterFlags::UPDATE);
    player.mark_random_shrine_used(shrine_type);
    RandomShrineEdgeApplyResult::Used { exp }
}

pub(crate) fn apply_random_shrine_kindness(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
) -> RandomShrineKindnessApplyResult {
    if !character.flags.contains(CharacterFlags::PK) {
        return RandomShrineKindnessApplyResult::AlreadyKind;
    }

    character.flags.remove(CharacterFlags::PK);
    character.flags.insert(CharacterFlags::UPDATE);
    player.mark_random_shrine_used(shrine_type);
    RandomShrineKindnessApplyResult::Used
}

pub(crate) fn apply_random_shrine_braveness(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
    shrine_level: u8,
) -> RandomShrineBravenessApplyResult {
    if !player.has_used_random_shrine(51) {
        return RandomShrineBravenessApplyResult::Coward;
    }

    let level = character
        .level
        .saturating_add(5)
        .min(u32::from(shrine_level));
    let exp = legacy_level_value(level);
    let gold = exp / 10;
    character.exp = character.exp.saturating_add(exp);
    character.gold = character.gold.saturating_add(gold);
    character
        .flags
        .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
    player.mark_random_shrine_used(shrine_type);
    RandomShrineBravenessApplyResult::Used { exp, gold }
}

pub(crate) fn apply_random_shrine_vitality(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_type: u8,
) -> RandomShrineVitalityApplyResult {
    if character.flags.contains(CharacterFlags::NOEXP) {
        return RandomShrineVitalityApplyResult::NoExp;
    }

    if character.values.len() < 2 {
        character
            .values
            .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
    }
    for values in &mut character.values[..2] {
        if values.len() < CHARACTER_VALUE_NAMES.len() {
            values.resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
    }

    let value = if character.flags.contains(CharacterFlags::WARRIOR) {
        CharacterValue::Hp
    } else {
        CharacterValue::Mana
    };
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let cap = if seyan { 100 } else { 115 };
    let value_index = value as usize;
    let current = character.values[1][value_index];
    let amount = (cap - current).clamp(0, 5);
    if amount < 1 {
        return RandomShrineVitalityApplyResult::Capped;
    }

    let mut cost = 0_u32;
    for n in 0..amount {
        cost = cost.saturating_add(legacy_raise_cost(
            value_index,
            i32::from(current.saturating_add(n)),
            seyan,
        ));
    }

    character.values[1][value_index] = current.saturating_add(amount);
    character.values[0][value_index] = character.values[0][value_index].saturating_add(amount);
    character.exp_used = character.exp_used.saturating_add(cost);
    character.exp = character.exp.saturating_add(cost);
    character.flags.insert(CharacterFlags::UPDATE);
    player.mark_random_shrine_used(shrine_type);

    RandomShrineVitalityApplyResult::Used {
        value,
        amount,
        cost,
    }
}

pub(crate) fn apply_random_shrine_continuity(
    player: &mut PlayerRuntime,
    character: &mut Character,
    shrine_level: u8,
) -> RandomShrineContinuityApplyResult {
    if player.random_shrine_continuity < 10 {
        player.random_shrine_continuity = 10;
    }

    if shrine_level < player.random_shrine_continuity {
        return RandomShrineContinuityApplyResult::AlreadyVisited {
            opens_gate: shrine_level == 99,
        };
    }
    if shrine_level > player.random_shrine_continuity {
        return RandomShrineContinuityApplyResult::NeedYoungerBrother;
    }

    player.random_shrine_continuity = shrine_level.saturating_add(1);
    let level = character
        .level
        .saturating_add(5)
        .min(u32::from(shrine_level));
    let exp = legacy_level_value(level) / 6;
    character.exp = character.exp.saturating_add(exp);
    character.flags.insert(CharacterFlags::UPDATE);
    RandomShrineContinuityApplyResult::Used {
        exp,
        opens_gate: shrine_level == 99,
    }
}

pub(crate) fn pick_berry_template(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("lizard_brown_berry"),
        2 => Some("picked_flower_h"),
        3 => Some("picked_flower_i"),
        4 => Some("picked_flower_j"),
        _ => None,
    }
}

pub(crate) fn alchemy_flower_template(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("alc_flower1"),
        2 => Some("alc_flower2"),
        3 => Some("alc_flower3"),
        4 => Some("alc_flower4"),
        5 => Some("alc_flower5"),
        6 => Some("alc_flower6"),
        7 => Some("alc_flower7"),
        8 => Some("alc_mushroom1"),
        9 => Some("alc_mushroom2"),
        10 => Some("alc_mushroom3"),
        11 => Some("alc_mushroom4"),
        12 => Some("alc_mushroom5"),
        13 => Some("alc_mushroom6"),
        14 => Some("alc_mushroom7"),
        15 => Some("alc_mushroom8"),
        16 => Some("alc_mushroom9"),
        17 => Some("alc_berry1"),
        18 => Some("alc_berry2"),
        19 => Some("alc_berry3"),
        20 => Some("alc_berry4"),
        _ => None,
    }
}

pub(crate) fn pick_berry_ripe_seconds(character: &Character) -> u64 {
    match character.professions[profession::HERBALIST] {
        value if value >= 30 => 60 * 60 * 4,
        value if value >= 20 => 60 * 60 * 8,
        value if value >= 10 => 60 * 60 * 12,
        _ => 60 * 60 * 24,
    }
}

pub(crate) fn apply_pick_berry(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    kind: u8,
    location_id: u32,
    realtime_seconds: u64,
) -> PickBerryApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return PickBerryApplyResult::CursorOccupied;
    }

    let Some(template) = pick_berry_template(kind) else {
        return PickBerryApplyResult::Bug;
    };

    let ripe_seconds = pick_berry_ripe_seconds(character);
    let Some(player) = player else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if let Some(last_used) = player.flower_last_used_seconds(location_id) {
        if realtime_seconds.saturating_sub(last_used) < ripe_seconds {
            return PickBerryApplyResult::NotRipe;
        }
    }

    let Some(item_name) = grant_template_item_to_cursor(world, loader, character_id, template)
    else {
        return PickBerryApplyResult::CursorOccupied;
    };
    player.mark_flower_used(location_id, realtime_seconds);
    PickBerryApplyResult::Picked(item_name)
}

pub(crate) fn apply_pick_alchemy_flower(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    kind: u8,
    location_id: u32,
    realtime_seconds: u64,
) -> PickBerryApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return PickBerryApplyResult::CursorOccupied;
    }

    let Some(template) = alchemy_flower_template(kind) else {
        return PickBerryApplyResult::Bug;
    };

    let ripe_seconds = pick_berry_ripe_seconds(character);
    let Some(player) = player else {
        return PickBerryApplyResult::MissingPlayer;
    };
    if let Some(last_used) = player.flower_last_used_seconds(location_id) {
        if realtime_seconds.saturating_sub(last_used) < ripe_seconds {
            return PickBerryApplyResult::NotRipe;
        }
    }

    let Some(item_name) = grant_template_item_to_cursor(world, loader, character_id, template)
    else {
        return PickBerryApplyResult::CursorOccupied;
    };
    player.mark_flower_used(location_id, realtime_seconds);
    PickBerryApplyResult::Picked(item_name)
}

pub(crate) fn apply_flask_ingredient_added(
    world: &mut World,
    character_id: CharacterId,
    flask_id: ItemId,
    cursor_item_id: ItemId,
    ingredient_kind: u8,
) -> Option<String> {
    let ingredient_name = world.items.get(&cursor_item_id)?.name.clone();
    let flask = world.items.get_mut(&flask_id)?;
    let size = flask.driver_data.first().copied().unwrap_or_default();
    let used = flask.driver_data.get(1).copied().unwrap_or_default();

    flask.name = "Unfinished Potion".to_string();
    match size {
        1 => {
            flask.sprite = 50204 + i32::from(used);
            flask.description = "A small flask containing some strange liquid.".to_string();
        }
        2 => {
            flask.sprite = 50207 + i32::from(used);
            flask.description = "A flask containing some strange liquid.".to_string();
        }
        3 => {
            flask.sprite = 50243 + i32::from(used);
            flask.description = "A big flask containing some strange liquid.".to_string();
        }
        _ => {}
    }
    if flask.driver_data.len() <= usize::from(ingredient_kind) + 10 {
        flask
            .driver_data
            .resize(usize::from(ingredient_kind) + 11, 0);
    }
    flask.driver_data[1] = flask.driver_data[1].saturating_add(1);
    let ingredient_slot = usize::from(ingredient_kind) + 10;
    flask.driver_data[ingredient_slot] = flask.driver_data[ingredient_slot].saturating_add(1);

    world.items.remove(&cursor_item_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        if character.cursor_item == Some(cursor_item_id) {
            character.cursor_item = None;
        }
        character.flags.insert(CharacterFlags::ITEMS);
    }
    Some(ingredient_name)
}

pub(crate) const ALCHEMY_INGREDIENT_NAMES: [&str; 29] = [
    "Adygalah",
    "Bhalkissa",
    "Chrysado",
    "Domari",
    "Elithah",
    "Firuba",
    "Ghethiye",
    "Akond",
    "Barun",
    "Chylmoth",
    "Dizul",
    "Edyak",
    "Forud",
    "Ghestroz",
    "Hangot",
    "Ivnan",
    "Azmey",
    "Beelough",
    "Ciuba",
    "Dyelshi",
    "Fiery Stone",
    "Icy Stone",
    "Earth Stone",
    "Hell Stone",
    "",
    "",
    "",
    "",
    "",
];

pub(crate) fn flask_ingredient_feedback(ingredient_counts: [u8; 29]) -> Vec<String> {
    ALCHEMY_INGREDIENT_NAMES
        .iter()
        .enumerate()
        .filter_map(|(idx, name)| {
            let count = ingredient_counts[idx];
            if count == 0 || name.is_empty() {
                None
            } else {
                Some(format!("Contains {count} parts {name}."))
            }
        })
        .collect()
}

pub(crate) fn grant_template_item_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    template: &str,
) -> Option<String> {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }
    let item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    let item_id = item.id;
    let item_name = item.name.clone();
    let character = world.characters.get_mut(&character_id)?;
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn grant_salt_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    units: u32,
) -> bool {
    if units == 0
        || world
            .characters
            .get(&character_id)
            .is_none_or(|character| character.cursor_item.is_some())
    {
        return false;
    }
    let Ok(mut item) = loader.instantiate_item_template("salt", Some(character_id)) else {
        return false;
    };
    let item_id = item.id;
    item.value = item.value.saturating_mul(units);
    item.driver_data.resize(item.driver_data.len().max(4), 0);
    item.driver_data[0..4].copy_from_slice(&units.to_le_bytes());
    item.sprite = if units >= 10_000 {
        13212
    } else if units >= 1_000 {
        13211
    } else if units >= 100 {
        13210
    } else if units >= 10 {
        13209
    } else {
        13208
    };
    item.description = format!("{units} ounces of {}.", item.name);
    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    if character.cursor_item.is_some() {
        return false;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    true
}

pub(crate) fn apply_lab2_water_altar(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
) -> Lab2WaterApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return Lab2WaterApplyResult::MissingPlayer;
    };

    let mut water_bowls = Vec::new();
    if let Some(cursor_item_id) = character.cursor_item {
        if world.items.get(&cursor_item_id).is_some_and(|item| {
            item.driver == ugaris_core::item_driver::IDR_LAB2_WATER
                && item.driver_data.first().copied() == Some(4)
        }) {
            water_bowls.push(cursor_item_id);
        }
    }
    for item_id in character.inventory[INVENTORY_START_INVENTORY..]
        .iter()
        .flatten()
    {
        if world.items.get(item_id).is_some_and(|item| {
            item.driver == ugaris_core::item_driver::IDR_LAB2_WATER
                && item.driver_data.first().copied() == Some(4)
        }) {
            water_bowls.push(*item_id);
        }
    }

    let mut converted = 0;
    for old_item_id in water_bowls {
        let Some(mut new_item) = loader
            .instantiate_item_template("lab2_holywaterbowl", Some(character_id))
            .ok()
        else {
            return Lab2WaterApplyResult::TemplateMissing;
        };
        let new_item_id = new_item.id;
        let Some(character) = world.characters.get_mut(&character_id) else {
            return Lab2WaterApplyResult::MissingPlayer;
        };

        if character.cursor_item == Some(old_item_id) {
            character.cursor_item = Some(new_item_id);
        } else if let Some(slot) = character
            .inventory
            .iter_mut()
            .find(|slot| **slot == Some(old_item_id))
        {
            *slot = Some(new_item_id);
        } else {
            continue;
        }

        character.flags.insert(CharacterFlags::ITEMS);
        new_item.carried_by = Some(character_id);
        world.items.remove(&old_item_id);
        world.add_item(new_item);
        converted += 1;
    }

    Lab2WaterApplyResult::Converted(converted)
}

pub(crate) fn lab2_grave_clue_text(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    book: u8,
) -> String {
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return "Congratulations, you detected bug no. 12/HIHO/17. Please report this to the development team.".to_string();
    };
    player
        .legacy_lab2_grave_clue_text(book)
        .unwrap_or_else(|| "This grave is empty".to_string())
}

pub(crate) fn lab2_special_grave_template(kind: u8) -> Option<&'static str> {
    match kind {
        1 => Some("lab2_elias_hat"),
        2 => Some("lab2_elias_cape"),
        3 => Some("lab2_elias_boots"),
        4 => Some("lab2_elias_belt"),
        5 => Some("lab2_elias_amulet"),
        6 => Some("lab2_arathas_ring"),
        _ => None,
    }
}

pub(crate) fn lab2_grave_number(world: &World, item_id: ItemId) -> Option<usize> {
    let mut graves: Vec<_> = world
        .items
        .values()
        .filter(|item| item.driver == IDR_LAB2_GRAVE)
        .filter(|item| !matches!(item.driver_data.first().copied().unwrap_or_default(), 1..=4))
        .map(|item| (item.y, item.x, item.id.0, item.id))
        .collect();
    graves.sort_unstable_by_key(|(y, x, id, _)| (*y, *x, *id));
    graves
        .into_iter()
        .position(|(_, _, _, candidate_id)| candidate_id == item_id)
}

pub(crate) fn apply_lab2_grave_open(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    fixed_item: u8,
) -> bool {
    let Some(grave) = world.items.get(&item_id) else {
        return false;
    };
    let (grave_x, grave_y) = (grave.x, grave.y);

    let mut special_item = fixed_item;
    if special_item == 0 {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            special_item = player
                .legacy_lab2_special_grave_kind_at(grave_x, grave_y)
                .unwrap_or_default();
        }
    }

    if special_item == 0
        && lab2_grave_number(world, item_id).is_some_and(|grave_number| {
            runtime
                .player_for_character_mut(character_id)
                .is_some_and(|player| player.legacy_lab2_grave_cleared(grave_number))
        })
    {
        world.queue_system_text(character_id, "This grave is empty");
        return world.open_empty_lab2_grave(item_id);
    }

    let character_id_new = runtime.allocate_character_id();
    let opener_serial = world
        .characters
        .get(&character_id)
        .map(|character| character.serial)
        .unwrap_or_default();
    let spawn_template =
        if special_item == 5 || (special_item == 0 && runtime_random_below(100) > 66) {
            "lab2_skeleton"
        } else {
            "lab2_undead"
        };

    let Ok((mut undead, inventory_items)) =
        loader.instantiate_character_template(spawn_template, character_id_new)
    else {
        return false;
    };
    undead.dir = Direction::Down as u8;
    undead.flags.remove(CharacterFlags::RESPAWN);
    undead.driver = CDR_LAB2UNDEAD;
    if !matches!(
        undead.driver_state,
        Some(CharacterDriverState::Lab2Undead(_))
    ) {
        undead.driver_state = Some(CharacterDriverState::Lab2Undead(Default::default()));
    }
    if let Some(CharacterDriverState::Lab2Undead(data)) = undead.driver_state.as_mut() {
        data.grave_item_id = Some(item_id);
        data.opened_by_character_id = Some(character_id);
        data.opened_by_serial = opener_serial;
    }

    if special_item == 5 {
        undead.name = "Elias Skeleton".to_string();
        undead.values[1][CharacterValue::Hp as usize] = 5;
        undead.values[1][CharacterValue::Attack as usize] = 5;
        undead.values[1][CharacterValue::Parry as usize] = 5;
    } else if special_item == 6 {
        undead.name = "Undead Arathas".to_string();
    }

    undead.hp = i32::from(undead.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    undead.endurance = i32::from(undead.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    undead.mana = i32::from(undead.values[0][CharacterValue::Mana as usize]) * POWERSCALE;

    let mut created_special_item = None;
    if let Some(template) = lab2_special_grave_template(special_item) {
        if let Some(slot) = undead.inventory[INVENTORY_START_INVENTORY..]
            .iter()
            .position(Option::is_none)
            .map(|index| index + INVENTORY_START_INVENTORY)
        {
            if let Ok(mut item) = loader.instantiate_item_template(template, Some(character_id_new))
            {
                let item_id = item.id;
                item.carried_by = Some(character_id_new);
                undead.inventory[slot] = Some(item_id);
                world.add_item(item);
                created_special_item = Some(item_id);
            }
        }
    }

    if !world.spawn_character(undead, usize::from(grave_x), usize::from(grave_y)) {
        if let Some(item_id) = created_special_item {
            world.items.remove(&item_id);
        }
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    let serial = world
        .characters
        .get(&character_id_new)
        .map(|character| character.serial)
        .unwrap_or_default();
    world.open_lab2_grave(item_id, character_id_new, serial)
}

pub(crate) fn grant_ice_itemspawn_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    template: &str,
) -> IceItemSpawnGrantResult {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return IceItemSpawnGrantResult::Bug;
    }
    let item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok();
    let Some(item) = item else {
        return IceItemSpawnGrantResult::Bug;
    };
    let item_id = item.id;
    let item_name = item.name.clone();
    let should_schedule_melting = item.driver == IDR_MELTINGKEY;
    let Some(character) = world.characters.get(&character_id) else {
        return IceItemSpawnGrantResult::Bug;
    };
    if character.cursor_item.is_some() {
        return IceItemSpawnGrantResult::Bug;
    }
    if is_one_carry_driver(item.driver)
        && character
            .inventory
            .iter()
            .filter_map(|slot| slot.and_then(|id| world.items.get(&id)))
            .chain(
                character
                    .cursor_item
                    .and_then(|id| world.items.get(&id))
                    .into_iter(),
            )
            .any(|carried| carried.driver == item.driver)
    {
        return IceItemSpawnGrantResult::OneCarry { item_name };
    }
    if item.flags.contains(ItemFlags::BONDTAKE) && item.owner_id != character.id.0 as i32 {
        return IceItemSpawnGrantResult::CannotCarry;
    }
    let Some(character) = world.characters.get_mut(&character_id) else {
        return IceItemSpawnGrantResult::Bug;
    };
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    if should_schedule_melting {
        world.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
    }
    IceItemSpawnGrantResult::Granted { item_name }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum IceItemSpawnGrantResult {
    Granted { item_name: String },
    OneCarry { item_name: String },
    CannotCarry,
    Bug,
}

pub(crate) fn grant_warmfire_scroll_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
) -> Option<String> {
    let (x, y) = world
        .characters
        .get(&character_id)
        .map(|character| (character.x as u8, character.y as u8))?;
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return None;
    }
    let mut item = loader
        .instantiate_item_template("ice_scroll", Some(character_id))
        .ok()?;
    if item.driver_data.len() < 2 {
        item.driver_data.resize(2, 0);
    }
    item.driver_data[0] = x;
    item.driver_data[1] = y;
    let item_id = item.id;
    let item_name = item.name.clone();
    let character = world.characters.get_mut(&character_id)?;
    if character.cursor_item.is_some() {
        return None;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn grant_template_item_smart(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    template: &str,
) -> Option<String> {
    let mut item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    let item_name = item.name.clone();
    let (give_result, drop_x, drop_y) = {
        let character = world.characters.get_mut(&character_id)?;
        let result = give_item_to_character(character, &mut item, GiveItemFlags::ALLOW_DROP);
        (result, usize::from(character.x), usize::from(character.y))
    };
    match give_result {
        GiveItemResult::Ok => {}
        GiveItemResult::Dropped => {
            if !world.map.drop_item_extended(&mut item, drop_x, drop_y, 1) {
                return None;
            }
        }
        GiveItemResult::Money => {}
        GiveItemResult::Full | GiveItemResult::Failed => return None,
    }
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn raise_skeleton_from_template(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_item_id: ItemId,
    template: &str,
) -> bool {
    let Some((x, y)) = world
        .items
        .get(&item_id)
        .map(|item| (usize::from(item.x), usize::from(item.y)))
    else {
        return false;
    };
    let raised_id = runtime.allocate_character_id();
    let Ok((raised, inventory_items)) = loader.instantiate_character_template(template, raised_id)
    else {
        return false;
    };
    let raised_serial = raised.serial;
    if !world.spawn_character(raised, x, y) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_skelraise_raise(
        item_id,
        character_id,
        cursor_item_id,
        raised_id,
        raised_serial,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ZombieShrineApplyResult {
    NeedsOffering(u8),
    Gift(String),
    Experience(u32),
    Bonus {
        message: &'static str,
        driver: u16,
        strength: i32,
        duration_ticks: i32,
    },
    MissingGift,
    MissingPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArkhataPoolApplyResult {
    Gift(String),
    Vanished,
    MissingGift,
    MissingPlayer,
    MissingCursor,
}

pub(crate) fn zombie_shrine_required_skull(shrine_type: u8) -> u32 {
    match shrine_type {
        0 => IID_AREA2_ZOMBIESKULL1,
        1 => IID_AREA2_ZOMBIESKULL2,
        _ => IID_AREA2_ZOMBIESKULL3,
    }
}

pub(crate) fn zombie_shrine_offering_message(shrine_type: u8) -> &'static str {
    match shrine_type {
        0 => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone.",
        1 => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone and silver.",
        _ => "You sense that this ancient shrine used to receive gifts. Strange gifts. You feel a craving for bone and gold.",
    }
}

pub(crate) fn zombie_shrine_reward_template(
    shrine_type: u8,
    roll: u32,
    flags: CharacterFlags,
) -> Option<&'static str> {
    match shrine_type {
        0 => match roll {
            0 | 1 | 20 | 21 => Some("zombie_skull2"),
            2..=9 => Some("torch"),
            10 | 11 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion1"
            } else {
                "healing_potion1"
            }),
            12 | 13 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion1"
            } else {
                "mana_potion1"
            }),
            _ => None,
        },
        1 => match roll {
            0 | 1 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion2"
            } else {
                "healing_potion2"
            }),
            2 | 11 | 12 => Some("zombie_skull3"),
            3 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion2"
            } else {
                "mana_potion2"
            }),
            _ => None,
        },
        _ => match roll {
            0 | 1 => Some(if flags.contains(CharacterFlags::MAGE) {
                "mana_potion3"
            } else {
                "healing_potion3"
            }),
            2 | 3 => Some(if flags.contains(CharacterFlags::WARRIOR) {
                "healing_potion3"
            } else {
                "mana_potion3"
            }),
            _ => None,
        },
    }
}

pub(crate) fn zombie_shrine_experience(shrine_type: u8, roll: u32) -> Option<u32> {
    match shrine_type {
        0 if roll == 14 || roll == 15 => Some(250),
        1 if (4..=6).contains(&roll) => Some(750),
        2..=u8::MAX if (4..=6).contains(&roll) => Some(2250),
        _ => None,
    }
}

pub(crate) fn zombie_shrine_bonus(
    shrine_type: u8,
    roll: u32,
    flags: CharacterFlags,
) -> Option<(&'static str, u16, i32, i32)> {
    match shrine_type {
        0 => match roll {
            16 => Some((
                "You have been protected for a short while.",
                IDR_ARMOR,
                5 * 20,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            17 => Some((
                "You are more dangerous for a short while.",
                IDR_WEAPON,
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            18 => Some((
                "Your capacity was increased for a short while.",
                if flags.contains(CharacterFlags::WARRIOR) {
                    IDR_HP
                } else {
                    IDR_MANA
                },
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            19 => Some((
                "Your capacity was increased for a short while.",
                if flags.contains(CharacterFlags::MAGE) {
                    IDR_MANA
                } else {
                    IDR_HP
                },
                5,
                TICKS_PER_SECOND as i32 * 60 * 5,
            )),
            _ => None,
        },
        1 => match roll {
            7 => Some((
                "You have been protected for a while.",
                IDR_ARMOR,
                10 * 20,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            8 => Some((
                "You are more dangerous for a while.",
                IDR_WEAPON,
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            9 => Some((
                "Your capacity was increased for a while.",
                if flags.contains(CharacterFlags::WARRIOR) {
                    IDR_HP
                } else {
                    IDR_MANA
                },
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            10 => Some((
                "Your capacity was increased for a while.",
                if flags.contains(CharacterFlags::MAGE) {
                    IDR_MANA
                } else {
                    IDR_HP
                },
                10,
                TICKS_PER_SECOND as i32 * 60 * 15,
            )),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn apply_zombie_shrine(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    shrine_type: u8,
    random_seed: u64,
) -> ZombieShrineApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return ZombieShrineApplyResult::MissingPlayer;
    };
    let Some(cursor_item_id) = character.cursor_item else {
        return ZombieShrineApplyResult::NeedsOffering(shrine_type);
    };
    if world
        .items
        .get(&cursor_item_id)
        .is_none_or(|item| item.template_id != zombie_shrine_required_skull(shrine_type))
    {
        return ZombieShrineApplyResult::NeedsOffering(shrine_type);
    }
    let character_flags = character.flags;

    let Some(character) = world.characters.get_mut(&character_id) else {
        return ZombieShrineApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    world.items.remove(&cursor_item_id);

    let roll_max = match shrine_type {
        0 => 22,
        1 => 13,
        _ => 7,
    };
    let roll = legacy_random(random_seed, roll_max);
    if let Some(template) = zombie_shrine_reward_template(shrine_type, roll, character_flags) {
        return match grant_template_item_to_cursor(world, loader, character_id, template) {
            Some(item_name) => ZombieShrineApplyResult::Gift(item_name),
            None => ZombieShrineApplyResult::MissingGift,
        };
    }
    if let Some(exp_added) = zombie_shrine_experience(shrine_type, roll) {
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.exp = character.exp.saturating_add(exp_added);
        }
        return ZombieShrineApplyResult::Experience(exp_added);
    }
    if let Some((message, driver, strength, duration_ticks)) =
        zombie_shrine_bonus(shrine_type, roll, character_flags)
    {
        world.install_bonus_spell(character_id, driver, strength, duration_ticks);
        return ZombieShrineApplyResult::Bonus {
            message,
            driver,
            strength,
            duration_ticks,
        };
    }

    ZombieShrineApplyResult::MissingGift
}

pub(crate) fn apply_arkhata_pool(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    cursor_item_id: ItemId,
    random_seed: u64,
) -> ArkhataPoolApplyResult {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return ArkhataPoolApplyResult::MissingPlayer;
    };
    if character.cursor_item != Some(cursor_item_id) {
        return ArkhataPoolApplyResult::MissingCursor;
    }
    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    world.items.remove(&cursor_item_id);

    let template = match legacy_random(random_seed, 70) {
        22 | 33 => Some("Red_Scroll"),
        42 => Some("Buddah_Statue"),
        _ => None,
    };
    let Some(template) = template else {
        return ArkhataPoolApplyResult::Vanished;
    };
    match grant_template_item_smart(world, loader, character_id, template) {
        Some(item_name) => ArkhataPoolApplyResult::Gift(item_name),
        None => ArkhataPoolApplyResult::MissingGift,
    }
}

pub(crate) fn apply_assemble_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_item_id: ItemId,
    template: &str,
) -> AssembleApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    if character.cursor_item != Some(cursor_item_id) {
        return AssembleApplyResult::MissingItem;
    }
    let Some(slot) = character
        .inventory
        .iter()
        .position(|slot_item| *slot_item == Some(item_id))
    else {
        return AssembleApplyResult::MissingItem;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
        || !world.items.contains_key(&cursor_item_id)
    {
        return AssembleApplyResult::MissingItem;
    }

    let Ok(new_item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    let new_item_id = new_item.id;

    world.items.remove(&cursor_item_id);
    world.items.remove(&item_id);
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = None;
    character.inventory[slot] = Some(new_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(new_item);
    AssembleApplyResult::Assembled
}

pub(crate) fn apply_caligar_key_final(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_item_id: ItemId,
) -> AssembleApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    if character.cursor_item != Some(cursor_item_id) {
        return AssembleApplyResult::MissingItem;
    }
    let Some(slot) = character
        .inventory
        .iter()
        .position(|slot_item| *slot_item == Some(item_id))
    else {
        return AssembleApplyResult::MissingItem;
    };
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
        || !world.items.contains_key(&cursor_item_id)
    {
        return AssembleApplyResult::MissingItem;
    }

    let Ok(new_item) =
        loader.instantiate_item_template("caligar_palace_chest_key", Some(character_id))
    else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    let new_item_id = new_item.id;

    world.items.remove(&cursor_item_id);
    world.items.remove(&item_id);
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = Some(new_item_id);
    character.inventory[slot] = None;
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(new_item);
    AssembleApplyResult::Assembled
}

pub(crate) fn apply_palace_key_split(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    cursor_part_sprite: i32,
    carried_part_sprite: i32,
) -> AssembleApplyResult {
    let Some(character) = world.characters.get(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return AssembleApplyResult::MissingItem;
    }
    if world
        .items
        .get(&item_id)
        .is_none_or(|item| item.carried_by != Some(character_id))
    {
        return AssembleApplyResult::MissingItem;
    }

    let Ok(mut cursor_item) =
        loader.instantiate_item_template("palace_key_part1", Some(character_id))
    else {
        return AssembleApplyResult::TemplateUnavailable;
    };
    cursor_item.sprite = cursor_part_sprite;
    let cursor_item_id = cursor_item.id;

    let Some(item) = world.items.get_mut(&item_id) else {
        return AssembleApplyResult::MissingItem;
    };
    item.sprite = carried_part_sprite;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AssembleApplyResult::MissingPlayer;
    };
    character.cursor_item = Some(cursor_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(cursor_item);
    AssembleApplyResult::Assembled
}
