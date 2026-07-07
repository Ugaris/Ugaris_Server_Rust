use super::*;

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

/// C `/setseyan` (`command.c:9989-9996`, `CF_GOD`-gated,
/// `cmdcmp(ptr, "setseyan", 8)` - `minlen == strlen("setseyan")`, so this
/// is an exact-word match only, no abbreviations) plus `cmd_setseyan`
/// (`command.c:3055-3078`): looks up an online character by name (no
/// self-fallback) and rerolls them into a plain Seyan'Du via the
/// already-ported `turn_seyan` (`tool.c:4278-4389`,
/// `World::apply_turn_seyan` + `PlayerRuntime::clear_turn_seyan_ppd`),
/// the same reroll `/goto`'s sibling gate-fight reward path
/// (`World::apply_gate_fight_reward`'s class-8 case) already drives.
/// C sends the confirmation message to the *target* (`co`), not the
/// caller (`log_char(co, ...)` in `cmd_setseyan`) - only the "no one by
/// that name" error goes to the caller.
pub(crate) fn apply_setseyan_command(
    world: &mut World,
    loader: &ZoneLoader,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("setseyan") {
        return None;
    }

    let caller = world.characters.get(&character_id)?;
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

    let Some(base_values) = loader
        .character_templates
        .get("seyan_m")
        .map(|template| template.base_values.clone())
    else {
        return Some(KeyringCommandResult::default());
    };

    let applied = world.apply_turn_seyan(target_id, &base_values);
    if !applied {
        return Some(KeyringCommandResult::default());
    }
    if let Some(player) = runtime.player_for_character_mut(target_id) {
        player.clear_turn_seyan_ppd();
    }

    let mut result = KeyringCommandResult {
        inventory_changed: target_id == character_id,
        name_changed: target_id == character_id,
        ..Default::default()
    };
    result
        .other_messages
        .push((target_id, "You are a Seyan'Du now.".to_string()));
    Some(result)
}
