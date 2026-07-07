use super::*;

/// C `cmd_milinfo`/`cmd_milpref`/`cmd_milsolve`'s local `diff_names[5]`
/// table (`command.c:5077,5175,5504`), letter for letter.
pub(crate) const MILITARY_DIFFICULTY_NAMES: [&str; 5] =
    ["easy", "normal", "hard", "impossible", "insane"];

/// C `mis[].type`'s 1/2/3 -> name mapping for an *active mission slot*
/// (`cmd_milinfo`/`cmd_milsolve`, `command.c:5113-5126,5554-5566`) - unlike
/// the preference-display mapping below, an out-of-range type here means
/// "Unknown" (defensive default for corrupt/impossible slot data), not
/// "no preference set".
pub(crate) fn military_mission_slot_type_name(mission_type: i32) -> &'static str {
    match mission_type {
        1 => "Demon",
        2 => "Ratling",
        3 => "Silver",
        _ => "Unknown",
    }
}

/// C `mission_type_preference`'s 1/2/3 -> name mapping, used by
/// `cmd_milinfo`/`cmd_milpref` when displaying the *preference* (0 = "None"
/// really does mean "no preference set", `command.c:5142-5146,5229-5232`).
pub(crate) fn military_type_preference_name(type_preference: i32) -> &'static str {
    match type_preference {
        1 => "Demon",
        2 => "Ratling",
        3 => "Silver",
        _ => "None",
    }
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

/// Parses the `x y area` triple used by `/setjaillocation` and
/// `/setastonlocation` (C `command.c:8036-8050`/`8076-8090`): `atoi` at the
/// current pointer, then skip ascii digits, then skip whitespace, repeated
/// three times.
pub(crate) fn parse_legacy_xyz_triple(rest: &str) -> (i32, i32, i32) {
    let mut ptr = rest.trim_start();
    let x = legacy_atoi_prefix(ptr) as i32;
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    ptr = ptr.trim_start();
    let y = legacy_atoi_prefix(ptr) as i32;
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    ptr = ptr.trim_start();
    let area = legacy_atoi_prefix(ptr) as i32;
    (x, y, area)
}

pub(crate) fn apply_admin_character_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if let Some(result) = if world
        .characters
        .get(&character_id)
        .is_some_and(|caller| caller.flags.contains(CharacterFlags::GOD))
    {
        apply_legacy_tick_tuning_command(runtime, &lower, rest)
            .or_else(|| apply_legacy_communication_tuning_command(runtime, &lower, rest))
            .or_else(|| apply_legacy_game_settings_tuning_command(world, &lower, rest))
            .or_else(|| apply_global_settings_command(world, &lower))
    } else {
        None
    } {
        return Some(result);
    }

    if matches!(
        lower.as_str(),
        "setdecaytime"
            | "setplayerbodytime"
            | "setnpcbodytime"
            | "setnpcbodytimearea32"
            | "setrespawntime"
            | "setsewerrespawntime"
            | "setlagouttime"
            | "setregentime"
            | "sethollerdist"
            | "setshoutdist"
            | "setsaydist"
            | "setemotedist"
            | "setquietsaydist"
            | "setwhisperdist"
            | "sethollercost"
            | "setshoutcost"
            | "setsplots"
            | "setspmany"
            | "setspsome"
            | "setspfew"
            | "setsprare"
            | "setspultra"
            | "setorbrespawndays"
            | "setmaxjewelcount"
            | "settunnelexpdivider"
            | "settunnelmillexp"
            | "setraregolemchance"
            | "setdungeontime"
            | "setbranfoexpbase"
            | "setbranexpbase"
            | "setpentvismaxpents"
            | "setpentmaxpower"
            | "setmaxsilvergolemtype"
            | "setnormaldropchance"
            | "setraredropchance"
            | "setraredropmultiplier"
            | "setbasedropmultiplier"
            | "setleveldivisor"
            | "setraregolemboost"
            | "setgolemhpmultiplier"
            | "setdemonlordaccess"
            | "setsolvemaxdivisor"
            | "setdemonpowerdeduction"
            | "setpentvaluemultiplier"
            | "setpentworthdivisor"
            | "setluckypentchance"
            | "setpowerincrement"
            | "setpentmaxtraining"
            | "setpentrandomspawn"
            | "setpentspawncount"
            | "setexpsolve"
            | "setclanreflection"
            | "setmaxclanbonus"
            | "setjaillocation"
            | "setastonlocation"
            | "setspecialdropmult"
            | "setdropproblow"
            | "setdropprobmid"
            | "setdropprobhigh"
            | "reloadloot"
            | "setlootmod"
            | "global"
    ) {
        return None;
    }

    if lower.len() >= 4 && "prof".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["--- Profile ---".to_string(), "---------------".to_string()],
            ..Default::default()
        });
    }

    // C `/profinfo` (`command.c:7496-7500`, `cmdcmp(ptr, "profinfo", 5)`,
    // `CF_GOD`-gated). Distinct from `/prof`/`cmd_show_prof` above: C's
    // `profinfo` sends one header line to the player and then calls
    // `show_prof()` (`server.c:934-986`), which is entirely `xlog()`
    // console-only output - the caller never receives the actual
    // cycle-profiler dump. A faithful port is therefore just the header
    // line; there is also no Rust equivalent of the underlying `proftab`
    // rdtsc-cycle profiler to port even if C's player-facing behavior
    // were different.
    if lower.len() >= 5 && "profinfo".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["Profiling Information:".to_string()],
            ..Default::default()
        });
    }

    // C `/poolstats` (`command.c:7503-7506`, `cmdcmp(ptr, "poolstats", 5)`,
    // `CF_GOD`-gated). Same pattern as `/profinfo`: C sends one header
    // line to the player, then `log_connection_pool_state()`
    // (`database_connection_pool.c:23-37`) writes the actual pool
    // occupancy/request-counter data to the console via `xlog()` only -
    // the caller never sees it. C's connection pool is also a hand-rolled
    // fixed-size MySQL connection array with its own counters, not
    // analogous to sqlx's `PgPool` internals, so even a "richer than C"
    // version would need new instrumentation. Faithful port: header line
    // only.
    if lower.len() >= 5 && "poolstats".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["Connection Pool Statistics:".to_string()],
            ..Default::default()
        });
    }

    // C `/memstats` (`command.c:7476-7493`, `cmdcmp(ptr, "memstats", 5)`,
    // `CF_GOD`-gated). Unlike `/profinfo`/`/poolstats` above, C's
    // `memstats` sends every data line to the player via `log_char`, so a
    // faithful port needs real numbers, not just the header. C reports
    // live occupancy against fixed-capacity C arrays (`used_chars` of
    // `MAXCHARS`, `used_items` of `MAXITEM`, `used_effects` of
    // `MAXEFFECT`, `used_containers` of `MAXCONTAINER` - the first three
    // are runtime-configurable globals in the C oracle, not even C
    // compile-time constants), plus a heap-allocation byte counter
    // (`mem_usage`) and a pending-notify-message counter (`used_msgs`).
    // Rust's `World` has no fixed-capacity arrays at all (its character/
    // item/effect stores are unbounded `HashMap`s - see `world/mod.rs`),
    // so there is no "/MAX" denominator to report; the three occupancy
    // counts are reported here as plain live counts instead. `mem_usage`
    // and `used_msgs` have no Rust analogue whatsoever (no allocation-
    // tracking, no persistent notify-queue-depth concept - pending
    // notifications are drained to packets every tick, not held in a
    // countable queue), so both are reported as a fixed `0`, matching the
    // established "no real Rust equivalent -> always report the harmless
    // constant" convention (e.g. `#accleanup`'s always-`0` heartbeat-log
    // count, `world_events.rs`). `used_containers` has no dedicated Rust
    // collection either - `world/consistency.rs`'s doc comment: "is this
    // item a container" is derived from `Item.content_id != 0`, not a
    // separate store - so it is computed here the same way.
    if lower.len() >= 5 && "memstats".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let containers = world
            .items
            .values()
            .filter(|item| item.content_id != 0)
            .count();
        return Some(KeyringCommandResult {
            messages: vec![
                "Memory Usage Statistics:".to_string(),
                "Total memory usage: 0 KB".to_string(),
                format!("Characters: {} used", world.characters.len()),
                format!("Items: {} used", world.items.len()),
                format!("Effects: {} used", world.effects.len()),
                format!("Containers: {containers} used"),
                "Messages: 0 used".to_string(),
            ],
            ..Default::default()
        });
    }

    // C `/querystats` (`command.c:6588-6618`, `cmdcmp(ptr, "querystats",
    // 5)`, `CF_GOD`-gated). Unlike `/profinfo`/`/poolstats`/`/memstats`
    // above, this reply needs a live `PgCharacterRepository` read, which
    // this dispatcher has no access to - see `ugaris-core`'s
    // `world/querystats.rs` module doc comment for the full scoping
    // rationale (only `save_char_cnt`/`exit_char_cnt`/`load_char_cnt` are
    // tracked; every other C counter this command reads has no Rust
    // instrumentation) - so this just queues the lookup for
    // `apply_querystats_events` to resolve and reply via
    // `World::queue_system_text` once drained.
    if lower.len() >= 5 && "querystats".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        world.queue_querystats_lookup(character_id);
        return Some(KeyringCommandResult::default());
    }

    if lower.len() >= 6 && "staffcode".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, code_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let mut letters = code_text.trim_start().chars();
        let first = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let second = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let code = format!("{first}{second}");
        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let target_name = target.name.clone();
        target.staff_code = code.clone();
        runtime.staff_codes.insert(target_id, code.clone());
        return Some(KeyringCommandResult {
            messages: vec![format!("Set {target_name}'s staff code to {code}.")],
            ..Default::default()
        });
    }

    if lower == "reset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if target.values.len() < 2 {
            target
                .values
                .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
        }
        if target.values[1].len() < CHARACTER_VALUE_NAMES.len() {
            target.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
        for index in 0..=CharacterValue::Immunity as usize {
            let cap = if index <= CharacterValue::Strength as usize {
                10
            } else {
                1
            };
            if target.values[1][index] > cap {
                target.values[1][index] = cap;
            }
        }
        for value in [CharacterValue::Rage, CharacterValue::Duration] {
            let index = value as usize;
            if target.values[1][index] > 1 {
                target.values[1][index] = 1;
            }
        }
        target.exp_used = 0;
        target.flags.insert(CharacterFlags::UPDATE);
        return Some(KeyringCommandResult {
            inventory_changed: target_id == character_id,
            name_changed: target_id == character_id,
            ..Default::default()
        });
    }

    if lower == "resetgift" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, area_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let area_id = legacy_atoi_prefix(area_text.trim_start());
        if !(0..=63).contains(&area_id) {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid area ID. Must be between 0 and 63.".to_string()],
                ..Default::default()
            });
        }

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Could not retrieve player data.".to_string()],
                ..Default::default()
            });
        };
        let was_set = target_player.xmas_tree_marked(area_id as u16);
        target_player.unmark_xmas_tree(area_id as u16);
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.as_str())
            .unwrap_or(name);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset gift flag for {} in area {} (was {}).",
                target_name,
                area_id,
                if was_set { "set" } else { "not set" }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "questlog".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let name = rest
            .trim_start()
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target_name) = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target_player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Failed to get quest data for {target_name}")],
                ..Default::default()
            });
        };

        let mut messages = vec![format!("Quest log for {target_name}:")];
        for (quest_id, entry) in target_player.quest_log.entries().iter().enumerate() {
            if entry.flags != 0 {
                messages.push(format!(
                    "Quest #{}: {}, Done level: {}",
                    quest_id,
                    if (entry.flags & QF_OPEN) != 0 {
                        "Open"
                    } else {
                        "Closed"
                    },
                    entry.done
                ));
            }
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "listitem".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let item_id = ItemId(legacy_atoi_prefix(rest).max(0) as u32);
        let Some(item) = world.items.get(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid item number or item doesn't exist".to_string()],
                ..Default::default()
            });
        };

        let mut messages = vec![
            format!("Item #{}: {}", item.id.0, item.name),
            format!("Description: {}", item.description),
            format!("Flags: 0x{:x}", item.flags.bits()),
            format!(
                "Driver: {}, ID: {}, Sprite: {}",
                item.driver, item.template_id, item.sprite
            ),
        ];
        if let Some(carried_by) = item.carried_by {
            let carrier_name = world
                .characters
                .get(&carried_by)
                .map(|character| character.name.as_str())
                .unwrap_or("Unknown");
            messages.push(format!("Carried by: {} ({})", carrier_name, carried_by.0));
        } else if item.x != 0 {
            messages.push(format!("Position: {},{}", item.x, item.y));
        }
        for n in 0..ugaris_core::entity::MAX_MODIFIERS {
            let modifier_index = item.modifier_index[n];
            if modifier_index != 0 {
                let skill_name = if modifier_index > 0 {
                    value_name(modifier_index)
                } else {
                    "unknown"
                };
                messages.push(format!(
                    "Mod #{}: {:+} to {}",
                    n, item.modifier_value[n], skill_name
                ));
            }
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "setkarma".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let mut split = rest.splitn(2, char::is_whitespace);
        let name = split.next().unwrap_or_default();
        let karma_text = split.next().unwrap_or_default().trim_start();
        let karma =
            legacy_atoi_prefix(karma_text).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            });
        };
        let old_karma = target.karma;
        target.karma = karma;
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Changed {}'s karma from {} to {}",
                target.name, old_karma, target.karma
            )],
            ..Default::default()
        });
    }

    if lower == "setexpmod" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.exp_modifier;
            world.settings.exp_modifier = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Global experience modifier changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcoreexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.hardcore_exp_bonus;
            world.settings.hardcore_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcoremilexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            // C's `hardcore_military_exp_bonus` global is a single value
            // read directly by `give_military_pts`/`give_military_pts_no_npc`
            // (`tool.c:3249-3306`); stored on `world.settings` (like
            // `exp_modifier`/`hardcore_exp_bonus`) instead of `ServerRuntime`
            // so `World::give_military_pts` (`ugaris-core`, no `ServerRuntime`
            // access) can read the live-tunable value directly.
            let old_value = world.settings.hardcore_military_exp_bonus;
            world.settings.hardcore_military_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore military experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "sethardcorekillexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let value = legacy_atof_prefix(rest);
        if (1.0..=3.0).contains(&value) {
            let old_value = runtime.hardcore_kill_exp_bonus;
            runtime.hardcore_kill_exp_bonus = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore kill experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec!["Invalid value. Please specify a number between 1.0 and 3.0".to_string()],
            ..Default::default()
        });
    }

    if lower.len() >= 5 && "listchars".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let mut character_ids: Vec<_> = world.characters.keys().copied().collect();
        character_ids.sort_by_key(|id| id.0);

        let mut count = 0;
        let mut players = 0;
        let mut npcs = 0;
        let mut messages = vec!["Active characters:".to_string()];
        for id in character_ids {
            let Some(character) = world.characters.get(&id) else {
                continue;
            };
            if character.flags.is_empty() {
                continue;
            }
            count += 1;
            if character.flags.contains(CharacterFlags::PLAYER) {
                players += 1;
                messages.push(format!(
                    "Player: {:3} - {} (L{})",
                    id.0, character.name, character.level
                ));
            } else {
                npcs += 1;
                if count < 50 {
                    messages.push(format!(
                        "NPC:    {:3} - {} (L{}, D:{})",
                        id.0, character.name, character.level, character.driver
                    ));
                }
            }
        }
        messages.push(format!(
            "Total: {count} characters ({players} players, {npcs} NPCs)"
        ));
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/clearmerchantstores <id>` (`command.c:7510-7538`), `CF_GOD`-gated
    // (`cmdcmp(ptr, "clearmerchantstores", 10)`). Resets an online
    // merchant's inventory to empty and its gold to the default starting
    // amount (`ch[merchant_cn].gold = 10000`), matching C's
    // "Default starting gold" comment verbatim. Unlike C, which destroys
    // each carried item entity one at a time (`remove_item_char`/
    // `destroy_item` over `it[]`), the Rust `MerchantStore.wares` slots own
    // their `Item` data directly (no separate item-table entries to free),
    // so clearing is just overwriting every slot with `None`.
    if lower.len() >= 10 && "clearmerchantstores".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let merchant_id = CharacterId(legacy_atoi_prefix(rest.trim_start()).max(0) as u32);
        let Some(merchant) = world.characters.get(&merchant_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            });
        };
        if merchant.driver != CDR_MERCHANT {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            });
        }
        let merchant_name = merchant.name.clone();

        world.ensure_merchant_store(merchant_id);
        let Some(store) = world.merchant_stores.get_mut(&merchant_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            });
        };
        store.gold = 10_000;
        for ware in store.wares.iter_mut() {
            *ware = None;
        }

        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Merchant {} (ID: {}) inventory cleared and gold reset",
                merchant_name, merchant_id.0
            )],
            clear_merchant_store_requested: Some(merchant_id),
            ..Default::default()
        });
    }

    // C `/checksanity` (`command.c:7443-7457`), `CF_GOD`-gated
    // (`cmdcmp(ptr, "checksanity", 5)`). Runs the full self-healing
    // `consistency_check_*` sweep (`World::consistency_check`, see
    // `world/consistency.rs`'s module doc comment) and reports the same
    // four aggregate error counts C does. C's per-anomaly `elog` console
    // lines aren't reproduced (see that module's doc comment for the
    // established untracked-console-side-effect convention).
    if lower.len() >= 5 && "checksanity".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let report = world.consistency_check();
        return Some(KeyringCommandResult {
            messages: vec![
                "Running consistency checks...".to_string(),
                format!("Item errors: {}", report.item_errors),
                format!("Map errors: {}", report.map_errors),
                format!("Character errors: {}", report.char_errors),
                format!("Container errors: {}", report.container_errors),
                "Consistency check complete".to_string(),
            ],
            ..Default::default()
        });
    }

    if lower == "setskill" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (name, pos, val) = parse_setskill_args(rest);
        let Some(target_id) = find_online_character_by_name(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if pos < 0 || pos >= CHARACTER_VALUE_NAMES.len() as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Position out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if !(0..=255).contains(&val) {
            return Some(KeyringCommandResult {
                messages: vec!["Value out of bounds.".to_string()],
                ..Default::default()
            });
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        if target.values.len() < 2 {
            target
                .values
                .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
        }
        if target.values[1].len() < CHARACTER_VALUE_NAMES.len() {
            target.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
        let pos = pos as usize;
        let old_value = target.values[1][pos];
        let old_exp_used = target.exp_used;
        target.values[1][pos] = val as i16;
        target.exp_used = legacy_calc_exp_used(target);
        target.flags.insert(CharacterFlags::UPDATE);
        let diff = i64::from(target.exp_used) - i64::from(old_exp_used);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Skill: {} (pos {}), Old value: {}, New value: {}, exp used changed by {}.",
                value_name(pos as i16),
                pos,
                old_value,
                target.values[1][pos],
                diff
            )],
            inventory_changed: true,
            name_changed: target_id == character_id,
            ..Default::default()
        });
    }

    if lower == "setlevel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let level = legacy_atoi_prefix(rest).max(0) as u32;
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.level = level;
            character.exp = level2exp(level);
            if character.values.len() < 2 {
                character
                    .values
                    .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
            }
            if character.values[1].len() < CHARACTER_VALUE_NAMES.len() {
                character.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
            }

            if level < 30 {
                character.flags.remove(CharacterFlags::ARCH);
                character.values[1][CharacterValue::Duration as usize] = 0;
                character.values[1][CharacterValue::Rage as usize] = 0;
            }
            if level > 35 {
                character.flags.insert(CharacterFlags::ARCH);
                let mage_only = character.flags.contains(CharacterFlags::MAGE)
                    && !character.flags.contains(CharacterFlags::WARRIOR);
                let warrior_only = character.flags.contains(CharacterFlags::WARRIOR)
                    && !character.flags.contains(CharacterFlags::MAGE);
                if mage_only && character.values[1][CharacterValue::Duration as usize] == 0 {
                    character.values[1][CharacterValue::Duration as usize] = 1;
                }
                if warrior_only && character.values[1][CharacterValue::Rage as usize] == 0 {
                    character.values[1][CharacterValue::Rage as usize] = 1;
                }
            }
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        }
        world.clear_character_spell_slots_and_effects(character_id);
        return Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "exp".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }
        if exp != 0 {
            give_exp_with_runtime_modifiers(world, target_id, exp, area_id);
            let target = world
                .characters
                .get(&target_id)
                .expect("target just checked");
            return Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            });
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        });
    }

    if lower == "milexp" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }
        if exp != 0 {
            // C `cmd_milexp` -> `give_military_pts_no_npc(co, val, 1)`
            // (`command.c:3048`, `tool.c:3281-3299`): `pts` is the typed
            // amount (goes to `military_points`, hardcore-multiplied by
            // `hardcore_military_exp_bonus`), while `exps` is a *fixed* `1`
            // that routes through `give_exp` (and `normal_exp`), regardless
            // of the typed amount. `World::give_military_pts` (`crates/
            // ugaris-core/src/world/military.rs`) is the shared port of
            // `give_military_pts_no_npc` itself, including the rank-
            // promotion feedback text and the above-Sergeant-Major
            // server-wide "Grats:" broadcast that this call site previously
            // skipped entirely.
            let points = exp.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            world.give_military_pts(target_id, points, 1, area_id);
            let Some(target) = world.characters.get(&target_id) else {
                return Some(KeyringCommandResult::default());
            };
            return Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} military exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            });
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        });
    }

    // C `cmd_labsolved` (`command.c:3081-3130`, dispatched from
    // `/labsolved`, `command.c:6043-6046`, `CF_GOD`-gated): the same
    // self-or-named-target + trailing-value parsing shape as `/exp`/
    // `/milexp` above (empty text, or text starting with a digit, means
    // self and that leading text is the value; otherwise the first word is
    // a target name and the value follows it) - reused via the shared
    // `parse_exp_command_target` helper rather than re-deriving it. A
    // nonzero value toggles that lab number's solved bit (valid range
    // `1..=63`, XOR not OR - invoking it twice on the same number
    // un-solves it again) in `PlayerRuntime::lab_solved_bits`; a value
    // outside that range reports "Lab number is out of bounds." without
    // toggling anything. Either way (including a zero/absent value, which
    // is display-only) every currently-solved lab number is then listed,
    // one message per solved bit, lowest to highest. C's `cmdcmp(ptr,
    // "labsolved", 8)` accepts the 8-char prefix `labsolve` too, not just
    // the full 9-char word.
    if lower.len() >= 8 && "labsolved".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, val) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get lab data for {target_name}.")],
                ..Default::default()
            });
        };

        let mut messages = Vec::new();
        if val != 0 {
            if !(1..=63).contains(&val) {
                messages.push("Lab number is out of bounds.".to_string());
            } else {
                player.lab_solved_bits ^= 1u64 << val;
            }
        }
        for n in 0..64u32 {
            if player.lab_solved_bits & (1u64 << n) != 0 {
                messages.push(format!("{target_name} has solved lab {n}."));
            }
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milinfo` (`command.c:5071-5160`, `CF_GOD`-gated,
    // `command.c:10085-10091`): full-word `/milinfo [name]`, self if no
    // name given.
    if lower == "milinfo" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    });
                }
            }
        };
        let Some(target) = world.characters.get(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let target_name = target.name.clone();
        let military_points = target.military_points;
        let military_normal_exp = target.military_normal_exp;
        let current_yday = world.date.yday + 1;

        let Some(player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let mut messages = vec![
            format!("Military Info for {target_name}:"),
            format!(
                "Rank: {} (Military points: {military_points})",
                army_rank_name(army_rank_for_points(military_points))
            ),
            format!(
                "Current recommendation points: {}",
                player.military_current_pts()
            ),
            format!("Total military experience earned: {military_normal_exp}"),
        ];

        let took_mission = player.military_took_mission();
        if took_mission != 0 {
            let mission_idx = (took_mission - 1).max(0) as usize;
            let mission = player.military_mission(mission_idx);
            let type_str = military_mission_slot_type_name(mission.mission_type);
            let verb = if mission.mission_type == 3 {
                "Mining"
            } else {
                "Slaying"
            };
            let diff_name = MILITARY_DIFFICULTY_NAMES
                .get(mission_idx)
                .copied()
                .unwrap_or("Unknown");
            messages.push(format!(
                "Current mission: {type_str} {verb} (Difficulty: {diff_name})"
            ));
            if mission.mission_type == 1 || mission.mission_type == 2 {
                messages.push(format!(
                    "Target: {} level {} enemies",
                    mission.opt1, mission.opt2
                ));
            } else if mission.mission_type == 3 {
                messages.push(format!("Target: {} silver", mission.opt1));
            }
        } else {
            messages.push("No active mission".to_string());
        }

        let type_pref = player.mission_type_preference();
        messages.push(format!(
            "Mission type preference: {type_pref} ({})",
            military_type_preference_name(type_pref)
        ));

        let diff_pref = player.mission_difficulty_preference();
        let diff_pref_name = if (0..5).contains(&diff_pref) {
            MILITARY_DIFFICULTY_NAMES[diff_pref as usize]
        } else {
            "None"
        };
        messages.push(format!(
            "Mission difficulty preference: {diff_pref} ({diff_pref_name})"
        ));

        messages.push(format!(
            "Mission generation day: {} (Today: {current_yday})",
            player.mission_yday()
        ));
        messages.push(format!(
            "Mission taken day: {}",
            player.military_took_yday()
        ));
        messages.push(format!(
            "Mission solved day: {}",
            player.military_solved_yday()
        ));
        messages.push(format!(
            "Mission last Reroll day: {}",
            player.military_reroll_yday()
        ));

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milpref` (`command.c:5169-5249`, `CF_GOD`-gated): sets a
    // player's mission type/difficulty preference. Name is required
    // (unlike `milinfo`/`milreset`/`milsolve`'s self-fallback) - C prints
    // the 3-line usage block instead.
    if lower == "milpref" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /milpref <character> <type> <difficulty>".to_string(),
                    "Types: 0=none, 1=demon, 2=ratling, 3=silver".to_string(),
                    "Difficulties: 0=easy, 1=normal, 2=hard, 3=impossible, 4=insane, -1=none"
                        .to_string(),
                ],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        // C parses `type`/`diff` as two whitespace-separated integer
        // tokens after the name (`command.c:5200-5217`), both defaulting
        // to `-1` if absent. `diff`'s own acceptance range (`-1..=4`)
        // means an *omitted* difficulty argument is itself a valid "no
        // preference" value that overwrites any existing preference - a
        // real, reproduced C quirk (verified by reading the C source
        // directly), not a bug in this port.
        let mut tokens = remainder.split_whitespace();
        let type_value = tokens
            .next()
            .map(legacy_atoi_prefix)
            .map(|value| value as i32)
            .unwrap_or(-1);
        let diff_value = tokens
            .next()
            .map(legacy_atoi_prefix)
            .map(|value| value as i32)
            .unwrap_or(-1);

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let mut messages = Vec::new();
        if (0..=3).contains(&type_value) {
            player.set_mission_type_preference(type_value);
            messages.push(format!(
                "Set mission type preference to {type_value} ({}) for {target_name}",
                military_type_preference_name(type_value)
            ));
        }
        if (-1..5).contains(&diff_value) {
            player.set_mission_difficulty_preference(diff_value);
            let diff_name = if diff_value >= 0 {
                MILITARY_DIFFICULTY_NAMES[diff_value as usize]
            } else {
                "None"
            };
            messages.push(format!(
                "Set mission difficulty preference to {diff_value} ({diff_name}) for {target_name}"
            ));
        }
        // Reset mission generation day to force new missions with these
        // preferences (`command.c:5243`, unconditional).
        player.set_mission_yday(0);
        messages.push(
            "New missions will be generated with these preferences when player visits the Military Master"
                .to_string(),
        );

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milreset` (`command.c:5258-5304`, `CF_GOD`-gated): resets
    // mission/advisor cooldowns, self if no name given.
    if lower == "milreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
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

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        player.set_mission_yday(0);
        player.set_military_solved_yday(0);
        player.set_military_took_mission(0);
        player.set_military_reroll_yday(0);
        for advisor in 0..MILITARY_PPD_MAXADVISOR {
            player.set_military_advisor_last(advisor, 0);
        }

        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset all mission and advisor cooldowns for {target_name}"
            )],
            ..Default::default()
        });
    }

    // C `cmd_milpoints` (`command.c:5313-5384`, `CF_GOD`-gated): grants
    // raw military points to a named player. Name is required (no self
    // fallback). Deliberately does NOT call `World::give_military_pts`
    // (see the promotion-block comment below for why).
    if lower == "milpoints" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /milpoints <character> <points>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return Some(KeyringCommandResult {
                messages: vec!["Please specify number of points to grant.".to_string()],
                ..Default::default()
            });
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let target_name = target.name.clone();
        let old_rank = army_rank_for_points(target.military_points);
        target.military_points = target.military_points.saturating_add(points);
        let new_points = target.military_points;
        let new_rank = army_rank_for_points(new_points);

        // C inlines its own promotion check here rather than calling
        // `give_military_pts_no_npc`: no hardcore bonus, no `give_exp`/
        // `normal_exp` touch, a hardcoded `newrank < 25` promotion cap
        // (not `MAX_ARMY_RANK`=40), and its own message text - confirmed
        // by reading `cmd_milpoints` directly, distinct from
        // `give_military_pts_no_npc` (`tool.c:3281-3309`), which is *not*
        // called from this command.
        let messages = if new_rank > old_rank && new_rank < 25 {
            if new_rank > 9 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(COL_MAUVE);
                broadcast.extend_from_slice(
                    format!(
                        "Grats: {target_name} is a {} now!",
                        army_rank_name(new_rank)
                    )
                    .as_bytes(),
                );
                world.queue_channel_broadcast(6, broadcast);
            }
            vec![format!(
                "Granted {points} military points to {target_name}, promoting to {}!",
                army_rank_name(new_rank)
            )]
        } else {
            vec![format!(
                "Granted {points} military points to {target_name} (total: {new_points})"
            )]
        };

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_milrec` (`command.c:5393-5446`, `CF_GOD`-gated): grants
    // "recommendation points" (`ppd->current_pts`) to a named player.
    if lower == "milrec" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /milrec <character> <points>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Please specify number of recommendation points to grant.".to_string()
                ],
                ..Default::default()
            });
        }

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };
        let new_total = player.military_current_pts().saturating_add(points);
        player.set_military_current_pts(new_total);

        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Granted {points} recommendation points to {target_name} (total: {new_total})"
            )],
            ..Default::default()
        });
    }

    // C `cmd_milstats` (`command.c:5456-5489`, `CF_GOD`-gated): first
    // scans for the Military Master NPC (`ch[n].driver ==
    // CDR_MILITARY_MASTER`) and bails out with this exact message if none
    // exists - no `CDR_MILITARY_MASTER` driver/NPC has been ported to
    // Rust yet (see the "Military ranks" task's REMAINING note in
    // `PORTING_TODO.md`), so that branch always fires here, matching C's
    // own behavior in any area where the NPC hasn't been spawned.
    if lower == "milstats" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        return Some(KeyringCommandResult {
            messages: vec!["Could not find Military Master NPC.".to_string()],
            ..Default::default()
        });
    }

    // C `cmd_milsolve` (`command.c:5498-5613`, `CF_GOD`-gated): force-
    // completes a player's active mission, self if no name given, with an
    // optional trailing `announce` flag that also broadcasts a high-rank
    // promotion and notifies the target player directly.
    if lower == "milsolve" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    });
                }
            }
        };
        let announce = remainder
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("announce");

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let current_yday = world.date.yday as i32;

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            });
        };

        let took_mission = player.military_took_mission();
        if took_mission == 0 {
            return Some(KeyringCommandResult {
                messages: vec![format!("{target_name} does not have an active mission.")],
                ..Default::default()
            });
        }
        let mission_idx = (took_mission - 1).max(0) as usize;
        let mission = player.military_mission(mission_idx);
        let mission_type_str = military_mission_slot_type_name(mission.mission_type);
        let exp_reward = mission.exp;
        let points_reward = mission.pts;

        player.set_military_solved_mission(true);
        player.set_military_solved_yday(current_yday + 1);
        player.set_military_took_mission(0);

        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult::default());
        };
        let old_mil_pts = target.military_points;
        target.military_points = target.military_points.saturating_add(points_reward);
        target.military_normal_exp = target.military_normal_exp.saturating_add(exp_reward);
        let new_mil_pts = target.military_points;
        let is_player = target.flags.contains(CharacterFlags::PLAYER);

        let old_rank = army_rank_for_points(old_mil_pts);
        let new_rank = army_rank_for_points(new_mil_pts);
        let diff_name = MILITARY_DIFFICULTY_NAMES
            .get(mission_idx)
            .copied()
            .unwrap_or("Unknown");

        let messages = if new_rank > old_rank && new_rank < 25 {
            if new_rank > 9 && announce {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(COL_MAUVE);
                broadcast.extend_from_slice(
                    format!(
                        "Grats: {target_name} is a {} now!",
                        army_rank_name(new_rank)
                    )
                    .as_bytes(),
                );
                world.queue_channel_broadcast(6, broadcast);
            }
            vec![format!(
                "Completed {diff_name} {mission_type_str} mission for {target_name}! Rewards: {points_reward} mil pts, {exp_reward} exp. Promoted to {}!",
                army_rank_name(new_rank)
            )]
        } else {
            vec![format!(
                "Completed {diff_name} {mission_type_str} mission for {target_name}! Rewards: {points_reward} mil pts, {exp_reward} exp."
            )]
        };

        if is_player && announce {
            world.queue_system_text(
                target_id,
                format!(
                    "A staff member has completed your {diff_name} {mission_type_str} mission for you. You received {points_reward} military points and {exp_reward} experience."
                ),
            );
        }

        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_setrd`/`cmd_clearrd`/`cmd_solverd` (`command.c:1837-2010`, all
    // `CF_GOD`-gated): admin tools for the Area 14 "random dungeon" shrine
    // continuity system (`DRD_RANDOMSHRINE_PPD`, i.e. `PlayerRuntime::
    // random_shrine_continuity`/`random_shrine_used_words`). All three
    // share C's "bare number = self, else name then number" argument shape
    // (`isdigit(*ptr) ? co = cn : co = lookup_char(...)`), reproduced via
    // the existing `parse_exp_command_target` helper. C's actual
    // `lookup_char` here is a latent bug - it searches the character-
    // *template* table (`ch_temp[]`, used by `/create`), not online
    // characters - so, matching the established convention of every other
    // "target by name" admin command in this file (`/milrec`,
    // `/milpoints`, `/milsolve`), the online-character lookup baked into
    // `parse_exp_command_target` is used instead of reproducing that bug.
    //
    // C always resends the quest log (`sendquestlog(cn, ch[cn].player)`)
    // to the ACTING character `cn`, never the target `co`, even when
    // targeting another player - reproduced verbatim below via
    // `legacy_questlog_payload`/`sessions_for_character` (matching
    // `military.rs`'s `apply_military_mission_kill_check`, the only other
    // non-login `sendquestlog` call site in this crate).
    //
    // C's `shrine_index = (rd_number - 10) * 10 + i` arithmetic in
    // `cmd_clearrd`/`cmd_solverd` can exceed the 256-bit `used[]` bitset
    // for `rd_number` above ~35 (already an out-of-bounds write in C,
    // undefined behavior there); Rust bounds-checks via `u8::try_from` and
    // silently skips any `shrine_index` that doesn't fit, instead of
    // panicking.
    if lower == "setrd" || lower == "clearrd" || lower == "solverd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let (target_id, target_name, rd_number) =
            parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            });
        }
        if !(10..=99).contains(&rd_number) {
            return Some(KeyringCommandResult {
                messages: vec!["RD number must be between 10 and 99.".to_string()],
                ..Default::default()
            });
        }
        let rd_number = rd_number as u32;

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            });
        };

        let message = match lower.as_str() {
            "setrd" => {
                target_player.random_shrine_continuity = rd_number as u8;
                format!("Set continuity shrine for {target_name} to RD {rd_number}.")
            }
            "clearrd" => {
                for i in 0..10u32 {
                    let shrine_index = (rd_number - 10) * 10 + i;
                    if let Ok(shrine) = u8::try_from(shrine_index) {
                        target_player.clear_random_shrine_used(shrine);
                    }
                }
                format!("Cleared all used shrines for {target_name} in RD {rd_number}.")
            }
            _ => {
                for i in 0..10u32 {
                    // C skips `i == 9`, the continuity shrine (the last
                    // slot of each RD level's 10 shrines).
                    if i == 9 {
                        continue;
                    }
                    let shrine_index = (rd_number - 10) * 10 + i;
                    if let Ok(shrine) = u8::try_from(shrine_index) {
                        target_player.mark_random_shrine_used(shrine);
                    }
                }
                format!(
                    "Marked all non-continuity shrines as used for {target_name} in RD {rd_number}."
                )
            }
        };

        if let Some(caller_player) = runtime.player_for_character(character_id) {
            let payload = legacy_questlog_payload(caller_player);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }

        return Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        });
    }

    // C `/changetunnel` (`command.c:2045-2085`, `CF_GOD`-gated): sets an
    // online target's `tunnel_ppd::clevel` directly, no self-fallback -
    // an empty/unmatched name always reports "no one by the name".
    if lower == "changetunnel" || lower == "settunnel" || lower == "cleartunnel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let mut tokens = remainder.trim_start().split_whitespace();
        let level = tokens.next().map(legacy_atoi_prefix).unwrap_or(0) as i32;
        let amount = tokens.next().map(legacy_atoi_prefix).unwrap_or(0) as i32;

        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };

        if !(MIN_TUNNEL_LEVEL..=MAX_TUNNEL_LEVEL).contains(&level) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Invalid tunnel level. Must be between {MIN_TUNNEL_LEVEL} and {MAX_TUNNEL_LEVEL}."
                )],
                ..Default::default()
            });
        }

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            });
        };

        let (caller_message, target_message) = match lower.as_str() {
            "changetunnel" => {
                target_player.set_tunnel_clevel(level);
                (
                    format!("Set {target_name}'s tunnel level to {level}."),
                    format!("Your tunnel level has been set to {level} by a god."),
                )
            }
            "settunnel" => {
                target_player.set_tunnel_used(level, amount.clamp(0, u8::MAX as i32) as u8);
                (
                    format!(
                        "Set {target_name}'s completed amount for tunnel level {level} to {amount}."
                    ),
                    format!(
                        "Your completed amount for tunnel level {level} has been set to {amount} by a god."
                    ),
                )
            }
            _ => {
                target_player.set_tunnel_used(level, 0);
                (
                    format!("Cleared {target_name}'s completed amount for tunnel level {level}."),
                    format!(
                        "Your completed amount for tunnel level {level} has been cleared by a god."
                    ),
                )
            }
        };

        let mut result = KeyringCommandResult {
            messages: vec![caller_message],
            ..Default::default()
        };
        if target_id != character_id {
            result.other_messages.push((target_id, target_message));
        }
        return Some(result);
    }

    // C `/solvetunnel` (`command.c:2199-2222`, `CF_GOD`-gated, self
    // only): C's own reward call (`give_reward(cn, ppd, door_type)`) is
    // commented out in the oracle itself, so this is a message-only
    // no-op there too - nothing to mutate here either.
    if lower == "solvetunnel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }

        let exptype = legacy_atoi_prefix(rest.trim_start());
        if exptype != 0 && exptype != 1 {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid exp type. Must be 0 (exp) or 1 (military exp).".to_string()],
                ..Default::default()
            });
        }

        let reward_name = if exptype == 0 {
            "experience"
        } else {
            "military experience"
        };
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Solved current tunnel and granted {reward_name} reward."
            )],
            ..Default::default()
        });
    }

    let Some(character) = world.characters.get_mut(&character_id) else {
        return Some(KeyringCommandResult::default());
    };

    let is_lqmaster = character.flags.contains(CharacterFlags::GOD)
        || character.flags.contains(CharacterFlags::EVENTMASTER)
        || (area_id == 20 && character.flags.contains(CharacterFlags::LQMASTER));

    if lower == "noexp" {
        if !character.flags.contains(CharacterFlags::NOEXP)
            && is_gatekeeper_room(area_id, character)
        {
            return Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoExp mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            });
        }
        character.flags.toggle(CharacterFlags::NOEXP);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned NoExp mode {}.",
                if character.flags.contains(CharacterFlags::NOEXP) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "nolevel" {
        if !character.flags.contains(CharacterFlags::NOLEVEL)
            && is_gatekeeper_room(area_id, character)
        {
            return Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoLevel mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            });
        }
        character.flags.toggle(CharacterFlags::NOLEVEL);
        let enabled = character.flags.contains(CharacterFlags::NOLEVEL);
        return Some(KeyringCommandResult {
            messages: vec![if enabled {
                "NoLevel mode enabled. You will not level up until you disable this mode."
                    .to_string()
            } else {
                "NoLevel mode disabled. You will now gain levels normally.".to_string()
            }],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "itemmod" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (pos, nr, val) = parse_itemmod_args(rest);
        let Some(item_id) = character.cursor_item else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        if pos < 0 || pos >= ugaris_core::entity::MAX_MODIFIERS as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Pos out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if nr < 0 || nr >= CHARACTER_VALUE_NAMES.len() as i64 {
            return Some(KeyringCommandResult {
                messages: vec!["Nr out of bounds.".to_string()],
                ..Default::default()
            });
        }
        if !(0..22).contains(&val) {
            return Some(KeyringCommandResult {
                messages: vec!["Val out of bounds.".to_string()],
                ..Default::default()
            });
        }
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        item.modifier_index[pos as usize] = nr as i16;
        item.modifier_value[pos as usize] = val as i16;
        let mut messages: Vec<String> = legacy_item_look_text(item, &character_snapshot)
            .lines()
            .map(str::to_string)
            .collect();
        messages.push(format!(
            "Item modified: {} (skill {}) at pos {} with value {}",
            value_name(nr as i16),
            nr,
            pos,
            val
        ));
        return Some(KeyringCommandResult {
            messages,
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower == "itemdesc" || lower == "itemname" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let Some(item_id) = character.cursor_item else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        let trimmed = rest.trim_start();
        let text = legacy_truncate_c_string(trimmed, 79);
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            });
        };
        if lower == "itemdesc" {
            item.description = text;
        } else {
            item.name = text;
        }
        return Some(KeyringCommandResult {
            messages: legacy_item_look_text(item, &character_snapshot)
                .lines()
                .map(str::to_string)
                .collect(),
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 4 && "saves".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let saves = legacy_atoi_prefix(rest).clamp(0, i64::from(u8::MAX)) as u8;
        character.saves = saves;
        return Some(KeyringCommandResult::default());
    }

    // C `/saveall` (`command.c:7460-7473`, `cmdcmp(ptr, "saveall", 4)`,
    // `CF_GOD`-gated). Must be checked after the `saves` block above
    // (matching C's own line order, 6278 before 7460): `cmdcmp(ptr,
    // "saves", 4)` matches the literal input "save" first in C, so
    // "/save" is `saves` (a stat setter) not `saveall`, and only
    // "/savea"/"/saveal"/"/saveall" reach this block. See the
    // `save_all_requested` doc comment on `KeyringCommandResult` for what
    // the `main.rs` call site does with the flag.
    if lower.len() >= 4 && "saveall".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        return Some(KeyringCommandResult {
            messages: vec![
                "Forcing save of all players...".to_string(),
                "Player data saved".to_string(),
                "Forcing save of merchant inventories...".to_string(),
                "Merchant data saved".to_string(),
            ],
            save_all_requested: true,
            ..Default::default()
        });
    }

    // C `/shutdown` (`command.c:6068-6086`, `cmdcmp(ptr, "shutdown", 8)`,
    // `CF_GOD`-gated). `minlen` equals the full word length, so unlike most
    // commands here no abbreviation is accepted - only the exact word
    // "shutdown" (case-insensitive) reaches this block.
    if lower == "shutdown" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        // C: `ptr += len; while (isspace(*ptr)) ptr++; diff = atoi(ptr);
        // while (isdigit(*ptr)) ptr++; while (isspace(*ptr)) ptr++; down =
        // atoi(ptr);` - note the `isdigit`-skip does not step over a
        // leading `-` sign, so a negative `diff` leaves `down` parsed from
        // the exact same substring (a real, reproducible C quirk).
        let ptr = rest.trim_start();
        let diff = legacy_atoi_prefix(ptr);
        let after_digits = ptr
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        let down = legacy_atoi_prefix(after_digits);
        apply_shutdown_command(world, runtime, diff, down);
        return Some(KeyringCommandResult::default());
    }

    if lower == "sprite" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.sprite = legacy_atoi_prefix(rest) as i32;
        return Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 2 && "immortal".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::IMMORTAL);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Immortal is {}.",
                if character.flags.contains(CharacterFlags::IMMORTAL) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "infrared".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INFRARED);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Infrared is {}.",
                if character.flags.contains(CharacterFlags::INFRARED) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "invisible".starts_with(&lower) {
        if !is_lqmaster {
            return None;
        }
        character.flags.toggle(CharacterFlags::INVISIBLE);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Invisible is {}.",
                if character.flags.contains(CharacterFlags::INVISIBLE) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        });
    }

    if lower == "xray" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::XRAY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned x-ray mode {}.",
                if character.flags.contains(CharacterFlags::XRAY) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        });
    }

    if lower.len() >= 3 && "spy".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        character.flags.toggle(CharacterFlags::SPY);
        let enabled = character.flags.contains(CharacterFlags::SPY);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned spy mode {}. You will {} see all tells, clan, alliance, club, area, and mirror chat.",
                if enabled { "on" } else { "off" },
                if enabled { "now" } else { "no longer" }
            )],
            ..Default::default()
        });
    }

    if lower == "setxmas" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let flag = legacy_atoi_prefix(rest.trim_start()) as i32;
        let old_value = runtime_effective_xmas_flag(runtime);
        runtime.xmas_special_override = Some(flag);
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Setting christmas special to {flag}, old value was {old_value}."
            )],
            ..Default::default()
        });
    }

    if lower.len() >= 6 && "dlight".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.dlight_override = legacy_atoi_prefix(rest) as i32;
        let override_value = (runtime.dlight_override != 0).then_some(runtime.dlight_override);
        world.date = GameDate::calculate(
            START_TIME + world.date.realtime,
            area_id as i32,
            override_value,
        );
        return Some(KeyringCommandResult::default());
    }

    if lower.len() >= 6 && "showattack".starts_with(&lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        runtime.show_attack = !runtime.show_attack;
        world.show_attack_debug = runtime.show_attack;
        return Some(KeyringCommandResult::default());
    }

    if lower == "joinclan" || lower == "joinclub" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if lower == "joinclan" {
            if (0..LEGACY_MAX_CLAN).contains(&nr) {
                character.clan = nr as u16;
                character.clan_rank = 4;
                character.clan_serial = world.clan_registry.serial(nr as u16);
            }
        } else if (0..LEGACY_MAX_CLUB).contains(&nr) {
            character.clan = (nr + LEGACY_CLUB_OFFSET) as u16;
            character.clan_rank = 2;
            character.clan_serial = 0;
        }
        return Some(KeyringCommandResult {
            name_changed: true,
            ..Default::default()
        });
    }

    // C `killclan` (`src/system/command.c:6468-6482`): sets the target
    // clan's debt sky-high (`kill_clan`, `clan.c:1413-1416`) so the next
    // weekly `update_treasure` tick (`clan.c:1154-1160`, `debt >= 2000`)
    // deletes it. `update_treasure`/the whole treasury economy isn't
    // ported (see the clan task's REMAINING notes), so this deletes the
    // clan immediately via [`ClanRegistry::delete_clan`] - the eventual
    // real-world outcome of C's `kill_clan`, without the week-long delay.
    // C emits no player feedback for this command; matched exactly (no
    // messages either way).
    if lower == "killclan" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if (1..LEGACY_MAX_CLAN).contains(&nr) {
            world.clan_registry.delete_clan(nr as u16);
        }
        return Some(KeyringCommandResult::default());
    }

    // C `killclub` (`src/system/command.c:6484-6497`), `CF_GOD`-gated.
    // Genuine C bug kept for fidelity: the bounds check guarding the
    // `kill_club` call compares `nr` against `MAXCLAN` (32,
    // `crate::commands_chat::LEGACY_MAX_CLAN`'s C counterpart), not
    // `MAXCLUB` (16384) - copy-paste leftover from the adjacent
    // `killclan` block above (`club.c`'s own `kill_club(int cnr)` itself
    // correctly bounds-checks against `MAXCLUB`, so this cap only bites
    // at the command layer). `kill_club` (`club.c:132-138`) doesn't clear
    // the club's name - it zeroes `money` and sets `paid = 1` so the next
    // `ClubRegistry::tick_billing` weekly pass deletes it for
    // nonpayment, exactly like `killclan`'s `kill_clan`/`update_treasure`
    // relationship. No player feedback either way, matched exactly.
    if lower == "killclub" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if (1..LEGACY_MAX_CLAN).contains(&nr) {
            world.club_registry.kill_club(nr as u16);
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/setclanjewels` (`command.c:7563-7596`), `CF_GOD`-gated. Directly
    // assigns `clan[clan_nr].treasure.jewels`, a distinct storage system
    // from the `GameSettings`-backed `set*` tuning-knob family closed out
    // in an earlier iteration (see this task's REMAINING notes). Args are
    // whitespace-separated `<clan_nr> <jewels> [do_log]`; `do_log`
    // defaults to `1` (log to the clan log) exactly like C's `int do_log =
    // 1; if (*ptr) do_log = atoi(ptr);`. Out-of-range clan numbers,
    // negative jewel counts, or an in-range clan number with no clan
    // actually created there (C's array is preallocated for every
    // in-range slot and would silently write through it anyway - a
    // footgun, not a feature - but this registry has no such slot; see
    // `ClanRegistry::set_jewels`) all report the same "Invalid clan
    // number or jewel count" message C emits only for the former two
    // cases.
    if lower == "setclanjewels" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let mut tokens = rest.split_whitespace();
        let clan_nr = tokens.next().map(legacy_atoi_prefix).unwrap_or(0);
        let jewels = tokens.next().map(legacy_atoi_prefix).unwrap_or(0);
        let do_log = tokens.next().map(legacy_atoi_prefix).unwrap_or(1);
        let old_jewels = (clan_nr > 0 && clan_nr < LEGACY_MAX_CLAN && jewels >= 0)
            .then(|| {
                world
                    .clan_registry
                    .set_jewels(clan_nr as u16, jewels as i32)
            })
            .flatten();
        let Some(old_jewels) = old_jewels else {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid clan number or jewel count".to_string()],
                ..Default::default()
            });
        };
        let clan_nr = clan_nr as u16;
        let clan_name = world.clan_registry.name(clan_nr).unwrap_or("").to_string();
        let messages = vec![format!(
            "Clan {clan_nr} ({clan_name}) jewels changed from {old_jewels} to {jewels}"
        )];
        let clan_log_entry = (do_log != 0).then(|| {
            (
                clan_nr,
                world.clan_registry.serial(clan_nr),
                1u8,
                format!(
                    "God {} changed clan jewels from {old_jewels} to {jewels}",
                    character.name
                ),
            )
        });
        return Some(KeyringCommandResult {
            messages,
            clan_log_entry,
            ..Default::default()
        });
    }

    // C `cmd_renclan` (`src/system/command.c:4497-4531`), dispatched at
    // `command.c:9646` gated on `CF_STAFF | CF_GOD`. Renames an existing
    // clan; only usable while standing in Aston (`areaID == 3`).
    if lower == "renclan" {
        if !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        if area_id != 3 {
            return Some(KeyringCommandResult {
                messages: vec!["Sorry, this command only works in Aston.".to_string()],
                ..Default::default()
            });
        }
        let rest = rest.trim_start();
        let nr = legacy_atoi_prefix(rest);
        let name = rest
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        if !(1..LEGACY_MAX_CLAN).contains(&nr) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Clan number must be between 1 and {}.",
                    LEGACY_MAX_CLAN - 1
                )],
                ..Default::default()
            });
        }
        let name: String = name.chars().take(78).collect();
        let messages = match world.clan_registry.set_name(nr as u16, &name) {
            Ok(()) => vec![format!("Clan {nr} name changed to \"{name}\".")],
            Err(_) => vec![format!("No clan by that number ({nr}).")],
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_renclub` (`src/system/command.c:4548-4585`), dispatched at
    // `command.c:9650` gated on `CF_STAFF | CF_GOD`. Renames an existing
    // club; only usable "nearby a clubmaster" per C's message text, but
    // the actual gate C checks is the same `areaID == 3` as `/renclan`
    // (`club.c` has no clubmaster-proximity concept - the message is
    // aspirational/copy-pasted text, not a real distinct check).
    // `ClubRegistry::rename_club` folds C's three separate failure modes
    // (invalid characters, name too long, name already taken) into one
    // `Err`, matching C's own single combined "didn't work" message for
    // all three (`rename_club` returning `0`).
    if lower == "renclub" {
        if !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        if area_id != 3 {
            return Some(KeyringCommandResult {
                messages: vec!["Sorry, this command only works nearby a clubmaster.".to_string()],
                ..Default::default()
            });
        }
        let rest = rest.trim_start();
        let nr = legacy_atoi_prefix(rest);
        let name = rest
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        if !(1..LEGACY_MAX_CLUB).contains(&nr) {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Club number must be between 1 and {}.",
                    LEGACY_MAX_CLUB - 1
                )],
                ..Default::default()
            });
        }
        let name: String = name.chars().take(78).collect();
        let messages = match world.club_registry.rename_club(nr as u16, &name) {
            Ok(()) => vec![format!("Club {nr} name changed to \"{name}\".")],
            Err(_) => {
                vec!["That didn't work. The name is either taken or illegal.".to_string()]
            }
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `cmd_flag` (`command.c:2870-2937`), the shared by-name flag-
    // toggle body of `/god` (`CF_GOD`), `/setsir` (`CF_WON`), `/staff`
    // (`CF_STAFF`), `/emaster` (`CF_EVENTMASTER`), `/devel`
    // (`CF_DEVELOPER`), `/hardcore` (`CF_HARDCORE`), and `/qmaster`
    // (`CF_LQMASTER`) - all dispatched at `command.c:9257-9337`, all
    // `CF_GOD`-gated, all full-word only (`cmdcmp`'s `minlen` equals the
    // command's own length for every one of these seven, so no
    // abbreviation is accepted - matched with `lower == "..."`, not
    // `starts_with`). See `World::apply_cmd_flag_command`'s doc comment
    // for the online/offline message-shape split.
    if let Some((flag, flag_name)) = match lower.as_str() {
        "god" => Some((CharacterFlags::GOD, "god")),
        "setsir" => Some((CharacterFlags::WON, "sir/lady")),
        "staff" => Some((CharacterFlags::STAFF, "staff")),
        "emaster" => Some((CharacterFlags::EVENTMASTER, "master of events")),
        "devel" => Some((CharacterFlags::DEVELOPER, "developer")),
        "hardcore" => Some((CharacterFlags::HARDCORE, "hardcore")),
        "qmaster" => Some((CharacterFlags::LQMASTER, "qmaster")),
        _ => None,
    } {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|caller| caller.flags.contains(CharacterFlags::GOD))
        {
            return None;
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let messages = world.apply_cmd_flag_command(character_id, name, flag, flag_name);
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/goto` (`src/system/command.c:8453-8567`), gated on
    // `is_lqmaster(cn)` (`command.c:3331-3344`: `CF_GOD`, `CF_EVENTMASTER`,
    // or `CF_LQMASTER` while `areaID == 20`). See [`resolve_goto_jump_args`]
    // for the shared argument-parsing port (numeric `<x> <y> [area]
    // [mirror]`, `n`/`s`/`w`/`e` relative shorthand, `gl[]` shortcut name,
    // or online character name, in that priority order).
    if lower.len() >= 3 && "goto".starts_with(&lower) {
        let Some(character) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        let flags = character.flags;
        let (cx, cy) = (character.x, character.y);
        let is_lqmaster = flags.contains(CharacterFlags::GOD)
            || flags.contains(CharacterFlags::EVENTMASTER)
            || (area_id == 20 && flags.contains(CharacterFlags::LQMASTER));
        if !is_lqmaster {
            return None;
        }
        let is_god = flags.contains(CharacterFlags::GOD);
        let resolved = resolve_goto_jump_args(world, cx, cy, rest);
        let GotoJumpTarget { x, y, mut a, m } = resolved;
        if (1..27).contains(&m) {
            if a == 0 {
                a = area_id as i32;
            }
        }
        if a == area_id as i32 && m == 0 {
            a = 0;
        }
        if !is_god {
            a = 0;
        }
        return Some(finish_goto_jump(world, character_id, x, y, a, m, "goto"));
    }

    // C `/jump` (`command.c:8570-8626`), gated on `CF_STAFF | CF_GOD`. Only
    // resolves a `gl[]` shortcut name (no numeric x/y form, no player-name
    // lookup), with an optional leading `<mirror>` digit token consumed
    // first, and refuses while busy (`ch[cn].action != AC_IDLE`) or within
    // 3 seconds of the last regen tick ("Pant, pant. Too tired."). Unlike
    // `/goto`, cross-area is *not* restricted to `CF_GOD` here (copied
    // as-is from C, which has no such check on this path).
    if lower == "jump" {
        let Some(character) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        let flags = character.flags;
        let (action, regen_ticker) = (character.action, character.regen_ticker);
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return None;
        }
        if action != 0
            || world.tick.0.saturating_sub(u64::from(regen_ticker)) < TICKS_PER_SECOND * 3
        {
            return Some(KeyringCommandResult {
                messages: vec!["Pant, pant. Too tired.".to_string()],
                ..Default::default()
            });
        }

        let mut ptr = rest.trim_start();
        let mut m = 0i32;
        if ptr.starts_with(|ch: char| ch.is_ascii_digit()) {
            m = legacy_atoi_prefix(ptr) as i32;
            ptr = ptr.trim_start_matches(|ch: char| !ch.is_whitespace());
            ptr = ptr.trim_start();
        }
        let (mut x, mut y, mut a) = (0i32, 0i32, 0i32);
        if let Some((gx, gy, ga)) = goto_list_lookup(ptr) {
            x = i32::from(gx);
            y = i32::from(gy);
            a = ga as i32;
        }
        if a == area_id as i32 && m == 0 {
            a = 0;
        }

        if x <= 0 || y <= 0 || !world.map.legacy_inner_bounds(x as usize, y as usize) {
            return Some(KeyringCommandResult {
                messages: vec!["hu?".to_string()],
                ..Default::default()
            });
        }
        return Some(finish_goto_jump(world, character_id, x, y, a, m, "jump"));
    }

    // C `/gotolist` (`command.c:236-245`, dispatched at `command.c:8815-
    // 8822`), `CF_GOD`-gated. Lists every `gl[]` shortcut with its
    // coordinates and area.
    if lower == "gotolist" {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::GOD))
        {
            return None;
        }
        let mut messages = vec!["Available /goto locations:".to_string()];
        messages.extend(
            GOTO_LIST
                .iter()
                .map(|(name, x, y, a)| format!("{name} (x:{x}, y:{y}, area:{a})")),
        );
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/gotosearch <term>` (`command.c:248-269`, dispatched at
    // `command.c:8823-8829`), `CF_GOD`-gated. Substring search is
    // case-sensitive (C `strstr`, not `strcasestr`) - copied as-is.
    if lower.len() >= 8 && "gotosearch".starts_with(&lower) {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::GOD))
        {
            return None;
        }
        let term = rest.trim_start();
        if term.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Please provide a search term.".to_string()],
                ..Default::default()
            });
        }
        let matches: Vec<_> = GOTO_LIST
            .iter()
            .filter(|(name, ..)| name.contains(term))
            .collect();
        let mut messages = vec!["Matching /goto locations:".to_string()];
        messages.extend(
            matches
                .iter()
                .map(|(name, x, y, a)| format!("{name} (x:{x}, y:{y}, area:{a})")),
        );
        if matches.is_empty() {
            messages.push("No matching locations found.".to_string());
        } else {
            messages.push(format!(
                "Found {} matching location{}.",
                matches.len(),
                if matches.len() == 1 { "" } else { "s" }
            ));
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/summon <name>` (`command.c:8628-8649`), `CF_GOD`-gated. Finds the
    // first character slot (any flags set, not just `CF_PLAYER` - so NPCs
    // can be summoned too) whose name case-insensitively matches the whole
    // remainder of the line, then teleports it next to the caller via
    // `teleport_char_driver` (C `drvlib.c:2651-2673`). No user-visible
    // message on success or failure - only the C `dlog` staff-action log
    // entry, approximated here with a `debug!` trace.
    if lower.len() >= 3 && "summon".starts_with(&lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (cx, cy) = (caller.x, caller.y);
        let name = rest.trim_start();
        if let Some(target_id) = find_online_character_by_name(world, name) {
            if world.teleport_char_driver(target_id, cx, cy) {
                if let Some(target) = world.characters.get(&target_id) {
                    debug!(target: "client_log", name = %target.name, id = target_id.0, "summon teleport");
                }
            }
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/kick <name>` (`command.c:8668-8698`), gated on `CF_STAFF|CF_GOD`
    // (no abbreviation, `cmdcmp(ptr, "kick", 4)` requires the exact
    // 4-letter word). Finds the first `CF_PLAYER` character whose name
    // case-insensitively matches the remainder of the line; on a match,
    // tells the caller "Kicked %s." (C `log_char`) and signals the call
    // site (via `kick_target`) to perform the full `exit_char` (save at
    // rest position + despawn) + `player_client_exit` (send `SV_EXIT`
    // with the kick reason, disconnect) teardown on the target - the same
    // deferred side effects as `/logout`, just targeting someone else.
    // On no match, tells the caller "No player by the name %s." The C
    // `dlog` staff-action audit log and `write_scrollback` (which emails
    // the *caller's own* scrollback buffer to game@ugaris.com as
    // moderation evidence - there is no email/CURL infra in this
    // codebase) are both skipped, matching the established convention for
    // untracked audit-only C side effects (see `/summon` above).
    if lower == "kick" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        let target = world.characters.values().find(|character| {
            character.flags.contains(CharacterFlags::PLAYER)
                && character.name.eq_ignore_ascii_case(name)
        });
        return Some(match target {
            Some(target) => KeyringCommandResult {
                messages: vec![format!("Kicked {}.", name)],
                kick_target: Some(target.id),
                ..Default::default()
            },
            None => KeyringCommandResult {
                messages: vec![format!("No player by the name {name}.")],
                ..Default::default()
            },
        });
    }

    // C `/summonall` (`command.c:8653-8667`), `CF_GOD`-gated. Teleports
    // every `CF_PLAYER` character next to the caller, one at a time (the
    // caller themselves is included in the iteration but is a no-op since
    // `teleport_char_driver` refuses moves under Manhattan distance 2).
    if lower == "summonall" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (cx, cy) = (caller.x, caller.y);
        let player_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        for target_id in player_ids {
            if world.teleport_char_driver(target_id, cx, cy) {
                if let Some(target) = world.characters.get(&target_id) {
                    debug!(target: "client_log", name = %target.name, id = target_id.0, "summonall teleport");
                }
            }
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/office` (`command.c:9670-9676`), `CF_GOD`-gated, `minlen=6` so
    // the full word must be typed (`cmdcmp(ptr, "office", 6)`, no
    // abbreviation). Teleports to the staff office in Aston (area 3,
    // x:11, y:195): via `change_area` when not already in area 3 (the
    // call site resolves `cross_area_transfer` via
    // `attempt_cross_area_transfer`, falling back to the "Nothing
    // happens" message on failure), or directly via `teleport_char_driver`
    // when already in Aston.
    if lower == "office" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        if area_id != 3 {
            return Some(KeyringCommandResult {
                cross_area_transfer: Some((3, 11, 195)),
                ..Default::default()
            });
        }
        if world.teleport_char_driver(character_id, 11, 195) {
            debug!(target: "client_log", "office teleport");
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/jail <name>`/`/unjail <name>` (`command.c:8861-8882`/
    // `8839-8858`), `CF_STAFF|CF_GOD`-gated, full-word only (`cmdcmp`'s
    // `minlen` equals each full word's length, no abbreviation accepted).
    // Trims leading whitespace off the argument, then hands it to
    // `World::queue_jail_lookup`, which does all further validation and
    // DB resolution - see that function's and `ugaris-server`'s
    // `apply_jail_events`'s doc comments for the full behavior. Always
    // returns a `default()` result immediately; the real reply arrives
    // later via `World::queue_system_text`, matching C's own fire-and-
    // forget async `lookup_name` DB-worker round-trip.
    if lower == "jail" || lower == "unjail" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let action = if lower == "jail" {
            ugaris_core::world::JailAction::Jail
        } else {
            ugaris_core::world::JailAction::Unjail
        };
        world.queue_jail_lookup(character_id, rest.trim_start(), action);
        return Some(KeyringCommandResult::default());
    }

    // C `/rmdeath <name>` (`command.c:8884-8903` dispatch ->
    // `cmd_removedeath`, `command.c:2006-2019`), `CF_GOD`-gated, full-word
    // only (`cmdcmp`'s `minlen` is 7, the full length of "rmdeath", no
    // abbreviation accepted). Trims leading whitespace off the argument,
    // then hands it to `World::queue_rmdeath_lookup`, which does all
    // further validation and DB resolution - see that function's and
    // `world/rmdeath.rs`'s module doc comment for the full behavior.
    // Always returns a `default()` result immediately; the real reply
    // arrives later via `World::queue_system_text`, matching C's own
    // fire-and-forget async `lookup_name` DB-worker round-trip (same
    // pattern as `/jail`/`/unjail` above).
    if lower == "rmdeath" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        world.queue_rmdeath_lookup(character_id, rest.trim_start());
        return Some(KeyringCommandResult::default());
    }

    // C `/rename <from> <to>` (`command.c:6517-6524` dispatch ->
    // `cmd_rename`, `command.c:2657-2676`), `CF_GOD`-gated, full-word
    // only (`cmdcmp`'s `minlen` is 6, the full length of "rename", no
    // abbreviation accepted). Parses two consecutive `isalpha`-only name
    // tokens (`take_legacy_alpha_name`, mirroring C's own two scan
    // loops, `command.c:2661-2670`), each truncated to the C buffer's
    // 79-byte cap; hands both to `World::queue_rename_command`, which
    // performs all further validation and DB resolution - see that
    // function's and `world/rename.rs`'s module doc comment for the full
    // behavior. Always returns a `default()` result immediately; the
    // real reply arrives later via `World::queue_system_text` (same
    // fire-and-forget async pattern as `/jail`/`/rmdeath` above).
    if lower == "rename" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (from, remainder) = take_legacy_alpha_name(rest.trim_start());
        let from = &from[..from.len().min(79)];
        let (to, _remainder) = take_legacy_alpha_name(remainder.trim_start());
        let to = &to[..to.len().min(79)];
        world.queue_rename_command(character_id, from, to);
        return Some(KeyringCommandResult::default());
    }

    // C `/lockname <name>`/`/unlockname <name>` (`command.c:6528-6543`
    // dispatch -> `cmd_lockname`/`cmd_unlockname`, `command.c:2679-2701`),
    // both `CF_GOD`-gated, full-word only (`cmdcmp`'s `minlen` is 8/10,
    // the full word length, no abbreviation accepted). Parses one
    // `isalpha`-only name token, truncated to the C buffer's 79-byte cap;
    // hands it to `World::queue_lockname_command`/
    // `queue_unlockname_command` - see those functions' and
    // `world/lockname.rs`'s module doc comment for the full behavior.
    if lower == "lockname" || lower == "unlockname" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        if lower == "lockname" {
            world.queue_lockname_command(character_id, name);
        } else {
            world.queue_unlockname_command(character_id, name);
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/punish <name> <level> <reason>` (`command.c:6500-6507` dispatch
    // -> `cmd_punish`, `command.c:2354-2406`), `CF_GOD|CF_STAFF`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 6, the full length of
    // "punish", no abbreviation accepted). Parses an `isalpha`-only name
    // token (`take_legacy_alpha_name`, truncated to the 79-byte buffer
    // cap like `/rename`), then `level = atoi(ptr); while (isdigit(*ptr))
    // ptr++;` (a leading `-`/`+` sign, if any, is *not* skipped by this
    // second loop even though `atoi` itself parsed it - a genuine C quirk
    // only reachable with a malformed negative level, reproduced here by
    // only ever skipping digit characters, never a sign), then the
    // remaining raw bytes (not alpha-filtered, unlike the name) become
    // `reason`, capped at 79 bytes with `reason_overflowed` recording
    // whether the original text was longer - see `World::
    // queue_punish_command`'s doc comment for the validation this hands
    // off to. Always returns a `default()` result immediately; the real
    // reply arrives later via `World::queue_system_text` (same
    // fire-and-forget async pattern as `/jail`/`/rmdeath`/`/rename`
    // above).
    if lower == "punish" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        let after_level = after_name.trim_start();
        let level = legacy_atoi_prefix(after_level) as i32;
        let digits_end = after_level
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(after_level.len());
        let reason_raw = after_level[digits_end..].trim_start();
        let reason_overflowed = reason_raw.len() > 79;
        let mut reason_end = reason_raw.len().min(79);
        while reason_end > 0 && !reason_raw.is_char_boundary(reason_end) {
            reason_end -= 1;
        }
        let reason = &reason_raw[..reason_end];
        let messages =
            world.queue_punish_command(character_id, name, level, reason, reason_overflowed);
        if messages.is_empty() {
            return Some(KeyringCommandResult::default());
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/unpunish <name> <note id>` (`command.c:6541-6547` dispatch ->
    // `cmd_unpunish`, `command.c:2706-2731`), `CF_GOD`-only-gated,
    // full-word only (`cmdcmp`'s `minlen` is 8, the full length of
    // "unpunish", no abbreviation accepted). Parses an `isalpha`-only name
    // token, truncated to the 79-byte buffer cap, then `atoi`'s the
    // remaining text as the note id. Always returns a `default()` result
    // immediately; the real reply arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish` above).
    if lower == "unpunish" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        let note_id = legacy_atoi_prefix(after_name.trim_start());
        let messages = world.queue_unpunish_command(character_id, name, note_id);
        if messages.is_empty() {
            return Some(KeyringCommandResult::default());
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/exterminate <name>` (`command.c:9657-9662` dispatch ->
    // `cmd_exterminate`, `command.c:2639-2651`), `CF_STAFF|CF_GOD`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 11, the full length of
    // "exterminate", no abbreviation accepted). Parses an `isalpha`-only
    // name token, truncated to the 79-byte buffer cap (C does no other
    // validation before handing off - see `World::queue_exterminate_
    // command`'s doc comment). Always returns a `default()` result
    // immediately; the real reply arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish`/`/unpunish` above) once `ugaris-server`'s `world_events.
    // rs::apply_exterminate_events` resolves the DB round trip.
    if lower == "exterminate" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        world.queue_exterminate_command(character_id, name);
        return Some(KeyringCommandResult::default());
    }

    // C `/look <name>` (`command.c:8990-9019`), `CF_GOD|CF_STAFF`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 4, the full length of
    // "look", no abbreviation accepted). Unlike `/punish`'s `take_legacy_
    // alpha_name`, C passes its *entire*, untokenized trimmed remainder
    // to `lookup_name` (no alpha-only prefix extraction) - see `World::
    // queue_look_command`'s doc comment for why that's safe to reproduce
    // as a plain `trim_start()`. Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish`/`/unpunish` above).
    if lower == "look" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        world.queue_look_command(character_id, rest.trim_start());
        return Some(KeyringCommandResult::default());
    }

    // C `/klog` (`command.c:9022-9024` -> `karmalog`), `CF_GOD|CF_STAFF`-
    // gated, full-word only (`cmdcmp`'s `minlen` is 4, the full length of
    // "klog"). Takes no argument at all. Always returns a `default()`
    // result immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look` above).
    if lower == "klog" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        world.queue_klog_command(character_id);
        return Some(KeyringCommandResult::default());
    }

    // C `/values <name>` (`command.c:8391-8399` -> `look_values`,
    // `command.c:501-519`), `CF_GOD|CF_STAFF`-gated, full-word only
    // (`cmdcmp`'s `minlen` is 6, the full length of "values", no
    // abbreviation accepted - same idiom as `/look`/`/klog` above, not
    // `/showvalues`'s open-to-everyone abbreviation gate below). Trims
    // leading whitespace, then passes the entire untokenized remainder
    // to `World::queue_values_command` (see `world/values.rs`'s module
    // doc comment for the contrast with `/showvalues`'s caller/target
    // role swap - `/values` keeps the caller as the caller, showing the
    // resolved target's own stats). Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look`/`/klog`/`/showvalues` above).
    if lower == "values" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        world.queue_values_command(character_id, rest.trim_start());
        return Some(KeyringCommandResult::default());
    }

    // C `/showvalues <name>` (`command.c:8401-8409` -> `show_values`,
    // `command.c:521-537`), no permission gate - unlike `/values`/`/look`/
    // `/klog`, any player can use this. Full-word *abbreviation*
    // (`cmdcmp(ptr, "showvalues", 4)`'s `minlen` is 4, only the length of
    // "show" - same idiom as the already-ported `/showattack` above,
    // `starts_with` rather than an exact `lower ==` match) - trims
    // leading whitespace, then passes the *entire* untokenized remainder
    // to `World::queue_showvalues_command` (see `world/values.rs`'s
    // module doc comment for the full behavior, including the caller/
    // target role swap between `show_values` and `show_values_bg`).
    // Always returns a `default()` result immediately; every reply line
    // arrives later via `World::queue_system_text` (same fire-and-forget
    // async pattern as `/look`/`/klog` above).
    if lower.len() >= 4 && "showvalues".starts_with(&lower) {
        world.queue_showvalues_command(character_id, rest.trim_start());
        return Some(KeyringCommandResult::default());
    }

    // C `/allow <name>` (`command.c:8371-8378` -> `allow_body`,
    // `src/system/death.c:1013-1029`), no permission gate - any player
    // can use this, same as `/showvalues` above. Full-word
    // *abbreviation* (`cmdcmp(ptr, "allow", 3)`'s `minlen` is 3, "all"
    // up to "allow" all match - same idiom as `/showvalues`'s
    // `starts_with` check) - trims leading whitespace, then passes the
    // entire untokenized remainder to `World::queue_allow_command` (see
    // `world/allow.rs`'s module doc comment for the full behavior:
    // grants the resolved target access to every grave the caller owns,
    // never the caller's own kills). Always returns a `default()` result
    // immediately; every reply line arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/look`/`/klog`/`/showvalues` above).
    if lower.len() >= 3 && "allow".starts_with(&lower) {
        world.queue_allow_command(character_id, rest.trim_start());
        return Some(KeyringCommandResult::default());
    }

    // C `/showflags` (`command.c:8798-8805`, `cmd_show_flags`,
    // `command.c:4839-5061`), `CF_GOD`-gated, full-word only (`cmdcmp`'s
    // `minlen` is 9, the full length of "showflags", so no abbreviation
    // is accepted - matched with `lower == "showflags"`, not
    // `starts_with`). Target is resolved by scanning every currently
    // loaded character (`getfirst_char`/`getnext_char`, no `CF_PLAYER`
    // filter - reused via `find_online_character_by_name`), by the
    // `isalpha`-only name token (`command.c:4845-4847`, trailing
    // non-alpha text is simply ignored). Every set bit is reported, one
    // per line, in C's exact `if (flags & CF_X)` declaration order - note
    // `CF_SPY` is (deliberately, matching C) never checked here.
    if lower == "showflags" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let target_flags = world.characters[&target_id].flags;
        let mut messages = vec![format!("Flags for player {name}:")];
        for (flag, label) in SHOW_FLAGS_ORDER {
            if target_flags.contains(*flag) {
                messages.push((*label).to_string());
            }
        }
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/toggleflag` (`command.c:8807-8814`, `cmd_toggle_flag`,
    // `command.c:4784-4837`), `CF_GOD`-gated, full-word only (`minlen`
    // 10 == "toggleflag".len()). Name token is the same `isalpha`-only
    // scan as `/showflags`; the flag-name token that follows is C's
    // `!isspace`-only scan (`command.c:4799`, so it may contain digits
    // or punctuation, unlike the name), resolved case-insensitively via
    // [`character_flag_by_name`] (C `get_flag_by_name`,
    // `command.c:4590-4782` - also never maps `CF_SPY`). C additionally
    // calls `update_char(co)` when the toggled bit is `CF_UPDATE`,
    // `CF_ITEMS`, or `CF_PROF`, forcing an immediate client refresh
    // regardless of the toggle's new on/off state; this port only
    // toggles the in-memory bit (which the normal per-tick update
    // pipeline already consumes whenever it becomes set), so an
    // immediate refresh on the *clearing* transition is a known,
    // accepted gap for this rarely-used raw-flag debug command.
    if lower == "toggleflag" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let flag_name = remainder
            .trim_start()
            .split_whitespace()
            .next()
            .unwrap_or("");
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let Some(flag) = character_flag_by_name(flag_name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, unknown flag: {flag_name}")],
                ..Default::default()
            });
        };
        let target = world.characters.get_mut(&target_id).expect("just resolved");
        target.flags.toggle(flag);
        let state = if target.flags.contains(flag) {
            "ON"
        } else {
            "OFF"
        };
        return Some(KeyringCommandResult {
            messages: vec![format!("Flag {flag_name} turned {state} for {name}")],
            ..Default::default()
        });
    }

    // Pentagram debug commands (`command.c:10416-10465`, all `CF_GOD`-
    // gated, `cmdcmp` minlen == full word length so every name below is an
    // exact-word match, no abbreviations). `pent_find_player` (`command.c
    // :1150-1160`) has no self-fallback, unlike `/milinfo`'s family - a
    // player name is always required, and "not found" uses its own
    // distinct message text rather than the `/milinfo`-family's "Sorry, no
    // one by the name ... around."
    if lower == "pentinfo" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let target_arg = rest.trim_start();
        if target_arg.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /pentinfo <player>".to_string()],
                ..Default::default()
            });
        }
        let (name, _) = take_legacy_alpha_name(target_arg);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            });
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
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower == "setpentcount" || lower == "setpentstatus" || lower == "setpentbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let usage = match lower.as_str() {
            "setpentcount" => "Usage: /setpentcount <player> <count>",
            "setpentstatus" => "Usage: /setpentstatus <player> <0|1>",
            _ => "Usage: /setpentbonus <player> <bonus>",
        };
        let Some((name, value)) = parse_pent_name_and_int(rest) else {
            return Some(KeyringCommandResult {
                messages: vec![usage.to_string()],
                ..Default::default()
            });
        };
        let Some(target_id) = find_online_character_by_name(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            });
        };
        let message = match lower.as_str() {
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
        return Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        });
    }

    if lower == "resetpent" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let target_arg = rest.trim_start();
        if target_arg.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /resetpent <player>".to_string()],
                ..Default::default()
            });
        }
        let (name, _) = take_legacy_alpha_name(target_arg);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Could not access pent data for {target_name}.")],
                ..Default::default()
            });
        };
        player.pentagram_debug = PentagramDebugData::default();
        return Some(KeyringCommandResult {
            messages: vec![format!("Reset all pentagram data for {target_name}.")],
            ..Default::default()
        });
    }

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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macrostats <player>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
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
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower == "macrohistory" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macrohistory <player>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
        };
        let ppd = &player.macro_ppd;
        let mut messages = vec![format!("=== Challenge History: {target_name} ===")];
        if ppd.history_count == 0 {
            messages.push("No challenge history recorded.".to_string());
            return Some(KeyringCommandResult {
                messages,
                ..Default::default()
            });
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
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    if lower == "summonmacro" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let caller_id = caller.id.0;
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /summonmacro <player>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
        };
        player.macro_ppd.force_summon = true;
        player.macro_ppd.summoned_by = caller_id;
        debug!(target: "client_log", name = %target_name, id = target_id.0, "macro_admin summon requested");
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Macro daemon will summon {target_name} on next check."
            )],
            ..Default::default()
        });
    }

    if lower == "macroimmune" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let caller_id = caller.id.0;
        if rest.trim_start().is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macroimmune <player> <minutes>".to_string(),
                    "Use 0 minutes to remove immunity.".to_string(),
                ],
                ..Default::default()
            });
        }
        let Some((name, minutes)) = parse_pent_name_and_int(rest) else {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macroimmune <player> <minutes>".to_string()],
                ..Default::default()
            });
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let now = world.date.realtime;
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
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
        return Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        });
    }

    if lower == "macrosuspicion" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        if rest.trim_start().is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macrosuspicion <player> <amount>".to_string(),
                    "Use negative amount to reduce suspicion.".to_string(),
                ],
                ..Default::default()
            });
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macrosuspicion <player> <amount>".to_string()],
                ..Default::default()
            });
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
        };
        let old_value = player.macro_ppd.suspicion;
        player.macro_ppd.suspicion = (old_value + amount).clamp(0, 100);
        let new_value = player.macro_ppd.suspicion;
        debug!(target: "client_log", name = %target_name, id = target_id.0, old_value, new_value, "macro_admin adjusted suspicion");
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "{target_name} suspicion: {old_value} -> {new_value}"
            )],
            ..Default::default()
        });
    }

    if lower == "macrokarma" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        if rest.trim_start().is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /macrokarma <player> <value>".to_string(),
                    "Sets karma to specified value (0-100).".to_string(),
                ],
                ..Default::default()
            });
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macrokarma <player> <value>".to_string()],
                ..Default::default()
            });
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
        };
        let old_value = player.macro_ppd.karma;
        player.macro_ppd.karma = amount.clamp(0, 100);
        let new_value = player.macro_ppd.karma;
        debug!(target: "client_log", name = %target_name, id = target_id.0, old_value, new_value, "macro_admin set karma");
        return Some(KeyringCommandResult {
            messages: vec![format!("{target_name} karma: {old_value} -> {new_value}")],
            ..Default::default()
        });
    }

    if lower == "macrofailures" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let usage = "Usage: /macrofailures <player> <count>".to_string();
        if rest.trim_start().is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![usage],
                ..Default::default()
            });
        }
        let Some((name, amount)) = parse_pent_name_and_int(rest) else {
            return Some(KeyringCommandResult {
                messages: vec![usage],
                ..Default::default()
            });
        };
        let Some(target_id) = find_online_macro_player(world, &name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
        };
        let old_value = player.macro_ppd.challenge_failures;
        player.macro_ppd.challenge_failures = amount.max(0);
        let new_value = player.macro_ppd.challenge_failures;
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "{target_name} consecutive failures: {old_value} -> {new_value}"
            )],
            ..Default::default()
        });
    }

    if lower == "macroreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: /macroreset <player>".to_string()],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_macro_player(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let now = world.date.realtime;
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Error: Could not access macro data for {target_name}."
                )],
                ..Default::default()
            });
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
        return Some(KeyringCommandResult {
            messages: vec![format!("Reset all macro stats for {target_name}.")],
            ..Default::default()
        });
    }

    if lower == "macrolist" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
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
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/showppd <name> <ppd>` (`command.c:8790-8837` dispatch,
    // `cmdcmp(ptr, "showppd", 7)` - `minlen` == `strlen("showppd")`, exact
    // word only, `CF_GOD`-gated) + `cmd_showppd` (`command.c:275-346`): an
    // online-only (not `lookup_name`-backed, unlike most other by-name
    // debug commands) `CF_GOD` debug dump of one named `struct` PPD block
    // for a target character. Only two PPD names are recognized in the C
    // source (verified by reading the whole function): `area1` prints
    // every field of `struct area1_ppd`, `area3` prints only
    // `kassim_state` out of `struct area3_ppd` (the other 17 fields of
    // that struct are simply never read by this command). Name/ppd-name
    // parsing mirrors C's own `isalpha`/`isalpha-or-isdigit` scan loops
    // exactly (`take_legacy_alpha_name`/`take_legacy_alnum_name`), and the
    // "not found"/"which ppd"/"no ppd by that name" messages are checked
    // in the same order C does: online-name lookup first, then the
    // remaining-argument-empty check, then the ppd-name match.
    if lower == "showppd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, remainder) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Sorry, no player by the name {name} online (offline chars not possible)."
                )],
                ..Default::default()
            });
        };
        let ppd_rest = remainder.trim_start();
        if ppd_rest.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Which ppd?".to_string()],
                ..Default::default()
            });
        }
        let (ppd_name, _) = take_legacy_alnum_name(ppd_rest);
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let messages = if ppd_name.eq_ignore_ascii_case("area1") {
            match runtime.player_for_character(target_id) {
                Some(player) => vec![
                    format!("Area1 ppd of {target_name}"),
                    format!(
                        "Yoakin state: {}, Yoakin seen timer: {}, Greeter state: {}, Greeter seen timer: {}",
                        player.area1_yoakin_state(),
                        player.area1_yoakin_seen_timer(),
                        player.area1_greeter_state(),
                        player.area1_greeter_seen_timer(),
                    ),
                    format!(
                        "AClerk state: {}, AClerk seen timer: {}, Cameron Hermit state: {}, Cameron Hermit seen timer: {}, Cameron Hermit kill count: {}",
                        player.area1_aclerk_state(),
                        player.area1_aclerk_seen_timer(),
                        player.area1_camhermit_state(),
                        player.area1_camhermit_seen_timer(),
                        player.area1_camhermit_kills(),
                    ),
                    format!(
                        "Jessica state: {}, Jessica seen timer: {}, Gwendolyn state: {}, Gwendolyn seen timer: {}",
                        player.area1_jessica_state(),
                        player.area1_jessica_seen_timer(),
                        player.area1_gwendy_state(),
                        player.area1_gwendy_seen_timer(),
                    ),
                    format!(
                        "Gerewin state: {}, Gerewin seen timer: {}, Lydia state: {}, Lydia seen timer: {}",
                        player.area1_gerewin_state(),
                        player.area1_gerewin_seen_timer(),
                        player.area1_lydia_state(),
                        player.area1_lydia_seen_timer(),
                    ),
                    format!(
                        "Asturin state: {}, Asturin seen timer: {}, Guiwynn state: {}, Guiwynn seen timer: {}",
                        player.area1_asturin_state(),
                        player.area1_asturin_seen_timer(),
                        player.area1_guiwynn_state(),
                        player.area1_guiwynn_seen_timer(),
                    ),
                    format!(
                        "Logain state: {}, Logain seen timer: {}, Brithildie state: {}, Brithildie seen timer: {}",
                        player.area1_logain_state(),
                        player.area1_logain_seen_timer(),
                        player.area1_brithildie_state(),
                        player.area1_brithildie_seen_timer(),
                    ),
                    format!(
                        "Jiu state: {}, Jiu seen timer: {}, Nook state: {}, Darkin state: {}",
                        player.area1_jiu_state(),
                        player.area1_jiu_seen_timer(),
                        player.area1_nook_state(),
                        player.area1_darkin_state(),
                    ),
                    format!(
                        "Terion state: {}, Shrike state: {}, Shrike fails: {}",
                        player.area1_terion_state(),
                        player.area1_shrike_state(),
                        player.area1_shrike_fails(),
                    ),
                    format!(
                        "Reskin state: {}, Reskin seen timer: {}, Reskin got bits: {}",
                        player.area1_reskin_state(),
                        player.area1_reskin_seen_timer(),
                        player.area1_reskin_got_bits(),
                    ),
                    format!(
                        "James state: {}, James flags: {}",
                        player.area1_james_state(),
                        player.area1_flags(),
                    ),
                ],
                None => vec![format!("Reading PPD {ppd_name} failed.")],
            }
        } else if ppd_name.eq_ignore_ascii_case("area3") {
            match runtime.player_for_character(target_id) {
                Some(player) => vec![format!("Kassim state: {}", player.area3_kassim_state())],
                None => vec![format!("Reading PPD {ppd_name} failed.")],
            }
        } else {
            vec![format!("Sorry, no ppd by the name {ppd_name}.")]
        };
        return Some(KeyringCommandResult {
            messages,
            ..Default::default()
        });
    }

    // C `/noarch` (`command.c:9049-9057`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "noarch", 6)` - `minlen == strlen("noarch")`, exact word only) plus
    // `cmd_noarch` (`command.c:3163-3192`): looks up an online character by
    // (case-insensitive) name - no self-fallback, a bare `/noarch` with no
    // name resolves an empty-string lookup that never matches any real
    // character name, reporting "Sorry, no one by the name  around." with
    // C's characteristic double space (`name` is empty, and its own
    // `log_char` format string has a single literal space before `%s`) -
    // then caps every one of the target's `value[1][0..=V_IMMUNITY]`
    // entries (indices `0..=37`, i.e. `CharacterValue::Hp` through
    // `CharacterValue::Immunity` inclusive) at `50` and clears `CF_ARCH`.
    // Unlike every other admin command in this file, C sends no
    // confirmation message at all on success - only the not-found error is
    // ever logged, and only to the caller.
    if lower == "noarch" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let Some(target) = world.characters.get_mut(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        for value in target.values[1]
            .iter_mut()
            .take(CharacterValue::Immunity as usize + 1)
        {
            if *value > 50 {
                *value = 50;
            }
        }
        target.flags.remove(CharacterFlags::ARCH);
        return Some(KeyringCommandResult::default());
    }

    // C `/noprof` (`command.c:9226-9235`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "noprof", 6)`, exact word only): unlike `/noarch` above, this takes
    // no argument at all and never advances `ptr` past the matched word,
    // so it always operates on the caller (`ch[cn]`) itself, never a named
    // target - resets every one of the caller's own `prof[0..P_MAX]`
    // entries (`PROFESSION_COUNT` = 20 here) to `0` and sets `CF_PROF`
    // (client refresh flag, a no-op here since this codebase has no
    // separate "dirty" flag propagation for professions). No message is
    // sent to the caller on success, matching C exactly.
    if lower == "noprof" {
        let Some(caller) = world.characters.get_mut(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        for profession in caller.professions.iter_mut() {
            *profession = 0;
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/fixit` (`command.c:9058-9066`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "fixit", 5)` - exact word only) plus `cmd_reset_questlog`
    // (`command.c:3194-3218`): looks up an *online* character by name
    // (alpha-only prefix, matching `take_legacy_alpha_name`; C's
    // `strcasecmp` requires an exact match against the full character
    // name, no self-fallback), reports "Sorry, no one by the name %s
    // around." on failure, otherwise wipes the target's entire quest log
    // PPD (`del_data`, reproduced as `QuestLog::default()`), fully
    // re-derives it (`questlog_init`, reproduced as
    // `PlayerRuntime::init_questlog`, which now actually runs since the
    // sentinel was just cleared by the wipe) and resends the fresh quest
    // log to the TARGET (`sendquestlog(co, ch[co].player)` - unlike
    // `/questfix` right below, this one operates on the right character
    // throughout).
    if lower == "fixit" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            target_player.quest_log = QuestLog::default();
            target_player.init_questlog();
            let payload = legacy_questlog_payload(target_player);
            for (session_id, _) in runtime.sessions_for_character(target_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        return Some(KeyringCommandResult::default());
    }

    // C `/questfix` (`command.c:9067-9075`, `CF_GOD`-gated, `cmdcmp(ptr,
    // "questfix", 8)` - exact word only) plus `cmd_reset_last_quest`
    // (`command.c:3221-3251`): shares `/fixit`'s name-lookup/not-found
    // path above, but its action is a genuine C bug - `set_data` is
    // called with the ACTING character `cn`, not the looked-up target
    // `co`, so it clears the CALLER's own quest-log init-complete
    // sentinel (`quest[MAXQUEST - 1].done = 0`), then calls
    // `questlog_init(co)` on the target (almost always a no-op, since an
    // online character's sentinel is virtually always already set), and
    // finally resends the CALLER's own now-desynced quest log
    // (`sendquestlog(cn, ch[cn].player)`). The practical effect: the
    // named argument only serves as an online-character existence check;
    // the caller's own quest log gets marked for full re-derivation on
    // their *next* login (the immediate resend still reflects the
    // unchanged pre-existing entries, since `init_questlog` is never
    // called on `cn` here). Reproduced verbatim, bug and all.
    if lower == "questfix" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            target_player.init_questlog();
        }
        if let Some(caller_player) = runtime.player_for_character_mut(character_id) {
            caller_player.quest_log.clear_init_complete();
            let payload = legacy_questlog_payload(caller_player);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        return Some(KeyringCommandResult::default());
    }

    // C `#ls <name> <dir>` / `#cat <name> <file>` (`command.c:9237-9253`
    // dispatch, `CF_GOD`-gated, `cmdcmp(ptr, "#ls", 3)`/`cmdcmp(ptr,
    // "#cat", 4)` - both exact-word only, no abbreviation) plus
    // `cmd_ls`/`cmd_cat` (`command.c:2794-2845`): a debug feature that
    // asks the TARGET character's own game client to list a directory
    // (`#ls`) or dump a file's contents (`#cat`) from the *client's*
    // local disk, not the server's - `plr_ls`/`plr_cat`
    // (`src/system/player.c:3750-3789`) just forward a raw `SV_LS`/
    // `SV_CAT` request packet to the target's connection; any actual
    // listing/content comes back later as a separate client-originated
    // packet this codebase does not yet parse (out of scope here, same
    // as the C dispatcher itself which never processes a reply). The
    // target name is matched by C's `getfirst_char`/`getnext_char` loop
    // with no `CF_PLAYER` filter (`find_online_character_by_name`
    // already replicates this - NPCs are valid targets too, they just
    // never have a live connection to actually receive anything), parsed
    // via `isalpha`-only `take_legacy_alpha_name` exactly like
    // `/fixit`/`/questfix` above. Unlike those two, the not-found message
    // here is `"Sorry, no one by the name {name} around."` (matches this
    // pair's own `log_char`, not `/clearppd`'s distinct "Player '...' not
    // found." text). The remainder after the name and its trailing
    // whitespace is the `dir`/`file` argument verbatim (may itself
    // contain spaces, never re-tokenized in C). C unconditionally logs
    // `"ls {dir} scheduled on {target}."` / `"cat {file} scheduled on
    // {target}."` to the caller once a target is found, even when
    // `plr_ls`/`plr_cat` internally no-ops (target has no live client
    // connection, i.e. `ch[co].player == 0` - modeled here as
    // `sessions_for_character` returning empty - or `dir`/`file` exceeds
    // the 200-byte cutoff `remote_fs_request` enforces) - reproduced by
    // sending the packet only when a session exists and the byte-count
    // check passes, but always returning the confirmation message
    // regardless.
    if lower == "ls" || lower == "cat" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let dir = after_name.trim_start();
        let mut builder = PacketBuilder::new();
        let sent = if lower == "ls" {
            builder.ls_request(dir)
        } else {
            builder.cat_request(dir)
        };
        if sent {
            let payload = builder.into_payload();
            for (session_id, _) in runtime.sessions_for_character(target_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
        let verb_word = if lower == "ls" { "ls" } else { "cat" };
        return Some(KeyringCommandResult {
            messages: vec![format!("{verb_word} {dir} scheduled on {target_name}.")],
            ..Default::default()
        });
    }

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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        // C `ac_cmd_help` (`anticheat.c:688-720`) - reproduced letter for
        // letter (minus the `COL_*` wrapping, matching `/global`'s own
        // established plain-text simplification for text-heavy admin
        // dumps) even though most of the listed subcommands are still
        // unported, since this is C's own static help text, not a
        // reflection of this codebase's current dispatch coverage.
        return Some(KeyringCommandResult {
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
        });
    }

    if lower == "acstatus" || lower == "acstats" || lower == "aclist" || lower == "acsuspicious" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }

        if lower == "acstatus" {
            // C `ac_cmd_status` (`anticheat.c:473-517`).
            let name = rest.trim_start();
            if name.is_empty() {
                return Some(KeyringCommandResult {
                    messages: vec!["Usage: #acstatus <player>".to_string()],
                    ..Default::default()
                });
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
                return Some(KeyringCommandResult {
                    messages: vec![format!("Player '{name}' not found online.")],
                    ..Default::default()
                });
            };
            let target_name = world.characters[&target_id].name.clone();
            let Some(session_id) = runtime
                .player_for_character(target_id)
                .and_then(|player| player.anticheat_session_id)
            else {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Player '{target_name}' has no connection data.")],
                    ..Default::default()
                });
            };
            world.queue_ac_status_lookup(character_id, target_name, session_id);
            return Some(KeyringCommandResult::default());
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
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let days_str = rest.trim_start();
        if days_str.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #accleanup <days>".to_string(),
                    "Deletes AC records older than <days> days.".to_string(),
                ],
                ..Default::default()
            });
        }
        let days =
            legacy_atoi_prefix(days_str).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if days < 7 {
            return Some(KeyringCommandResult {
                messages: vec!["Minimum retention is 7 days.".to_string()],
                ..Default::default()
            });
        }
        world.queue_ac_cleanup_lookup(character_id, days);
        return Some(KeyringCommandResult {
            messages: vec![format!("Cleaning up records older than {days} days...")],
            ..Default::default()
        });
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
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acreset <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_reset_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acflag <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_flag_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acunflag <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_unflag_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#actrust <player>` (`command.c:10205-10213` dispatch, `CF_GOD`-
    // only, exact-word; `ac_cmd_trust`, `anticheat.c:827-849`). Same
    // single-name-target resolution as `#acflag`/`#acunflag` above; no
    // status gate (C's own handler has none) - see `apply_ac_trust_events`
    // for the `ac_player_stats.is_trusted` mutation and confirmation
    // message.
    if lower == "actrust" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #actrust <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_trust_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#acuntrust <player>` (`command.c:10214-10222` dispatch, `CF_GOD`-
    // only, exact-word; `ac_cmd_untrust`, `anticheat.c:860-882`). Same
    // single-name-target resolution as `#actrust` above; the "untrust"
    // mirror of `apply_ac_trust_events`.
    if lower == "acuntrust" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acuntrust <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_untrust_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acwatch <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        target_player.ac_watch_enabled = !target_player.ac_watch_enabled;
        let message = if target_player.ac_watch_enabled {
            format!("Now watching {target_name} - detailed AC logging enabled.")
        } else {
            format!("Stopped watching {target_name}.")
        };
        return Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        });
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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        if rest.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acwarn <player> [reason]".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_warn_lookup(character_id, target_id, target_name, session_id, reason);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acsessions <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_sessions_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#acviolations <player>` (`command.c:10250-10255` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_violations`,
    // `anticheat.c:1019-1053`). Identical single-name-target resolution
    // shape to `#acsessions` right above - see `apply_ac_violations_events`
    // for the subscriber-id resolution and the recent-violation-history
    // query itself.
    if lower == "acviolations" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acviolations <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_violations_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#achistory <player>` (`command.c:10232-10239` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_history`, `anticheat.c:
    // 924-972`). Identical single-name-target resolution shape to
    // `#acsessions`/`#acviolations` above - see `apply_ac_history_events`
    // for the subscriber-id resolution and the lifetime `ac_player_stats`
    // rollup read itself.
    if lower == "achistory" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #achistory <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_history_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#acsharedip <player>` (`command.c:10259-10267` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_sharedip`, `anticheat.
    // c:1058-1088`). Identical single-name-target resolution shape to
    // `#acsessions`/`#acviolations`/`#achistory` above - see
    // `apply_ac_sharedip_events` for the subscriber-id resolution and the
    // shared-IP query itself.
    if lower == "acsharedip" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acsharedip <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_sharedip_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#acsharedhw <player>` (`command.c:10268-10276` dispatch,
    // `CF_GOD|CF_STAFF`-gated, exact-word; `ac_cmd_sharedhw`, `anticheat.
    // c:1096-1126`). Identical single-name-target resolution shape to
    // `#acsharedip` above - see `apply_ac_sharedhw_events` for the
    // subscriber-id resolution and the shared-hardware query itself.
    if lower == "acsharedhw" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let name = rest.trim_start();
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acsharedhw <player>".to_string()],
                ..Default::default()
            });
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
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found online.")],
                ..Default::default()
            });
        };
        let target_name = world.characters[&target_id].name.clone();
        let Some(session_id) = runtime
            .player_for_character(target_id)
            .and_then(|player| player.anticheat_session_id)
        else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{target_name}' has no connection data.")],
                ..Default::default()
            });
        };
        world.queue_ac_sharedhw_lookup(character_id, target_name, session_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#achighrisk` (`command.c:10277-10280` dispatch, `CF_GOD|
    // CF_STAFF`-gated, exact-word; `ac_cmd_highrisk`, `anticheat.c:1134-
    // 1157`). No player name to resolve - same no-target shape as
    // `#acsiglist` below, so this simply queues a caller id and lets
    // `apply_ac_highrisk_events` list every high-risk `ac_player_stats`
    // row.
    if lower == "achighrisk" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        world.queue_ac_highrisk_lookup(character_id);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let id_str = rest.trim_start();
        if id_str.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #aclookup <subscriber_id>".to_string()],
                ..Default::default()
            });
        }
        let subscriber_id = legacy_atoi_prefix(id_str);
        if subscriber_id <= 0 {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid subscriber ID.".to_string()],
                ..Default::default()
            });
        }
        world.queue_ac_lookup_lookup(character_id, subscriber_id);
        return Some(KeyringCommandResult::default());
    }

    // C `#acsiglist` (`command.c:10291-10294` dispatch, `CF_GOD`-only,
    // exact-word; `ac_cmd_siglist`, `anticheat.c:1192-1215`). No player
    // name to resolve - unlike every other command in this file except
    // `#accleanup` - so this simply queues a caller id and lets `apply_
    // ac_siglist_events` list every row in the new `ac_known_signatures`
    // table (`migrations/0016_ac_known_signatures.sql`).
    if lower == "acsiglist" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        world.queue_ac_siglist_lookup(character_id);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let args = rest.trim_start();
        if args.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #acsigadd <type> <value> <name>".to_string(),
                    "Types: hardware_hash, code_hash, dll_hash, process_name, hardware_id"
                        .to_string(),
                ],
                ..Default::default()
            });
        }
        let Some((sig_type, sig_value, name)) = parse_ac_sigadd_args(args) else {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acsigadd <type> <value> <name>".to_string()],
                ..Default::default()
            });
        };
        const VALID_SIGNATURE_TYPES: [&str; 5] = [
            "hardware_hash",
            "code_hash",
            "dll_hash",
            "process_name",
            "hardware_id",
        ];
        if !VALID_SIGNATURE_TYPES.contains(&sig_type.as_str()) {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Invalid type. Use: hardware_hash, code_hash, dll_hash, process_name, \
                     hardware_id"
                        .to_string(),
                ],
                ..Default::default()
            });
        }
        let created_by = caller.name.clone();
        world.queue_ac_sigadd_lookup(character_id, sig_type, sig_value, name, created_by);
        return Some(KeyringCommandResult::default());
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
            return Some(KeyringCommandResult::default());
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return None;
        }
        let id_str = rest.trim_start();
        if id_str.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #acsigdel <id>".to_string()],
                ..Default::default()
            });
        }
        let signature_id = legacy_atoi_prefix(id_str);
        if signature_id == 0 {
            return Some(KeyringCommandResult {
                messages: vec!["Invalid signature ID.".to_string()],
                ..Default::default()
            });
        }
        world.queue_ac_sigdel_lookup(character_id, signature_id);
        return Some(KeyringCommandResult::default());
    }

    // C `/clearppd <ppdname> [player]` (`command.c:10144-10146` dispatch,
    // `CF_GOD | CF_STAFF`-gated, `cmdcmp(ptr, "clearppd", 8)` - exact word
    // only; `cmd_clearppd`, `command.c:4214-4288`). A raw, PPD-name-
    // agnostic admin wipe over C's generic `del_data(co, ppd_id)` linked-
    // list removal - unlike every other command in this file, it performs
    // NO resend of the cleared data to either party (verified by reading
    // the whole C function body: no `send*`/`log_char` other than the
    // three messages reproduced below). Supports exactly three PPD names
    // (`keyring`, `questlog`, `alias`), matched case-insensitively.  An
    // optional second, whitespace-separated argument targets an online
    // *player* character (`ch[co].flags & CF_PLAYER`, so - unlike most
    // name-lookup commands in this file - NPCs never match) by exact
    // case-insensitive full-string match against the ENTIRE remaining
    // text (C's `strcasecmp(ch[co].name, ptr)`, not just a leading name
    // token - so any trailing text after a valid name breaks the match, a
    // genuine quirk reproduced here by using the raw trimmed remainder
    // rather than `take_legacy_alpha_name`); the miss message is "Player
    // '%s' not found." (deliberately distinct from every other command's
    // "Sorry, no one by the name %s around." - copied letter for
    // letter). Self-targets when no second argument is given. Since Rust
    // keeps these three PPDs as always-present plain fields rather than
    // lazily-allocated `del_data` blocks, "the PPD existed" (`del_data`'s
    // nonzero return) is modeled as "the field is currently non-default"
    // - exactly the set of players for whom C would actually have called
    // `set_data` at least once - so the found/not-found message split
    // matches observable behavior.
    if lower == "clearppd" {
        let Some(caller) = world.characters.get(&character_id) else {
            return Some(KeyringCommandResult::default());
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return None;
        }
        let caller_name = caller.name.clone();

        let rest = rest.trim_start();
        if rest.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: #clearppd <ppdname> [player]".to_string(),
                    "Available PPDs: keyring, questlog, alias".to_string(),
                ],
                ..Default::default()
            });
        }

        let mut parts = rest.splitn(2, char::is_whitespace);
        let ppd_name = parts.next().unwrap_or("").to_ascii_lowercase();
        let player_arg = parts.next().unwrap_or("").trim_start();

        let (target_id, target_name) = if player_arg.is_empty() {
            (character_id, caller_name.clone())
        } else {
            let found = world.characters.values().find(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(player_arg)
            });
            let Some(target) = found else {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Player '{player_arg}' not found.")],
                    ..Default::default()
                });
            };
            (target.id, target.name.clone())
        };

        let ppd_display_name = match ppd_name.as_str() {
            "keyring" => "keyring",
            "questlog" => "questlog",
            "alias" => "alias",
            _ => {
                return Some(KeyringCommandResult {
                    messages: vec![
                        format!("Unknown PPD: {ppd_name}"),
                        "Available PPDs: keyring, questlog, alias".to_string(),
                    ],
                    ..Default::default()
                });
            }
        };

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            });
        };

        let existed = match ppd_display_name {
            "keyring" => !target_player.keyring.is_empty(),
            "questlog" => !target_player.quest_log.is_empty(),
            _ => !target_player.aliases.is_empty(),
        };
        if existed {
            match ppd_display_name {
                "keyring" => target_player.keyring.clear(),
                "questlog" => target_player.quest_log = QuestLog::default(),
                _ => target_player.aliases.clear(),
            }
        }

        let mut result = KeyringCommandResult::default();
        if existed {
            result
                .messages
                .push(format!("Cleared {ppd_display_name} PPD for {target_name}."));
            if target_id != character_id {
                result.other_messages.push((
                    target_id,
                    format!("Your {ppd_display_name} data has been cleared by {caller_name}."),
                ));
            }
        } else {
            result.messages.push(format!(
                "No {ppd_display_name} PPD found for {target_name}."
            ));
        }
        return Some(result);
    }

    None
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

pub(crate) fn is_gatekeeper_room(area_id: u32, character: &Character) -> bool {
    area_id == 3 && (178..=210).contains(&character.x) && (196..=228).contains(&character.y)
}

/// C `cmd_show_flags`'s exact `if (flags & CF_X)` declaration order
/// (`command.c:4871-5059`). `CF_SPY` is genuinely never checked by C
/// here (nor mapped by `get_flag_by_name`), so it is intentionally
/// absent from both this table and [`character_flag_by_name`].
pub(crate) const SHOW_FLAGS_ORDER: &[(CharacterFlags, &str)] = &[
    (CharacterFlags::USED, "USED"),
    (CharacterFlags::IMMORTAL, "IMMORTAL"),
    (CharacterFlags::GOD, "GOD"),
    (CharacterFlags::PLAYER, "PLAYER"),
    (CharacterFlags::STAFF, "STAFF"),
    (CharacterFlags::INVISIBLE, "INVISIBLE"),
    (CharacterFlags::SHUTUP, "SHUTUP"),
    (CharacterFlags::KICKED, "KICKED"),
    (CharacterFlags::UPDATE, "UPDATE"),
    (CharacterFlags::RESERVED0, "RESERVED0"),
    (CharacterFlags::RESERVED1, "RESERVED1"),
    (CharacterFlags::DEAD, "DEAD"),
    (CharacterFlags::ITEMS, "ITEMS"),
    (CharacterFlags::RESPAWN, "RESPAWN"),
    (CharacterFlags::MALE, "MALE"),
    (CharacterFlags::FEMALE, "FEMALE"),
    (CharacterFlags::WARRIOR, "WARRIOR"),
    (CharacterFlags::MAGE, "MAGE"),
    (CharacterFlags::ARCH, "ARCH"),
    (CharacterFlags::RESERVED2, "RESERVED2"),
    (CharacterFlags::NOATTACK, "NOATTACK"),
    (CharacterFlags::HASNAME, "HASNAME"),
    (CharacterFlags::QUESTITEM, "QUESTITEM"),
    (CharacterFlags::INFRARED, "INFRARED"),
    (CharacterFlags::PK, "PK"),
    (CharacterFlags::ITEMDEATH, "ITEMDEATH"),
    (CharacterFlags::NODEATH, "NODEATH"),
    (CharacterFlags::NOBODY, "NOBODY"),
    (CharacterFlags::EDEMON, "EDEMON"),
    (CharacterFlags::FDEMON, "FDEMON"),
    (CharacterFlags::IDEMON, "IDEMON"),
    (CharacterFlags::NOGIVE, "NOGIVE"),
    (CharacterFlags::PLAYERLIKE, "PLAYERLIKE"),
    (CharacterFlags::RESERVED3, "RESERVED3"),
    (CharacterFlags::PAID, "PAID"),
    (CharacterFlags::PROF, "PROF"),
    (CharacterFlags::ALIVE, "ALIVE"),
    (CharacterFlags::DEMON, "DEMON"),
    (CharacterFlags::UNDEAD, "UNDEAD"),
    (CharacterFlags::HARDKILL, "HARDKILL"),
    (CharacterFlags::NOBLESS, "NOBLESS"),
    (CharacterFlags::AREACHANGE, "AREACHANGE"),
    (CharacterFlags::LAG, "LAG"),
    (CharacterFlags::RESERVED4, "RESERVED4"),
    (CharacterFlags::THIEFMODE, "THIEFMODE"),
    (CharacterFlags::NOTELL, "NOTELL"),
    (CharacterFlags::INFRAVISION, "INFRAVISION"),
    (CharacterFlags::NOMAGIC, "NOMAGIC"),
    (CharacterFlags::NONOMAGIC, "NONOMAGIC"),
    (CharacterFlags::OXYGEN, "OXYGEN"),
    (CharacterFlags::NOPLRATT, "NOPLRATT"),
    (CharacterFlags::ALLOWSWAP, "ALLOWSWAP"),
    (CharacterFlags::LQMASTER, "LQMASTER"),
    (CharacterFlags::HARDCORE, "HARDCORE"),
    (CharacterFlags::NONOTIFY, "NONOTIFY"),
    (CharacterFlags::SMALLUPDATE, "SMALLUPDATE"),
    (CharacterFlags::NOWHO, "NOWHO"),
    (CharacterFlags::WON, "WON"),
    (CharacterFlags::NOEXP, "NOEXP"),
    (CharacterFlags::DEVELOPER, "DEVELOPER"),
    (CharacterFlags::EVENTMASTER, "EVENTMASTER"),
    (CharacterFlags::XRAY, "XRAY"),
    (CharacterFlags::NOLEVEL, "NOLEVEL"),
];

/// C `get_flag_by_name` (`command.c:4590-4782`), used only by
/// `/toggleflag`. Case-insensitive name -> flag-bit lookup; returns
/// `None` for an unknown name (C's `return 0`).
pub(crate) fn character_flag_by_name(name: &str) -> Option<CharacterFlags> {
    SHOW_FLAGS_ORDER
        .iter()
        .find(|(_, label)| label.eq_ignore_ascii_case(name))
        .map(|(flag, _)| *flag)
}

/// C `gl[]` (`src/system/command.c:132-207`) - the shortcut-destination
/// table shared by `/goto` and `/jump`. Copied name/x/y/area digit for
/// digit.
pub(crate) const GOTO_LIST: &[(&str, u16, u16, u32)] = &[
    ("aston", 167, 188, 3),
    ("elysium", 12, 178, 3),
    ("fort", 126, 179, 1),
    ("zomb1", 5, 5, 2),
    ("zomb2", 3, 86, 2),
    ("skel2", 85, 85, 1),
    ("skel3", 184, 226, 1),
    ("mages", 154, 106, 1),
    ("knights", 163, 82, 1),
    ("trans", 130, 201, 3),
    ("mine", 231, 242, 12),
    ("hole", 236, 176, 3),
    ("lq", 245, 245, 20),
    ("bran", 203, 227, 29),
    ("hole2", 226, 164, 29),
    ("smuggle", 103, 107, 26),
    ("yendor", 41, 250, 14),
    ("grim", 210, 247, 31),
    ("exkor", 67, 108, 17),
    ("job", 228, 228, 32),
    ("tunnel", 250, 250, 33),
    ("teufel", 250, 250, 34),
    ("rds", 245, 250, 3),
    ("swamps", 239, 237, 5),
    ("satp", 229, 94, 3),
    ("creep", 195, 120, 3),
    ("ark", 27, 14, 37),
    ("jail", 186, 234, 3),
    ("lab1", 32, 242, 22),
    ("lab2", 70, 98, 22),
    ("lab3", 230, 250, 22),
    ("lab4", 147, 103, 22),
    ("lab5", 166, 243, 22),
    ("max5s", 26, 26, 30),
    ("max10s", 109, 108, 30),
    ("max15s", 130, 26, 30),
    ("max18s", 181, 16, 30),
    ("max20s", 57, 26, 30),
    ("max24s", 73, 109, 30),
    ("max28s", 78, 16, 30),
    ("max30s", 12, 122, 30),
    ("max34s", 143, 76, 30),
    ("max36s", 212, 6, 30),
    ("max38s", 49, 112, 30),
    ("max40s", 171, 90, 30),
    ("max42s", 150, 57, 30),
    ("max43s", 212, 67, 30),
    ("max44s", 243, 16, 30),
    ("max45s", 231, 65, 30),
    ("max46s", 171, 61, 30),
    ("max48s", 120, 15, 30),
    ("max50s", 211, 47, 30),
    ("max52s", 16, 39, 30),
    ("max60s", 35, 59, 30),
    ("max64s", 233, 54, 30),
    ("max68s", 88, 35, 30),
    ("max76s", 121, 59, 30),
    ("max84s", 28, 90, 30),
    ("max92s", 34, 65, 30),
    ("max100s", 75, 67, 30),
    ("max108s", 109, 78, 30),
    ("max160s", 14, 140, 30),
    ("max200s", 40, 134, 30),
    ("mineshop10", 43, 232, 12),
    ("mineshop20", 43, 203, 12),
    ("mineshop30", 43, 171, 12),
    ("mineshop40", 43, 139, 12),
    ("mineshop50", 43, 107, 12),
    ("mineshop60", 43, 75, 12),
    ("mineshop70", 43, 43, 12),
    ("mineshop80", 43, 11, 12),
    ("mineshop90", 13, 239, 31),
    ("mineshop100", 13, 207, 31),
    ("mineshop110", 13, 175, 31),
    ("mineshop120", 13, 143, 31),
    ("teufeltp", 224, 248, 34),
    ("teufelicegambler", 84, 186, 34),
    ("teufelfiregambler", 123, 227, 34),
    ("teufelearthgambler", 248, 238, 34),
];

pub(crate) fn goto_list_lookup(name: &str) -> Option<(u16, u16, u32)> {
    GOTO_LIST
        .iter()
        .find(|(candidate, ..)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, x, y, a)| (*x, *y, *a))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GotoJumpTarget {
    x: i32,
    y: i32,
    a: i32,
    m: i32,
}

/// C `/goto`'s argument resolution (`command.c:8460-8535`). `ptr` is the
/// text after `"goto "` (already `trim_start`-ed by the caller isn't
/// required; this trims itself, matching C's `while (isspace(*ptr))
/// ptr++;`). Mirrors the exact pointer-stepping quirks of the original,
/// including the fact that a name lookup (`x == atoi(ptr) == 0` branch)
/// compares the *entire remaining string* against `gl[].name`/character
/// names (C `strcasecmp(gl[n].name, ptr)` with the untouched `ptr`) - so a
/// trailing mirror argument after a name is silently ignored (the name
/// simply fails to match anything, since the full remaining text no
/// longer equals just the name). `jump` doesn't call this: it has its own
/// simpler resolution (mirror-prefix, then a single `gl[]` name lookup,
/// no numeric/relative form) ported directly in the dispatcher above.
pub(crate) fn resolve_goto_jump_args(
    world: &World,
    caller_x: u16,
    caller_y: u16,
    args: &str,
) -> GotoJumpTarget {
    let mut ptr = args.trim_start();
    let x_val = legacy_atoi_prefix(ptr) as i32;
    let (mut x, mut y, mut a) = (0i32, 0i32, 0i32);
    if x_val == 0 {
        // Full remaining text (unmodified) is the name candidate - copies
        // the C `strcasecmp(gl[n].name, ptr)`/`strcasecmp(ch[n].name,
        // ptr)` full-string comparison exactly.
        if let Some((gx, gy, ga)) = goto_list_lookup(ptr) {
            x = i32::from(gx);
            y = i32::from(gy);
            a = ga as i32;
        } else if let Some(target_id) = find_online_character_by_name(world, ptr) {
            if let Some(target) = world.characters.get(&target_id) {
                x = i32::from(target.x);
                y = i32::from(target.y);
            }
        }
        // `ptr` is NOT advanced by the name lookup in C (strcasecmp
        // doesn't move the pointer) - the final "consume one token, then
        // parse m" step below still operates on the original `ptr`.
    } else {
        x = x_val;
        ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
        ptr = ptr.trim_start();
        let y_val = legacy_atoi_prefix(ptr) as i32;
        if y_val == 0 {
            match ptr.chars().next().map(|ch| ch.to_ascii_lowercase()) {
                Some('n') => {
                    y = i32::from(caller_y) - x;
                    x = i32::from(caller_x) - x;
                }
                Some('s') => {
                    y = i32::from(caller_y) + x;
                    x = i32::from(caller_x) + x;
                }
                Some('w') => {
                    y = i32::from(caller_y) + x;
                    x = i32::from(caller_x) - x;
                }
                Some('e') => {
                    y = i32::from(caller_y) - x;
                    x = i32::from(caller_x) + x;
                }
                _ => {
                    x = 0;
                    y = 0;
                }
            }
            // `ptr` still points at the direction-letter token (or
            // whatever failed to parse as a direction).
        } else {
            y = y_val;
            ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
            ptr = ptr.trim_start();
            a = legacy_atoi_prefix(ptr) as i32;
            // `ptr` still points at `a`'s token.
        }
    }

    // Consume whatever token `ptr` currently points at, then parse `m`
    // from the remainder (C `while (!isspace(*ptr) && *ptr) ptr++; while
    // (isspace(*ptr)) ptr++; m = atoi(ptr);`).
    ptr = ptr.trim_start_matches(|ch: char| !ch.is_whitespace());
    ptr = ptr.trim_start();
    let m = legacy_atoi_prefix(ptr) as i32;

    GotoJumpTarget { x, y, a, m }
}

/// Shared tail of `/goto` (`command.c:8537-8567`) and `/jump`
/// (`command.c:8608-8625`): apply the mirror change (if any), then either
/// same-area `teleport_char_driver` or the cross-area `change_area`
/// handoff via `cross_area_transfer` (the `main.rs` call site resolves it
/// through `attempt_cross_area_transfer`, falling back to the "target
/// area server is down" message on failure, matching every other
/// cross-area teleport site in this codebase).
pub(crate) fn finish_goto_jump(
    world: &mut World,
    character_id: CharacterId,
    x: i32,
    y: i32,
    a: i32,
    m: i32,
    verb: &'static str,
) -> KeyringCommandResult {
    let mirror_changed = (1..27).contains(&m).then_some(m as u32);

    if a != 0 {
        return KeyringCommandResult {
            cross_area_transfer: Some((
                a.clamp(0, i32::from(u16::MAX)) as u16,
                x.clamp(0, i32::from(u16::MAX)) as u16,
                y.clamp(0, i32::from(u16::MAX)) as u16,
            )),
            mirror_changed,
            ..Default::default()
        };
    }

    if x <= 0
        || y <= 0
        || !world
            .map
            .legacy_inner_bounds(x.max(0) as usize, y.max(0) as usize)
    {
        return KeyringCommandResult {
            mirror_changed,
            ..Default::default()
        };
    }

    if world.teleport_char_driver(character_id, x as u16, y as u16) {
        debug!(target: "client_log", verb, x, y, "goto/jump teleport");
    }

    KeyringCommandResult {
        mirror_changed,
        ..Default::default()
    }
}
