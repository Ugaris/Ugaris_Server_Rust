use super::*;

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

/// C `/lastseen <name>` (`command.c:9027-9046`), `cmdcmp(ptr, "lastseen",
/// 4)` so any prefix from `"last"` up to the full word matches, case-
/// insensitively, no permission gate (every player can use it). Trims
/// only leading whitespace off the argument (`while (isspace(*ptr))
/// ptr++;`, `command.c:9033-9035`) before handing it to `World::
/// queue_lastseen_lookup`, which does all further validation and DB
/// resolution - see that function's and `ugaris-server`'s
/// `apply_lastseen_events`'s doc comments for the full behavior. Always
/// returns a `default()` result immediately; the real reply arrives
/// later via `World::queue_system_text`, matching C's own fire-and-
/// forget async `lastseen()`/`db_lastseen()` DB-worker round-trip.
pub(crate) fn apply_lastseen_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.len() < 4 || !"lastseen".starts_with(&verb.to_ascii_lowercase()) {
        return None;
    }
    world.queue_lastseen_lookup(character_id, rest.trim_start());
    Some(KeyringCommandResult::default())
}

/// C `/complain <name> [reason...]` (`command.c:8769-8776`, `cmdcmp(ptr,
/// "complain", 4)` so any prefix from `"comp"` up to the full word
/// matches, case-insensitively, no permission gate), dispatching to
/// `cmd_complain` (`system/command.c:2281-2352`). Trims only leading
/// whitespace off the argument, matching the dispatcher's own `while
/// (isspace(*ptr)) ptr++;`.
///
/// Every branch that needs only the caller's own state is handled
/// synchronously here, in C source order:
/// - empty argument -> the "need at least the name" message, no PPD
///   write.
/// - `misc_ppd.complaint_date() == 0` (never seen the disclaimer) -> the
///   one-time `COL_LIGHT_RED` disclaimer, stamping `complaint_date = 1`
///   so a repeated invocation passes this gate.
/// - non-`CF_GOD` caller within 60 seconds of the last `complaint_date`
///   stamp -> the rate-limit message, *also* restamping `complaint_date
///   = realtime` - a genuine C quirk (`command.c:2306-2309`) that resets
///   the cooldown window on every rejected retry, not just on a
///   successful complaint; preserved as-is.
/// - the parsed name (`isalpha` run, capped at 75 bytes) matching
///   `"lag"`/`"laggy"` -> the lag-specific rejection, no PPD write.
/// - the parsed name matching `"bug"`/`"why"`/`"the"`/`"too"`/`"this"`/
///   `"can"` -> the generic "no player by that name" rejection, no PPD
///   write.
///
/// Anything else is handed to `World::queue_complain_lookup`, which
/// applies C's own tighter `3..=40` length bound and (if it passes)
/// queues the DB round trip resolved by `ugaris-server`'s
/// `apply_complain_events` - see that function's doc comment for the
/// success/failure reply shapes and the deferred `complaint_date =
/// realtime` stamp on success. C's `write_scrollback` (emailing the
/// complaint to `game@ugaris.com`) has no Rust equivalent (no email/CURL
/// infra exists in this codebase - the same established omission as
/// `/kick`'s `dlog`).
pub(crate) fn apply_complain_command(
    world: &mut World,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
    is_god: bool,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if verb.len() < 4 || !"complain".starts_with(&verb.to_ascii_lowercase()) {
        return None;
    }
    let rest = rest.trim_start();
    let realtime_seconds = realtime_seconds.min(i32::MAX as u64) as i32;

    if rest.is_empty() {
        return Some(KeyringCommandResult {
            messages: vec![
                "Sorry, you need to enter at least the name of the player you're complaining about."
                    .to_string(),
            ],
            ..Default::default()
        });
    }

    if player.complaint_date() == 0 {
        player.record_complaint(1);
        return Some(KeyringCommandResult {
            message_bytes: vec![legacy_light_red_text_bytes(
                "Complaints are meant as a way to complain about verbal attacks by another \
                 player, or to report a scam. If you wish to complain about something else, \
                 please email game@ugaris.com. No complaint has been sent. Repeat the command \
                 if you still want to send your complaint.",
            )],
            ..Default::default()
        });
    }

    if !is_god && realtime_seconds - player.complaint_date() < 60 {
        player.record_complaint(realtime_seconds);
        return Some(KeyringCommandResult {
            messages: vec![
                "Sorry, we do not accept more than one complaint per minute.".to_string(),
            ],
            ..Default::default()
        });
    }

    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .take(75)
        .collect();
    let lower = name.to_ascii_lowercase();
    if lower == "lag" || lower == "laggy" {
        return Some(KeyringCommandResult {
            messages: vec![
                "Sorry, the complaint command is meant to complain about players, not lag."
                    .to_string(),
            ],
            ..Default::default()
        });
    }
    if matches!(
        lower.as_str(),
        "bug" | "why" | "the" | "too" | "this" | "can"
    ) {
        return Some(KeyringCommandResult {
            messages: vec![format!("Sorry, no player by the name '{name}' found.")],
            ..Default::default()
        });
    }

    world.queue_complain_lookup(character_id, &name);
    Some(KeyringCommandResult::default())
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
    let verb = legacy_pk_command_verb(verb)?;
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

/// C `/steal` (`command.c:9732-9735`, `cmdcmp(ptr, "steal", 5)`, no
/// permission gate - any player can try) dispatching unconditionally to
/// `cmd_steal` (`src/system/prof.c:106-222`). All of the actual game logic
/// lives in [`ugaris_core::world::World::attempt_steal`]; this function
/// only turns its [`ugaris_core::world::StealOutcome`] into player-facing
/// text/bytes and applies the one epiphenomenon `World` can't own itself:
/// the `add_pk_steal` PK-ppd bump (C `prof.c:226`), which lives on
/// `PlayerRuntime` in `ugaris-server`, not `World`.
pub(crate) fn apply_steal_command(
    world: &mut World,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
    realtime_seconds: u64,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    // C `cmdcmp(ptr, "steal", 5)`: `minlen == strlen("steal")`, so this is
    // an exact case-insensitive word match, not a prefix abbreviation.
    if !verb.eq_ignore_ascii_case("steal") {
        return None;
    }

    let outcome = world.attempt_steal(character_id);

    let self_message = match &outcome {
        StealOutcome::NotAThief => Some("You are not a thief, you cannot steal.".to_string()),
        StealOutcome::NotIdle => Some("You can only steal when standing still.".to_string()),
        StealOutcome::HandFull => Some("Please free your hand (mouse cursor) first.".to_string()),
        StealOutcome::OutOfMap => Some("Out of map.".to_string()),
        StealOutcome::NoOneThere => Some("There's no one to steal from.".to_string()),
        StealOutcome::CannotAttack => {
            Some("You cannot steal from someone you are not allowed to attack.".to_string())
        }
        StealOutcome::ArenaOrClan => Some("You cannot steal inside an arena.".to_string()),
        StealOutcome::NotAPlayer => Some("You can only steal from players.".to_string()),
        StealOutcome::Lagging => Some("You cannot steal from lagging players.".to_string()),
        StealOutcome::LiveQuests => Some("You cannot steal in Live Quests.".to_string()),
        StealOutcome::VictimBusy => {
            Some("You cannot steal from someone if your victim is not standing still.".to_string())
        }
        StealOutcome::NothingToSteal => Some("You could not find anything to steal.".to_string()),
        StealOutcome::WouldBeCaught => {
            Some("You'd get caught for sure. You decide not to try.".to_string())
        }
        // C: `destroy_item(in); elog(...); return;` - no message at all.
        StealOutcome::ItemLostSilently => None,
        StealOutcome::Caught { victim_name, .. } => Some(format!(
            "{victim_name} noticed your attempt and stopped you from stealing."
        )),
        StealOutcome::StolenNoticed {
            victim_name,
            item_name,
            ..
        } => Some(format!(
            "{victim_name} noticed your theft, but you managed to steal a {item_name} anyway."
        )),
        StealOutcome::StolenUnnoticed {
            victim_name,
            item_name,
        } => Some(format!(
            "You stole a {item_name} without {victim_name} noticing."
        )),
    };

    let mut result = KeyringCommandResult {
        messages: self_message.into_iter().collect(),
        ..Default::default()
    };

    let attacker_name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
        .unwrap_or_default();

    match &outcome {
        StealOutcome::Caught { victim_id, .. } => {
            result.target_message_bytes.push((
                *victim_id,
                legacy_light_red_text_bytes(&format!("{attacker_name} tried to steal from you!")),
            ));
        }
        StealOutcome::StolenNoticed {
            victim_id,
            item_name,
            ..
        } => {
            result.target_message_bytes.push((
                *victim_id,
                legacy_light_red_text_bytes(&format!("{attacker_name} stole your {item_name}!")),
            ));
        }
        _ => {}
    }

    if matches!(
        outcome,
        StealOutcome::StolenNoticed { .. } | StealOutcome::StolenUnnoticed { .. }
    ) {
        result.inventory_changed = true;
        let is_pk = world
            .characters
            .get(&character_id)
            .is_some_and(|character| {
                character
                    .flags
                    .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
            });
        if is_pk {
            player.add_pk_steal(realtime_seconds);
        }
    }

    Some(result)
}
