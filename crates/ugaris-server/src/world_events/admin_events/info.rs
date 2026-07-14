use super::*;

/// C `command.c`'s `lastseen:` handler's async DB round-trip
/// (`lastseen`/`db_lastseen`, `database_lookup.c:142-157` +
/// `database_notes.c:352-390`): resolves every `World::
/// drain_pending_lastseen_lookups` entry (queued by validly-shaped
/// `/lastseen <name>` arguments - see `World::queue_lastseen_lookup`'s
/// doc comment for the synchronous invalid-name fast path) against the
/// DB and delivers the reply via `World::queue_system_text` (C's
/// `tell_chat(0, rID, 1, ...)`, this codebase's direct-to-character
/// system-text channel).
///
/// Message shape mirrors `db_lastseen` exactly:
/// - no DB row -> "No character by the name %s." - the exact same text
///   the command dispatcher's own `lookup_name` `== -1` branch uses
///   (`command.c:9041`), since a player can't tell the two cases apart.
/// - `CF_GOD` row -> "%s was seen quite recently." (C never computes an
///   elapsed time for staff, `database_notes.c:378-379`).
/// - otherwise -> "%s was last seen %d days, %d hours, %d minutes ago.",
///   from `now - last_activity` where `last_activity` is `LastSeenInfo::
///   last_activity_unix` (already `max(login_time, logout_time,
///   created_at)`, computed in SQL - see `ugaris-db`'s `FIND_LAST_SEEN_SQL`
///   doc comment).
///
/// No-ops entirely (silent, matching every other offline-DB-lookup event
/// in this file) when no `character_repository` is configured or the
/// query itself errors.
pub(crate) async fn apply_lastseen_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let lookups = world.drain_pending_lastseen_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let reply = match repository.find_last_seen(&lookup.target_name).await {
            Ok(Some(info)) => lastseen_reply_message(&info, now_unix),
            Ok(None) => format!("No character by the name {}.", lookup.target_name),
            Err(_) => continue,
        };
        world.queue_system_text(lookup.requester_id, reply);
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_lastseen_events`], split out so
/// the day/hour/minute arithmetic (`database_notes.c:381-386`) can be
/// unit-tested without a live database.
pub(crate) fn lastseen_reply_message(info: &ugaris_db::LastSeenInfo, now_unix: i64) -> String {
    if info.is_god {
        return format!("{} was seen quite recently.", info.name);
    }
    let elapsed = now_unix - info.last_activity_unix;
    format!(
        "{} was last seen {} days, {} hours, {} minutes ago.",
        info.name,
        elapsed / (60 * 60 * 24),
        (elapsed / (60 * 60)) % 24,
        (elapsed / 60) % 60
    )
}

/// `#querystats`/`/querystats`'s async round trip - see `ugaris-core`'s
/// `world/querystats.rs` module doc comment for exactly which C counters
/// this scoped-down port tracks (and why the rest are omitted rather than
/// faked). `PgCharacterRepository::query_stats` is a synchronous
/// in-memory atomic read, not a real query, but is still routed through
/// this tick-loop drain (rather than answered directly in
/// `commands_admin.rs`) since command dispatch has no visibility into
/// `character_repository` - the same architectural constraint every
/// other DB-backed command in this file works around.
///
/// No-ops entirely (silent) when no `character_repository` is configured,
/// matching every sibling offline-DB-lookup event's convention.
pub(crate) fn apply_querystats_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_querystats_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let stats = repository.query_stats();
        for line in querystats_lines(stats) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure formatting half of `apply_querystats_events`, split out for unit
/// testing without needing a live `PgCharacterRepository` - matches
/// `ac_status_lines`'s established pattern for this file. Reproduces C's
/// `"Database Query Statistics:"` header and `"Character operations:"`
/// subheader/line verbatim (`command.c:6596,6601-604`); every other C
/// line (`Total queries`/`Average query time`/`Other operations`/`Query
/// type statistics`) is omitted, not faked, since nothing in `ugaris-db`
/// increments those counters - see `ugaris-core`'s `world/querystats.rs`
/// module doc comment.
pub(crate) fn querystats_lines(stats: ugaris_db::CharacterQueryStats) -> Vec<String> {
    vec![
        "Database Query Statistics:".to_string(),
        "Character operations:".to_string(),
        format!(
            "Save chars: {}, Exit chars: {}, Load chars: {}",
            stats.save_char_cnt, stats.exit_char_cnt, stats.load_char_cnt
        ),
    ]
}

/// `/look <name>`'s async DB round trip (C `command.c:8990-9019`'s inline
/// handler + `read_notes`/`db_read_notes`/`list_punishment`,
/// `src/system/database/database_lookup.c:116-124` + `database_notes.c:
/// 164-215` + `src/system/punish.c:26-38`): resolves every `World::
/// drain_pending_look_requests` entry (queued by `World::
/// queue_look_command`) by name via `find_login_target` (C's synchronous
/// `lookup_name`), then lists every `kind = 1` note filed against the
/// resolved character, each row's creator name resolved via
/// `find_name_by_id` (C `lookup_ID`).
///
/// - no matching character -> "No character by the name %s." (folds C's
///   `ID == -1` case; C's `ID == 0` "lookup in progress" case has no
///   analogue here, see `World::queue_look_command`'s doc comment).
/// - a match -> "Looking up character: %s (ID: %d)" (C's own immediate
///   confirmation line, `command.c:9016`), then "Start of Notes:", one
///   `format_look_note_line` per `kind = 1` row (oldest first, matching
///   `NotesRepository::list_notes_for_character`'s `order by id`), then
///   "End of Notes" - every other note `kind` is silently skipped (C's
///   own `default: xlog(...)` branch never reaches the player either).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_look_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
) -> usize {
    let requests = world.drain_pending_look_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.requester_id,
                format!("No character by the name {}.", request.target_name),
            );
            continue;
        };
        world.queue_system_text(
            request.requester_id,
            format!(
                "Looking up character: {} (ID: {})",
                summary.name, summary.id.0
            ),
        );
        let Ok(notes) = notes_repository.list_notes_for_character(summary.id).await else {
            continue;
        };
        world.queue_system_text(request.requester_id, "Start of Notes:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                request.requester_id,
                format_look_note_line(note.id, &punishment, &creator_name, note.created_at),
            );
        }
        world.queue_system_text(request.requester_id, "End of Notes".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_look_events`], split out so
/// the date arithmetic can be unit-tested without a live database. C
/// `list_punishment` (`src/system/punish.c:26-38`)'s `localtime` is
/// approximated in UTC, matching this codebase's established convention
/// (see `clan_log.rs`'s `format_clan_log_entries` doc comment) since no
/// `chrono`/timezone-database dependency exists in this workspace.
pub(crate) fn format_look_note_line(
    note_id: i64,
    note: &PunishmentNote,
    creator_name: &str,
    created_at: i64,
) -> String {
    let (year, month, day) = civil_from_unix_seconds(created_at.max(0) as u64);
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "P{note_id}: Level: {}, Exp: {}, Karma: {}, Creator: {creator_name}, Date: {month:02}/{day:02}/{year:04} {hour:02}:{minute:02}:{second:02}, Reason: {}",
        note.level, note.exp, note.karma, note.reason
    )
}

/// `/klog`'s async DB round trip (C `command.c:9022-9024` -> `karmalog`
/// -> `db_karmalog`/`karmalog_s`, `src/system/database/database_notes.c:
/// 230-275`): resolves every `World::drain_pending_klog_requests` entry
/// (queued by `World::queue_klog_command`, which takes no argument -
/// unlike `/look`, there is nothing to validate before queuing) against
/// a single shared `NotesRepository::list_recent_notes` query (the last
/// 24 hours, matching C's `date >= now - 86400` cutoff), reused across
/// every requester in the drained batch rather than re-querying per
/// caller.
///
/// Replies "Karmalog:", one `format_klog_line` per `kind = 1` row (newest
/// first, matching `list_recent_notes`'s `order by date desc`), then
/// "---" (C's own trailing separator, `database_notes.c:273`) - every
/// other note `kind` is silently skipped, same as `/look`. A row whose
/// target or creator id no longer resolves to a name falls back to
/// `"*unknown*"`, matching C `lookup_ID`'s own `"*unknown*"` fallback for
/// a cache slot with no name recorded (this codebase has no analogue of
/// C's separate `"**deleted**"` case, since it has no in-memory
/// name/ID cache to distinguish "never resolved" from "resolved to a
/// numeric placeholder" - see `CharacterRepository::find_name_by_id`'s
/// doc comment).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_klog_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    now_unix: i64,
) -> usize {
    let requesters = world.drain_pending_klog_requests();
    if requesters.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let since_unix = now_unix - 60 * 60 * 24;
    let Ok(notes) = notes_repository.list_recent_notes(since_unix).await else {
        return 0;
    };
    let mut applied = 0;
    for requester_id in requesters {
        world.queue_system_text(requester_id, "Karmalog:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(target_id) = note.target_id else {
                continue;
            };
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let offender_name = match character_repository.find_name_by_id(target_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                requester_id,
                format_klog_line(
                    &offender_name,
                    punishment.karma,
                    &creator_name,
                    &punishment.reason,
                    note.created_at,
                ),
            );
        }
        world.queue_system_text(requester_id, "---".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_klog_events`] - see that
/// function's doc comment. C `karmalog_s` (`database_notes.c:227-244`)
/// prints only the time of day, not the full date (unlike
/// [`format_look_note_line`]'s sibling `list_punishment` format).
pub(crate) fn format_klog_line(
    offender_name: &str,
    karma: i32,
    creator_name: &str,
    reason: &str,
    created_at: i64,
) -> String {
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "{offender_name}, {karma} Karma from {creator_name} for {reason} at {hour:02}:{minute:02}:{second:02}."
    )
}

/// `/showvalues <name>`'s async DB round trip (C `command.c:8401-8409` ->
/// `show_values`, `command.c:521-537` + its `server_chat` body
/// `show_values_bg`, `src/system/tool.c:2940-3096`): resolves every
/// `World::drain_pending_showvalues_requests` entry (queued by `World::
/// queue_showvalues_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch; C's `ID == 0` "lookup in progress" case has no analogue
///   here, same as every sibling name-lookup command in this codebase).
/// - a match -> the caller always gets the "Sent." confirmation (C logs
///   this unconditionally once `lookup_name` succeeds, regardless of
///   which area server - if any - currently has the target loaded), then
///   the caller's own `show_values_lines` stat block is delivered to the
///   target *only if the target happens to be loaded in this process's
///   `World`* - C's real delivery goes through `tell_chat`'s own
///   cross-area chat relay, which this codebase does not have yet (see
///   the "Cross-area transfer" `PORTING_TODO.md` entry's gap (2) and
///   `world/values.rs`'s module doc comment for the full single-process
///   caveat).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_showvalues_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let requests = world.drain_pending_showvalues_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(caller) = world.characters.get(&request.caller_id) else {
            continue;
        };
        let lines = show_values_lines(caller, &world.items);
        world.queue_system_text(request.caller_id, "Sent.".to_string());
        if world.characters.contains_key(&summary.id) {
            for line in lines {
                world.queue_system_text(summary.id, line);
            }
        }
        applied += 1;
    }
    applied
}

/// `/values <name>`'s async DB round trip (C `command.c:8391-8399` ->
/// `look_values`, `command.c:501-519` + its `server_chat` body
/// `look_values_bg`, `src/system/tool.c:2882-2939`): resolves every
/// `World::drain_pending_values_requests` entry (queued by `World::
/// queue_values_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), same as `/showvalues` above.
///
/// Unlike `/showvalues`'s caller/target role swap, `/values` keeps the
/// caller as the caller throughout: every reply line goes back to
/// `request.caller_id`, showing the *resolved target's* stats (see
/// `world/values.rs`'s module doc comment for the contrast, and C's own
/// `tell_chat(0, cnID, 1, ...)` calls in `look_values_bg`, all addressed
/// to the caller `cnID`, never the target `coID`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch).
/// - a match not currently loaded in this process's `World` -> silent
///   no-op (C's `if (!co) return;` in `look_values_bg` - no message at
///   all, matching this codebase's single-process-only cross-area chat
///   caveat, see `world/values.rs`'s module doc comment).
/// - a match with no resolvable `find_paid_until_info` row (a data
///   inconsistency - a live `World` character with no matching DB
///   `accounts` join - never hit for a real player) -> silent no-op,
///   same as the offline case.
/// - a loaded match -> the caller receives every `values_lines` line:
///   `PlayerRuntime::stats_online_time`/`bank_gold`/`current_mirror_id`
///   come from `ServerRuntime::player_for_character` when the target has
///   a live session (defaulting to `0`/`0`/this server's own `mirror_id`
///   when it does not - e.g. an offline-but-somehow-`World`-resident
///   NPC, never hit for a real logged-in player); the mirror-area
///   section name comes from `section_at(area_id, x, y)` (C's
///   `get_section_name`, implicitly scoped to this server process's own
///   `areaID`), falling back to `""` when no section matches (see
///   `values_lines`'s own doc comment for why).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_values_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
    mirror_id: u16,
    now_unix: i64,
) -> usize {
    let requests = world.drain_pending_values_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(target) = world.characters.get(&summary.id).cloned() else {
            continue;
        };
        let Ok(Some(paid_info)) = character_repository.find_paid_until_info(summary.id).await
        else {
            continue;
        };
        let (paid_till, is_paid) = compute_paid_till(
            paid_info.raw_paid_until_unix,
            paid_info.account_created_at_unix,
            now_unix,
        );
        let (online_minutes, bank_gold, current_mirror) = runtime
            .player_for_character(summary.id)
            .map(|player| {
                (
                    player.stats_online_time(),
                    player.bank_gold,
                    player.current_mirror_id,
                )
            })
            .unwrap_or((0, 0, mirror_id));
        let section_name = section_at(area_id, usize::from(target.x), usize::from(target.y))
            .map(|section| section.name)
            .unwrap_or("");
        let lines = values_lines(
            &target,
            &world.items,
            is_paid,
            paid_till,
            now_unix,
            online_minutes,
            bank_gold,
            current_mirror,
            mirror_id,
            area_id,
            section_name,
        );
        for line in lines {
            world.queue_system_text(request.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// `/allow <name>`'s async DB round trip (C `command.c:8371-8378` ->
/// `allow_body`, `src/system/death.c:1013-1029` + its `server_chat` body
/// `allow_body_db`, `death.c:1045-1067`): resolves every `World::
/// drain_pending_allow_requests` entry (queued by `World::
/// queue_allow_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), then grants the resolved target access to
/// every grave `World::grant_grave_access_to` finds owned by the caller
/// in this process's own `World` (see `world/allow.rs`'s module doc
/// comment for the single-process-only caveat shared with every other
/// name-lookup command here).
///
/// - no matching character -> "No player by that name." (C's `coID ==
///   -1` branch).
/// - a match -> C's `allow_body` unconditionally logs "Order
///   scheduled." once `lookup_name` resolves, then `allow_body_db`
///   (run per-area-server against the broadcast) replies "Area %d:
///   Allowed access to %d corpses." with its own local count - both
///   lines are sent here, in that order, once resolution completes
///   (this codebase collapses C's two-step broadcast-then-local-reply
///   into one async round trip, matching every other documented
///   cross-area gap).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_allow_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
) -> usize {
    let requests = world.drain_pending_allow_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        world.queue_system_text(request.caller_id, "Order scheduled.".to_string());
        let count = world.grant_grave_access_to(request.caller_id, summary.id);
        world.queue_system_text(
            request.caller_id,
            format!("Area {area_id}: Allowed access to {count} corpses."),
        );
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod lastseen_tests {
    use super::*;
    use ugaris_db::LastSeenInfo;

    #[test]
    fn god_characters_get_the_fixed_recently_message() {
        let info = LastSeenInfo {
            name: "Godmode".to_string(),
            is_god: true,
            last_activity_unix: 0,
        };
        assert_eq!(
            lastseen_reply_message(&info, 1_000_000),
            "Godmode was seen quite recently."
        );
    }

    #[test]
    fn elapsed_time_is_broken_into_days_hours_minutes() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 0,
        };
        // 2 days, 3 hours, 4 minutes = 2*86400 + 3*3600 + 4*60 seconds.
        let now = 2 * 86_400 + 3 * 3_600 + 4 * 60;
        assert_eq!(
            lastseen_reply_message(&info, now),
            "Player was last seen 2 days, 3 hours, 4 minutes ago."
        );
    }

    #[test]
    fn recently_active_player_reports_zero_across_the_board() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 500,
        };
        assert_eq!(
            lastseen_reply_message(&info, 500),
            "Player was last seen 0 days, 0 hours, 0 minutes ago."
        );
    }
}

#[cfg(test)]
mod querystats_tests {
    use super::*;

    #[test]
    fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[test]
    fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_querystats_lookup(CharacterId(7));

        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_querystats_lookups().is_empty());
    }

    #[test]
    fn querystats_lines_reproduce_the_scoped_c_header_and_counters() {
        let stats = ugaris_db::CharacterQueryStats {
            save_char_cnt: 12,
            exit_char_cnt: 3,
            load_char_cnt: 7,
        };
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 12, Exit chars: 3, Load chars: 7".to_string(),
            ]
        );
    }

    #[test]
    fn querystats_lines_reports_zero_counters_faithfully() {
        let stats = ugaris_db::CharacterQueryStats::default();
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 0, Exit chars: 0, Load chars: 0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod klog_tests {
    use super::*;

    #[test]
    fn format_klog_line_matches_c_karmalog_s_shape_time_only_no_date() {
        // 1_000_000_000 unix seconds = 2001-09-09 01:46:40 UTC.
        let line = format_klog_line("Baddie", -4, "Godmode", "being mean", 1_000_000_000);
        assert_eq!(
            line,
            "Baddie, -4 Karma from Godmode for being mean at 01:46:40."
        );
    }

    #[tokio::test]
    async fn no_klog_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_klog_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_klog_command(CharacterId(1));
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_klog_requests().is_empty());
    }
}

#[cfg(test)]
mod showvalues_tests {
    use super::*;

    #[tokio::test]
    async fn no_showvalues_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_showvalues_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_showvalues_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_showvalues_requests().is_empty());
    }
}
