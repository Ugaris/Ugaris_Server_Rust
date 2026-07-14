use super::*;

/// A single row of C's static `struct demon_lord demon_lords[]` table
/// (`command.c:1358-1382`), copied digit-for-digit and letter-for-letter:
/// 48 entries (`NUM_DEMON_LORDS`), `level` ascending, `class` the NPC class ID
/// `PlayerRuntime::has_first_kill` bit-tests against, `name` the exact
/// display string (`"Earth/Fire/Ice Demon Lord <level>"`).
struct DemonLordEntry {
    level: i32,
    class: i32,
    name: &'static str,
}

const DEMON_LORDS: &[DemonLordEntry] = &[
    DemonLordEntry {
        level: 8,
        class: 258,
        name: "Earth Demon Lord 8",
    },
    DemonLordEntry {
        level: 10,
        class: 259,
        name: "Earth Demon Lord 10",
    },
    DemonLordEntry {
        level: 12,
        class: 260,
        name: "Earth Demon Lord 12",
    },
    DemonLordEntry {
        level: 14,
        class: 261,
        name: "Earth Demon Lord 14",
    },
    DemonLordEntry {
        level: 16,
        class: 262,
        name: "Earth Demon Lord 16",
    },
    DemonLordEntry {
        level: 18,
        class: 263,
        name: "Earth Demon Lord 18",
    },
    DemonLordEntry {
        level: 20,
        class: 264,
        name: "Earth Demon Lord 20",
    },
    DemonLordEntry {
        level: 22,
        class: 265,
        name: "Earth Demon Lord 22",
    },
    DemonLordEntry {
        level: 24,
        class: 266,
        name: "Earth Demon Lord 24",
    },
    DemonLordEntry {
        level: 26,
        class: 267,
        name: "Earth Demon Lord 26",
    },
    DemonLordEntry {
        level: 28,
        class: 268,
        name: "Earth Demon Lord 28",
    },
    DemonLordEntry {
        level: 30,
        class: 269,
        name: "Earth Demon Lord 30",
    },
    DemonLordEntry {
        level: 32,
        class: 270,
        name: "Earth Demon Lord 32",
    },
    DemonLordEntry {
        level: 34,
        class: 271,
        name: "Earth Demon Lord 34",
    },
    DemonLordEntry {
        level: 36,
        class: 272,
        name: "Earth Demon Lord 36",
    },
    DemonLordEntry {
        level: 38,
        class: 273,
        name: "Earth Demon Lord 38",
    },
    DemonLordEntry {
        level: 40,
        class: 274,
        name: "Fire Demon Lord 40",
    },
    DemonLordEntry {
        level: 42,
        class: 275,
        name: "Fire Demon Lord 42",
    },
    DemonLordEntry {
        level: 44,
        class: 276,
        name: "Fire Demon Lord 44",
    },
    DemonLordEntry {
        level: 46,
        class: 277,
        name: "Fire Demon Lord 46",
    },
    DemonLordEntry {
        level: 48,
        class: 278,
        name: "Fire Demon Lord 48",
    },
    DemonLordEntry {
        level: 50,
        class: 279,
        name: "Fire Demon Lord 50",
    },
    DemonLordEntry {
        level: 52,
        class: 280,
        name: "Fire Demon Lord 52",
    },
    DemonLordEntry {
        level: 54,
        class: 281,
        name: "Fire Demon Lord 54",
    },
    DemonLordEntry {
        level: 56,
        class: 282,
        name: "Fire Demon Lord 56",
    },
    DemonLordEntry {
        level: 58,
        class: 283,
        name: "Fire Demon Lord 58",
    },
    DemonLordEntry {
        level: 60,
        class: 284,
        name: "Fire Demon Lord 60",
    },
    DemonLordEntry {
        level: 62,
        class: 285,
        name: "Fire Demon Lord 62",
    },
    DemonLordEntry {
        level: 64,
        class: 286,
        name: "Fire Demon Lord 64",
    },
    DemonLordEntry {
        level: 66,
        class: 287,
        name: "Fire Demon Lord 66",
    },
    DemonLordEntry {
        level: 68,
        class: 288,
        name: "Fire Demon Lord 68",
    },
    DemonLordEntry {
        level: 70,
        class: 289,
        name: "Fire Demon Lord 70",
    },
    DemonLordEntry {
        level: 72,
        class: 290,
        name: "Ice Demon Lord 72",
    },
    DemonLordEntry {
        level: 74,
        class: 291,
        name: "Ice Demon Lord 74",
    },
    DemonLordEntry {
        level: 76,
        class: 292,
        name: "Ice Demon Lord 76",
    },
    DemonLordEntry {
        level: 78,
        class: 293,
        name: "Ice Demon Lord 78",
    },
    DemonLordEntry {
        level: 80,
        class: 294,
        name: "Ice Demon Lord 80",
    },
    DemonLordEntry {
        level: 82,
        class: 295,
        name: "Ice Demon Lord 82",
    },
    DemonLordEntry {
        level: 84,
        class: 296,
        name: "Ice Demon Lord 84",
    },
    DemonLordEntry {
        level: 86,
        class: 297,
        name: "Ice Demon Lord 86",
    },
    DemonLordEntry {
        level: 88,
        class: 298,
        name: "Ice Demon Lord 88",
    },
    DemonLordEntry {
        level: 90,
        class: 299,
        name: "Ice Demon Lord 90",
    },
    DemonLordEntry {
        level: 92,
        class: 300,
        name: "Ice Demon Lord 92",
    },
    DemonLordEntry {
        level: 94,
        class: 301,
        name: "Ice Demon Lord 94",
    },
    DemonLordEntry {
        level: 96,
        class: 302,
        name: "Ice Demon Lord 96",
    },
    DemonLordEntry {
        level: 98,
        class: 303,
        name: "Ice Demon Lord 98",
    },
    DemonLordEntry {
        level: 100,
        class: 304,
        name: "Ice Demon Lord 100",
    },
    DemonLordEntry {
        level: 102,
        class: 305,
        name: "Ice Demon Lord 102",
    },
];

/// C `cmd_demonlords` (`command.c:1394-1461`, dispatched unconditionally -
/// no permission flag gate - from `command.c:8938-8946`). Reports "Thou
/// hast not yet vanquished any demon lords..." (`COL_LIGHT_RED`) if the
/// caller's `first_kill_ppd` bitmask has none of the 48 demon-lord classes
/// set; otherwise walks the level-ascending table, stopping once a lord's
/// level exceeds `player_level + 10` (C: `if (demon_lords[i].level >
/// player_level + 10) break;`), coloring each name `COL_VIOLET` if killed
/// or `COL_LIGHT_RED` if not, and grouping three names per line - matching
/// C's `demon_buf` accumulation, which appends a trailing `\n` *inside*
/// the same message every third entry (`strncat(demon_buf, "\n", ...)`
/// before the `log_char` that flushes the group) and, if the final group
/// has fewer than three entries, flushes it after the loop with no
/// trailing newline. Returns `None` (falls through, matching every other
/// self-query command in this file) only if the caller has no live
/// `PlayerRuntime` - never actually reachable since the command dispatcher
/// only runs for connected players, mirroring C's `if (!ppd) { log_char(
/// ...); return 1; }` guard, which is likewise practically unreachable
/// (`set_data` on an always-valid `DRD_FIRSTKILL_PPD` slot).
pub(crate) fn apply_demonlords_command(
    world: &World,
    runtime: &ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("demonlords") {
        return None;
    }

    let player = runtime.player_for_character(character_id)?;
    let player_level = world.characters.get(&character_id)?.level as i32;

    let any_killed = DEMON_LORDS
        .iter()
        .any(|lord| player.has_first_kill(lord.class));
    if !any_killed {
        return Some(KeyringCommandResult {
            message_bytes: vec![legacy_achievement_colored_line(
                COL_LIGHT_RED,
                "Thou hast not yet vanquished any demon lords, brave adventurer.",
            )],
            ..Default::default()
        });
    }

    let mut lines = vec![legacy_achievement_colored_line(
        COL_ORANGE,
        "Demon Lords status:",
    )];
    let mut row: Vec<u8> = Vec::new();
    let mut shown_in_row = 0u32;
    for lord in DEMON_LORDS {
        if lord.level > player_level + 10 {
            break;
        }
        let color = if player.has_first_kill(lord.class) {
            COL_VIOLET
        } else {
            COL_LIGHT_RED
        };
        row.extend_from_slice(color);
        row.extend_from_slice(lord.name.as_bytes());
        row.extend_from_slice(COL_RESET);
        row.push(b' ');
        shown_in_row += 1;
        if shown_in_row == 3 {
            row.push(b'\n');
            lines.push(std::mem::take(&mut row));
            shown_in_row = 0;
        }
    }
    if !row.is_empty() {
        lines.push(row);
    }

    Some(KeyringCommandResult {
        message_bytes: lines,
        ..Default::default()
    })
}

/// C `get_area_name` (`command.c:1476-1494`): a handful of named areas plus
/// a `"Area %d"` fallback for everything else.
fn legacy_area_name(area_id: u32) -> String {
    match area_id {
        1 => "Cameron".to_string(),
        3 => "Aston".to_string(),
        17 => "Exkhordon".to_string(),
        18 => "Bone Tower".to_string(),
        25 => "Rodney's Warped World".to_string(),
        other => format!("Area {other}"),
    }
}

/// C `cmd_orbs` (`command.c:1498-1559`, dispatched from `command.c:8905-
/// 8917` gated on `ch[cn].exp >= 81000`, i.e. level 30 - the gate check and
/// its plain, uncolored rejection message live in the dispatcher, not
/// `cmd_orbs` itself, and are reproduced here in the same order). Walks the
/// caller's `orbspawn_ppd` (`DRD_ORBSPAWN_PPD`, ported as
/// `PlayerRuntime::orb_spawns`), decoding each non-zero `ID[n]` back into
/// `x | y<<8 | area<<16` (the exact encoding `apply_orb_spawn`,
/// `area_apply.rs`, already writes), reporting "Ready to grab!" once
/// `base_orb_respawn_time_days` have elapsed since `last_used`, else the
/// remaining whole days, then a summary line with the average wait over
/// the not-yet-ready orbs (`0` if every orb is ready, C's `orb_count -
/// ready_count > 0` ternary). No permission-flag gate beyond the level-30
/// exp check.
pub(crate) fn apply_orbs_command(
    world: &World,
    runtime: &ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("orbs") {
        return None;
    }

    let player = runtime.player_for_character(character_id)?;
    let character = world.characters.get(&character_id)?;

    if character.exp < 81000 {
        return Some(KeyringCommandResult {
            messages: vec![
                "Thou hast to reach level 30 to fathom understanding the mysteries of the orbs and their timers."
                    .to_string(),
            ],
            ..Default::default()
        });
    }

    if player.orb_spawns.is_empty() {
        return Some(KeyringCommandResult {
            message_bytes: vec![legacy_achievement_colored_line(
                COL_LIGHT_RED,
                "Ye have not yet discovered any orbs, brave adventurer.",
            )],
            ..Default::default()
        });
    }

    let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
    let respawn_days = i64::from(world.settings.base_orb_respawn_time_days);
    let orb_count = player.orb_spawns.len() as i64;
    let mut ready_count = 0i64;
    let mut total_days = 0i64;

    let mut lines = vec![legacy_achievement_colored_line(
        COL_ORANGE,
        "Orb Locations:",
    )];
    for orb in &player.orb_spawns {
        let x = orb.location_id & 0xFF;
        let y = (orb.location_id >> 8) & 0xFF;
        let area = orb.location_id >> 16;
        let area_name = legacy_area_name(area);
        let elapsed_days =
            (realtime_seconds.saturating_sub(orb.last_used_seconds) / (60 * 60 * 24)) as i64;
        let days_until_spawn = respawn_days - elapsed_days;

        let mut line = Vec::new();
        line.extend_from_slice(b"Orb at ");
        line.extend_from_slice(COL_ORANGE);
        line.extend_from_slice(format!("({x}, {y})").as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" in ");
        line.extend_from_slice(COL_VIOLET);
        line.extend_from_slice(area_name.as_bytes());
        line.extend_from_slice(COL_RESET);
        if days_until_spawn <= 0 {
            line.extend_from_slice(b" - ");
            line.extend_from_slice(COL_YELLOW);
            line.extend_from_slice(b"Ready to grab!");
            line.extend_from_slice(COL_RESET);
            ready_count += 1;
        } else {
            line.extend_from_slice(b" - Ready in ");
            line.extend_from_slice(COL_LIGHT_RED);
            line.extend_from_slice(format!(" {days_until_spawn} days").as_bytes());
            line.extend_from_slice(COL_RESET);
            total_days += days_until_spawn;
        }
        lines.push(line);
    }

    let average_days = if orb_count - ready_count > 0 {
        total_days as f64 / (orb_count - ready_count) as f64
    } else {
        0.0
    };
    let mut summary = Vec::new();
    summary.extend_from_slice(COL_ORANGE);
    summary.extend_from_slice(b"Summary:");
    summary.extend_from_slice(COL_RESET);
    summary.extend_from_slice(format!(" {orb_count} orbs total, ").as_bytes());
    summary.extend_from_slice(COL_YELLOW);
    summary.extend_from_slice(format!(" {ready_count} ready ").as_bytes());
    summary.extend_from_slice(COL_RESET);
    summary.extend_from_slice(b", Average spawn time: ");
    summary.extend_from_slice(COL_LIGHT_RED);
    summary.extend_from_slice(format!(" {average_days:.1} days").as_bytes());
    summary.extend_from_slice(COL_RESET);
    lines.push(summary);

    Some(KeyringCommandResult {
        message_bytes: lines,
        ..Default::default()
    })
}

/// 365 days in seconds, the fixed respawn timer C hardcodes for every GU
/// mines/RD99 treasure chest case in `cmd_treasure`'s switch
/// (`command.c:1608-1650`) and for each Brannington dig spot
/// (`command.c:1682`, `365 * 24 * 60 * 60`).
const GU_TREASURE_RESPAWN_SECONDS: u64 = 365 * 24 * 60 * 60;

/// The `(name, respawn_seconds)` table from `cmd_treasure`'s `switch (nr)`
/// (`command.c:1608-1650`), indexed by treasure number. C iterates
/// `nr` 56..=104 but skips straight from 65 to 101 (`if (nr == 65) { nr =
/// 101; }`), so only 56..=64 and 101..=104 are ever named; every other `nr`
/// in that range hits the `default: "Unknown Chest"` arm, which is
/// unreachable in practice since the loop never visits it.
fn legacy_treasure_chest_name(nr: u16) -> &'static str {
    match nr {
        56 => "Mines level 10 (GU)",
        57 => "Mines level 20 (GU)",
        58 => "Mines level 30 (GU)",
        59 => "Mines level 40 (GU)",
        60 => "Mines level 50 (GU)",
        61 => "Mines level 60 (GU)",
        62 => "Mines level 70 (GU)",
        63 => "Mines level 80 (GU)",
        64 => "RD99 chest",
        101 => "Mines level 90 (GU)",
        102 => "Mines level 100 (GU)",
        103 => "Mines level 110 (GU)",
        104 => "Mines level 120 (GU)",
        _ => "Unknown Chest",
    }
}

/// C `cmd_treasure` (`command.c:1570-1704`, dispatched unconditionally
/// from `command.c:8944-8952`, no permission gate). Reports every GU
/// mines/RD99 chest (`DRD_TREASURE_CHEST_PPD`, ported as
/// `PlayerRuntime::chest_last_access_seconds`) and Brannington Forest dig
/// spot (`DRD_TREASURE_DIG_PPD`, ported as
/// `PlayerRuntime::treasure_dig_last_seconds`) the caller has ever
/// touched, each either "Ready!"/"Ready to dig!" or a `days, hours,
/// minutes` countdown to its 365-day respawn, then a discovered/ready
/// summary line spanning both categories.
pub(crate) fn apply_treasures_command(
    world: &World,
    runtime: &ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("treasures") {
        return None;
    }

    let player = runtime.player_for_character(character_id)?;
    let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;

    let mut lines = vec![legacy_achievement_colored_line(
        COL_ORANGE,
        "Treasures Status (only shows treasures ye have found):",
    )];
    let mut total_count = 0i64;
    let mut ready_count = 0i64;

    let countdown_line = |name: &str, tg: u64| -> Vec<u8> {
        let days = tg / (60 * 60 * 24);
        let hours = (tg / (60 * 60)) % 24;
        let minutes = (tg / 60) % 60;
        let mut line = Vec::new();
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(name.as_bytes());
        line.extend_from_slice(b":");
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" ");
        line.extend_from_slice(COL_LIGHT_RED);
        line.extend_from_slice(
            format!(" {days} days, {hours} hours, {minutes} minutes").as_bytes(),
        );
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" remain");
        line
    };

    for nr in (56u16..=64).chain(101..=104) {
        let last_access = player.chest_last_access_seconds(nr as u8);
        if last_access == 0 {
            continue;
        }
        total_count += 1;
        let name = legacy_treasure_chest_name(nr);
        let ready_at = last_access.saturating_add(GU_TREASURE_RESPAWN_SECONDS);
        if ready_at <= realtime_seconds {
            ready_count += 1;
            let mut line = Vec::new();
            line.extend_from_slice(COL_LIGHT_GREEN);
            line.extend_from_slice(name.as_bytes());
            line.extend_from_slice(b": ");
            line.extend_from_slice(COL_YELLOW);
            line.extend_from_slice(b"Ready!");
            line.extend_from_slice(COL_RESET);
            lines.push(line);
        } else {
            lines.push(countdown_line(name, ready_at - realtime_seconds));
        }
    }

    const DIG_SPOT_NAMES: [&str; 5] = [
        "Brannington (Forester's Quest) Dead Tree",
        "Brannington (Forester's Quest) Heart of Fire",
        "Brannington (Forester's Quest) Empty Bucket",
        "Brannington (Forester's Quest) Stone Circle",
        "Brannington (Forester's Quest) Bags",
    ];
    for (index, name) in DIG_SPOT_NAMES.iter().enumerate() {
        let last_dig = player.treasure_dig_last_seconds(index as u8);
        if last_dig == 0 {
            continue;
        }
        total_count += 1;
        let ready_at = last_dig.saturating_add(GU_TREASURE_RESPAWN_SECONDS);
        if ready_at <= realtime_seconds {
            ready_count += 1;
            let mut line = Vec::new();
            line.extend_from_slice(COL_LIGHT_GREEN);
            line.extend_from_slice(name.as_bytes());
            line.extend_from_slice(b": ");
            line.extend_from_slice(COL_YELLOW);
            line.extend_from_slice(b"Ready to dig!");
            line.extend_from_slice(COL_RESET);
            lines.push(line);
        } else {
            lines.push(countdown_line(name, ready_at - realtime_seconds));
        }
    }

    let mut summary = Vec::new();
    summary.extend_from_slice(COL_ORANGE);
    summary.extend_from_slice(b"Summary:");
    summary.extend_from_slice(COL_RESET);
    summary.extend_from_slice(format!(" {total_count} treasures discovered, ").as_bytes());
    summary.extend_from_slice(COL_YELLOW);
    summary.extend_from_slice(format!(" {ready_count} ready to loot").as_bytes());
    summary.extend_from_slice(COL_RESET);
    lines.push(summary);

    Some(KeyringCommandResult {
        message_bytes: lines,
        ..Default::default()
    })
}

/// C `cmd_tunnel` (`command.c:1712-1774`, dispatched unconditionally from
/// `command.c:8920-8926`, no permission gate). Reports the caller's
/// current Gorwin-assigned tunnel level (`gorwin_ppd::tunnel_level`, or a
/// computed default when it's still `0` - not yet initialized) and its
/// completion count, then optionally reports the same for an explicit
/// `level` argument.
pub(crate) fn apply_tunnel_command(
    world: &World,
    runtime: &ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("tunnel") {
        return None;
    }

    let player = runtime.player_for_character(character_id)?;
    let character = world.characters.get(&character_id)?;
    let arg = rest.trim();

    // C `cmd_tunnel` (`command.c:1722-1739`): if the Gorwin-assigned
    // level is unset (`0`), derive a default from the caller's own level
    // - below 20 always gets 10; up to 100 gets `level - 10`; above 100
    // searches upward from 90 for the first not-yet-completed level,
    // stopping at `min(level - 10, 200)` (copied digit for digit,
    // including the search's exact stopping point).
    let mut current_level = player.gorwin_tunnel_level();
    if current_level == 0 {
        let char_level = character.level as i32;
        if char_level < 20 {
            current_level = 10;
        } else if char_level <= 100 {
            current_level = char_level - 10;
        } else {
            let mut n = 90;
            while n < char_level - 10 && n < 200 {
                if player.tunnel_used(n) < 1 {
                    break;
                }
                n += 1;
            }
            current_level = n;
        }
    }
    current_level = current_level.clamp(10, 200);

    let mut lines = Vec::new();

    let mut line1 = Vec::new();
    line1.extend_from_slice(b"Your current tunnel level is: ");
    line1.extend_from_slice(COL_ORANGE);
    line1.extend_from_slice(format!(" {current_level}").as_bytes());
    line1.extend_from_slice(COL_RESET);
    lines.push(line1);

    let used_current = player.tunnel_used(current_level);
    let mut line2 = Vec::new();
    line2.extend_from_slice(b"You have completed this level ");
    line2.extend_from_slice(COL_LIGHT_GREEN);
    line2.extend_from_slice(format!(" {used_current}").as_bytes());
    line2.extend_from_slice(COL_RESET);
    line2.extend_from_slice(b" times.");
    lines.push(line2);

    if used_current >= MAX_TUNNEL_USES {
        lines.push(legacy_achievement_colored_line(
            COL_LIGHT_RED,
            "You have reached the maximum number of rewarded completions for this level.",
        ));
    } else {
        let remaining = MAX_TUNNEL_USES - used_current;
        let mut line = Vec::new();
        line.extend_from_slice(b"You can complete this level ");
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(format!(" {remaining}").as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" more times for rewards.");
        lines.push(line);
    }

    if !arg.is_empty() {
        let level: i32 = arg.parse().unwrap_or(0);
        if (10..=200).contains(&level) {
            let used = player.tunnel_used(level);
            let mut line = Vec::new();
            line.extend_from_slice(b"Tunnel level ");
            line.extend_from_slice(COL_ORANGE);
            line.extend_from_slice(format!(" {level}").as_bytes());
            line.extend_from_slice(COL_RESET);
            line.extend_from_slice(b": completed ");
            line.extend_from_slice(COL_LIGHT_GREEN);
            line.extend_from_slice(format!(" {used}").as_bytes());
            line.extend_from_slice(COL_RESET);
            line.extend_from_slice(b" times.");
            lines.push(line);

            if used >= MAX_TUNNEL_USES {
                lines.push(legacy_achievement_colored_line(
                    COL_LIGHT_RED,
                    "Maximum number of rewarded completions reached for this level.",
                ));
            } else {
                let remaining = MAX_TUNNEL_USES - used;
                let mut line = Vec::new();
                line.extend_from_slice(b"This level can be completed ");
                line.extend_from_slice(COL_LIGHT_GREEN);
                line.extend_from_slice(format!(" {remaining}").as_bytes());
                line.extend_from_slice(COL_RESET);
                line.extend_from_slice(b" more times for rewards.");
                lines.push(line);
            }
        } else {
            lines.push(legacy_achievement_colored_line(
                COL_LIGHT_RED,
                "Invalid tunnel level. Please choose a level between 10 and 200.",
            ));
        }
    }

    Some(KeyringCommandResult {
        message_bytes: lines,
        ..Default::default()
    })
}

/// C `cmd_tunnellist` (`command.c:1785-1834`, dispatched unconditionally
/// from `command.c:8929-8935`, no permission gate). Lists every tunnel
/// level from `MIN_TUNNEL_LEVEL` (10) up to
/// `max(highest_completed, max(10, char_level - 10))` (capped at 200),
/// colored by completion state (violet = maxed out, light red =
/// partially completed, plain = never touched). Requires at least one
/// completed level or reports a rejection instead.
pub(crate) fn apply_tunnellist_command(
    world: &World,
    runtime: &ServerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("tunnels") {
        return None;
    }

    let player = runtime.player_for_character(character_id)?;
    let character = world.characters.get(&character_id)?;

    let mut tunnel_done = false;
    let mut highest_completed = 10;
    for n in 10..=200 {
        if player.tunnel_used(n) >= 1 {
            tunnel_done = true;
            highest_completed = n;
        }
    }

    if !tunnel_done {
        return Some(KeyringCommandResult {
            message_bytes: vec![legacy_achievement_colored_line(
                COL_LIGHT_RED,
                "Ye must complete at least one tunnel before thou canst check this.",
            )],
            ..Default::default()
        });
    }

    let mut lines = vec![legacy_achievement_colored_line(
        COL_ORANGE,
        "Tunnels status:",
    )];

    let char_level = character.level as i32;
    let mut max_level = highest_completed.max(10.max(char_level - 10));
    if max_level > 200 {
        max_level = 200;
    }

    let mut buf = Vec::new();
    for n in 10..=max_level {
        let used = player.tunnel_used(n);
        if used >= MAX_TUNNEL_USES {
            buf.extend_from_slice(COL_VIOLET);
            buf.extend_from_slice(format!(" {n}").as_bytes());
            buf.extend_from_slice(COL_RESET);
            buf.extend_from_slice(b" ");
        } else if used >= 1 {
            buf.extend_from_slice(COL_LIGHT_RED);
            buf.extend_from_slice(format!(" {n}").as_bytes());
            buf.extend_from_slice(COL_RESET);
            buf.extend_from_slice(b" ");
        } else {
            buf.extend_from_slice(format!("{n} ").as_bytes());
        }
    }
    lines.push(buf);

    Some(KeyringCommandResult {
        message_bytes: lines,
        ..Default::default()
    })
}
