use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_anticheat(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C's Anti-Cheat Admin Commands family (`command.c:10148-10192`):
    // `#achelp`/`#acstatus <name>`/`#acstats`/`#aclist`/`#acsuspicious`,
    // all `CF_GOD|CF_STAFF`-gated, exact-word only (`cmdcmp`'s `minlen`
    // equals each command's full length, so no abbreviation is accepted
    // for any of them). See `crates/ugaris-core/src/world/anticheat.rs`'s
    // module doc comment for why `#acstatus`/`#acstats`/`#aclist`/
    // `#acsuspicious` need an async DB round trip in this codebase
    // (unlike C's synchronous in-memory `player[nr]->ac` struct read):
    // the online-name-scan (C's `ac_find_player`, `CF_PLAYER`-filtered,
    // first match by iteration order - ties broken by ascending
    // character id here for determinism, same convention as
    // `world/clanmaster.rs`'s sibling helper) plus the
    // `PlayerRuntime::anticheat_session_id` lookup happen here,
    // synchronously, before queuing to `World` for the DB half. Only
    // these six of the ~20-member family are ported so far (see
    // `PORTING_TODO.md`'s remaining-text-commands task's REMAINING note);
    // `acreset`/`acflag`/`acwatch`/`acunflag`/`actrust`/`acuntrust`/
    // `acwarn`/`acsessions`/`acviolations`/`achistory`/`acsharedip`/
    // `acsharedhw`/`achighrisk`/`aclookup` are also ported, further below
    // (the last two, `achighrisk`/`aclookup`, need no online-name-scan at
    // all - see their own dispatch arms). `#accleanup`/`#acsiglist`/
    // `#acsigadd`/`#acsigdel` (below, further down) need no name
    // resolution at all, so they aren't part of this shared `lower ==`
    // arm.
    if lower == "achelp" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        // C `ac_cmd_help` (`anticheat.c:688-720`) - reproduced letter for
        // letter (minus the `COL_*` wrapping, matching `/global`'s own
        // established plain-text simplification for text-heavy admin
        // dumps) even though most of the listed subcommands are still
        // unported, since this is C's own static help text, not a
        // reflection of this codebase's current dispatch coverage.
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "--- Anti-Cheat Commands ---".to_string(),
                "#achelp - Show this help".to_string(),
                "#acstats - Global AC statistics".to_string(),
                "#aclist - List online players with AC status".to_string(),
                "#acsuspicious - List suspicious/flagged players".to_string(),
                "--- Player Commands ---".to_string(),
                "#acstatus <name> - Show player's AC status".to_string(),
                "#achistory <name> - Show player's violation history".to_string(),
                "#acsessions <name> - Show player's recent sessions".to_string(),
                "#acviolations <name> - Show player's violations".to_string(),
                "#acflag <name> - Flag player for review".to_string(),
                "#acunflag <name> - Remove flagged status".to_string(),
                "#actrust <name> - Mark player as trusted".to_string(),
                "#acuntrust <name> - Remove trusted status".to_string(),
                "#acreset <name> - Reset player's AC data (God)".to_string(),
                "#acwarn <name> [reason] - Issue AC warning".to_string(),
                "#acwatch <name> - Toggle detailed logging".to_string(),
                "--- Multi-Account Detection ---".to_string(),
                "#acsharedip <name> - Show accounts sharing IP".to_string(),
                "#acsharedhw <name> - Show accounts sharing hardware".to_string(),
                "--- Database Queries ---".to_string(),
                "#achighrisk - Show high-risk players".to_string(),
                "#aclookup <id> - Lookup by subscriber ID".to_string(),
                "--- Signature Management ---".to_string(),
                "#acsiglist - List known bad signatures".to_string(),
                "#acsigadd <type> <value> <name> - Add signature (God)".to_string(),
                "#acsigdel <id> - Delete signature (God)".to_string(),
                "--- Maintenance ---".to_string(),
                "#accleanup <days> - Cleanup old records (God)".to_string(),
            ],
            ..Default::default()
        }));
    }

    if lower == "acstatus" || lower == "acstats" || lower == "aclist" || lower == "acsuspicious" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }

        if lower == "acstatus" {
            // C `ac_cmd_status` (`anticheat.c:473-517`).
            let name = rest.trim_start();
            if name.is_empty() {
                return ControlFlow::Break(Some(KeyringCommandResult {
                    messages: vec!["Usage: #acstatus <player>".to_string()],
                    ..Default::default()
                }));
            }
            let mut candidates: Vec<&Character> = world
                .characters
                .values()
                .filter(|character| {
                    character.flags.contains(CharacterFlags::PLAYER)
                        && character.name.eq_ignore_ascii_case(name)
                })
                .collect();
            candidates.sort_by_key(|character| character.id.0);
            let Some(target_id) = candidates.first().map(|character| character.id) else {
                return ControlFlow::Break(Some(KeyringCommandResult {
                    messages: vec![format!("Player '{name}' not found online.")],
                    ..Default::default()
                }));
            };
            let target_name = world.characters[&target_id].name.clone();
            let Some(session_id) = runtime
                .player_for_character(target_id)
                .and_then(|player| player.anticheat_session_id)
            else {
                return ControlFlow::Break(Some(KeyringCommandResult {
                    messages: vec![format!("Player '{target_name}' has no connection data.")],
                    ..Default::default()
                }));
            };
            world.queue_ac_status_lookup(character_id, target_name, session_id);
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        }

        // `#acstats`/`#aclist`/`#acsuspicious` (`ac_cmd_stats`/
        // `ac_cmd_list`/`ac_cmd_suspicious`, `anticheat.c:604-628,721-780`):
        // gather every currently online `CF_PLAYER` character with a known
        // anticheat session - see module doc comment for why a player with
        // no session (DB not configured, or the session row failed to
        // create at login) is simply omitted rather than padded with
        // defaults. `#acsuspicious`'s own status >= AC_STATUS_SUSPICIOUS
        // filter can't happen here since status only becomes known after
        // the async DB round trip - see `apply_ac_suspicious_events`.
        let mut player_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        player_ids.sort_by_key(|id| id.0);
        let targets: Vec<AcOnlineTarget> = player_ids
            .into_iter()
            .filter_map(|id| {
                let session_id = runtime.player_for_character(id)?.anticheat_session_id?;
                let name = world.characters.get(&id)?.name.clone();
                Some(AcOnlineTarget { name, session_id })
            })
            .collect();
        if lower == "acstats" {
            world.queue_ac_stats_lookup(character_id, targets);
        } else if lower == "aclist" {
            world.queue_ac_list_lookup(character_id, targets);
        } else {
            world.queue_ac_suspicious_lookup(character_id, targets);
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#accleanup <days>` (`command.c:10314-10319` dispatch, `CF_GOD`-
    // only, unlike its `CF_GOD|CF_STAFF` siblings above; `ac_cmd_cleanup`,
    // `anticheat.c:1267-1285`). A pure maintenance action with no name to
    // resolve, so - unlike `#acstatus`/`#acstats`/`#aclist`/`#acsuspicious`
    // - `days` is parsed and validated entirely synchronously here; only
    // the delete itself needs the async DB round trip (see
    // `apply_ac_cleanup_events`). C emits the "Cleaning up..." progress
    // line synchronously (its DB call is same-thread), so the immediate
    // reply below stands in for that line; the final "Cleanup complete"
    // line is queued separately once the async delete finishes.
    if lower == "accleanup" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let days_str = rest.trim_start();
        if days_str.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #accleanup <days>".to_string(),
                    "Deletes AC records older than <days> days.".to_string(),
                ],
                ..Default::default()
            }));
        }
        let days =
            legacy_atoi_prefix(days_str).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if days < 7 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Minimum retention is 7 days.".to_string()],
                ..Default::default()
            }));
        }
        world.queue_ac_cleanup_lookup(character_id, days);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("Cleaning up records older than {days} days...")],
            ..Default::default()
        }));
    }

    // C `#acreset <player>` (`command.c:10157-10165` dispatch, `CF_GOD`-
    // only, exact-word; `ac_cmd_reset`, `anticheat.c:527-561`). Same
    // single-name-target resolution as `#acstatus` above (online-
    // `CF_PLAYER`-name scan, ascending-id tiebreak, then
    // `PlayerRuntime::anticheat_session_id` lookup), but the DB half is a
    // mutation, not a read - see `apply_ac_reset_events` for the
    // confirmation message, which is queued only after the reset
    // actually succeeds (this codebase has no synchronous in-memory
    // `player[nr]->ac` struct to mutate directly, unlike C, whose
    // "Reset anti-cheat data for %s." reply is unconditional and
    // same-thread).
    if lower == "acreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acreset <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_reset_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acflag <player>` (`command.c:10167-10174` dispatch, `CF_GOD|
    // CF_STAFF`-gated, exact-word; `ac_cmd_flag`, `anticheat.c:568-593`).
    // Same single-name-target resolution as `#acstatus`/`#acreset` above
    // (online-`CF_PLAYER`-name scan, ascending-id tiebreak, then
    // `PlayerRuntime::anticheat_session_id` lookup); the DB half sets
    // `status` to `AC_STATUS_FLAGGED` rather than resetting counters -
    // see `apply_ac_flag_events` for the confirmation message, queued
    // only after the mutation actually succeeds (C's own reply is
    // unconditional and same-thread, mutating an in-memory struct that
    // always exists once a connection does).
    if lower == "acflag" {
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
                messages: vec!["Usage: #acflag <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_flag_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acunflag <player>` (`command.c:10196-10203` dispatch, `CF_GOD`-
    // only, unlike `#acflag`'s `CF_GOD|CF_STAFF` - exact-word; `ac_cmd_
    // unflag`, `anticheat.c:790-823`). Same single-name-target resolution
    // as `#acflag`/`#acreset` above; the "is not flagged" status gate
    // itself can't happen here (this codebase only knows the session id
    // exists synchronously, not its current status) - see
    // `apply_ac_unflag_events` for that check and the confirmation
    // message.
    if lower == "acunflag" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acunflag <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_unflag_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#actrust <player>` (`command.c:10205-10213` dispatch, `CF_GOD`-
    // only, exact-word; `ac_cmd_trust`, `anticheat.c:827-849`). Same
    // single-name-target resolution as `#acflag`/`#acunflag` above; no
    // status gate (C's own handler has none) - see `apply_ac_trust_events`
    // for the `ac_player_stats.is_trusted` mutation and confirmation
    // message.
    if lower == "actrust" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #actrust <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_trust_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acuntrust <player>` (`command.c:10214-10222` dispatch, `CF_GOD`-
    // only, exact-word; `ac_cmd_untrust`, `anticheat.c:860-882`). Same
    // single-name-target resolution as `#actrust` above; the "untrust"
    // mirror of `apply_ac_trust_events`.
    if lower == "acuntrust" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acuntrust <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_untrust_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acwatch <player>` (`command.c:10223-10231` dispatch, `CF_GOD|
    // CF_STAFF`-gated, exact-word; `ac_cmd_watch`, `anticheat.c:894-921`).
    // Purely in-memory in C (toggles `player[nr]->ac.watch_mode`) and
    // stays purely in-memory here too - see `PlayerRuntime::
    // ac_watch_enabled`'s doc comment for why the flag currently has no
    // other effect beyond the toggle message. Unlike every other member
    // of this family this needs no DB round trip at all (the target's
    // `PlayerRuntime` is mutated directly), so it replies synchronously.
    if lower == "acwatch" {
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
                messages: vec!["Usage: #acwatch <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        target_player.ac_watch_enabled = !target_player.ac_watch_enabled;
        let message = if target_player.ac_watch_enabled {
            format!("Now watching {target_name} - detailed AC logging enabled.")
        } else {
            format!("Stopped watching {target_name}.")
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        }));
    }

    // C `#acwarn <player> [reason]` (`command.c:10323-10329` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_warn`, `anticheat.c:
    // 1291-1314`). Same single-name-target resolution as `#acflag`/
    // `#acwatch` above, but keeps `target_id` around too (not just
    // `target_name`/`session_id`) since the target itself, not just the
    // caller, receives a message - see `apply_ac_warn_events` for the
    // subscriber-id resolution and all four reply lines. Name/reason
    // split reproduces C's `sscanf(args, "%39s %255[^\n]", target,
    // reason)` (first whitespace-delimited token, capped at 39 chars, as
    // the name; the rest of the line, capped at 255 chars, as the
    // reason) with `reason`'s C-side pre-seeded default ("Anti-cheat
    // warning") applied here when the second token is absent/empty.
    if lower == "acwarn" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        if rest.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acwarn <player> [reason]".to_string()],
                ..Default::default()
            }));
        }
        let mut parts = rest.splitn(2, char::is_whitespace);
        let name: String = parts.next().unwrap_or("").chars().take(39).collect();
        let reason_raw = parts.next().unwrap_or("").trim_start();
        let reason: String = if reason_raw.is_empty() {
            "Anti-cheat warning".to_string()
        } else {
            reason_raw.chars().take(255).collect()
        };
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(&name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_warn_lookup(character_id, target_id, target_name, session_id, reason);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsessions <player>` (`command.c:10241-10249` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_sessions`, `anticheat.c:
    // 975-1017`). Same single-name-target resolution as `#acwarn`/
    // `#actrust` above (online `CF_PLAYER` name scan, first match by
    // ascending character id, then `PlayerRuntime::anticheat_session_id`)
    // - see `apply_ac_sessions_events` for the subscriber-id resolution
    // and the recent-session-history query itself.
    if lower == "acsessions" {
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
                messages: vec!["Usage: #acsessions <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_sessions_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acviolations <player>` (`command.c:10250-10255` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_violations`,
    // `anticheat.c:1019-1053`). Identical single-name-target resolution
    // shape to `#acsessions` right above - see `apply_ac_violations_events`
    // for the subscriber-id resolution and the recent-violation-history
    // query itself.
    if lower == "acviolations" {
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
                messages: vec!["Usage: #acviolations <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_violations_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#achistory <player>` (`command.c:10232-10239` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_history`, `anticheat.c:
    // 924-972`). Identical single-name-target resolution shape to
    // `#acsessions`/`#acviolations` above - see `apply_ac_history_events`
    // for the subscriber-id resolution and the lifetime `ac_player_stats`
    // rollup read itself.
    if lower == "achistory" {
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
                messages: vec!["Usage: #achistory <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_history_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsharedip <player>` (`command.c:10259-10267` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_sharedip`, `anticheat.
    // c:1058-1088`). Identical single-name-target resolution shape to
    // `#acsessions`/`#acviolations`/`#achistory` above - see
    // `apply_ac_sharedip_events` for the subscriber-id resolution and the
    // shared-IP query itself.
    if lower == "acsharedip" {
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
                messages: vec!["Usage: #acsharedip <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_sharedip_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsharedhw <player>` (`command.c:10268-10276` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_sharedhw`, `anticheat.
    // c:1096-1126`). Identical single-name-target resolution shape to
    // `#acsharedip` above - see `apply_ac_sharedhw_events` for the
    // subscriber-id resolution and the shared-hardware query itself.
    if lower == "acsharedhw" {
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
                messages: vec!["Usage: #acsharedhw <player>".to_string()],
                ..Default::default()
            }));
        }
        let mut candidates: Vec<&Character> = world
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        let Some(target_id) = candidates.first().map(|character| character.id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            }));
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            }));
        };
        world.queue_ac_sharedhw_lookup(character_id, target_name, session_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#achighrisk` (`command.c:10277-10280` dispatch, `CF_GOD|
    // CF_STAFF`-gated, exact-word; `ac_cmd_highrisk`, `anticheat.c:1134-
    // 1157`). No player name to resolve - same no-target shape as
    // `#acsiglist` below, so this simply queues a caller id and lets
    // `apply_ac_highrisk_events` list every high-risk `ac_player_stats`
    // row.
    if lower == "achighrisk" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        world.queue_ac_highrisk_lookup(character_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#aclookup <subscriber_id>` (`command.c:10282-10289` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_lookup`, `anticheat.c:
    // 1158-1191`). Unlike every other member of this family, the target
    // is a raw numeric subscriber (account) id (C's own `atoi(id_str)`),
    // not an online character name - parsed and range-checked (`<= 0`
    // rejected, matching C's own check) directly here, with no online-
    // name-scan at all.
    if lower == "aclookup" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let id_str = rest.trim_start();
        if id_str.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #aclookup <subscriber_id>".to_string()],
                ..Default::default()
            }));
        }
        let subscriber_id = legacy_atoi_prefix(id_str);
        if subscriber_id <= 0 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid subscriber ID.".to_string()],
                ..Default::default()
            }));
        }
        world.queue_ac_lookup_lookup(character_id, subscriber_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsiglist` (`command.c:10291-10294` dispatch, `CF_GOD`-only,
    // exact-word; `ac_cmd_siglist`, `anticheat.c:1192-1215`). No player
    // name to resolve - unlike every other command in this file except
    // `#accleanup` - so this simply queues a caller id and lets `apply_
    // ac_siglist_events` list every row in the new `ac_known_signatures`
    // table (`migrations/0016_ac_known_signatures.sql`).
    if lower == "acsiglist" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        world.queue_ac_siglist_lookup(character_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsigadd <type> <value> <name>` (`command.c:10296-10302`
    // dispatch, `CF_GOD`-only, exact-word; `ac_cmd_sigadd`, `anticheat.c:
    // 1216-1245`). Reproduces C's `sscanf(args, "%31s %255s %63[^\n]",
    // type, value, name)` three-token parse: `type`/`value` are the
    // first two whitespace-delimited tokens, `name` is everything after
    // the second token's trailing whitespace run (so it may itself
    // contain spaces, unlike `type`/`value`), each truncated to the same
    // buffer sizes C's stack arrays hold (31/255/63 bytes). `type` is
    // then checked against the same fixed five-member allow-list C's
    // `strcmp` chain checks, case-sensitively (no `to_ascii_lowercase`
    // anywhere in the C original). The DB insert itself is async (see
    // `apply_ac_sigadd_events`), so - unlike C's own unconditional,
    // same-thread "Added signature: ..." reply - the confirmation is
    // only sent once that insert actually succeeds.
    if lower == "acsigadd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let args = rest.trim_start();
        if args.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #acsigadd <type> <value> <name>".to_string(),
                    "Types: hardware_hash, code_hash, dll_hash, process_name, hardware_id"
                        .to_string(),
                ],
                ..Default::default()
            }));
        }
        let Some((sig_type, sig_value, name)) = parse_ac_sigadd_args(args) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acsigadd <type> <value> <name>".to_string()],
                ..Default::default()
            }));
        };
        const VALID_SIGNATURE_TYPES: [&str; 5] = [
            "hardware_hash",
            "code_hash",
            "dll_hash",
            "process_name",
            "hardware_id",
        ];
        if !VALID_SIGNATURE_TYPES.contains(&sig_type.as_str()) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Invalid type. Use: hardware_hash, code_hash, dll_hash, process_name, \
                     hardware_id"
                        .to_string(),
                ],
                ..Default::default()
            }));
        }
        let created_by = caller.name.clone();
        world.queue_ac_sigadd_lookup(character_id, sig_type, sig_value, name, created_by);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `#acsigdel <id>` (`command.c:10305-10311` dispatch, `CF_GOD`-only,
    // exact-word; `ac_cmd_sigdel`, `anticheat.c:1246-1266`). `id` is
    // parsed with the same `atoi` + `== 0` invalid-id rejection C uses
    // (C then casts to `unsigned int`, so a negative input wraps around
    // to a huge, practically-never-matching id rather than being
    // rejected outright; this port instead keeps the parsed value as a
    // signed `i64` and lets the DB lookup's own "not found" branch
    // handle it - functionally equivalent, since a negative id can never
    // match a `bigserial` primary key either way, without needing to
    // replicate the exact wrapped bit pattern).
    if lower == "acsigdel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let id_str = rest.trim_start();
        if id_str.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: #acsigdel <id>".to_string()],
                ..Default::default()
            }));
        }
        let signature_id = legacy_atoi_prefix(id_str);
        if signature_id == 0 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid signature ID.".to_string()],
                ..Default::default()
            }));
        }
        world.queue_ac_sigdel_lookup(character_id, signature_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    ControlFlow::Continue(())
}

/// `#acsigadd <type> <value> <name>`'s argument parse (`ac_cmd_sigadd`'s
/// `sscanf(args, "%31s %255s %63[^\n]", type, value, name)`,
/// `anticheat.c:1223-1227`): `type`/`value` are the first two
/// whitespace-delimited tokens (any run of whitespace between/around
/// them is skipped, matching scanf's own `" "` conversion-skip
/// semantics), `name` is everything remaining after the second token's
/// trailing whitespace run - unlike `type`/`value`, it may itself contain
/// spaces, since `%63[^\n]` matches everything up to a newline, not just
/// up to the next space. Each token is truncated to the same buffer size
/// (minus the null terminator) C's local stack arrays hold. Returns
/// `None` when fewer than three tokens are present, matching `sscanf`
/// returning `< 3`.
pub(crate) fn parse_ac_sigadd_args(args: &str) -> Option<(String, String, String)> {
    let trimmed = args.trim_start();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let sig_type = parts.next().unwrap_or("");
    if sig_type.is_empty() {
        return None;
    }
    let after_type = parts.next().unwrap_or("").trim_start();
    let mut parts = after_type.splitn(2, char::is_whitespace);
    let sig_value = parts.next().unwrap_or("");
    if sig_value.is_empty() {
        return None;
    }
    let name = parts.next().unwrap_or("").trim_start();
    if name.is_empty() {
        return None;
    }
    Some((
        legacy_truncate_c_string(sig_type, 31),
        legacy_truncate_c_string(sig_value, 255),
        legacy_truncate_c_string(name, 63),
    ))
}
