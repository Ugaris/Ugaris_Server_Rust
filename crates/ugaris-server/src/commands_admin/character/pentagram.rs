use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_pentagram(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // Pentagram debug commands (`command.c:10416-10465`, all `CF_GOD`-
    // gated, `cmdcmp` minlen == full word length so every name below is an
    // exact-word match, no abbreviations). `pent_find_player` (`command.c
    // :1150-1160`) has no self-fallback, unlike `/milinfo`'s family - a
    // player name is always required, and "not found" uses its own
    // distinct message text rather than the `/milinfo`-family's "Sorry, no
    // one by the name ... around."
    if lower == "pentinfo" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let target_arg = rest.trim_start();
        if target_arg.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /pentinfo <player>".to_string()],
                ..Default::default()
            }));
        }
        let (name, _) = take_legacy_alpha_name(target_arg);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            }));
        };
        let pent = &player.pentagram_debug;
        let mut messages = vec![
            format!("=== Pentagram Data for {target_name} ==="),
            format!("Status: {} (0=normal, 1=5-of-color)", pent.status),
            format!("Pent Count: {} (current run)", pent.pent_cnt),
            format!("Lucky Pents: {} (this solve)", pent.lucky_pents_this_solve),
            format!("Bonus: {} exp", pent.bonus),
        ];
        let active = pent.pent_it.iter().filter(|&&it| it != 0).count();
        messages.push(format!("Active Pentagrams: {active}/6"));
        const PENT_COLOR_NAMES: [&str; 4] = ["none", "red", "green", "blue"];
        for i in 0..6 {
            if pent.pent_it[i] != 0 {
                let color = usize::try_from(pent.pent_color[i])
                    .ok()
                    .and_then(|c| PENT_COLOR_NAMES.get(c))
                    .copied()
                    .unwrap_or("?");
                messages.push(format!(
                    "  [{i}] color={color} value={} worth={}",
                    pent.pent_value[i], pent.pent_worth[i]
                ));
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    if lower == "setpentcount" || lower == "setpentstatus" || lower == "setpentbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let usage = match lower {
            "setpentcount" => "Usage: /setpentcount <player> <count>",
            "setpentstatus" => "Usage: /setpentstatus <player> <0|1>",
            _ => "Usage: /setpentbonus <player> <bonus>",
        };
        let Some((name, value)) = parse_pent_name_and_int(rest) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![usage.to_string()],
                ..Default::default()
            }));
        };
        let Some(target_id) = find_online_character_by_name(world, &name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            }));
        };
        let message = match lower {
            "setpentcount" => {
                let old = player.pentagram_debug.pent_cnt;
                player.pentagram_debug.pent_cnt = value;
                format!("Set pent_cnt for {target_name}: {old} -> {value}")
            }
            "setpentstatus" => {
                let old = player.pentagram_debug.status;
                player.pentagram_debug.status = value;
                format!("Set pent status for {target_name}: {old} -> {value}")
            }
            _ => {
                let old = player.pentagram_debug.bonus;
                player.pentagram_debug.bonus = value;
                format!("Set pent bonus for {target_name}: {old} -> {value}")
            }
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        }));
    }

    if lower == "resetpent" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let target_arg = rest.trim_start();
        if target_arg.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /resetpent <player>".to_string()],
                ..Default::default()
            }));
        }
        let (name, _) = take_legacy_alpha_name(target_arg);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            }));
        };
        player.pentagram_debug = PentagramDebugData::default();
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("Reset all pentagram data for {target_name}.")],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

/// C `sscanf(args, "%79s %d", name, &value) != 2` (`command.c`'s
/// `pent_cmd_setcount`/`pent_cmd_setstatus`/`pent_cmd_setbonus`): the
/// first whitespace-delimited token is the player name (no length cap
/// enforced here, matching how the rest of this file's admin commands
/// already treat `take_legacy_alpha_name` targets - real character names
/// never approach the C buffer's 79-byte cap), the second must start with
/// an optional sign followed by at least one digit or the whole match
/// fails (mirroring `sscanf`'s requirement of exactly 2 successful
/// conversions, not `legacy_atoi_prefix`'s silent-zero-on-no-digit
/// fallback used by the self-fallback command families elsewhere in this
/// file).
pub(crate) fn parse_pent_name_and_int(rest: &str) -> Option<(String, i32)> {
    let rest = rest.trim_start();
    let mut split = rest.splitn(2, char::is_whitespace);
    let name = split.next().unwrap_or("");
    if name.is_empty() {
        return None;
    }
    let remainder = split.next().unwrap_or("").trim_start();
    let after_sign = remainder
        .strip_prefix('-')
        .or_else(|| remainder.strip_prefix('+'))
        .unwrap_or(remainder);
    if !after_sign
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_digit)
    {
        return None;
    }
    Some((name.to_string(), legacy_atoi_prefix(remainder) as i32))
}
