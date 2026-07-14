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

/// C's `isalpha(*ptr) || isdigit(*ptr)` scan loop (e.g.
/// `cmd_showppd`'s `ppdName` parse, `src/system/command.c:299-300`).
pub(crate) fn take_legacy_alnum_name(text: &str) -> (&str, &str) {
    let end = text
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_alphanumeric()).then_some(index))
        .unwrap_or(text.len());
    text.split_at(end)
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
