use super::*;

/// C `give_exp(cn, val)` (`src/system/tool.c:1371-1423`). Thin wrapper
/// around the canonical `World::give_exp` (`ugaris-core/src/world/exp.rs`),
/// which now owns the full algorithm (multipliers read from
/// `world.settings.exp_modifier`/`hardcore_exp_bonus`, the `CF_NOLEVEL`
/// exp-band clamp, and the `check_levelup` tail call) so it is usable both
/// from server-crate call sites and from `ugaris-core` item drivers, which
/// only ever have `&mut World` available. Kept as a named wrapper (rather
/// than inlining `world.give_exp` at every call site) purely so call sites
/// read the same as their C `give_exp(cn, val)` counterparts.
pub(crate) fn give_exp_with_runtime_modifiers(
    world: &mut World,
    character_id: CharacterId,
    base_exp: i64,
    area_id: u32,
) {
    world.give_exp(character_id, base_exp, area_id);
}

/// C `cmd_milinfo`/`cmd_milpref`/`cmd_milsolve`'s local `diff_names[5]`
/// table (`command.c:5077,5175,5504`), letter for letter.
const MILITARY_DIFFICULTY_NAMES: [&str; 5] = ["easy", "normal", "hard", "impossible", "insane"];

/// C `mis[].type`'s 1/2/3 -> name mapping for an *active mission slot*
/// (`cmd_milinfo`/`cmd_milsolve`, `command.c:5113-5126,5554-5566`) - unlike
/// the preference-display mapping below, an out-of-range type here means
/// "Unknown" (defensive default for corrupt/impossible slot data), not
/// "no preference set".
fn military_mission_slot_type_name(mission_type: i32) -> &'static str {
    match mission_type {
        1 => "Demon",
        2 => "Ratling",
        3 => "Silver",
        _ => "Unknown",
    }
}

/// C `mission_type_preference`'s 1/2/3 -> name mapping, used by
/// `cmd_milinfo`/`cmd_milpref` when displaying the *preference* (0 = "None"
/// really does mean "no preference set", `command.c:5142-5146,5229-5232`).
fn military_type_preference_name(type_preference: i32) -> &'static str {
    match type_preference {
        1 => "Demon",
        2 => "Ratling",
        3 => "Silver",
        _ => "None",
    }
}

pub(crate) fn legacy_lookup_skill(input: &str) -> Option<i16> {
    let token = input
        .chars()
        .take(19)
        .collect::<String>()
        .to_ascii_lowercase();
    let value = match token.as_str() {
        "endurance" => CharacterValue::Endurance,
        "hp" | "health" | "hitpoints" => CharacterValue::Hp,
        "mana" => CharacterValue::Mana,
        "wis" | "wisdom" => CharacterValue::Wisdom,
        "int" | "intuition" => CharacterValue::Intelligence,
        "agi" | "agility" => CharacterValue::Agility,
        "str" | "strength" => CharacterValue::Strength,
        "bart" | "bartering" => CharacterValue::Barter,
        "perc" | "perception" => CharacterValue::Percept,
        "stealth" => CharacterValue::Stealth,
        "hand" | "handtohand" | "hand-to-hand" | "hand2hand" => CharacterValue::Hand,
        "wc" | "warcry" => CharacterValue::Warcry,
        "sh" | "surround" | "surroundhit" => CharacterValue::Surround,
        "bc" | "bodycontrol" | "body-control" => CharacterValue::BodyControl,
        "ss" | "speedskill" | "speed" => CharacterValue::SpeedSkill,
        "heal" => CharacterValue::Heal,
        "fire" | "fireball" => CharacterValue::Fireball,
        "tactics" | "tac" | "tact" => CharacterValue::Tactics,
        "duration" | "dur" => CharacterValue::Duration,
        "rage" => CharacterValue::Rage,
        "bless" => CharacterValue::Bless,
        "freeze" | "frz" | "fre" => CharacterValue::Freeze,
        "ms" | "magicshield" => CharacterValue::MagicShield,
        "lf" | "lightning" | "flash" => CharacterValue::Flash,
        "pulse" | "pul" => CharacterValue::Pulse,
        "dagger" | "dag" => CharacterValue::Dagger,
        "staff" | "sta" => CharacterValue::Staff,
        "sword" | "sw" => CharacterValue::Sword,
        "twohand" | "twohanded" | "two-handed" | "two-hand" | "2hand" | "2h" | "th" => {
            CharacterValue::TwoHand
        }
        "attack" | "att" => CharacterValue::Attack,
        "parry" | "par" => CharacterValue::Parry,
        "immunity" | "imm" | "immy" => CharacterValue::Immunity,
        _ => return None,
    };
    Some(value as i16)
}

pub(crate) fn parse_itemmod_args(rest: &str) -> (i64, i64, i64) {
    let mut ptr = rest.trim_start();
    let pos = legacy_atoi_prefix(ptr);
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    ptr = ptr.trim_start();
    let token = ptr
        .split_once(char::is_whitespace)
        .map(|(token, _)| token)
        .unwrap_or(ptr);
    let nr = legacy_lookup_skill(token)
        .map(i64::from)
        .unwrap_or_else(|| legacy_atoi_prefix(ptr));
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_alphanumeric());
    ptr = ptr.trim_start();
    let val = legacy_atoi_prefix(ptr);
    (pos, nr, val)
}

pub(crate) fn parse_setskill_args(rest: &str) -> (String, i64, i64) {
    let mut chars = rest.trim_start().char_indices();
    let mut name_end = 0;
    for (idx, ch) in &mut chars {
        if !ch.is_ascii_alphabetic() {
            break;
        }
        name_end = idx + ch.len_utf8();
    }
    let name = rest.trim_start()[..name_end.min(79)].to_string();
    let mut ptr = &rest.trim_start()[name_end..];
    ptr = ptr.trim_start();
    let token = ptr
        .split_once(char::is_whitespace)
        .map(|(token, _)| token)
        .unwrap_or(ptr);
    let pos = legacy_lookup_skill(token)
        .map(i64::from)
        .unwrap_or_else(|| legacy_atoi_prefix(ptr));
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_alphanumeric());
    ptr = ptr.trim_start();
    let val = legacy_atoi_prefix(ptr);
    (name, pos, val)
}

pub(crate) fn parse_exp_command_target(
    world: &World,
    character_id: CharacterId,
    rest: &str,
) -> (CharacterId, String, i64) {
    let mut text = rest.trim_start();
    if text.is_empty() || text.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        let name = world
            .characters
            .get(&character_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        return (character_id, name, legacy_atoi_prefix(text));
    }

    let mut split = text.splitn(2, char::is_whitespace);
    let name = split.next().unwrap_or_default();
    text = split.next().unwrap_or_default();
    let target_id = find_online_character_by_name(world, name).unwrap_or(CharacterId(0));
    (target_id, name.to_string(), legacy_atoi_prefix(text))
}

pub(crate) fn legacy_skill_start(value: usize) -> i32 {
    match value {
        0..=6 => 10,
        42 => -1,
        11..=41 => 1,
        _ => -1,
    }
}

pub(crate) fn legacy_skill_cost_factor(value: usize) -> i32 {
    match value {
        0..=2 | 42 => 3,
        3..=6 => 2,
        11..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

pub(crate) fn legacy_skillmax(character: &Character) -> i32 {
    if !character.flags.contains(CharacterFlags::ARCH) {
        return 50;
    }
    if character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE)
    {
        110
    } else {
        125
    }
}

pub(crate) fn legacy_raise_cost(value: usize, current: i32, seyan: bool) -> u32 {
    let nr = current - legacy_skill_start(value) + 1 + 5;
    let cost = nr * nr * nr * legacy_skill_cost_factor(value);
    let cost = if seyan { cost * 4 / 30 } else { cost / 10 };
    cost.max(1) as u32
}

pub(crate) fn legacy_supermax_canraise(value: usize) -> i32 {
    match value {
        3..=6 => 2,
        11 | 12..=24 | 25..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

pub(crate) fn legacy_supermax_cost(character: &Character, value: usize, current: i32) -> u32 {
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    (legacy_supermax_canraise(value) * 3_000_000) as u32 + legacy_raise_cost(value, current, seyan)
}

pub(crate) fn legacy_calc_exp_used(character: &Character) -> u32 {
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let Some(bare_values) = character.values.get(1) else {
        return 0;
    };
    let mut exp = 0_u32;
    for value in 0..CHARACTER_VALUE_NAMES.len() {
        let bare = i32::from(*bare_values.get(value).unwrap_or(&0));
        if bare == 0 || legacy_skill_cost_factor(value) == 0 {
            continue;
        }
        for n in (legacy_skill_start(value) + 1)..=bare {
            let current = n - 1;
            let cost = if character.flags.contains(CharacterFlags::PLAYER)
                && current >= legacy_skillmax(character)
            {
                legacy_supermax_cost(character, value, current)
            } else {
                legacy_raise_cost(value, current, seyan)
            };
            exp = exp.saturating_add(cost);
        }
    }
    exp
}

pub(crate) fn apply_gold_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("ggold") {
        let Some(character) = world.characters.get_mut(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let amount = legacy_atoi_prefix(rest).saturating_mul(100);
        if amount >= 0 {
            let amount = u32::try_from(amount).unwrap_or(u32::MAX);
            character.gold = character.gold.saturating_add(amount);
        } else {
            let amount = u32::try_from(amount.unsigned_abs()).unwrap_or(u32::MAX);
            character.gold = character.gold.saturating_sub(amount);
        }
        character.flags.insert(CharacterFlags::ITEMS);
        return Some(KeyringCommandResult {
            inventory_changed: true,
            ..Default::default()
        });
    }
    if !verb.eq_ignore_ascii_case("gold") {
        return None;
    }

    let Some(amount) = legacy_atoi_prefix(rest).checked_mul(100) else {
        return Some(KeyringCommandResult {
            messages: vec!["Hu?".to_string()],
            ..Default::default()
        });
    };
    if amount < 1 {
        return Some(KeyringCommandResult {
            messages: vec!["Hu?".to_string()],
            ..Default::default()
        });
    }
    let amount = amount as u64;

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if amount > u64::from(character.gold) {
        return Some(KeyringCommandResult {
            messages: vec!["You do not have that much gold.".to_string()],
            ..Default::default()
        });
    }
    if character.cursor_item.is_some() {
        return Some(KeyringCommandResult {
            messages: vec!["Please free your hand (mouse cursor) first.".to_string()],
            ..Default::default()
        });
    }

    let amount = amount as u32;
    if !grant_money_to_cursor(world, loader, character_id, amount) {
        return Some(KeyringCommandResult {
            messages: vec!["Please free your hand (mouse cursor) first.".to_string()],
            ..Default::default()
        });
    }
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.gold = character.gold.saturating_sub(amount);
    }
    Some(KeyringCommandResult {
        inventory_changed: true,
        ..Default::default()
    })
}

pub(crate) fn apply_create_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"create".starts_with(&lower) {
        return None;
    }

    let character = world.characters.get(&character_id)?;
    if !character.flags.contains(CharacterFlags::GOD) {
        return None;
    }
    if character.cursor_item.is_some() {
        return Some(KeyringCommandResult {
            messages: vec!["Please empty your mouse cursor first.".to_string()],
            ..Default::default()
        });
    }

    let template = rest.trim_start();
    let Ok(mut item) = loader.instantiate_item_template(template, Some(character_id)) else {
        return Some(KeyringCommandResult {
            messages: vec!["No such template exists.".to_string()],
            ..Default::default()
        });
    };
    let item_id = item.id;
    item.carried_by = Some(character_id);
    world.add_item(item);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = Some(item_id);
        character.flags.insert(CharacterFlags::ITEMS);
    }

    Some(KeyringCommandResult {
        inventory_changed: true,
        ..Default::default()
    })
}

pub(crate) fn apply_create_orb_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("create_orb") {
        return None;
    }

    let character = world.characters.get(&character_id)?;
    if !character.flags.contains(CharacterFlags::GOD) {
        return None;
    }

    let rest = rest.trim_start();
    let (modifier, value) = if rest.is_empty() {
        (
            legacy_orb_value_from_seed(world.tick.0 + u64::from(character_id.0)) as i16,
            1,
        )
    } else if let Some(skill) = legacy_lookup_skill(rest) {
        (skill, 1)
    } else {
        let value = legacy_atoi_prefix(rest);
        let Some(skill) = (if value > 0 {
            let skill_text = rest
                .trim_start_matches(|ch: char| ch.is_ascii_digit())
                .trim_start();
            legacy_lookup_skill(skill_text)
        } else {
            None
        }) else {
            return Some(KeyringCommandResult::default());
        };
        (skill, value.clamp(1, 255) as u8)
    };

    let inventory_changed =
        grant_created_orb(world, loader, character_id, modifier, value).is_some();
    Some(KeyringCommandResult {
        inventory_changed,
        ..Default::default()
    })
}

pub(crate) fn grant_created_orb(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    modifier: i16,
    value: u8,
) -> Option<ItemId> {
    let value_name = CHARACTER_VALUE_NAMES.get(usize::try_from(modifier).ok()?)?;
    let Ok(mut item) = loader.instantiate_item_template("empty_orb", Some(character_id)) else {
        return None;
    };
    item.name = if value == 1 {
        format!("Orb of {value_name}")
    } else {
        format!("Orb of {value} {value_name}")
    };
    ensure_drdata_len(&mut item, 2);
    item.driver_data[0] = u8::try_from(modifier).ok()?;
    item.driver_data[1] = value;
    let item_id = item.id;
    let character = world.characters.get_mut(&character_id)?;
    match give_item_to_character(character, &mut item, GiveItemFlags::NONE) {
        GiveItemResult::Ok => {
            world.add_item(item);
            Some(item_id)
        }
        GiveItemResult::Money
        | GiveItemResult::Dropped
        | GiveItemResult::Full
        | GiveItemResult::Failed => None,
    }
}

pub(crate) fn apply_status_command(
    character: &Character,
    player: &PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.is_empty() || !"status".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![
        "Lag Control Settings:".to_string(),
        format!("Max. Lag [/MAXLAG]: {} sec.", player.max_lag_seconds),
    ];

    let has_spell = |value: CharacterValue| character.values[1][value as usize] > 0;
    if has_spell(CharacterValue::Flash) {
        messages.push("Don't use Ball Lightning [/NOBALL]: Off.".to_string());
    }
    if has_spell(CharacterValue::Bless) {
        messages.push("Don't use Bless [/NOBLESS]: Off.".to_string());
    }
    if has_spell(CharacterValue::Fireball) {
        messages.push("Don't use Fireball [/NOFIREBALL]: Off.".to_string());
    }
    if has_spell(CharacterValue::Flash) {
        messages.push("Don't use Lightning Flash [/NOFLASH]: Off.".to_string());
    }
    if has_spell(CharacterValue::Freeze) {
        messages.push("Don't use Freeze [/NOFREEZE]: Off.".to_string());
    }
    if has_spell(CharacterValue::Heal) {
        messages.push("Don't use Heal [/NOHEAL]: Off.".to_string());
    }
    if has_spell(CharacterValue::MagicShield) {
        messages.push("Don't use Magic Shield [/NOSHIELD]: Off.".to_string());
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push("Don't use Pulse [/NOPULSE]: Off.".to_string());
    }
    if has_spell(CharacterValue::Warcry) {
        messages.push("Don't use Warcry [/NOWARCRY]: Off.".to_string());
    }

    messages.extend([
        "Don't use Healing Potions [/NOLIFE]: Off.".to_string(),
        "Don't use Mana Potions [/NOMANA]: Off.".to_string(),
        "Don't use Combo Potions [/NOCOMBO]: Off.".to_string(),
        "Don't use Recall Scroll [/NORECALL]: Off.".to_string(),
        "Don't Move [/NOMOVE]: Off.".to_string(),
        "Automation Settings:".to_string(),
    ]);
    if has_spell(CharacterValue::Bless) {
        messages.push("Automatic Re-Bless [/AUTOBLESS]: Off.".to_string());
    }
    if has_spell(CharacterValue::Pulse) {
        messages.push("Automatic Pulse [/AUTOPULSE]: Off.".to_string());
    }
    messages.extend([
        format!(
            "Automatic Turning [/AUTOTURN]: {}.",
            if player.autoturn_enabled { "On" } else { "Off" }
        ),
        "Protection Settings:".to_string(),
        format!(
            "Allow others to bless me [/ALLOWBLESS]: {}.",
            if character.flags.contains(CharacterFlags::NOBLESS) {
                "No"
            } else {
                "Yes"
            }
        ),
        "Account Status:".to_string(),
        if character.flags.contains(CharacterFlags::PAID) {
            "Paid Account".to_string()
        } else {
            "Trial Account".to_string()
        },
    ]);

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

pub(crate) fn apply_legacy_tick_tuning_command(
    runtime: &mut ServerRuntime,
    lower: &str,
    rest: &str,
) -> Option<KeyringCommandResult> {
    struct TickTuningSpec {
        command: &'static str,
        min_len: usize,
        min: i32,
        max: i32,
        field: fn(&mut ServerRuntime) -> &mut i32,
        success: &'static str,
        invalid: &'static str,
    }

    let ticks = TICKS_PER_SECOND as i32;
    let specs = [
        TickTuningSpec {
            command: "setdecaytime",
            min_len: 12,
            min: 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.item_decay_time,
            success: "Item decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (1-60 minutes)",
        },
        TickTuningSpec {
            command: "setplayerbodytime",
            min_len: 17,
            min: 5 * 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.player_body_decay_time,
            success: "Player body decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (5-60 minutes)",
        },
        TickTuningSpec {
            command: "setnpcbodytime",
            min_len: 14,
            min: 30 * ticks,
            max: 30 * 60 * ticks,
            field: |runtime| &mut runtime.npc_body_decay_time,
            success: "NPC body decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (0.5-30 minutes)",
        },
        TickTuningSpec {
            command: "setnpcbodytimearea32",
            min_len: 20,
            min: 5 * 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.npc_body_decay_time_area32,
            success: "NPC body decay time for area 32 changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (5-60 minutes)",
        },
        TickTuningSpec {
            command: "setrespawntime",
            min_len: 14,
            min: 30 * ticks,
            max: 10 * 60 * ticks,
            field: |runtime| &mut runtime.npc_respawn_timer,
            success: "NPC respawn time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (0.5-10 minutes)",
        },
        TickTuningSpec {
            command: "setsewerrespawntime",
            min_len: 19,
            min: 60 * 60,
            max: 60 * 60 * 24 * 7,
            field: |runtime| &mut runtime.sewer_item_respawn_time,
            success: "Sewer item respawn time changed from {old} to {new} seconds ({old_hours} to {new_hours} hours)",
            invalid: "Invalid value. Please specify a time between 3600 and 604800 seconds (1 hour to 7 days)",
        },
        TickTuningSpec {
            command: "setlagouttime",
            min_len: 13,
            min: 60 * ticks,
            max: 30 * 60 * ticks,
            field: |runtime| &mut runtime.lagout_time,
            success: "Lagout time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (1-30 minutes)",
        },
        TickTuningSpec {
            command: "setregentime",
            min_len: 12,
            min: 1,
            max: 24,
            field: |runtime| &mut runtime.regen_time,
            success: "Regeneration time changed from {old} to {new} ticks",
            invalid: "Invalid value. Please specify a time between 1 and 24 ticks",
        },
    ];

    for spec in specs {
        if lower.len() < spec.min_len || !spec.command.starts_with(lower) {
            continue;
        }

        let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if (spec.min..=spec.max).contains(&value) {
            let field = (spec.field)(runtime);
            let old = *field;
            *field = value;
            return Some(KeyringCommandResult {
                messages: vec![spec
                    .success
                    .replace("{old}", &old.to_string())
                    .replace("{new}", &value.to_string())
                    .replace("{old_minutes}", &(old / (60 * ticks)).to_string())
                    .replace("{new_minutes}", &(value / (60 * ticks)).to_string())
                    .replace("{old_hours}", &(old / (60 * 60)).to_string())
                    .replace("{new_hours}", &(value / (60 * 60)).to_string())],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![spec
                .invalid
                .replace("{min}", &spec.min.to_string())
                .replace("{max}", &spec.max.to_string())],
            ..Default::default()
        });
    }

    None
}

pub(crate) fn apply_legacy_communication_tuning_command(
    runtime: &mut ServerRuntime,
    lower: &str,
    rest: &str,
) -> Option<KeyringCommandResult> {
    struct CommunicationTuningSpec {
        command: &'static str,
        min_len: usize,
        min: i32,
        max: i32,
        field: fn(&mut ServerRuntime) -> &mut i32,
        success: &'static str,
        invalid: &'static str,
        display_divisor: i32,
    }

    let say_dist = SAY_DIST as i32;
    let specs = [
        CommunicationTuningSpec {
            command: "sethollerdist",
            min_len: 13,
            min: say_dist,
            max: say_dist * 5,
            field: |runtime| &mut runtime.holler_dist,
            success: "Holler distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setshoutdist",
            min_len: 12,
            min: say_dist,
            max: say_dist * 4,
            field: |runtime| &mut runtime.shout_dist,
            success: "Shout distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setsaydist",
            min_len: 10,
            min: say_dist / 2,
            max: say_dist * 2,
            field: |runtime| &mut runtime.say_dist,
            success: "Say distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setemotedist",
            min_len: 12,
            min: say_dist / 4,
            max: say_dist,
            field: |runtime| &mut runtime.emote_dist,
            success: "Emote distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setquietsaydist",
            min_len: 15,
            min: say_dist / 6,
            max: say_dist / 2,
            field: |runtime| &mut runtime.quietsay_dist,
            success: "Quiet say distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setwhisperdist",
            min_len: 14,
            min: 1,
            max: say_dist / 2,
            field: |runtime| &mut runtime.whisper_dist,
            success: "Whisper distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "sethollercost",
            min_len: 13,
            min: 5 * POWERSCALE,
            max: 20 * POWERSCALE,
            field: |runtime| &mut runtime.holler_cost,
            success: "Holler cost changed from {old} to {new} endurance points",
            invalid: "Invalid value. Please specify a cost between 5 and 20 endurance points",
            display_divisor: POWERSCALE,
        },
        CommunicationTuningSpec {
            command: "setshoutcost",
            min_len: 12,
            min: 2 * POWERSCALE,
            max: 10 * POWERSCALE,
            field: |runtime| &mut runtime.shout_cost,
            success: "Shout cost changed from {old} to {new} endurance points",
            invalid: "Invalid value. Please specify a cost between 2 and 10 endurance points",
            display_divisor: POWERSCALE,
        },
    ];

    for spec in specs {
        if lower.len() < spec.min_len || !spec.command.starts_with(lower) {
            continue;
        }
        let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if (spec.min..=spec.max).contains(&value) {
            let field = (spec.field)(runtime);
            let old = *field;
            *field = value;
            return Some(KeyringCommandResult {
                messages: vec![spec
                    .success
                    .replace("{old}", &(old / spec.display_divisor).to_string())
                    .replace("{new}", &(value / spec.display_divisor).to_string())],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![spec
                .invalid
                .replace("{min}", &(spec.min / spec.display_divisor).to_string())
                .replace("{max}", &(spec.max / spec.display_divisor).to_string())],
            ..Default::default()
        });
    }

    None
}

pub(crate) fn apply_admin_character_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if let Some(result) = if world
        .characters
        .get(&character_id)
        .is_some_and(|caller| caller.flags.contains(CharacterFlags::GOD))
    {
        apply_legacy_tick_tuning_command(runtime, &lower, rest)
            .or_else(|| apply_legacy_communication_tuning_command(runtime, &lower, rest))
    } else {
        None
    } {
        return Some(result);
    }

    if matches!(
        lower.as_str(),
        "setdecaytime"
            | "setplayerbodytime"
            | "setnpcbodytime"
            | "setnpcbodytimearea32"
            | "setrespawntime"
            | "setsewerrespawntime"
            | "setlagouttime"
            | "setregentime"
            | "sethollerdist"
            | "setshoutdist"
            | "setsaydist"
            | "setemotedist"
            | "setquietsaydist"
            | "setwhisperdist"
            | "sethollercost"
            | "setshoutcost"
    ) {
        return None;
    }

    if lower.len() >= 4 && "prof".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["--- Profile ---".to_string(), "---------------".to_string()],
            ..Default::default()
        });
    }

    if lower.len() >= 6 && "staffcode".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, code_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let mut letters = code_text.trim_start().chars();
        let first = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let second = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let code = format!("{first}{second}");
        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let target_name = target.name.clone();
        target.staff_code = code.clone();
        runtime.staff_codes.insert(target_id, code.clone());
        return Some(KeyringCommandResult {
            messages: vec![format!("Set {target_name}'s staff code to {code}.")],
            ..Default::default()
        });
    }

    if lower == "reset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if target.values.len() < 2 {
            target
                .values
                .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
        }
        if target.values[1].len() < CHARACTER_VALUE_NAMES.len() {
            target.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
        for index in 0..=CharacterValue::Immunity as usize {
            let cap = if index <= CharacterValue::Strength as usize {
                10
            } else {
                1
            };
            if target.values[1][index] > cap {
                target.values[1][index] = cap;
            }
        }
        for value in [CharacterValue::Rage, CharacterValue::Duration] {
            let index = value as usize;
            if target.values[1][index] > 1 {
                target.values[1][index] = 1;
            }
        }
        target.exp_used = 0;
        target.flags.insert(CharacterFlags::UPDATE);
        return Some(KeyringCommandResult {
            inventory_changed: target_id == character_id,
            name_changed: target_id == character_id,
            ..Default::default()
        });
    }

    if lower == "resetgift" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, area_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let area_id = legacy_atoi_prefix(area_text.trim_start());
        if !(0..=63).contains(&area_id) {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid area ID. Must be between 0 and 63.".to_string()],
                ..Default::default()
            });
        }

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Could not retrieve player data.".to_string()],
                ..Default::default()
            });
        };
        let was_set = target_player.xmas_tree_marked(area_id as u16);
        target_player.unmark_xmas_tree(area_id as u16);
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.as_str())
            .unwrap_or(name);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset gift flag for {} in area {} (was {}).",
                target_name,
                area_id,
                if was_set { "set" } else { "not set" }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "questlog".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let name = rest
            .trim_start()
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target_name) = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target_player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Failed to get quest data for {target_name}")],
                ..Default::default()
            });
        };

        let mut messages = vec![format!("Quest log for {target_name}:")];
        for (quest_id, entry) in target_player.quest_log.entries().iter().enumerate() {
            if entry.flags != 0 {
                messages.push(format!(
                    "Quest #{}: {}, Done level: {}",
                    quest_id,
                    if (entry.flags & QF_OPEN) != 0 {
                        "Open"
                    } else {
                        "Closed"
                    },
                    entry.done
                ));
            }
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "listitem".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let item_id = ItemId(legacy_atoi_prefix(rest).max(0) as u32);
        let Some(item) = world.items.get(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid item number or item doesn't exist".to_string()],
                ..Default::default()
            });
        };

        let mut messages = vec![
            format!("Item #{}: {}", item.id.0, item.name),
            format!("Description: {}", item.description),
            format!("Flags: 0x{:x}", item.flags.bits()),
            format!(
                "Driver: {}, ID: {}, Sprite: {}",
                item.driver, item.template_id, item.sprite
            ),
        ];
        if let Some(carried_by) = item.carried_by {
            let carrier_name = world
                .characters
                .get(&carried_by)
                .map(|character| character.name.as_str())
                .unwrap_or("Unknown");
            messages.push(format!("Carried by: {} ({})", carrier_name, carried_by.0));
        } else if item.x != 0 {
            messages.push(format!("Position: {},{}", item.x, item.y));
        }
        for n in 0..ugaris_core::entity::MAX_MODIFIERS {
            let modifier_index = item.modifier_index[n];
            if modifier_index != 0 {
                let skill_name = if modifier_index > 0 {
                    value_name(modifier_index)
                } else {
                    "unknown"
                };
                messages.push(format!(
                    "Mod #{}: {:+} to {}",
                    n, item.modifier_value[n], skill_name
                ));
            }
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "setkarma".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let mut split = rest.splitn(2, char::is_whitespace);
        let name = split.next().unwrap_or_default();
        let karma_text = split.next().unwrap_or_default().trim_start();
        let karma =
            legacy_atoi_prefix(karma_text).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let old_karma = target.karma;
        target.karma = karma;
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Changed {}'s karma from {} to {}",
                target.name, old_karma, target.karma
            )],
            ..Default::default()
        });
    }

    if lower == "setexpmod" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.exp_modifier;
            world.settings.exp_modifier = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Global experience modifier changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcoreexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.hardcore_exp_bonus;
            world.settings.hardcore_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcoremilexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            // C's `hardcore_military_exp_bonus` global is a single value
            // read directly by `give_military_pts`/`give_military_pts_no_npc`
            // (`tool.c:3249-3306`); stored on `world.settings` (like
            // `exp_modifier`/`hardcore_exp_bonus`) instead of `ServerRuntime`
            // so `World::give_military_pts` (`ugaris-core`, no `ServerRuntime`
            // access) can read the live-tunable value directly.
            let old_value = world.settings.hardcore_military_exp_bonus;
            world.settings.hardcore_military_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore military experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcorekillexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (1.0..=3.0).contains(&value) {
            let old_value = runtime.hardcore_kill_exp_bonus;
            runtime.hardcore_kill_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore kill experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec!["Invalid value. Please specify a number between 1.0 and 3.0".to_string()],
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "listchars".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let mut character_ids: Vec<_> = world.characters.keys().copied().collect();
        character_ids.sort_by_key(|id| id.0);

        let mut count = 0;
        let mut players = 0;
        let mut npcs = 0;
        let mut messages = vec!["Active characters:".to_string()];
        for id in character_ids {
            let Some(character) = world.characters.get(&id) else {
                continue;
            };
            if character.flags.is_empty() {
                continue;
            }
            count += 1;
            if character.flags.contains(CharacterFlags::PLAYER) {
                players += 1;
                messages.push(format!(
                    "Player: {:3} - {} (L{})",
                    id.0, character.name, character.level
                ));
            } else {
                npcs += 1;
                if count < 50 {
                    messages.push(format!(
                        "NPC:    {:3} - {} (L{}, D:{})",
                        id.0, character.name, character.level, character.driver
                    ));
                }
            }
        }
        messages.push(format!(
            "Total: {count} characters ({players} players, {npcs} NPCs)"
        ));
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower == "setskill" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (name, pos, val) = parse_setskill_args(rest);
        let Some(target_id) = find_online_character_by_name(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if pos < 0 || pos >= CHARACTER_VALUE_NAMES.len() as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Position out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if !(0..=255).contains(&val) {
            return Some(KeyringCommandResult {
                messages: vec!["Value out of bounds.".to_string()],
                ..Default::default()
            });
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        if target.values.len() < 2 {
            target
                .values
                .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
        }
        if target.values[1].len() < CHARACTER_VALUE_NAMES.len() {
            target.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
        let pos = pos as usize;
        let old_value = target.values[1][pos];
        let old_exp_used = target.exp_used;
        target.values[1][pos] = val as i16;
        target.exp_used = legacy_calc_exp_used(target);
        target.flags.insert(CharacterFlags::UPDATE);
        let diff = i64::from(target.exp_used) - i64::from(old_exp_used);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Skill: {} (pos {}), Old value: {}, New value: {}, exp used changed by {}.",
                value_name(pos as i16),
                pos,
                old_value,
                target.values[1][pos],
                diff
            )],
            inventory_changed: true,
            name_changed: target_id == character_id,
            ..Default::default()
        });
    }

    if lower == "setlevel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let level = legacy_atoi_prefix(rest).max(0) as u32;
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.level = level;
            character.exp = level2exp(level);
            if character.values.len() < 2 {
                character
                    .values
                    .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
            }
            if character.values[1].len() < CHARACTER_VALUE_NAMES.len() {
                character.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
            }

            if level < 30 {
                character.flags.remove(CharacterFlags::ARCH);
                character.values[1][CharacterValue::Duration as usize] = 0;
                character.values[1][CharacterValue::Rage as usize] = 0;
            }
            if level > 35 {
                character.flags.insert(CharacterFlags::ARCH);
                let mage_only = character.flags.contains(CharacterFlags::MAGE)
                    && !character.flags.contains(CharacterFlags::WARRIOR);
                let warrior_only = character.flags.contains(CharacterFlags::WARRIOR)
                    && !character.flags.contains(CharacterFlags::MAGE);
                if mage_only && character.values[1][CharacterValue::Duration as usize] == 0 {
                    character.values[1][CharacterValue::Duration as usize] = 1;
                }
                if warrior_only && character.values[1][CharacterValue::Rage as usize] == 0 {
                    character.values[1][CharacterValue::Rage as usize] = 1;
                }
            }
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        }
        world.clear_character_spell_slots_and_effects(character_id);
        return Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "exp".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }
        if exp != 0 {
            give_exp_with_runtime_modifiers(world, target_id, exp, area_id);
            let target = world
                .characters
                .get(&target_id)
                .expect("target just checked");
            return Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            });
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        });
    }

    if lower == "milexp" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }
        if exp != 0 {
            // C `cmd_milexp` -> `give_military_pts_no_npc(co, val, 1)`
            // (`command.c:3048`, `tool.c:3281-3299`): `pts` is the typed
            // amount (goes to `military_points`, hardcore-multiplied by
            // `hardcore_military_exp_bonus`), while `exps` is a *fixed* `1`
            // that routes through `give_exp` (and `normal_exp`), regardless
            // of the typed amount. `World::give_military_pts` (`crates/
            // ugaris-core/src/world/military.rs`) is the shared port of
            // `give_military_pts_no_npc` itself, including the rank-
            // promotion feedback text and the above-Sergeant-Major
            // server-wide "Grats:" broadcast that this call site previously
            // skipped entirely.
            let points = exp.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            world.give_military_pts(target_id, points, 1, area_id);
            let Some(target) = world.characters.get(&target_id) else {
                return Some(KeyringCommandResult::default());
            };
            return Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} military exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            });
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        });
    }

    // C `cmd_milinfo` (`command.c:5071-5160`, `CF_GOD`-gated,
    // `command.c:10085-10091`): full-word `/milinfo [name]`, self if no
    // name given.
    if lower == "milinfo" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    });
                }
            }
        };
        let Some(target) = world.characters.get(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let target_name = target.name.clone();
        let military_points = target.military_points;
        let military_normal_exp = target.military_normal_exp;
        let current_yday = world.date.yday + 1;

        let Some(player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let mut messages = vec![
            format!("Military Info for {target_name}:"),
            format!(
                "Rank: {} (Military points: {military_points})",
                army_rank_name(army_rank_for_points(military_points))
            ),
            format!(
                "Current recommendation points: {}",
                player.military_current_pts()
            ),
            format!("Total military experience earned: {military_normal_exp}"),
        ];

        let took_mission = player.military_took_mission();
        if took_mission != 0 {
            let mission_idx = (took_mission - 1).max(0) as usize;
            let mission = player.military_mission(mission_idx);
            let type_str = military_mission_slot_type_name(mission.mission_type);
            let verb = if mission.mission_type == 3 {
                "Mining"
            } else {
                "Slaying"
            };
            let diff_name = MILITARY_DIFFICULTY_NAMES
                .get(mission_idx)
                .copied()
                .unwrap_or("Unknown");
            messages.push(format!(
                "Current mission: {type_str} {verb} (Difficulty: {diff_name})"
            ));
            if mission.mission_type == 1 || mission.mission_type == 2 {
                messages.push(format!(
                    "Target: {} level {} enemies",
                    mission.opt1, mission.opt2
                ));
            } else if mission.mission_type == 3 {
                messages.push(format!("Target: {} silver", mission.opt1));
            }
        } else {
            messages.push("No active mission".to_string());
        }

        let type_pref = player.mission_type_preference();
        messages.push(format!(
            "Mission type preference: {type_pref} ({})",
            military_type_preference_name(type_pref)
        ));

        let diff_pref = player.mission_difficulty_preference();
        let diff_pref_name = if (0..5).contains(&diff_pref) {
            MILITARY_DIFFICULTY_NAMES[diff_pref as usize]
        } else {
            "None"
        };
        messages.push(format!(
            "Mission difficulty preference: {diff_pref} ({diff_pref_name})"
        ));

        messages.push(format!(
            "Mission generation day: {} (Today: {current_yday})",
            player.mission_yday()
        ));
        messages.push(format!(
            "Mission taken day: {}",
            player.military_took_yday()
        ));
        messages.push(format!(
            "Mission solved day: {}",
            player.military_solved_yday()
        ));
        messages.push(format!(
            "Mission last Reroll day: {}",
            player.military_reroll_yday()
        ));

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milpref` (`command.c:5169-5249`, `CF_GOD`-gated): sets a
    // player's mission type/difficulty preference. Name is required
    // (unlike `milinfo`/`milreset`/`milsolve`'s self-fallback) - C prints
    // the 3-line usage block instead.
    if lower == "milpref" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /milpref <character> <type> <difficulty>".to_string(),
                    "Types: 0=none, 1=demon, 2=ratling, 3=silver".to_string(),
                    "Difficulties: 0=easy, 1=normal, 2=hard, 3=impossible, 4=insane, -1=none"
                        .to_string(),
                ],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        // C parses `type`/`diff` as two whitespace-separated integer
        // tokens after the name (`command.c:5200-5217`), both defaulting
        // to `-1` if absent. `diff`'s own acceptance range (`-1..=4`)
        // means an *omitted* difficulty argument is itself a valid "no
        // preference" value that overwrites any existing preference - a
        // real, reproduced C quirk (verified by reading the C source
        // directly), not a bug in this port.
        let mut tokens = remainder.split_whitespace();
        let type_value = tokens
            .next()
            .map(legacy_atoi_prefix)
            .map(|value| value as i32)
            .unwrap_or(-1);
        let diff_value = tokens
            .next()
            .map(legacy_atoi_prefix)
            .map(|value| value as i32)
            .unwrap_or(-1);

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let mut messages = Vec::new();
        if (0..=3).contains(&type_value) {
            player.set_mission_type_preference(type_value);
            messages.push(format!(
                "Set mission type preference to {type_value} ({}) for {target_name}",
                military_type_preference_name(type_value)
            ));
        }
        if (-1..5).contains(&diff_value) {
            player.set_mission_difficulty_preference(diff_value);
            let diff_name = if diff_value >= 0 {
                MILITARY_DIFFICULTY_NAMES[diff_value as usize]
            } else {
                "None"
            };
            messages.push(format!(
                "Set mission difficulty preference to {diff_value} ({diff_name}) for {target_name}"
            ));
        }
        // Reset mission generation day to force new missions with these
        // preferences (`command.c:5243`, unconditional).
        player.set_mission_yday(0);
        messages.push(
            "New missions will be generated with these preferences when player visits the Military Master"
                .to_string(),
        );

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milreset` (`command.c:5258-5304`, `CF_GOD`-gated): resets
    // mission/advisor cooldowns, self if no name given.
    if lower == "milreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    });
                }
            }
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        player.set_mission_yday(0);
        player.set_military_solved_yday(0);
        player.set_military_took_mission(0);
        player.set_military_reroll_yday(0);
        for advisor in 0..MILITARY_PPD_MAXADVISOR {
            player.set_military_advisor_last(advisor, 0);
        }

        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset all mission and advisor cooldowns for {target_name}"
            )],
            ..Default::default()
        });
    }

    // C `cmd_milpoints` (`command.c:5313-5384`, `CF_GOD`-gated): grants
    // raw military points to a named player. Name is required (no self
    // fallback). Deliberately does NOT call `World::give_military_pts`
    // (see the promotion-block comment below for why).
    if lower == "milpoints" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /milpoints <character> <points>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return Some(KeyringCommandResult {
                messages: vec!["Please specify number of points to grant.".to_string()],
                ..Default::default()
            });
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let target_name = target.name.clone();
        let old_rank = army_rank_for_points(target.military_points);
        target.military_points = target.military_points.saturating_add(points);
        let new_points = target.military_points;
        let new_rank = army_rank_for_points(new_points);

        // C inlines its own promotion check here rather than calling
        // `give_military_pts_no_npc`: no hardcore bonus, no `give_exp`/
        // `normal_exp` touch, a hardcoded `newrank < 25` promotion cap
        // (not `MAX_ARMY_RANK`=40), and its own message text - confirmed
        // by reading `cmd_milpoints` directly, distinct from
        // `give_military_pts_no_npc` (`tool.c:3281-3309`), which is *not*
        // called from this command.
        let messages = if new_rank > old_rank && new_rank < 25 {
            if new_rank > 9 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(COL_MAUVE);
                broadcast.extend_from_slice(
                    format!(
                        "Grats: {target_name} is a {} now!",
                        army_rank_name(new_rank)
                    )
                    .as_bytes(),
                );
                world.queue_channel_broadcast(6, broadcast);
            }
            vec![format!(
                "Granted {points} military points to {target_name}, promoting to {}!",
                army_rank_name(new_rank)
            )]
        } else {
            vec![format!(
                "Granted {points} military points to {target_name} (total: {new_points})"
            )]
        };

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milrec` (`command.c:5393-5446`, `CF_GOD`-gated): grants
    // "recommendation points" (`ppd->current_pts`) to a named player.
    if lower == "milrec" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /milrec <character> <points>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Please specify number of recommendation points to grant.".to_string()
                ],
                ..Default::default()
            });
        }

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };
        let new_total = player.military_current_pts().saturating_add(points);
        player.set_military_current_pts(new_total);

        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Granted {points} recommendation points to {target_name} (total: {new_total})"
            )],
            ..Default::default()
        });
    }

    // C `cmd_milstats` (`command.c:5456-5489`, `CF_GOD`-gated): first
    // scans for the Military Master NPC (`ch[n].driver ==
    // CDR_MILITARY_MASTER`) and bails out with this exact message if none
    // exists - no `CDR_MILITARY_MASTER` driver/NPC has been ported to
    // Rust yet (see the "Military ranks" task's REMAINING note in
    // `PORTING_TODO.md`), so that branch always fires here, matching C's
    // own behavior in any area where the NPC hasn't been spawned.
    if lower == "milstats" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["Could not find Military Master NPC.".to_string()],
            ..Default::default()
        });
    }

    // C `cmd_milsolve` (`command.c:5498-5613`, `CF_GOD`-gated): force-
    // completes a player's active mission, self if no name given, with an
    // optional trailing `announce` flag that also broadcasts a high-rank
    // promotion and notifies the target player directly.
    if lower == "milsolve" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    });
                }
            }
        };
        let announce = remainder
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("announce");

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let current_yday = world.date.yday as i32;

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let took_mission = player.military_took_mission();
        if took_mission == 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!("{target_name} does not have an active mission.")],
                ..Default::default()
            });
        }
        let mission_idx = (took_mission - 1).max(0) as usize;
        let mission = player.military_mission(mission_idx);
        let mission_type_str = military_mission_slot_type_name(mission.mission_type);
        let exp_reward = mission.exp;
        let points_reward = mission.pts;

        player.set_military_solved_mission(true);
        player.set_military_solved_yday(current_yday + 1);
        player.set_military_took_mission(0);

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let old_mil_pts = target.military_points;
        target.military_points = target.military_points.saturating_add(points_reward);
        target.military_normal_exp = target.military_normal_exp.saturating_add(exp_reward);
        let new_mil_pts = target.military_points;
        let is_player = target.flags.contains(CharacterFlags::PLAYER);

        let old_rank = army_rank_for_points(old_mil_pts);
        let new_rank = army_rank_for_points(new_mil_pts);
        let diff_name = MILITARY_DIFFICULTY_NAMES
            .get(mission_idx)
            .copied()
            .unwrap_or("Unknown");

        let messages = if new_rank > old_rank && new_rank < 25 {
            if new_rank > 9 && announce {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(COL_MAUVE);
                broadcast.extend_from_slice(
                    format!(
                        "Grats: {target_name} is a {} now!",
                        army_rank_name(new_rank)
                    )
                    .as_bytes(),
                );
                world.queue_channel_broadcast(6, broadcast);
            }
            vec![format!(
                "Completed {diff_name} {mission_type_str} mission for {target_name}! Rewards: {points_reward} mil pts, {exp_reward} exp. Promoted to {}!",
                army_rank_name(new_rank)
            )]
        } else {
            vec![format!(
                "Completed {diff_name} {mission_type_str} mission for {target_name}! Rewards: {points_reward} mil pts, {exp_reward} exp."
            )]
        };

        if is_player && announce {
            world.queue_system_text(
                target_id,
                format!(
                    "A staff member has completed your {diff_name} {mission_type_str} mission for you. You received {points_reward} military points and {exp_reward} experience."
                ),
            );
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    let Some(character) = world.characters.get_mut(&character_id) else {
        return Some(KeyringCommandResult::default());
    };

    let is_lqmaster = character.flags.contains(CharacterFlags::GOD)
        || character.flags.contains(CharacterFlags::EVENTMASTER)
        || (area_id == 20 && character.flags.contains(CharacterFlags::LQMASTER));

    if lower == "noexp" {
        if !character.flags.contains(CharacterFlags::NOEXP)
            && is_gatekeeper_room(area_id, character)
        {
            return Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoExp mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            });
        }
        character.flags.toggle(CharacterFlags::NOEXP);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned NoExp mode {}.",
                if character.flags.contains(CharacterFlags::NOEXP) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "nolevel" {
        if !character.flags.contains(CharacterFlags::NOLEVEL)
            && is_gatekeeper_room(area_id, character)
        {
            return Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoLevel mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            });
        }
        character.flags.toggle(CharacterFlags::NOLEVEL);
        let enabled = character.flags.contains(CharacterFlags::NOLEVEL);
        return Some(KeyringCommandResult {
            messages: vec![if enabled {
                "NoLevel mode enabled. You will not level up until you disable this mode."
                    .to_string()
            } else {
                "NoLevel mode disabled. You will now gain levels normally.".to_string()
            }],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "itemmod" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (pos, nr, val) = parse_itemmod_args(rest);
        let Some(item_id) = character.cursor_item else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        if pos < 0 || pos >= ugaris_core::entity::MAX_MODIFIERS as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Pos out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if nr < 0 || nr >= CHARACTER_VALUE_NAMES.len() as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Nr out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if !(0..22).contains(&val) {
            return Some(KeyringCommandResult {
                messages: vec!["Val out of bounds.".to_string()],
                ..Default::default()
            });
        }
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        item.modifier_index[pos as usize] = nr as i16;
        item.modifier_value[pos as usize] = val as i16;
        let mut messages: Vec<String> = legacy_item_look_text(item, &character_snapshot)
            .lines()
            .map(str::to_string)
            .collect();
        messages.push(format!(
            "Item modified: {} (skill {}) at pos {} with value {}",
            value_name(nr as i16),
            nr,
            pos,
            val
        ));
        return Some(KeyringCommandResult {
            messages,
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "itemdesc" || lower == "itemname" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let Some(item_id) = character.cursor_item else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        let trimmed = rest.trim_start();
        let text = legacy_truncate_c_string(trimmed, 79);
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        if lower == "itemdesc" {
            item.description = text;
        } else {
            item.name = text;
        }
        return Some(KeyringCommandResult {
            messages: legacy_item_look_text(item, &character_snapshot)
                .lines()
                .map(str::to_string)
                .collect(),
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 4 && "saves".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let saves = legacy_atoi_prefix(rest).clamp(0, i64::from(u8::MAX)) as u8;
        character.saves = saves;
        return Some(KeyringCommandResult::default());
    }

    if lower == "sprite" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.sprite = legacy_atoi_prefix(rest) as i32;
        return Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 2 && "immortal".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::IMMORTAL);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Immortal is {}.",
                if character.flags.contains(CharacterFlags::IMMORTAL) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "infrared".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INFRARED);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Infrared is {}.",
                if character.flags.contains(CharacterFlags::INFRARED) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "invisible".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INVISIBLE);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Invisible is {}.",
                if character.flags.contains(CharacterFlags::INVISIBLE) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower == "xray" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::XRAY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned x-ray mode {}.",
                if character.flags.contains(CharacterFlags::XRAY) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "spy".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::SPY);
        let enabled = character.flags.contains(CharacterFlags::SPY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned spy mode {}. You will {} see all tells, clan, alliance, club, area, and mirror chat.",
                if enabled { "on" } else { "off" },
                if enabled { "now" } else { "no longer" }
            )],
            ..Default::default()
        });
    }

    if lower == "setxmas" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let flag = legacy_atoi_prefix(rest.trim_start()) as i32;
        let old_value = runtime_effective_xmas_flag(runtime);
        runtime.xmas_special_override = Some(flag);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Setting christmas special to {flag}, old value was {old_value}."
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 6 && "dlight".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.dlight_override = legacy_atoi_prefix(rest) as i32;
        let override_value = (runtime.dlight_override != 0).then_some(runtime.dlight_override);
        world.date = GameDate::calculate(
            START_TIME + world.date.realtime,
            area_id as i32,
            override_value,
        );
        return Some(KeyringCommandResult::default());
    }

    if lower.len() >= 6 && "showattack".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.show_attack = !runtime.show_attack;
        world.show_attack_debug = runtime.show_attack;
        return Some(KeyringCommandResult::default());
    }

    if lower == "joinclan" || lower == "joinclub" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if lower == "joinclan" {
            if (0..LEGACY_MAX_CLAN).contains(&nr) {
                character.clan = nr as u16;
                character.clan_rank = 4;
                character.clan_serial = world.clan_registry.serial(nr as u16);
            }
        } else if (0..LEGACY_MAX_CLUB).contains(&nr) {
            character.clan = (nr + LEGACY_CLUB_OFFSET) as u16;
            character.clan_rank = 2;
            character.clan_serial = 0;
        }
        return Some(KeyringCommandResult {
            name_changed: true,
            ..Default::default()
        });
    }

    // C `killclan` (`src/system/command.c:6468-6482`): sets the target
    // clan's debt sky-high (`kill_clan`, `clan.c:1413-1416`) so the next
    // weekly `update_treasure` tick (`clan.c:1154-1160`, `debt >= 2000`)
    // deletes it. `update_treasure`/the whole treasury economy isn't
    // ported (see the clan task's REMAINING notes), so this deletes the
    // clan immediately via [`ClanRegistry::delete_clan`] - the eventual
    // real-world outcome of C's `kill_clan`, without the week-long delay.
    // C emits no player feedback for this command; matched exactly (no
    // messages either way).
    if lower == "killclan" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if (1..LEGACY_MAX_CLAN).contains(&nr) {
            world.clan_registry.delete_clan(nr as u16);
        }
        return Some(KeyringCommandResult::default());
    }

    // C `cmd_renclan` (`src/system/command.c:4497-4531`), dispatched at
    // `command.c:9646` gated on `CF_STAFF | CF_GOD`. Renames an existing
    // clan; only usable while standing in Aston (`areaID == 3`).
    if lower == "renclan" {
        if !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        if area_id != 3 {
            return Some(KeyringCommandResult {
                messages: vec!["Sorry, this command only works in Aston.".to_string()],
                ..Default::default()
            });
        }
        let rest = rest.trim_start();
        let nr = legacy_atoi_prefix(rest);
        let name = rest
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        if !(1..LEGACY_MAX_CLAN).contains(&nr) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Clan number must be between 1 and {}.",
                    LEGACY_MAX_CLAN - 1
                )],
                ..Default::default()
            });
        }
        let name: String = name.chars().take(78).collect();
        let messages = match world.clan_registry.set_name(nr as u16, &name) {
            Ok(()) => vec![format!("Clan {nr} name changed to \"{name}\".")],
            Err(_) => vec![format!("No clan by that number ({nr}).")],
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    None
}

pub(crate) fn is_gatekeeper_room(area_id: u32, character: &Character) -> bool {
    area_id == 3 && (178..=210).contains(&character.x) && (196..=228).contains(&character.y)
}
