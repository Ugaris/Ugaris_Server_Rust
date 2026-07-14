use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_macro_ac(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // Macro daemon admin/debug commands (`command.c:660-1123`). `/macrostats`/
    // `/macrohistory`/`/macrolist` are `CF_GOD|CF_STAFF`-gated; `/summonmacro`/
    // `/macroimmune`/`/macrosuspicion`/`/macrokarma`/`/macrofailures`/
    // `/macroreset` are `CF_GOD`-only. Every `cmdcmp` minlen below equals the
    // full word length, so all are exact-word matches, no abbreviations
    // (`/macrohelp` is the tenth and final member of this family - already
    // ported, see `commands_player.rs::macro_help_lines`). `macro_find_player`
    // (`command.c:650-658`) is a `CF_PLAYER`-only online name scan, unlike
    // `find_online_character_by_name`'s no-filter scan used by most of this
    // file's other by-name debug commands - reproduced below as
    // `find_online_macro_player` rather than widening that shared helper's
    // contract. The real macro-daemon detection engine (`macro_driver`,
    // `src/module/base.c:802-1243`: activity tracking, challenge generation
    // and checking, reward/failure handling, cross-server "challenge room"
    // teleport) is NOT ported - see the doc comment on
    // `PlayerRuntime::macro_ppd` - so these commands only read/write the raw
    // PPD storage a future driver port would consume; add a dedicated
    // `PORTING_TODO.md` task for that engine before relying on any of this
    // having gameplay effect. `/macrostats`'s C sibling also prints a live
    // "Anticheat Bot Score" line from `ac_anomaly_get_bot_score` - skipped
    // here since it would require wiring this command into the async
    // `#acsessions`-style DB-lookup queue for a single optional line; a
    // future iteration closing that gap should reuse
    // `AntiCheatRepository`'s existing session/bot-score plumbing rather
    // than adding a new one.
    if lower == "macrostats" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macrostats <player>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let ppd = &player.macro_ppd;
        let now = world.date.realtime;
        let mut messages = vec![
            format!("=== Macro Daemon Stats: {target_name} ==="),
            format!("Karma: {} | Suspicion: {}", ppd.karma, ppd.suspicion),
            format!(
                "Challenges - Passed: {} | Failed: {} | Consecutive Fails: {}",
                ppd.total_passed, ppd.total_failed, ppd.challenge_failures
            ),
            "Last Activity:".to_string(),
            format!(
                "  Exp Gain: {} | Combat: {} | Gold Change: {}",
                macro_activity_ago(ppd.last_exp_gain, now),
                macro_activity_ago(ppd.last_combat, now),
                macro_activity_ago(ppd.last_gold_change, now),
            ),
        ];
        if ppd.immune_until > now {
            let remaining = ppd.immune_until - now;
            messages.push(format!(
                "IMMUNE for {} minutes (granted by ID {})",
                remaining / 60,
                ppd.immune_by
            ));
        }
        if ppd.force_summon {
            messages.push(format!(
                "FORCE SUMMON PENDING (requested by ID {})",
                ppd.summoned_by
            ));
        }
        if ppd.in_challenge_room {
            messages.push("Currently in challenge room".to_string());
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    if lower == "macrohistory" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macrohistory <player>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let ppd = &player.macro_ppd;
        let mut messages = vec![format!("=== Challenge History: {target_name} ===")];
        if ppd.history_count == 0 {
            messages.push("No challenge history recorded.".to_string());
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages,
                ..Default::default()
            }));
        }
        let now = world.date.realtime;
        let count = ppd.history_count.min(MACRO_HISTORY_SIZE as i32);
        for i in 0..count {
            let idx = (ppd.history_index - 1 - i).rem_euclid(MACRO_HISTORY_SIZE as i32) as usize;
            let entry = ppd.history[idx];
            let ago_minutes = (now - entry.timestamp) / 60;
            let result = if entry.passed { "PASS" } else { "FAIL" };
            let type_name = macro_challenge_type_name(entry.challenge_type);
            if entry.passed && entry.response_time > 0 {
                messages.push(format!(
                    "{}. [{type_name}] {result} - {}s response ({ago_minutes} min ago)",
                    i + 1,
                    entry.response_time
                ));
            } else {
                messages.push(format!(
                    "{}. [{type_name}] {result} ({ago_minutes} min ago)",
                    i + 1
                ));
            }
        }
        messages.push(format!("Total challenges: {}", ppd.history_count));
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    if lower == "summonmacro" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let caller_id = caller.id.0;
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /summonmacro <player>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        player.macro_ppd.force_summon = true;
        player.macro_ppd.summoned_by = caller_id;
        debug!(target: "client_log", name = %target_name, id = target_id.0, "macro_admin summon requested");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Macro daemon will summon {target_name} on next check."
            )],
            ..Default::default()
        }));
    }

    if lower == "macroimmune" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let caller_id = caller.id.0;
        if rest.trim_start().is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macroimmune <player> <minutes>".to_string(),
                    "Use 0 minutes to remove immunity.".to_string(),
                ],
                ..Default::default()
            }));
        }
        let Some((name, minutes)) = parse_pent_name_and_int(rest) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macroimmune <player> <minutes>".to_string()],
                ..Default::default()
            }));
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
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
        let now = world.date.realtime;
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let message = if minutes <= 0 {
            player.macro_ppd.immune_until = 0;
            player.macro_ppd.immune_by = 0;
            format!("Removed macro daemon immunity from {target_name}.")
        } else {
            player.macro_ppd.immune_until = now + i64::from(minutes) * 60;
            player.macro_ppd.immune_by = caller_id;
            format!("Granted {target_name} immunity from macro daemon for {minutes} minutes.")
        };
        debug!(target: "client_log", name = %target_name, id = target_id.0, minutes, "macro_admin set immunity");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        }));
    }

    if lower == "macrosuspicion" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        if rest.trim_start().is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macrosuspicion <player> <amount>".to_string(),
                    "Use negative amount to reduce suspicion.".to_string(),
                ],
                ..Default::default()
            }));
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macrosuspicion <player> <amount>".to_string()],
                ..Default::default()
            }));
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let old_value = player.macro_ppd.suspicion;
        player.macro_ppd.suspicion = (old_value + amount).clamp(0, 100);
        let new_value = player.macro_ppd.suspicion;
        debug!(target: "client_log", name = %target_name, id = target_id.0, old_value, new_value, "macro_admin adjusted suspicion");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "{target_name} suspicion: {old_value} -> {new_value}"
            )],
            ..Default::default()
        }));
    }

    if lower == "macrokarma" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        if rest.trim_start().is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macrokarma <player> <value>".to_string(),
                    "Sets karma to specified value (0-100).".to_string(),
                ],
                ..Default::default()
            }));
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macrokarma <player> <value>".to_string()],
                ..Default::default()
            }));
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let old_value = player.macro_ppd.karma;
        player.macro_ppd.karma = amount.clamp(0, 100);
        let new_value = player.macro_ppd.karma;
        debug!(target: "client_log", name = %target_name, id = target_id.0, old_value, new_value, "macro_admin set karma");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("{target_name} karma: {old_value} -> {new_value}")],
            ..Default::default()
        }));
    }

    if lower == "macrofailures" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let usage = "Usage: /macrofailures <player> <count>".to_string();
        if rest.trim_start().is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![usage],
                ..Default::default()
            }));
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![usage],
                ..Default::default()
            }));
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
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
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let old_value = player.macro_ppd.challenge_failures;
        player.macro_ppd.challenge_failures = amount.max(0);
        let new_value = player.macro_ppd.challenge_failures;
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "{target_name} consecutive failures: {old_value} -> {new_value}"
            )],
            ..Default::default()
        }));
    }

    if lower == "macroreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /macroreset <player>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
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
        let now = world.date.realtime;
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            }));
        };
        let ppd = &mut player.macro_ppd;
        ppd.karma = 50;
        ppd.suspicion = 0;
        ppd.challenge_failures = 0;
        ppd.total_passed = 0;
        ppd.total_failed = 0;
        ppd.history_count = 0;
        ppd.history_index = 0;
        ppd.immune_until = 0;
        ppd.immune_by = 0;
        ppd.force_summon = false;
        ppd.summoned_by = 0;
        ppd.nextcheck = now + 60 * 5;
        debug!(target: "client_log", name = %target_name, id = target_id.0, "macro_admin reset stats");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("Reset all macro stats for {target_name}.")],
            ..Default::default()
        }));
    }

    if lower == "macrolist" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let now = world.date.realtime;
        let mut players: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        players.sort_by_key(|id| id.0);
        let mut messages = vec![
            "=== Online Players - Macro Status ===".to_string(),
            "Name                 Karma  Susp  Pass/Fail  Status".to_string(),
            "---------------------------------------------------".to_string(),
        ];
        let mut count = 0;
        for player_id in players {
            let Some(player) = runtime.player_for_character(player_id) else {
                continue;
            };
            let name = world
                .characters
                .get(&player_id)
                .map(|character| character.name.clone())
                .unwrap_or_default();
            let ppd = &player.macro_ppd;
            let status = if ppd.in_challenge_room {
                "CHALLENGED"
            } else if ppd.immune_until > now {
                "IMMUNE"
            } else if ppd.force_summon {
                "PENDING"
            } else if ppd.suspicion >= 50 {
                "SUSPICIOUS"
            } else {
                "OK"
            };
            messages.push(format!(
                "{name:<20} {:>5}  {:>4}  {:>4}/{:<4}  {status}",
                ppd.karma, ppd.suspicion, ppd.total_passed, ppd.total_failed
            ));
            count += 1;
        }
        messages.push("---------------------------------------------------".to_string());
        messages.push(format!("Total: {count} players"));
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

/// C `macro_find_player` (`command.c:650-658`): an online, `CF_PLAYER`-only,
/// case-insensitive name scan - the macro-daemon admin commands' own
/// by-name lookup, distinct from `find_online_character_by_name`'s
/// no-flag-filter scan used elsewhere in this file.
pub(crate) fn find_online_macro_player(world: &World, name: &str) -> Option<CharacterId> {
    world
        .characters
        .values()
        .find(|character| {
            character.flags.contains(CharacterFlags::PLAYER)
                && character.name.eq_ignore_ascii_case(name)
        })
        .map(|character| character.id)
}

/// C `macro_cmd_stats`'s inline "%ds ago"/"never" formatting
/// (`command.c:703-719`), extracted into a shared helper since the same
/// three-field shape repeats for exp/combat/gold.
pub(crate) fn macro_activity_ago(last: i64, now: i64) -> String {
    if last > 0 {
        format!("{}s ago", now - last)
    } else {
        "never".to_string()
    }
}

/// C `macro_challenge_type_name` (`command.c:631-644`).
pub(crate) fn macro_challenge_type_name(challenge_type: i32) -> &'static str {
    match challenge_type {
        0 => "Math",
        1 => "Type Word",
        2 => "Reverse",
        3 => "Multiple Choice",
        _ => "Unknown",
    }
}
