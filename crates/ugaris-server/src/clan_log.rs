//! `/clanlog` and `/clearclanlog` text commands.
//!
//! Ports `src/system/clanlog.c` (`cmd_clanlog` and its `clanlog_player`/
//! `clanlog_clan`/`clanlog_prio`/`clanlog_hours` flag parsers) and
//! `command.c`'s `/clearclanlog` GM handler (`command.c:7541-7559`), on
//! top of the DB layer in `ugaris_db::clan_log` (`add_clanlog`/
//! `lookup_clanlog`/`db_read_clanlog`, `database_notes.c`).
//!
//! This slice wires the *read* side (`/clanlog`) and the *admin clear*
//! side (`/clearclanlog`). [`write_clan_log_entry`] is the first *write*
//! call site: `found_clan`/`add_member`/`remove_member` via the
//! clanmaster NPC (`crate::world_events::apply_clanmaster_events`,
//! `crate::clanmaster` in `ugaris-core`) now call it for the "Clan was
//! founded by %s"/"%s was added to clan by %s"/"%s was fired from clan by
//! %s" entries. The daily relation-tick entries and the rank/website/
//! message-edit entries still have no live call site (the relation tick
//! has no game-loop caller yet, and rank/website/message editing needs a
//! real `/clan` command parser - see the "Clan system" P3 task in
//! `PORTING_TODO.md`).
//!
//! Like `/ah` (`auction.rs`), this feature is entirely DB-backed with no
//! in-memory `World` representation of log rows, so both commands are
//! unavailable (matching the "auction house unavailable" precedent, not
//! a silent no-op) when the server runs without `--database-url`; the
//! write helper below silently no-ops the same way when unconfigured.

use super::*;

use ugaris_db::{
    ClanLogEntry, ClanLogFilter, ClanLogRepository, PgClanLogRepository, CLAN_LOG_DISPLAY_LIMIT,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ClanLogCommandResult {
    pub(crate) messages: Vec<String>,
    pub(crate) message_bytes: Vec<Vec<u8>>,
}

/// A validated, ready-to-run `/clanlog` query (`cmd_clanlog`'s local
/// `coID`/`clan`/`serial`/`prio`/`start`/`end` variables once parsing
/// succeeds, `clanlog.c:169-243`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClanLogQuery {
    pub(crate) clan: u16,
    pub(crate) serial: u32,
    pub(crate) character_id: u32,
    pub(crate) prio: u8,
    pub(crate) from_time: i64,
    pub(crate) to_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClanLogParseOutcome {
    /// A runnable query, plus any message that must be shown *before*
    /// the results (C's "Changed clan to %d." notice, `clanlog.c:236`).
    Query {
        query: ClanLogQuery,
        leading_messages: Vec<String>,
    },
    /// A terminal single-purpose message (help text or a validation
    /// error) with nothing left to query.
    Messages(Vec<String>),
}

/// C `clanlog_help` (`clanlog.c:28-52`), color macros stripped (message
/// text carried as plain lines - see `legacy_help_line_bytes` for the
/// generic house-style coloring convention used for other multi-line
/// `/*help` commands in this codebase).
fn clan_log_help_lines() -> Vec<String> {
    [
        "=== Clan Log Help ===",
        " ",
        "Usage: /clanlog [options]",
        " ",
        "--- Filter Options ---",
        "-p <player>  Show only entries involving this player",
        "-c <clan#>   Show only entries for this clan number",
        "-x <prio>    Priority filter (1=important, 100=trivial). Default: 20",
        "-i           Show internal clan log (same as -x 50 -c <your clan>)",
        " ",
        "--- Time Options ---",
        "-s <hours>   Start time: show entries from the last N hours",
        "-e <hours>   End time: show entries at least N hours old",
        " ",
        "--- Examples ---",
        "/clanlog                       Last 24 hours, priority 20+",
        "/clanlog -i                    Your clan's internal log",
        "/clanlog -p Ishtar -c 4        Ishtar's activity in clan 4",
        "/clanlog -x 5 -s 48 -e 24      High priority, 24-48 hours ago",
        " ",
        "Note: Priority > 20 restricts output to your own clan.",
    ]
    .iter()
    .map(|line| line.to_string())
    .collect()
}

/// C `atoi(ptr)` immediately followed by a *separate* `while
/// (isdigit(*ptr)) ptr++` re-scan from the same (whitespace-trimmed)
/// starting position (`clanlog_clan`/`clanlog_prio`/`clanlog_hours`,
/// `clanlog.c:96-146`). Because the digit-only re-scan never accounts
/// for a leading sign character, a negative input like `-c -5` parses
/// the value `-5` via `atoi` but leaves the cursor sitting on the `-`
/// sign afterward (unconsumed) - reproduced exactly rather than "fixed",
/// per the porting rules on faithfully copying odd edge cases.
fn parse_int_and_advance(input: &str) -> (i64, &str) {
    let after_ws = input.trim_start();
    let value = legacy_atoi_prefix(after_ws);
    let advanced = after_ws.trim_start_matches(|ch: char| ch.is_ascii_digit());
    (value, advanced)
}

/// C `clanlog_player`'s name-token scan (`clanlog.c:56-79`): up to 75
/// bytes, stopping at whitespace or end of input.
fn take_name_token(input: &str) -> (&str, &str) {
    let bytes = input.as_bytes();
    let mut len = 0;
    while len < 75 && len < bytes.len() && !bytes[len].is_ascii_whitespace() {
        len += 1;
    }
    (&input[..len], &input[len..])
}

/// C `cmd_clanlog` (`clanlog.c:169-243`). Returns `None` exactly when C's
/// `-p <name>` lookup fails to resolve an online player (`clanlog_player`
/// sets `repeat=1`, and `cmd_clanlog` then `return 0`s without printing
/// anything - the top-level command dispatcher treats that as "command
/// not recognized" and falls through to whatever comes next). C's
/// `lookup_name` resolves against a persistent cross-restart name index;
/// this port only has the currently-online character list to search,
/// documented simplification.
pub(crate) fn parse_clan_log_args(
    world: &World,
    character: &Character,
    args: &str,
    now: i64,
) -> Option<ClanLogParseOutcome> {
    let mut character_id_filter: u32 = 0;
    let mut clan: u16 = 0;
    let mut serial: u32 = 0;
    let mut prio: u8 = 20;
    let mut start: i64 = 0;
    let mut end: i64 = 0;

    let mut cursor = args;
    loop {
        let Some(ch) = cursor.chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            cursor = &cursor[ch.len_utf8()..];
            continue;
        }
        if ch != '-' {
            return Some(ClanLogParseOutcome::Messages(clan_log_help_lines()));
        }
        cursor = &cursor[1..];
        let Some(flag) = cursor.chars().next() else {
            return Some(ClanLogParseOutcome::Messages(clan_log_help_lines()));
        };
        match flag {
            'p' => {
                let (name_token, rest) = take_name_token(cursor[1..].trim_start());
                if name_token.is_empty() || name_token.len() > 70 {
                    return Some(ClanLogParseOutcome::Messages(vec![
                        "Invalid name".to_string()
                    ]));
                }
                match find_online_character_by_name(world, name_token) {
                    Some(id) => character_id_filter = id.0,
                    None => return None,
                }
                cursor = rest;
            }
            'c' => {
                let (value, rest) = parse_int_and_advance(&cursor[1..]);
                if !(1..LEGACY_MAX_CLAN).contains(&value) {
                    return Some(ClanLogParseOutcome::Messages(vec![
                        "Clan number out of bounds".to_string(),
                    ]));
                }
                clan = value as u16;
                serial = world.clan_registry.serial(clan);
                cursor = rest;
            }
            'x' => {
                let (value, rest) = parse_int_and_advance(&cursor[1..]);
                if !(1..=100).contains(&value) {
                    return Some(ClanLogParseOutcome::Messages(vec![
                        "Priority out of bounds".to_string(),
                    ]));
                }
                prio = value as u8;
                cursor = rest;
            }
            's' => {
                let (value, rest) = parse_int_and_advance(&cursor[1..]);
                let computed = now - value * 3600;
                if !(0..=now).contains(&computed) {
                    return Some(ClanLogParseOutcome::Messages(vec![
                        "Hours out of bounds".to_string()
                    ]));
                }
                start = computed;
                cursor = rest;
            }
            'e' => {
                let (value, rest) = parse_int_and_advance(&cursor[1..]);
                let computed = now - value * 3600;
                if !(0..=now).contains(&computed) {
                    return Some(ClanLogParseOutcome::Messages(vec![
                        "Hours out of bounds".to_string()
                    ]));
                }
                end = computed;
                cursor = rest;
            }
            'i' => {
                prio = 50;
                clan = character.clan;
                cursor = &cursor[1..];
            }
            _ => {
                return Some(ClanLogParseOutcome::Messages(clan_log_help_lines()));
            }
        }
    }

    if start == 0 {
        start = now - 60 * 60 * 24;
    }
    if end == 0 {
        end = now;
    }
    if start > end {
        return Some(ClanLogParseOutcome::Messages(vec![
            "Start time may not be greater than end time.".to_string(),
        ]));
    }

    let mut leading_messages = Vec::new();
    if prio > 20 {
        if character.clan == 0 {
            return Some(ClanLogParseOutcome::Messages(vec![
                "Only clan members may set a priority greater than 20.".to_string(),
            ]));
        }
        if clan != character.clan {
            clan = character.clan;
            leading_messages.push(format!("Changed clan to {clan}."));
        }
    }

    Some(ClanLogParseOutcome::Query {
        query: ClanLogQuery {
            clan,
            serial,
            character_id: character_id_filter,
            prio,
            from_time: start,
            to_time: end,
        },
        leading_messages,
    })
}

fn push_colored_line(out: &mut Vec<Vec<u8>>, color: &[u8], text_str: &str) {
    let mut line = Vec::with_capacity(color.len() + text_str.len() + COL_RESET.len());
    line.extend_from_slice(color);
    line.extend_from_slice(text_str.as_bytes());
    line.extend_from_slice(COL_RESET);
    out.push(line);
}

/// C `db_read_clanlog` (`database_notes.c:283-345`). `strftime`'s
/// `localtime` is approximated in UTC, matching this codebase's existing
/// convention (see `legacy_achievement_date`'s doc comment) since no
/// `chrono`/timezone-database dependency exists in this workspace.
pub(crate) fn format_clan_log_entries(
    entries: &[ClanLogEntry],
    world: &World,
    now: i64,
) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if entries.is_empty() {
        push_colored_line(&mut out, COL_DARK_GRAY, "No matching clan log entries.");
        return out;
    }

    // `COL_HEADING` (`text.rs`) is a plain alias of `COL_ORANGE`, which
    // is what's already imported at the crate root.
    push_colored_line(&mut out, COL_ORANGE, "=== Clan Log ===");

    let display_count = entries.len().min(CLAN_LOG_DISPLAY_LIMIT);
    for entry in &entries[..display_count] {
        let clan_name = if world.clan_registry.serial(entry.clan) == entry.serial {
            world.clan_registry.name(entry.clan).map(str::to_string)
        } else {
            None
        }
        .unwrap_or_else(|| format!("Former clan {}", entry.clan));

        let (year, month, day) = civil_from_unix_seconds(entry.time_unix.max(0) as u64);
        let seconds_of_day = entry.time_unix.max(0) as u64 % 86_400;
        let hour = seconds_of_day / 3600;
        let minute = (seconds_of_day % 3600) / 60;

        let mut line = Vec::new();
        line.extend_from_slice(COL_DARK_GRAY);
        line.extend_from_slice(
            format!(
                " {hour:02}:{minute:02} {month:02}/{day:02}/{:02}",
                year % 100
            )
            .as_bytes(),
        );
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(b" ");
        line.extend_from_slice(COL_ORANGE);
        line.extend_from_slice(clan_name.as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(format!(": {}", entry.content).as_bytes());
        out.push(line);
    }

    if entries.len() > CLAN_LOG_DISPLAY_LIMIT {
        let cutoff_time = entries[CLAN_LOG_DISPLAY_LIMIT].time_unix;
        let hours = (now - cutoff_time + 60 * 60 - 1) / (60 * 60);
        push_colored_line(
            &mut out,
            COL_DARK_GRAY,
            &format!("Not all entries displayed. Use -s {hours} to continue."),
        );
    }

    out
}

fn plain_line_bytes(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// C `add_clanlog` (`src/system/database/database_notes.c:74-104`),
/// called by whichever live clan-mutation call site needs it (currently
/// only `crate::world_events::apply_clanmaster_events`). Silently no-ops
/// (like every other DB-backed write in this codebase) when the server
/// runs without `--database-url`, and swallows a failed write rather than
/// propagating it - matching C's own `add_clanlog`, which only `elog`s on
/// failure and never surfaces the error to the caller.
pub(crate) async fn write_clan_log_entry(
    repository: &Option<PgClanLogRepository>,
    clan: u16,
    serial: u32,
    character_id: CharacterId,
    prio: u8,
    content: String,
    now_unix: i64,
) {
    let Some(repository) = repository else {
        return;
    };
    let _ = repository
        .add_entry(clan, serial, character_id, prio, &content, now_unix)
        .await;
}

/// C `command.c:9642-9644`'s `cmdcmp(ptr, "clanlog", 7)` dispatch to
/// `cmd_clanlog`, and `command.c:7541-7559`'s `clearclanlog` handler.
pub(crate) async fn apply_clan_log_command(
    world: &mut World,
    repository: &Option<PgClanLogRepository>,
    character_id: CharacterId,
    now_unix: i64,
    command: &str,
) -> Option<ClanLogCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');

    if verb.eq_ignore_ascii_case("clearclanlog") {
        return apply_clear_clan_log_command(world, repository, character_id, rest).await;
    }

    if !verb.eq_ignore_ascii_case("clanlog") {
        return None;
    }

    let character = world.characters.get(&character_id)?;
    let outcome = parse_clan_log_args(world, character, rest, now_unix)?;

    match outcome {
        ClanLogParseOutcome::Messages(messages) => Some(ClanLogCommandResult {
            message_bytes: messages
                .iter()
                .map(|message| legacy_help_line_bytes(message))
                .collect(),
            messages,
        }),
        ClanLogParseOutcome::Query {
            query,
            leading_messages,
        } => {
            let Some(repository) = repository else {
                return Some(ClanLogCommandResult {
                    messages: vec!["The clan log is currently unavailable.".to_string()],
                    ..Default::default()
                });
            };
            let filter = ClanLogFilter {
                clan: query.clan,
                serial: query.serial,
                character_id: query.character_id,
                prio: query.prio,
                from_time: query.from_time,
                to_time: query.to_time,
            };
            let entries = match repository.lookup(&filter).await {
                Ok(entries) => entries,
                Err(_) => {
                    return Some(ClanLogCommandResult {
                        messages: vec!["Failed to read clan log.".to_string()],
                        ..Default::default()
                    })
                }
            };
            let mut message_bytes: Vec<Vec<u8>> = leading_messages
                .iter()
                .map(|message| plain_line_bytes(message))
                .collect();
            message_bytes.extend(format_clan_log_entries(&entries, world, now_unix));
            Some(ClanLogCommandResult {
                message_bytes,
                ..Default::default()
            })
        }
    }
}

/// C `/clearclanlog` (`command.c:7541-7559`), `CF_GOD`-gated.
async fn apply_clear_clan_log_command(
    world: &mut World,
    repository: &Option<PgClanLogRepository>,
    character_id: CharacterId,
    rest: &str,
) -> Option<ClanLogCommandResult> {
    let character = world.characters.get(&character_id)?;
    if !character.flags.contains(CharacterFlags::GOD) {
        return None;
    }

    let clan_nr = legacy_atoi_prefix(rest.trim_start());
    if !(1..LEGACY_MAX_CLAN).contains(&clan_nr) {
        return Some(ClanLogCommandResult {
            messages: vec![format!(
                "Invalid clan number. Range is 1-{}",
                LEGACY_MAX_CLAN - 1
            )],
            ..Default::default()
        });
    }
    let clan_nr = clan_nr as u16;

    let Some(repository) = repository else {
        return Some(ClanLogCommandResult {
            messages: vec!["The clan log is currently unavailable.".to_string()],
            ..Default::default()
        });
    };

    let clan_name = world
        .clan_registry
        .name(clan_nr)
        .map(str::to_string)
        .unwrap_or_default();

    // C's `execute_query` (MySQL API convention: 0 on success, nonzero
    // on failure) is checked backwards here (`command.c:7550-7556`) - a
    // real legacy bug that swaps the two feedback lines. Preserved
    // digit-for-digit rather than "fixed": a *successful* delete reports
    // "Failed to clear clan log", and a failed one reports "... cleared".
    match repository.clear_clan(clan_nr).await {
        Ok(_) => Some(ClanLogCommandResult {
            messages: vec!["Failed to clear clan log".to_string()],
            ..Default::default()
        }),
        Err(_) => Some(ClanLogCommandResult {
            messages: vec![format!("Clan log for clan {clan_nr} ({clan_name}) cleared")],
            ..Default::default()
        }),
    }
}
