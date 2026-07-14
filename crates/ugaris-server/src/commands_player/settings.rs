use super::*;

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

/// C `/swap` (`command.c:8985-8988`, `cmdcmp(ptr, "swap", 0)`): swaps
/// places with the character directly in front, via [`World::char_swap`]
/// (already ported for the walk-into-someone auto-swap mechanic,
/// `world/actions.rs`'s `walk_swap_or_use_driver`). C's `minlen` of `0`
/// technically lets any non-empty prefix match, but which prefix length
/// actually reaches this `cmdcmp` call depends on the exact order of the
/// ~9000-line `command.c` if-chain (many other minlen-0 commands, e.g.
/// `say`/`shout`, are checked first and would swallow a bare `/s`); since
/// this port's dispatcher doesn't replicate that whole chain, only the
/// full word is accepted here, matching the same simplification already
/// used for the other minlen-0 chat commands (see
/// `commands_chat.rs::LocalSpeechKind::from_verb`). On success, stamps
/// `PlayerRuntime::record_swap` (C `ppd->swapped = realtime;`,
/// `do.c:1671-1673`). Neither C nor this port reports anything to the
/// player on success or failure (the C caller never inspects `char_swap`'s
/// `error`/return value).
pub(crate) fn apply_swap_command(
    world: &mut World,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("swap") {
        return None;
    }

    if world.char_swap(character_id) {
        let realtime_seconds = (world.tick.0 / TICKS_PER_SECOND) as i32;
        player.record_swap(realtime_seconds);
    }
    Some(KeyringCommandResult::default())
}

/// C `/logout` (`command.c:9737-9740` dispatch -> `cmd_logout`,
/// `player.c:4457-4471`), no permission gate but `minlen=6` so the full
/// word must be typed (`cmdcmp(ptr, "logout", 6)`, no abbreviation). Only
/// works while standing on a blue square (`MF_RESTAREA`); otherwise C logs
/// "You are not on a blue square." and does nothing else. On success C
/// silently (no `log_char` feedback) calls `exit_char` (save+despawn at the
/// character's rest position) then `player_client_exit` (send `SV_EXIT`
/// with reason text, drop the connection) - the actual save/despawn/
/// disconnect side effects are session-level and performed by the call
/// site when `logout_requested` is set; this function only validates the
/// blue-square precondition, matching C's own gate check.
pub(crate) fn apply_logout_command(
    world: &World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("logout") {
        return None;
    }

    let character = world.characters.get(&character_id)?;
    let on_rest_area = world
        .map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::RESTAREA));

    if !on_rest_area {
        return Some(KeyringCommandResult {
            messages: vec!["You are not on a blue square.".to_string()],
            ..Default::default()
        });
    }

    Some(KeyringCommandResult {
        logout_requested: true,
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

/// One entry of the `/noball`, `/nobless`, ..., `/autobless`, `/autopulse`
/// lag-control/automation toggle family (`command.c:9397-9591`): 16
/// `cmdcmp(ptr, "<name>", 5)`-gated toggles of a `struct lostcon_ppd`
/// (`src/module/lostcon.h:18-36`) boolean field, each followed by
/// `show_lostconppd` (ported as [`apply_status_command`]). `/autoturn` is
/// the 17th member of the same C family but already has its own
/// [`apply_autoturn_command`] (pre-existing), so it's excluded here to
/// avoid a duplicate dispatch entry.
struct LagControlToggleSpec {
    command: &'static str,
    field: fn(&mut PlayerRuntime) -> &mut bool,
}

const LAG_CONTROL_TOGGLE_SPECS: &[LagControlToggleSpec] = &[
    LagControlToggleSpec {
        command: "noball",
        field: |player| &mut player.no_ball,
    },
    LagControlToggleSpec {
        command: "nobless",
        field: |player| &mut player.no_bless,
    },
    LagControlToggleSpec {
        command: "nofireball",
        field: |player| &mut player.no_fireball,
    },
    LagControlToggleSpec {
        command: "noflash",
        field: |player| &mut player.no_flash,
    },
    LagControlToggleSpec {
        command: "nofreeze",
        field: |player| &mut player.no_freeze,
    },
    LagControlToggleSpec {
        command: "noheal",
        field: |player| &mut player.no_heal,
    },
    LagControlToggleSpec {
        command: "noshield",
        field: |player| &mut player.no_shield,
    },
    LagControlToggleSpec {
        command: "nowarcry",
        field: |player| &mut player.no_warcry,
    },
    LagControlToggleSpec {
        command: "nolife",
        field: |player| &mut player.no_life,
    },
    LagControlToggleSpec {
        command: "nomana",
        field: |player| &mut player.no_mana,
    },
    LagControlToggleSpec {
        command: "nocombo",
        field: |player| &mut player.no_combo,
    },
    LagControlToggleSpec {
        command: "nomove",
        field: |player| &mut player.no_move,
    },
    LagControlToggleSpec {
        command: "norecall",
        field: |player| &mut player.no_recall,
    },
    LagControlToggleSpec {
        command: "nopulse",
        field: |player| &mut player.no_pulse,
    },
    LagControlToggleSpec {
        command: "autobless",
        field: |player| &mut player.autobless_enabled,
    },
    LagControlToggleSpec {
        command: "autopulse",
        field: |player| &mut player.autopulse_enabled,
    },
];

pub(crate) fn apply_lag_control_toggle_command(
    character: &Character,
    player: &mut PlayerRuntime,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 5 {
        return None;
    }

    let spec = LAG_CONTROL_TOGGLE_SPECS
        .iter()
        .find(|spec| spec.command.starts_with(&lower))?;
    let field = (spec.field)(player);
    *field = !*field;
    apply_status_command(character, player, "/status")
}

/// C `/allowbless` (`command.c:9595-9600`), `cmdcmp(ptr, "allowbless", 5)`:
/// toggles `CF_NOBLESS` (inverted display: the flag means "don't allow",
/// the command name means "allow") then re-shows the lag-control panel via
/// `show_lostconppd`/[`apply_status_command`].
pub(crate) fn apply_allowbless_command(
    world: &mut World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 5 || !"allowbless".starts_with(&lower) {
        return None;
    }

    let character = world.characters.get_mut(&character_id)?;
    character.flags.toggle(CharacterFlags::NOBLESS);
    apply_status_command(character, player, "/status")
}

/// C `/killbless` (`command.c:9605-9617`), `cmdcmp(ptr, "killbless", 5)`:
/// no permission gate, any prefix from `"killb"` (5 chars, the `minlen`)
/// up to the full word matches, case-insensitively. Destroys the
/// caller's own bless spell item, if any (see `World::kill_bless_item`'s
/// doc comment for the exact scan/removal behavior), logging "Done." on
/// success or "No Bless found." otherwise.
pub(crate) fn apply_killbless_command(
    world: &mut World,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    if lower.len() < 5 || !"killbless".starts_with(&lower) {
        return None;
    }

    let destroyed = world.kill_bless_item(character_id);
    Some(KeyringCommandResult {
        messages: vec![if destroyed {
            "Done.".to_string()
        } else {
            "No Bless found.".to_string()
        }],
        inventory_changed: destroyed,
        ..Default::default()
    })
}
