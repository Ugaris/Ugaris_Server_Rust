use super::*;

pub(crate) fn staff_code_for<'a>(
    runtime: Option<&'a ServerRuntime>,
    world: &'a World,
    character_id: CharacterId,
) -> &'a str {
    if let Some(code) = world
        .characters
        .get(&character_id)
        .map(|character| character.staff_code.as_str())
        .filter(|code| !code.is_empty())
    {
        return code;
    }

    runtime
        .into_iter()
        .flat_map(|runtime| runtime.staff_codes.get(&character_id))
        .next()
        .map(String::as_str)
        .unwrap_or("")
}

#[cfg(test)]
pub(crate) fn runtime_staff_code(runtime: &ServerRuntime, character_id: CharacterId) -> &str {
    runtime
        .staff_codes
        .get(&character_id)
        .map(String::as_str)
        .unwrap_or("")
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct KeyringCommandResult {
    pub(crate) messages: Vec<String>,
    pub(crate) message_bytes: Vec<Vec<u8>>,
    pub(crate) target_message_bytes: Vec<(CharacterId, Vec<u8>)>,
    pub(crate) inventory_changed: bool,
    pub(crate) name_changed: bool,
    pub(crate) name_refresh: Vec<CharacterId>,
    /// Set when the command moved the character to a new mirror (C
    /// `ch[cn].mirror = m` in `/goto`/`/jump`, `command.c`). The call site
    /// must send the client a `mirror` packet, matching the same-area
    /// transport-travel mirror-change path.
    pub(crate) mirror_changed: Option<u32>,
}

pub(crate) fn legacy_light_red_text_bytes(message: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(COL_LIGHT_RED.len() + message.len() + COL_RESET.len());
    bytes.extend_from_slice(COL_LIGHT_RED);
    bytes.extend_from_slice(message.as_bytes());
    bytes.extend_from_slice(COL_RESET);
    bytes
}

pub(crate) fn legacy_help_line_bytes(line: &str) -> Vec<u8> {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 16);
    if line.starts_with("===") || line.starts_with("==") || line.starts_with("---") {
        out.extend_from_slice(COL_LIGHT_RED);
        out.extend_from_slice(bytes);
        out.extend_from_slice(COL_RESET);
        return out;
    }
    if line.starts_with("Note:") {
        out.extend_from_slice(COL_ORANGE);
        out.extend_from_slice(bytes);
        out.extend_from_slice(COL_RESET);
        return out;
    }
    if line.starts_with('/') || line.starts_with('#') {
        let split_at = bytes
            .iter()
            .position(|byte| byte.is_ascii_whitespace())
            .unwrap_or(bytes.len());
        out.extend_from_slice(COL_LIGHT_BLUE);
        out.extend_from_slice(&bytes[..split_at]);
        out.extend_from_slice(COL_RESET);
        color_help_parameters(&bytes[split_at..], &mut out);
        return out;
    }
    color_help_parameters(bytes, &mut out);
    out
}

pub(crate) fn color_help_parameters(bytes: &[u8], out: &mut Vec<u8>) {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            if let Some(end) = bytes[index..].iter().position(|byte| *byte == b'>') {
                let end = index + end + 1;
                out.extend_from_slice(COL_LIGHT_GREEN);
                out.extend_from_slice(&bytes[index..end]);
                out.extend_from_slice(COL_RESET);
                index = end;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
}

pub(crate) fn legacy_help_result(messages: Vec<String>) -> KeyringCommandResult {
    let message_bytes = messages
        .iter()
        .map(|message| legacy_help_line_bytes(message))
        .collect();
    KeyringCommandResult {
        messages,
        message_bytes,
        ..Default::default()
    }
}

pub(crate) fn normalize_text_command(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes)
        .ok()?
        .trim_matches(char::from(0))
        .trim();
    if text.is_empty() {
        return None;
    }
    Some(text.to_string())
}

pub(crate) fn legacy_alias_command_verb(verb: &str) -> Option<&'static str> {
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() >= 2 && "alias".starts_with(&lower) {
        return Some("alias");
    }
    if lower == "clearaliases" {
        return Some("clearaliases");
    }
    None
}

pub(crate) fn apply_alias_command(
    player: &mut PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = legacy_alias_command_verb(verb)?;
    if verb == "clearaliases" {
        player.aliases.clear();
        return Some(KeyringCommandResult {
            messages: vec!["Done. All gone now.".to_string()],
            ..Default::default()
        });
    }

    let rest = rest.trim_start();
    let mut from_end = rest.len();
    for (index, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            from_end = index;
            break;
        }
    }
    let from = rest[..from_end].chars().take(7).collect::<String>();
    if from.is_empty() {
        let messages = if player.aliases.is_empty() {
            vec!["None defined.".to_string()]
        } else {
            player
                .aliases
                .iter()
                .map(|alias| format!("{} -> {}", alias.from, alias.to))
                .collect()
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    let to = rest[from_end..]
        .trim_start()
        .chars()
        .take(55)
        .collect::<String>();
    if let Some(alias) = player
        .aliases
        .iter_mut()
        .find(|alias| alias.from.eq_ignore_ascii_case(&from))
    {
        if to.is_empty() {
            let old_from = alias.from.clone();
            let old_to = alias.to.clone();
            player
                .aliases
                .retain(|alias| !alias.from.eq_ignore_ascii_case(&from));
            return Some(KeyringCommandResult {
                messages: vec![format!("Erased {old_from} -> {old_to}.")],
                ..Default::default()
            });
        }
        alias.to = to.clone();
        return Some(KeyringCommandResult {
            messages: vec![format!("Replaced {from} -> {to}.")],
            ..Default::default()
        });
    }

    if to.is_empty() {
        return Some(KeyringCommandResult {
            messages: vec![format!("Alias {from} not found, could not delete.")],
            ..Default::default()
        });
    }
    if player.aliases.len() >= ugaris_core::player::ALIAS_MAX_ENTRIES {
        return Some(KeyringCommandResult {
            messages: vec!["Alias memory is full, cannot add.".to_string()],
            ..Default::default()
        });
    }
    player.aliases.push(CommandAlias {
        from: from.clone(),
        to: to.clone(),
    });
    Some(KeyringCommandResult {
        messages: vec![format!("Created {from} -> {to}.")],
        ..Default::default()
    })
}

pub(crate) fn find_online_character_by_name(world: &World, name: &str) -> Option<CharacterId> {
    world
        .characters
        .values()
        .find(|character| character.name.eq_ignore_ascii_case(name))
        .map(|character| character.id)
}

pub(crate) fn take_legacy_alpha_name(text: &str) -> (&str, &str) {
    let end = text
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_alphabetic()).then_some(index))
        .unwrap_or(text.len());
    text.split_at(end)
}

pub(crate) fn legacy_pk_command_verb(verb: &str) -> Option<&'static str> {
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("playerkiller") {
        return Some("playerkiller");
    }
    if verb.eq_ignore_ascii_case("iwilldie") {
        return Some("iwilldie");
    }
    if verb.len() >= 2 && "listhate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("listhate");
    }
    if verb.len() >= 3 && "hate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("hate");
    }
    if verb.len() >= 3 && "nohate".starts_with(&verb.to_ascii_lowercase()) {
        return Some("nohate");
    }
    if verb.eq_ignore_ascii_case("clearhate") {
        return Some("clearhate");
    }
    None
}

pub(crate) fn legacy_atoi_prefix(input: &str) -> i64 {
    let input = input.trim_start();
    let mut chars = input.chars().peekable();
    let sign = match chars.peek().copied() {
        Some('-') => {
            chars.next();
            -1
        }
        Some('+') => {
            chars.next();
            1
        }
        _ => 1,
    };
    let mut value = 0i64;
    let mut seen_digit = false;
    while let Some(ch) = chars.peek().copied() {
        let Some(digit) = ch.to_digit(10) else {
            break;
        };
        seen_digit = true;
        chars.next();
        value = value.saturating_mul(10).saturating_add(i64::from(digit));
    }
    if seen_digit {
        value.saturating_mul(sign)
    } else {
        0
    }
}

pub(crate) fn legacy_atof_prefix(input: &str) -> f64 {
    let input = input.trim_start();
    let bytes = input.as_bytes();
    let mut end = 0usize;

    if matches!(bytes.get(end), Some(b'+' | b'-')) {
        end += 1;
    }

    let mut saw_digit = false;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        saw_digit = true;
        end += 1;
    }

    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            saw_digit = true;
            end += 1;
        }
    }

    if saw_digit && matches!(bytes.get(end), Some(b'e' | b'E')) {
        let exp_start = end;
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let exp_digits_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if exp_digits_start == end {
            end = exp_start;
        }
    }

    if saw_digit {
        input[..end].parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    }
}

pub(crate) fn legacy_color_word(red: i64, green: i64, blue: i64) -> u16 {
    ((red << 10) + (green << 5) + blue) as u16
}

pub(crate) fn parse_legacy_color_triplet(rest: &str) -> [i64; 3] {
    let mut values = [0; 3];
    let mut input = rest;
    for value in &mut values {
        input = input.trim_start();
        *value = legacy_atoi_prefix(input);

        let digit_len = input
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        input = &input[digit_len..];
    }
    values
}

pub(crate) fn apply_color_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 4 && "color".starts_with(&lower) {
        let character = world.characters.get(&character_id)?;
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "c1={:X}, c2={:X}, c3={:X}",
                character.c1, character.c2, character.c3
            )],
            ..Default::default()
        });
    }

    let color_slot = match lower.as_str() {
        "col1" => Some(1),
        "col2" => Some(2),
        "col3" => Some(3),
        _ => None,
    }?;
    let [red, green, blue] = parse_legacy_color_triplet(rest);
    let color = legacy_color_word(red, green, blue);
    let character = world.characters.get_mut(&character_id)?;
    match color_slot {
        1 => character.c1 = color,
        2 => character.c2 = color,
        3 => character.c3 = color,
        _ => unreachable!(),
    }
    Some(KeyringCommandResult {
        name_changed: true,
        ..Default::default()
    })
}

pub(crate) fn legacy_description_text(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|ch| match ch {
            '"' => '\'',
            '%' => ' ',
            _ => ch,
        })
        .collect::<String>();

    let mut out = String::new();
    for ch in sanitized.chars() {
        if out.len() + ch.len_utf8() >= 160 {
            break;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn legacy_truncate_c_string(input: &str, max_bytes: usize) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if out.len() + ch.len_utf8() > max_bytes {
            break;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn apply_description_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 3 || !"description".starts_with(&lower) {
        return None;
    }

    let text = rest.trim_start();
    if text.is_empty() {
        return Some(KeyringCommandResult {
            messages: vec!["Sorry, you need to enter some text.".to_string()],
            ..Default::default()
        });
    }

    let description = legacy_description_text(text);
    let character = world.characters.get_mut(&character_id)?;
    character.description = description;
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Your description reads now: {}",
            character.description
        )],
        ..Default::default()
    })
}

pub(crate) fn apply_laugh_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("laugh") {
        return None;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if !character.flags.contains(CharacterFlags::GOD) {
        return None;
    }
    let (x, y) = (usize::from(character.x), usize::from(character.y));
    world.queue_sound_area(x, y, 13);
    Some(KeyringCommandResult::default())
}

pub(crate) fn apply_maxlag_command(
    player: &mut PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 4 || !"maxlag".starts_with(&lower) {
        return None;
    }

    let lag = legacy_atoi_prefix(rest);
    if !(3..=20).contains(&lag) {
        return Some(KeyringCommandResult {
            messages: vec!["Number must be between 3 and 20.".to_string()],
            ..Default::default()
        });
    }

    player.set_max_lag_seconds(lag as u8);
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Set delay for lag control to kick in to {} seconds.",
            player.max_lag_seconds
        )],
        ..Default::default()
    })
}

pub(crate) fn apply_hints_command(
    player: &mut PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 4 || !"hints".starts_with(&lower) {
        return None;
    }

    let disabled = player.toggle_hints();
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Hints turned {}.",
            if disabled { "off" } else { "on" }
        )],
        ..Default::default()
    })
}

pub(crate) fn apply_wimp_command(command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 4 || !"wimp".starts_with(&lower) {
        return None;
    }

    Some(KeyringCommandResult {
        messages: vec![
            "You're not in the live quest area. You'll have to wimp out on your own here... That means: RUN!"
                .to_string(),
        ],
        ..Default::default()
    })
}

pub(crate) fn apply_autoturn_command(
    character: &Character,
    player: &mut PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 5 || !"autoturn".starts_with(&lower) {
        return None;
    }

    player.toggle_autoturn();
    apply_status_command(character, player, "/status")
}

pub(crate) fn apply_lag_command(
    world: &mut World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("lag") {
        return None;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let turning_on = !character.flags.contains(CharacterFlags::LAG);
    let in_arena = world
        .map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::ARENA));

    if turning_on && in_arena {
        return Some(KeyringCommandResult {
            messages: vec!["You cannot simulate lag in an arena.".to_string()],
            ..Default::default()
        });
    }
    if turning_on && player.has_any_pk_hate() {
        return Some(KeyringCommandResult {
            messages: vec!["You cannot simulate lag while your hate list is not empty.".to_string()],
            ..Default::default()
        });
    }

    let Some(character) = world.characters.get_mut(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    character.flags.toggle(CharacterFlags::LAG);
    let enabled = character.flags.contains(CharacterFlags::LAG);
    let mut messages = vec![format!(
        "Turned artificial lag {}.",
        if enabled { "on" } else { "off" }
    )];
    if enabled {
        messages.push(
            "PLEASE turn this option off (type /lag again) before you complain about lag!"
                .to_string(),
        );
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

pub(crate) fn apply_time_command(date: GameDate, command: &str) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 2 || !"time".starts_with(&lower) {
        return None;
    }

    let mut messages = vec![format!(
        "It's {:02}:{:02} on the {}/{}/{}. Sunrise is at {:02}:{:02}, sunset at {:02}:{:02}. Moonrise is at {:02}:{:02}, moonset at {:02}:{:02}.",
        date.hour,
        date.minute,
        date.month + 1,
        date.mday + 1,
        date.year,
        date.sunrise / HOUR_LEN,
        (date.sunrise % HOUR_LEN) / MIN_LEN,
        date.sunset / HOUR_LEN,
        (date.sunset % HOUR_LEN) / MIN_LEN,
        date.moonrise / HOUR_LEN,
        (date.moonrise % HOUR_LEN) / MIN_LEN,
        date.moonset / HOUR_LEN,
        (date.moonset % HOUR_LEN) / MIN_LEN,
    )];

    if !date.fullmoon && !date.newmoon {
        if date.moonsize < 3 {
            messages.push("Quarter Moon.".to_string());
        } else if date.moonsize < 10 {
            messages.push("Half Moon.".to_string());
        } else {
            messages.push("Three Quarter Moon.".to_string());
        }
    }
    if date.newmoon {
        messages.push("Be careful, New Moon tonight!".to_string());
    }
    if date.fullmoon {
        messages.push("It's a fine day, Full Moon tonight!".to_string());
    }
    if date.summer_solstice {
        messages.push("It's a great day, it's Summer Solstice today!".to_string());
    }
    if date.winter_solstice {
        messages.push("It's a scary day, it's Winter Solstice today!".to_string());
    }
    if date.spring_equinox {
        messages.push("Everything is in balance, it's Spring Equinox today!".to_string());
    }
    if date.fall_equinox {
        messages.push("Everything is in balance, it's Fall Equinox today!".to_string());
    }

    if date.moonday < HALF_MOON_CYCLE {
        messages.push(format!(
            "Next full moon is in {} days.",
            HALF_MOON_CYCLE - date.moonday
        ));
    } else {
        messages.push(format!(
            "Next new moon is in {} days.",
            DAYS_PER_MOON_CYCLE - date.moonday
        ));
    }

    if date.yday < SPRING_EQUINOX_DAY {
        messages.push(format!(
            "Spring Equinox will be in {} days.",
            SPRING_EQUINOX_DAY - date.yday
        ));
    } else if date.yday < SUMMER_SOLSTICE_DAY {
        messages.push(format!(
            "Summer Solstice will be in {} days.",
            SUMMER_SOLSTICE_DAY - date.yday
        ));
    } else if date.yday < FALL_EQUINOX_DAY {
        messages.push(format!(
            "Fall Equinox will be in {} days.",
            FALL_EQUINOX_DAY - date.yday
        ));
    } else {
        messages.push(format!(
            "Winter Solstice will be in {} days.",
            DAYS_PER_YEAR - date.yday
        ));
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

pub(crate) fn apply_help_command(
    command: &str,
    flags: CharacterFlags,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.eq_ignore_ascii_case("achelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(anti_cheat_help_lines()));
    }
    if verb.eq_ignore_ascii_case("macrohelp") {
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(macro_help_lines()));
    }
    if verb.eq_ignore_ascii_case("penthelp") {
        if !flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(legacy_help_result(pentagram_help_lines()));
    }
    if !verb.eq_ignore_ascii_case("help") {
        return None;
    }

    let mut messages = vec![
        "=== PLAYER COMMANDS ===".to_string(),
        "== Communication Commands ==".to_string(),
        "/holler <text> - Say something with very long range (costs endurance points)".to_string(),
        "/shout <text> - Say something with extended range (costs endurance points)".to_string(),
        "/say <text> - Make your character say text to nearby players".to_string(),
        "/murmur <text> - Say something with reduced range (whisper alternative)".to_string(),
        "/whisper <text> - Say something with very short range".to_string(),
        "/tell <name> <text> - Send a private message to another player".to_string(),
        "/emote <text> - Express an action (Example: /emote jumps shows Player jumps)".to_string(),
        "/me <text> - Same as /emote (Example:  /me smiles  shows Player smiles)".to_string(),
        "== Emote Shortcuts ==".to_string(),
        "/wave - Wave at others (shortcut for /emote waves happily)".to_string(),
        "/bow - Bow to others (shortcut for /emote bows deeply)".to_string(),
        "/eg - Evil grin (shortcut for /emote grins evilly)".to_string(),
        "/slap <name> - Slap someone with a large trout (humorous emote)".to_string(),
        "/hugme - Show that you need a hug (shortcut for /emote is in need of a hug)".to_string(),
        "== Chat Channel Commands ==".to_string(),
        "/channels - List all available chat channels".to_string(),
        "/join <nr> - Join chat channel number <nr>".to_string(),
        "/leave <nr> - Leave chat channel number <nr>".to_string(),
        "/joinall - Join all channels from 1-13 at once".to_string(),
        "/ah - Various auction house commands".to_string(),
        "== Character & Interaction Commands ==".to_string(),
        "/description <text> - Change your character's description".to_string(),
        "/status - Show your lag control settings and account info".to_string(),
        "/time - Show the current game time and date".to_string(),
        "/weather - Display current weather conditions".to_string(),
        "/swap - Swap places with the player you're facing".to_string(),
        "/allow <name> - Allow another player to search your grave if you die".to_string(),
        "/lastseen <player> - Check when a player last logged into the game".to_string(),
        "/showvalues <player> - Show your stats to another player".to_string(),
        "/who - List all players currently in your area".to_string(),
        "/achievements - View your unlocked achievements".to_string(),
        "/achstats - View your achievement statistics".to_string(),
        "== Command Aliases ==".to_string(),
        "/aliases - Show your active command aliases".to_string(),
        "/alias <short> <long> - Create an alias (Example: \"/alias ty Thank you!\")".to_string(),
        "/alias <short> - Remove an existing alias".to_string(),
        "/clearaliases - Delete ALL your command aliases".to_string(),
        "== PvP & Security Commands ==".to_string(),
        "/playerkiller - Toggle player killing mode on/off".to_string(),
        "/iwilldie <id> - Confirm enabling player killer mode".to_string(),
        "/hate <name> - Add player to your PK list (only works in PK mode)".to_string(),
        "/nohate <name> - Remove player from your PK list".to_string(),
        "/listhate - Show all players on your PK list".to_string(),
        "/clearhate - Clear your entire PK list at once".to_string(),
        "/ignore <name> - Ignore a player in chat and tells".to_string(),
        "/clearignore - Remove ALL players from your ignore list".to_string(),
        "/notells - Toggle receiving private messages on/off".to_string(),
        "/complain <player> [reason] - Report abuse or scamming by a player".to_string(),
        "== Inventory & Gold Commands ==".to_string(),
        "/gold <amount> - Move gold coins to your cursor".to_string(),
        "/sort - Sort items in your inventory by value and type".to_string(),
        "/depotsort - Sort the contents of your storage depot".to_string(),
        "/accountdepotsort - Sort your account-wide storage depot".to_string(),
        "/keyring - View keys stored on your keyring".to_string(),
        "/keyring addall - Add all keys from inventory to keyring".to_string(),
        "/keyring remove <n> - Remove key number <n> from keyring".to_string(),
        "== Clan & Club Commands ==".to_string(),
        "/clan - Show information about the clans".to_string(),
        "/relation <nr> - Show clan <nr>'s diplomatic relations".to_string(),
        "/clanpots - Display information about your clan's potions".to_string(),
        "/clanlog - Check the clan logs (/clanlog -h for more details)".to_string(),
        "/club - Show information about clubs".to_string(),
        "== Character Development Commands ==".to_string(),
        "/set <spell nr> <key> - Change spell key mappings".to_string(),
        "/noexp - Toggle gaining experience on/off".to_string(),
        "/nolevel - Toggle preventing level-ups while continuing to earn exp".to_string(),
        "/hints - Toggle game hints on/off".to_string(),
        "/killbless - Remove all Bless effects from your character".to_string(),
        "== Thief-Specific Commands ==".to_string(),
        "/thief - Toggle thief mode on/off (thief characters only)".to_string(),
        "/steal - Attempt to steal an item from the character you're facing".to_string(),
        "== Game Information Commands ==".to_string(),
        "/orbs - Show available orbs and respawn timers".to_string(),
        "/tunnel <level> - Show progress on a specific tunnel level".to_string(),
        "/tunnels - Show list of all tunnel levels and their status".to_string(),
        "/treasures - Show information on treasures (mine chests, etc.)".to_string(),
        "/demonlords - Show information on demon lords and their status".to_string(),
        "== Lag Control Commands ==".to_string(),
        "/lag - Toggle artificial lag (for testing purposes)".to_string(),
        "/maxlag <seconds> - Set delay for lag control to activate (3-20 seconds)".to_string(),
        "/noball - Toggle using Ball Lightning spell during lag".to_string(),
        "/nobless - Toggle using Bless spell during lag".to_string(),
        "/nofireball - Toggle using Fireball spell during lag".to_string(),
        "/noflash - Toggle using Lightning Flash spell during lag".to_string(),
        "/nofreeze - Toggle using Freeze spell during lag".to_string(),
        "/noheal - Toggle using Heal spell during lag".to_string(),
        "/noshield - Toggle using Magic Shield spell during lag".to_string(),
        "/nowarcry - Toggle using Warcry during lag".to_string(),
        "/nopulse - Toggle using Pulse spell during lag".to_string(),
        "/nolife - Toggle using Healing Potions during lag".to_string(),
        "/nomana - Toggle using Mana Potions during lag".to_string(),
        "/nocombo - Toggle using Combo Potions during lag".to_string(),
        "/norecall - Toggle using Recall Scroll during lag".to_string(),
        "/nomove - Toggle character movement during lag".to_string(),
        "== Automation Commands ==".to_string(),
        "/autobless - Toggle automatic re-blessing when spell expires".to_string(),
        "/autoturn - Toggle automatic turning toward enemies".to_string(),
        "/autopulse - Toggle automatic pulse casting".to_string(),
        "/allowbless - Toggle allowing other players to bless you".to_string(),
        "== Miscellaneous Commands ==".to_string(),
        "/logout - Safely log out when standing on a blue square".to_string(),
        "/wimp - Exit from a Live Quest (may have consequences)".to_string(),
        "/help - Display this help text".to_string(),
    ];

    if flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
        messages.extend([
            "=== STAFF COMMANDS ===".to_string(),
            "== Player Management ==".to_string(),
            "/jump <name> <mirror> - Jump to a location or player in specified mirror".to_string(),
            "/look <name> - View a player's character information".to_string(),
            "/values <name> - View a player's stats and values".to_string(),
            "/kick <name> - Disconnect a player from the server".to_string(),
            "/nowho - Hide yourself from /who listings".to_string(),
            "/whostaff - List all staff members online".to_string(),
            "== Disciplinary Actions ==".to_string(),
            "/punish <name> <level> <reason> - Apply punishment to a player".to_string(),
            "/shutup <name> <minutes> - Prevent a player from talking".to_string(),
            "/exterminate <name> - Remove a player from the game".to_string(),
            "/jail <name> - Send a player to jail".to_string(),
            "/unjail <name> - Release a player from jail".to_string(),
            "/klog - Check karma logs".to_string(),
        ]);
    }

    if flags
        .intersects(CharacterFlags::EVENTMASTER | CharacterFlags::LQMASTER | CharacterFlags::GOD)
    {
        messages.push("=== EVENT/QUEST MASTER COMMANDS ===".to_string());
        if flags.contains(CharacterFlags::EVENTMASTER) {
            messages.extend([
                "== Event Master Commands ==".to_string(),
                "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            ]);
        }
        if flags.intersects(CharacterFlags::LQMASTER | CharacterFlags::GOD) {
            messages.extend([
                "== Quest Master Commands ==".to_string(),
                "/immortal - Toggle immortality status".to_string(),
                "/infrared - Toggle infrared vision".to_string(),
                "/invisible - Toggle invisibility".to_string(),
            ]);
            if area_id == 20 || area_id == 35 {
                messages.push(
                    "Note: Additional LQ commands are available in the Live Quest area".to_string(),
                );
            }
        }
    }

    if flags.contains(CharacterFlags::GOD) {
        messages.extend([
            "=== GOD COMMANDS ===".to_string(),
            "== Movement & Teleportation ==".to_string(),
            "/goto <x> <y> [area] [mirror] - Teleport to coordinates".to_string(),
            "/gotolist - List all available goto locations".to_string(),
            "/gotosearch <term> - Search for goto locations".to_string(),
            "/office - Teleport to staff office in Aston".to_string(),
            "/summon <name> - Bring a player to your location".to_string(),
            "/summonall - Bring all online players to your location".to_string(),
            "== Item Management ==".to_string(),
            "/create <name> - Create an item by template name".to_string(),
            "/create_orb [type] [value] - Create an orb with specific properties".to_string(),
            "/itemmod <pos> <skill> <val> - Modify item in cursor (position, skill, value)"
                .to_string(),
            "/itemname <name> - Change name of item in cursor".to_string(),
            "/itemdesc <text> - Change description of item in cursor".to_string(),
            "/listitem <id> - Show detailed information about an item".to_string(),
            "== Player Modification ==".to_string(),
            "/ggold <amount> - Give yourself gold coins".to_string(),
            "/exp [name] [amount] - Give experience to a player".to_string(),
            "/milexp [name] [amount] - Give military experience to a player".to_string(),
            "/setskill <name> <skill> <value> - Set a player's skill level".to_string(),
            "/setlevel <level> - Set your character level".to_string(),
            "/heal - Fully restore your health".to_string(),
            "/setseyan <name> - Make a player a Seyan'Du".to_string(),
            "/rmdeath <name> - Remove one death from player's record".to_string(),
            "/setkarma <name> <value> - Set a player's karma".to_string(),
            "/toggleflag <name> <flag> - Toggles a flag for a character - use with caution"
                .to_string(),
            "/saves <amount> - Set number of saves".to_string(),
            "== Quest & Progress Management ==".to_string(),
            "/resetgift <name> <area> - Reset a player's gift status for an area".to_string(),
            "/fixit <name> - Fix a player's questlog".to_string(),
            "/questfix <name> - Fix quests for a player".to_string(),
            "/reset <name> - Reset a player's skills".to_string(),
            "/noarch <name> - Remove arch status from a player".to_string(),
            "/noprof <name> - Remove professions from a player".to_string(),
            "/questlog <name> - View a player's quest log".to_string(),
            "/labsolved <name> [lab] - Show or toggle lab completion status".to_string(),
            "== Achievements ==".to_string(),
            "/achgive <name> <id> - Award achievement to player".to_string(),
            "/achfix [name] - Recheck and award earned achievements".to_string(),
            "/achclear [name] - Clear all achievements (dev only)".to_string(),
            "/achsync [name] - Force sync achievements to client".to_string(),
            "== Account Management ==".to_string(),
            "/rename <oldname> <newname> - Rename a player character".to_string(),
            "/lockname <name> - Lock a character name".to_string(),
            "/unlockname <name> - Unlock a character name".to_string(),
            "/unpunish <name> <id> - Remove a punishment".to_string(),
            "== Character Information ==".to_string(),
            "/showppd <name> <ppd> - Show player persistent data".to_string(),
            "/showflags <name> - Show which flags are enabled on a character".to_string(),
            "/listchars - List all active characters".to_string(),
            "== God Status Management ==".to_string(),
            "/immortal - Toggle immortality status".to_string(),
            "/invisible - Toggle invisibility".to_string(),
            "/infrared - Toggle infrared vision".to_string(),
            "/xray - Toggle x-ray vision mode".to_string(),
            "/sprite <num> - Change your sprite".to_string(),
            "/color - Show your color values".to_string(),
            "/col1 <r> <g> <b> - Set your primary colors".to_string(),
            "/col2 <r> <g> <b> - Set your secondary colors".to_string(),
            "/col3 <r> <g> <b> - Set your tertiary colors".to_string(),
            "/dlight <value> - Override dynamic lighting".to_string(),
            "/showattack - Toggle attack display".to_string(),
            "/spy - Toggle spy mode (see all tells, clan, alliance, club, area, mirror chat)"
                .to_string(),
            "== Server Management ==".to_string(),
            "/shutdown <minutes> <downtime> - Schedule server shutdown".to_string(),
            "/respawn - Force respawn check".to_string(),
            "/setxmas <value> - Set Christmas special flag".to_string(),
            "/global - Display current global game settings".to_string(),
            "/checksanity - Run consistency checks on game data".to_string(),
            "/saveall - Force save of all player data".to_string(),
            "== Diagnostics & Monitoring ==".to_string(),
            "/memstats - Show memory usage statistics".to_string(),
            "/profinfo - Show profiling information".to_string(),
            "/poolstats - Show database connection pool statistics".to_string(),
            "/querystats - Show database query statistics".to_string(),
            "/prof - Show memory profiling information".to_string(),
            "== Game Settings Management ==".to_string(),
            "/setexpmod <value> - Set global experience modifier".to_string(),
            "/sethardcoreexpbonus <value> - Set hardcore experience bonus".to_string(),
            "/sethardcoremilexpbonus <value> - Set hardcore military exp bonus".to_string(),
            "/sethardcorekillexpbonus <value> - Set hardcore kill exp bonus".to_string(),
            "/setdecaytime <ticks> - Set item decay time".to_string(),
            "/setplayerbodytime <ticks> - Set player body decay time".to_string(),
            "/setnpcbodytime <ticks> - Set NPC body decay time".to_string(),
            "/setnpcbodytimearea32 <ticks> - Set area 32 NPC body decay time".to_string(),
            "/setrespawntime <ticks> - Set NPC respawn time".to_string(),
            "/setlagouttime <ticks> - Set lagout time".to_string(),
            "/setregentime <ticks> - Set regeneration time".to_string(),
            "/setsewerrespawntime <seconds> - Set sewer item respawn time".to_string(),
            "== Communication Settings ==".to_string(),
            "/sethollerdist <tiles> - Set holler distance".to_string(),
            "/setshoutdist <tiles> - Set shout distance".to_string(),
            "/setsaydist <tiles> - Set say distance".to_string(),
            "/setemotedist <tiles> - Set emote distance".to_string(),
            "/setquietsaydist <tiles> - Set quiet say distance".to_string(),
            "/setwhisperdist <tiles> - Set whisper distance".to_string(),
            "/sethollercost <points> - Set holler endurance cost".to_string(),
            "/setshoutcost <points> - Set shout endurance cost".to_string(),
            "== Special Item Settings ==".to_string(),
            "/setsplots <value> - Set special item probability 'lots'".to_string(),
            "/setspmany <value> - Set special item probability 'many'".to_string(),
            "/setspsome <value> - Set special item probability 'some'".to_string(),
            "/setspfew <value> - Set special item probability 'few'".to_string(),
            "/setsprare <value> - Set special item probability 'rare'".to_string(),
            "/setspultra <value> - Set special item probability 'ultra'".to_string(),
            "== Orb & Tunnel Management ==".to_string(),
            "/setorbrespawndays <days> - Set orb respawn time".to_string(),
            "/settunnelexpdivider <value> - Set tunnel exp base value divider".to_string(),
            "/settunnelmillexp <value> - Set tunnel mill exp base value".to_string(),
            "/changetunnel <name> <level> - Change player's tunnel level".to_string(),
            "/settunnel <name> <level> <amount> - Set completion amount for tunnel".to_string(),
            "/cleartunnel <name> <level> - Clear tunnel completion status".to_string(),
            "/solvetunnel <type> - Simulate solving the current tunnel".to_string(),
            "== Shrine & Dungeon Management ==".to_string(),
            "/setrd <name> <number> - Set continuity shrine number".to_string(),
            "/clearrd <name> <number> - Clear used shrine bits".to_string(),
            "/solverd <name> <number> - Mark non-continuity shrines as used".to_string(),
            "== Clan & Club Management ==".to_string(),
            "/killclan <nr> - Destroy a clan".to_string(),
            "/killclub <nr> - Destroy a club".to_string(),
            "/joinclan <nr> - Join a specific clan".to_string(),
            "/joinclub <nr> - Join a specific club".to_string(),
            "/setmaxjewelcount <value> - Set maximum clan jewel count".to_string(),
            "/clearclanlog <clan> - Clear the clan log for a specific clan".to_string(),
            "/setclanjewels <clan> <count> [log] - Set clan jewel count".to_string(),
            "/renclan <nr> <name> - Rename clan with specified number".to_string(),
            "/renclub <nr> <name> - Rename club with specified number".to_string(),
            "== Military Administration ==".to_string(),
            "/milinfo [name] - View a player's military data and mission status".to_string(),
            "/milpref <name> <type> <difficulty> - Set a player's mission preferences".to_string(),
            "/milreset [name] - Reset a player's mission cooldowns and advisor timers".to_string(),
            "/milpoints <name> <points> - Grant military points to a player".to_string(),
            "/milrec <name> <points> - Grant recommendation points to a player".to_string(),
            "/milstats - View statistics about the military system".to_string(),
            "/milsolve [name] [announce] - Complete a player's current military mission"
                .to_string(),
            "== Weather System Management ==".to_string(),
            "/setweather <type> <intensity> - Set global weather".to_string(),
            "/clearweather - Clear weather globally".to_string(),
            "/setareaweather <area> <type> - Set weather for specific area".to_string(),
            "== Player Status Management ==".to_string(),
            "/god <name> - Toggle god status for a player".to_string(),
            "/staff <name> - Toggle staff status for a player".to_string(),
            "/staffcode <name> <code> - Set staff code for a player".to_string(),
            "/qmaster <name> - Toggle quest master status".to_string(),
            "/emaster <name> - Toggle event master status".to_string(),
            "/devel <name> - Toggle developer status".to_string(),
            "/setsir <name> - Toggle sir/lady status".to_string(),
            "/hardcore <name> - Toggle hardcore mode for a player".to_string(),
            "== Miscellaneous God Commands ==".to_string(),
            "/laugh - Play laugh sound effect".to_string(),
            "/ls <name> <file> - List files for a player".to_string(),
            "/cat <name> <file> - View file content for a player".to_string(),
            "/lollipop <name> - Send lollipop to a player".to_string(),
            "/clearmerchantstores <id> - Reset a merchant's inventory".to_string(),
        ]);
    }

    messages.push(
        "Type a command without parameters to get more information in some cases.".to_string(),
    );

    Some(legacy_help_result(messages))
}

pub(crate) fn anti_cheat_help_lines() -> Vec<String> {
    [
        "--- Anti-Cheat Commands ---",
        "#achelp - Show this help",
        "#acstats - Global AC statistics",
        "#aclist - List online players with AC status",
        "#acsuspicious - List suspicious/flagged players",
        "--- Player Commands ---",
        "#acstatus <name> - Show player's AC status",
        "#achistory <name> - Show player's violation history",
        "#acsessions <name> - Show player's recent sessions",
        "#acviolations <name> - Show player's violations",
        "#acflag <name> - Flag player for review",
        "#acunflag <name> - Remove flagged status",
        "#actrust <name> - Mark player as trusted",
        "#acuntrust <name> - Remove trusted status",
        "#acreset <name> - Reset player's AC data (God)",
        "#acwarn <name> [reason] - Issue AC warning",
        "#acwatch <name> - Toggle detailed logging",
        "--- Multi-Account Detection ---",
        "#acsharedip <name> - Show accounts sharing IP",
        "#acsharedhw <name> - Show accounts sharing hardware",
        "--- Database Queries ---",
        "#achighrisk - Show high-risk players",
        "#aclookup <id> - Lookup by subscriber ID",
        "--- Signature Management ---",
        "#acsiglist - List known bad signatures",
        "#acsigadd <type> <value> <name> - Add signature (God)",
        "#acsigdel <id> - Delete signature (God)",
        "--- Maintenance ---",
        "#accleanup <days> - Cleanup old records (God)",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn macro_help_lines() -> Vec<String> {
    [
        "=== Macro Daemon Admin Commands ===",
        "/macrostats <player> - Show player's macro stats",
        "/macrohistory <player> - Show challenge history",
        "/macrolist - List all players with macro status",
        "/summonmacro <player> - Force summon (GOD only)",
        "/macroimmune <player> <mins> - Grant immunity (GOD only)",
        "/macrosuspicion <player> <amt> - Adjust suspicion (GOD)",
        "/macrokarma <player> <val> - Set karma 0-100 (GOD)",
        "/macrofailures <player> <n> - Set failure count (GOD)",
        "/macroreset <player> - Reset all macro stats (GOD)",
        "/macrohelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn pentagram_help_lines() -> Vec<String> {
    [
        "=== Pentagram Debug Commands (GOD) ===",
        "/pentinfo <player> - Show pentagram data",
        "/setpentcount <player> <n> - Set pent_cnt (run count)",
        "/setpentstatus <player> <0|1> - Set status",
        "/setpentbonus <player> <n> - Set bonus exp",
        "/resetpent <player> - Reset all pent data",
        "/penthelp - Show this help",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn apply_pk_hate_command(
    world: &mut World,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let Some(verb) = legacy_pk_command_verb(verb) else {
        return None;
    };
    let name = rest.trim();

    match verb {
        "playerkiller" => {
            let mut messages = Vec::new();
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };

            if character.flags.contains(CharacterFlags::PK) {
                if character.action != action::IDLE
                    || world
                        .tick
                        .0
                        .saturating_sub(u64::from(character.regen_ticker))
                        < TICKS_PER_SECOND * 3
                {
                    messages.push("Pant, pant. Too tired.".to_string());
                } else if player.pk_last_kill.saturating_add(60 * 60 * 24 * 28)
                    > realtime_seconds.min(u64::from(u32::MAX)) as u32
                {
                    let elapsed = realtime_seconds.saturating_sub(u64::from(player.pk_last_kill))
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    let remaining = (u64::from(player.pk_last_kill) + 60 * 60 * 24 * 28)
                        .saturating_sub(realtime_seconds)
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    messages.push(format!(
                        "You have killed {elapsed:.2} days ago, you need to wait {remaining:.2} more days."
                    ));
                } else {
                    character.flags.remove(CharacterFlags::PK);
                    player.pk_kills = 0;
                    player.pk_deaths = 0;
                    player.pk_last_kill = 0;
                    player.pk_last_death = 0;
                    player.pk_hate.clear();
                }
            } else if character.level < 10 {
                messages.push(
                    "Sorry, you may not become a player killer before reaching level 10."
                        .to_string(),
                );
            } else if !character.flags.contains(CharacterFlags::PAID) {
                messages.push("Sorry, only paying players may become player killers.".to_string());
            } else {
                messages.push(format!(
                    "Please take a moment to consider this decision. If another player kills you, he will be able to take all your belongings, or kill you over and over again. Do you really want this? Type: '/iwilldie {}' to confirm.",
                    character.id.0
                ));
            }

            let status = if character.flags.contains(CharacterFlags::PK) {
                "on"
            } else {
                "off"
            };
            messages.push(format!("PK is {status}."));
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "iwilldie" => {
            let mut messages = Vec::new();
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };

            if character.flags.contains(CharacterFlags::PK) {
                if character.action != action::IDLE
                    || world
                        .tick
                        .0
                        .saturating_sub(u64::from(character.regen_ticker))
                        < TICKS_PER_SECOND * 3
                {
                    messages.push("Pant, pant. Too tired.".to_string());
                } else if player.pk_last_kill.saturating_add(60 * 60 * 24 * 28)
                    > realtime_seconds.min(u64::from(u32::MAX)) as u32
                {
                    let elapsed = realtime_seconds.saturating_sub(u64::from(player.pk_last_kill))
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    let remaining = (u64::from(player.pk_last_kill) + 60 * 60 * 24 * 28)
                        .saturating_sub(realtime_seconds)
                        as f64
                        / (60.0 * 60.0 * 24.0);
                    messages.push(format!(
                        "You have killed {elapsed:.2} days ago, you need to wait {remaining:.2} more days."
                    ));
                } else {
                    character.flags.remove(CharacterFlags::PK);
                    player.pk_kills = 0;
                    player.pk_deaths = 0;
                    player.pk_last_kill = 0;
                    player.pk_last_death = 0;
                    player.pk_hate.clear();
                }
            } else if character.level < 10 {
                messages.push(
                    "Sorry, you may not become a player killer before reaching level 10."
                        .to_string(),
                );
            } else if !character.flags.contains(CharacterFlags::PAID) {
                messages.push("Sorry, only paying players may become player killers.".to_string());
            } else if legacy_atoi_prefix(name) != i64::from(character.id.0) {
                messages.push("Please type: '/playerkiller' first.".to_string());
            } else {
                player.pk_kills = 0;
                player.pk_deaths = 0;
                player.pk_last_kill = 0;
                player.pk_last_death = 0;
                player.pk_hate.clear();
                character.flags.insert(CharacterFlags::PK);
            }

            let status = if character.flags.contains(CharacterFlags::PK) {
                "on"
            } else {
                "off"
            };
            messages.push(format!("PK is {status}."));
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "listhate" => {
            if !world
                .characters
                .get(&character_id)
                .is_some_and(|character| {
                    character
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                })
            {
                return Some(KeyringCommandResult::default());
            }
            let messages = if !player.has_any_pk_hate() {
                vec!["List is empty.".to_string()]
            } else {
                player
                    .active_pk_hate_ids()
                    .map(|hated_id| {
                        let name = world
                            .characters
                            .get(&CharacterId(hated_id))
                            .map(|character| character.name.as_str())
                            .unwrap_or("Unknown");
                        format!("Hate: {name}")
                    })
                    .collect()
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                ..Default::default()
            })
        }
        "clearhate" => {
            if world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.flags.contains(CharacterFlags::PK))
            {
                player.pk_hate.clear();
            }
            Some(KeyringCommandResult {
                messages: Vec::new(),
                inventory_changed: false,
                ..Default::default()
            })
        }
        "hate" => {
            let Some(target_id) = find_online_character_by_name(world, name) else {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Sorry, no one by the name {name} around.")],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let can_add = match (
                world.characters.get(&character_id),
                world.characters.get(&target_id),
            ) {
                (Some(source), Some(target)) => pk_hate_prerequisites(source, target),
                _ => false,
            };
            if can_add {
                player.add_pk_hate(target_id.0);
                if let Some(source) = world.characters.get_mut(&character_id) {
                    source.flags.remove(CharacterFlags::LAG);
                }
                return Some(KeyringCommandResult {
                    name_refresh: vec![character_id, target_id],
                    ..Default::default()
                });
            }
            Some(KeyringCommandResult::default())
        }
        "nohate" => {
            let Some(target_id) = find_online_character_by_name(world, name) else {
                if let Ok(target_id) = name.parse::<u32>() {
                    let removed = world
                        .characters
                        .get(&character_id)
                        .is_some_and(|source| source.flags.contains(CharacterFlags::PK))
                        && player.remove_pk_hate(target_id);
                    let mut name_refresh = Vec::new();
                    if removed {
                        name_refresh.push(character_id);
                    }
                    return Some(KeyringCommandResult {
                        messages: if removed {
                            vec!["Removed from hate list".to_string()]
                        } else {
                            Vec::new()
                        },
                        inventory_changed: false,
                        name_refresh,
                        ..Default::default()
                    });
                }
                return Some(KeyringCommandResult {
                    messages: vec![format!("Sorry, no player by the name {name}.")],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(source) = world.characters.get(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            if !source.flags.contains(CharacterFlags::PK) {
                return Some(KeyringCommandResult::default());
            }
            let removed = player.remove_pk_hate(target_id.0);
            let mut name_refresh = Vec::new();
            let messages = if removed {
                name_refresh.push(character_id);
                name_refresh.push(target_id);
                let target_name = world
                    .characters
                    .get(&target_id)
                    .map(|character| character.name.as_str())
                    .unwrap_or(name);
                vec![format!("Removed {target_name} from hate list")]
            } else {
                Vec::new()
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: false,
                name_refresh,
                ..Default::default()
            })
        }
        _ => None,
    }
}

fn legacy_achievement_colored_line(color: &[u8], text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(color.len() + text.len() + COL_RESET.len());
    out.extend_from_slice(color);
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(COL_RESET);
    out
}

/// `strftime(..., "%Y-%m-%d", localtime(&timestamp))` (`achievement.c:1416`)
/// approximated in UTC (no `chrono`/timezone database dependency in this
/// workspace); `timestamp` is always a real unlock time (`0` is filtered
/// out by the caller before this runs), so the u64 cast is safe.
fn legacy_achievement_date(timestamp: i64) -> String {
    let (year, month, day) = civil_from_unix_seconds(timestamp.max(0) as u64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// C `achievement_list` (`achievement.c:1421-1452`).
pub(crate) fn legacy_achievement_list_lines(data: &AccountAchievements) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    lines.push(legacy_achievement_colored_line(
        COL_ORANGE,
        "=== Your Achievements ===",
    ));

    let mut unlocked_count = 0usize;
    for ty in AchievementType::ALL {
        let achievement = &data.achievements[ty as usize];
        if achievement.timestamp == 0 {
            continue;
        }
        let def = achievement_def(ty);
        let date = legacy_achievement_date(achievement.timestamp);
        let mut line = Vec::new();
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(format!("[+] {}", def.name).as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(
            format!(
                " - {} ({date} by {})",
                def.description, achievement.achieved_by
            )
            .as_bytes(),
        );
        lines.push(line);
        unlocked_count += 1;
    }

    if unlocked_count == 0 {
        lines.push(b"You haven't unlocked any achievements yet. Keep playing!".to_vec());
    } else {
        lines.push(legacy_achievement_colored_line(
            COL_DARK_GRAY,
            &format!("Unlocked: {unlocked_count}/{ACHIEVEMENT_TYPE_COUNT} achievements"),
        ));
    }
    lines
}

/// C `achievement_show_stats` (`achievement.c:1453-1499`).
pub(crate) fn legacy_achievement_stats_lines(stats: &AchievementStats) -> Vec<Vec<u8>> {
    let heading = |text: &str| legacy_achievement_colored_line(COL_ORANGE, text);
    let plain = |text: String| text.into_bytes();
    vec![
        heading("=== Achievement Statistics ==="),
        heading("Gathering:"),
        plain(format!("  Flowers picked: {}", stats.flowers_picked)),
        plain(format!("  Mushrooms picked: {}", stats.mushrooms_picked)),
        plain(format!("  Berries picked: {}", stats.berries_picked)),
        plain(format!("  Potions brewed: {}", stats.potions_brewed)),
        heading("Combat:"),
        plain(format!("  Enemies killed: {}", stats.enemies_killed)),
        plain(format!("  Demons defeated: {}", stats.demons_defeated)),
        plain(format!(
            "    Earth: {}, Fire: {}, Ice: {}, Hell: {}",
            stats.demons_per_area[PentArea::Earth as usize],
            stats.demons_per_area[PentArea::Fire as usize],
            stats.demons_per_area[PentArea::Ice as usize],
            stats.demons_per_area[PentArea::Hell as usize],
        )),
        plain(format!("  PvP kills: {}", stats.pvp_kills)),
        heading("Pentagram:"),
        plain(format!("  Pents solved: {}", stats.pents_solved)),
        plain(format!(
            "    Earth: {}, Fire: {}, Ice: {}, Hell: {}",
            stats.pents_per_area[PentArea::Earth as usize],
            stats.pents_per_area[PentArea::Fire as usize],
            stats.pents_per_area[PentArea::Ice as usize],
            stats.pents_per_area[PentArea::Hell as usize],
        )),
        heading("Collection:"),
        plain(format!("  Chests opened: {}", stats.chests_opened)),
        plain(format!(
            "  Stones: Earth {}, Fire {}, Ice {}",
            stats.earth_stones, stats.fire_stones, stats.ice_stones,
        )),
        heading("Mining:"),
        plain(format!("  Silver mined: {}", stats.silver_mined)),
        plain(format!("  Gold mined: {}", stats.gold_mined)),
        heading("Missions:"),
        plain(format!("  Military missions: {}", stats.military_missions)),
        plain(format!("  Tunnel levels: {}", stats.tunnel_levels)),
    ]
}

/// C `achievement_award`'s congratulations text (`achievement.c:621-624`),
/// only ever shown with `show_congrats = 1`, which in the whole codebase
/// is exclusively the `/achgive` admin path (`command.c:9126`) - every
/// other C call site either passes `0` (silent, e.g. `achievement_fix_
/// all`) or is one of the not-yet-wired gameplay call sites tracked by
/// this task's "REMAINING" note. `achievement_send_to_client`'s
/// `SV_ACH_UNLOCK` packet (`achievement_unlock_payload` here) is sent
/// unconditionally in C regardless of `show_congrats` and is built by the
/// caller separately.
fn legacy_achievement_unlock_congrats_lines(name: &str, description: &str) -> [Vec<u8>; 2] {
    let mut line1 = Vec::new();
    line1.extend_from_slice(COL_LIGHT_GREEN);
    line1.extend_from_slice(b"Achievement Unlocked: ");
    line1.extend_from_slice(COL_ORANGE);
    line1.extend_from_slice(name.as_bytes());
    line1.extend_from_slice(COL_RESET);
    let line2 = legacy_achievement_colored_line(COL_DARK_GRAY, description);
    [line1, line2]
}

/// C `achievement_list`/`achievement_show_stats`/`achievement_fix_all`/
/// `achievement_clear_all`/`achievement_sync_all` plus the `/achgive`
/// admin-only give command (`achievement.c:1421-1810`, dispatched from
/// `command.c:9076-9227`). `/achievements`/`/achstats` are player-
/// accessible and always operate on the caller (no name argument in C);
/// the remaining four verbs are `CF_GOD`-gated and accept an optional
/// target name defaulting to the caller, mirroring `/reset`'s pattern
/// (`commands_admin.rs`'s `apply_admin_character_command`, `lower ==
/// "reset"` branch).
pub(crate) async fn apply_achievement_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    command: &str,
    now: i64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 6 && "achievements".starts_with(&lower) {
        let player = runtime.player_for_character(character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: legacy_achievement_list_lines(&player.achievement_data),
            ..Default::default()
        });
    }

    if lower.len() >= 8 && "achstats".starts_with(&lower) {
        let player = runtime.player_for_character(character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: legacy_achievement_stats_lines(&player.achievement_stats),
            ..Default::default()
        });
    }

    let is_give = lower.len() >= 7 && "achgive".starts_with(&lower);
    let is_fix = lower.len() >= 6 && "achfix".starts_with(&lower);
    let is_clear = lower.len() >= 8 && "achclear".starts_with(&lower);
    let is_sync = lower.len() >= 7 && "achsync".starts_with(&lower);
    if !is_give && !is_fix && !is_clear && !is_sync {
        return None;
    }
    let caller = world.characters.get(&character_id)?;
    if !caller.flags.contains(CharacterFlags::GOD) {
        return None;
    }

    if is_give {
        let rest = rest.trim_start();
        let (name, id_text) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /achgive <name> <achievement_id>".to_string(),
                    format!("Achievement IDs: 0-{}", ACHIEVEMENT_TYPE_COUNT - 1),
                ],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found.")],
                ..Default::default()
            });
        };
        let ach_id = legacy_atoi_prefix(id_text.trim_start());
        if ach_id < 0 || ach_id as usize >= ACHIEVEMENT_TYPE_COUNT {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Invalid achievement ID. Range: 0-{}",
                    ACHIEVEMENT_TYPE_COUNT - 1
                )],
                ..Default::default()
            });
        }
        let ty = AchievementType::ALL[ach_id as usize];
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let newly_unlocked =
            runtime
                .player_for_character_mut(target_id)
                .is_some_and(|target_player| {
                    target_player.achievement_data.award(ty, &target_name, now)
                });
        if newly_unlocked {
            let def = achievement_def(ty);
            let mut payloads = vec![achievement_unlock_payload(ty, now)];
            for line in legacy_achievement_unlock_congrats_lines(def.name, def.description) {
                payloads.push(ugaris_protocol::packet::system_text_bytes(&line));
            }
            send_raw_payloads_to_character(runtime, target_id, &payloads);
            record_achievement_firsts_and_announce(
                world,
                repository,
                target_id,
                &target_name,
                &[ty],
            )
            .await;
        }
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievement {ach_id} awarded to {target_name}.")],
            ..Default::default()
        });
    }

    // achfix / achclear / achsync share the "optional name, defaults to
    // self" target-resolution idiom (`command.c:9135-9227`).
    let name = rest.trim();
    let target_id = if name.is_empty() {
        character_id
    } else {
        match find_online_character_by_name(world, name) {
            Some(id) => id,
            None => {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Player '{name}' not found.")],
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

    if is_fix {
        let Some(target_character) = world.characters.get(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found.")],
                ..Default::default()
            });
        };
        let level = target_character.level as i32;
        let is_hardcore = target_character.flags.contains(CharacterFlags::HARDCORE);
        let is_won = target_character.flags.contains(CharacterFlags::WON);
        let professions = target_character.professions.clone();
        let mut unlocked = Vec::new();
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            unlocked.extend(check_level(
                &mut target_player.achievement_data,
                level,
                is_hardcore,
                &target_name,
                now,
            ));
            if is_won
                && target_player.achievement_data.award(
                    AchievementType::Ladykiller,
                    &target_name,
                    now,
                )
            {
                unlocked.push(AchievementType::Ladykiller);
            }
            unlocked.extend(fix_all_stat_thresholds(
                &mut target_player.achievement_data,
                &target_player.achievement_stats,
                &target_name,
                now,
            ));
            for prof_type in 0..=10i32 {
                if let Some(&prof_level) = professions.get(prof_type as usize) {
                    if prof_level > 0 {
                        unlocked.extend(check_profession(
                            &mut target_player.achievement_data,
                            prof_type,
                            prof_level as i32,
                            &target_name,
                            now,
                        ));
                    }
                }
            }
        }
        let payloads: Vec<_> = unlocked
            .iter()
            .map(|&ty| achievement_unlock_payload(ty, now))
            .collect();
        send_raw_payloads_to_character(runtime, target_id, &payloads);
        record_achievement_firsts_and_announce(
            world,
            repository,
            target_id,
            &target_name,
            &unlocked,
        )
        .await;
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievements fixed for {target_name}.")],
            ..Default::default()
        });
    }

    if is_clear {
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            clear_all(
                &mut target_player.achievement_data,
                &mut target_player.achievement_stats,
            );
        }
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievements cleared for {target_name}.")],
            ..Default::default()
        });
    }

    // is_sync
    let payloads = runtime
        .player_for_character(target_id)
        .map(|target_player| {
            achievement_sync_payloads(
                &target_player.achievement_data,
                &target_player.achievement_stats,
            )
        })
        .unwrap_or_default();
    send_raw_payloads_to_character(runtime, target_id, &payloads);
    Some(KeyringCommandResult {
        messages: vec![format!("Achievements synced to client for {target_name}.")],
        ..Default::default()
    })
}

/// Sends already-built protocol packets (e.g. `SV_ACH_UNLOCK`/`SV_ACH_SYNC`
/// or a pre-colored `SV_TEXT` congrats line) directly to every live
/// session for `character_id`, bypassing the `command_feedback_bytes`
/// pipeline's `system_text_bytes` re-wrap (which is only correct for raw,
/// not-yet-packetized text) - mirrors the tick loop's own deferred-
/// achievement-sync send pattern (`main.rs`'s `DEFERRED_ACHIEVEMENTS`
/// sweep).
fn send_raw_payloads_to_character(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    payloads: &[bytes::BytesMut],
) {
    for payload in payloads {
        for (session_id, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
}
