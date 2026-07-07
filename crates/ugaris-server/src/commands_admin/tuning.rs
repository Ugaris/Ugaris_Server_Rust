use super::*;

pub(crate) fn apply_legacy_tick_tuning_command(
    runtime: &mut ServerRuntime,
    lower: &str,
    rest: &str,
) -> Option<KeyringCommandResult> {
    struct TickTuningSpec {
        command: &'static str,
        min_len: usize,
        min: i32,
        max: i32,
        field: fn(&mut ServerRuntime) -> &mut i32,
        success: &'static str,
        invalid: &'static str,
    }

    let ticks = TICKS_PER_SECOND as i32;
    let specs = [
        TickTuningSpec {
            command: "setdecaytime",
            min_len: 12,
            min: 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.item_decay_time,
            success: "Item decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (1-60 minutes)",
        },
        TickTuningSpec {
            command: "setplayerbodytime",
            min_len: 17,
            min: 5 * 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.player_body_decay_time,
            success: "Player body decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (5-60 minutes)",
        },
        TickTuningSpec {
            command: "setnpcbodytime",
            min_len: 14,
            min: 30 * ticks,
            max: 30 * 60 * ticks,
            field: |runtime| &mut runtime.npc_body_decay_time,
            success: "NPC body decay time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (0.5-30 minutes)",
        },
        TickTuningSpec {
            command: "setnpcbodytimearea32",
            min_len: 20,
            min: 5 * 60 * ticks,
            max: 60 * 60 * ticks,
            field: |runtime| &mut runtime.npc_body_decay_time_area32,
            success: "NPC body decay time for area 32 changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (5-60 minutes)",
        },
        TickTuningSpec {
            command: "setrespawntime",
            min_len: 14,
            min: 30 * ticks,
            max: 10 * 60 * ticks,
            field: |runtime| &mut runtime.npc_respawn_timer,
            success: "NPC respawn time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (0.5-10 minutes)",
        },
        TickTuningSpec {
            command: "setsewerrespawntime",
            min_len: 19,
            min: 60 * 60,
            max: 60 * 60 * 24 * 7,
            field: |runtime| &mut runtime.sewer_item_respawn_time,
            success: "Sewer item respawn time changed from {old} to {new} seconds ({old_hours} to {new_hours} hours)",
            invalid: "Invalid value. Please specify a time between 3600 and 604800 seconds (1 hour to 7 days)",
        },
        TickTuningSpec {
            command: "setlagouttime",
            min_len: 13,
            min: 60 * ticks,
            max: 30 * 60 * ticks,
            field: |runtime| &mut runtime.lagout_time,
            success: "Lagout time changed from {old} to {new} ticks ({old_minutes} to {new_minutes} minutes)",
            invalid: "Invalid value. Please specify a time between {min} and {max} ticks (1-30 minutes)",
        },
        TickTuningSpec {
            command: "setregentime",
            min_len: 12,
            min: 1,
            max: 24,
            field: |runtime| &mut runtime.regen_time,
            success: "Regeneration time changed from {old} to {new} ticks",
            invalid: "Invalid value. Please specify a time between 1 and 24 ticks",
        },
    ];

    for spec in specs {
        if lower.len() < spec.min_len || !spec.command.starts_with(lower) {
            continue;
        }

        let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if (spec.min..=spec.max).contains(&value) {
            let field = (spec.field)(runtime);
            let old = *field;
            *field = value;
            return Some(KeyringCommandResult {
                messages: vec![spec
                    .success
                    .replace("{old}", &old.to_string())
                    .replace("{new}", &value.to_string())
                    .replace("{old_minutes}", &(old / (60 * ticks)).to_string())
                    .replace("{new_minutes}", &(value / (60 * ticks)).to_string())
                    .replace("{old_hours}", &(old / (60 * 60)).to_string())
                    .replace("{new_hours}", &(value / (60 * 60)).to_string())],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![spec
                .invalid
                .replace("{min}", &spec.min.to_string())
                .replace("{max}", &spec.max.to_string())],
            ..Default::default()
        });
    }

    None
}

pub(crate) fn apply_legacy_communication_tuning_command(
    runtime: &mut ServerRuntime,
    lower: &str,
    rest: &str,
) -> Option<KeyringCommandResult> {
    struct CommunicationTuningSpec {
        command: &'static str,
        min_len: usize,
        min: i32,
        max: i32,
        field: fn(&mut ServerRuntime) -> &mut i32,
        success: &'static str,
        invalid: &'static str,
        display_divisor: i32,
    }

    let say_dist = SAY_DIST as i32;
    let specs = [
        CommunicationTuningSpec {
            command: "sethollerdist",
            min_len: 13,
            min: say_dist,
            max: say_dist * 5,
            field: |runtime| &mut runtime.holler_dist,
            success: "Holler distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setshoutdist",
            min_len: 12,
            min: say_dist,
            max: say_dist * 4,
            field: |runtime| &mut runtime.shout_dist,
            success: "Shout distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setsaydist",
            min_len: 10,
            min: say_dist / 2,
            max: say_dist * 2,
            field: |runtime| &mut runtime.say_dist,
            success: "Say distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setemotedist",
            min_len: 12,
            min: say_dist / 4,
            max: say_dist,
            field: |runtime| &mut runtime.emote_dist,
            success: "Emote distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setquietsaydist",
            min_len: 15,
            min: say_dist / 6,
            max: say_dist / 2,
            field: |runtime| &mut runtime.quietsay_dist,
            success: "Quiet say distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "setwhisperdist",
            min_len: 14,
            min: 1,
            max: say_dist / 2,
            field: |runtime| &mut runtime.whisper_dist,
            success: "Whisper distance changed from {old} to {new} tiles",
            invalid: "Invalid value. Please specify a distance between {min} and {max} tiles",
            display_divisor: 1,
        },
        CommunicationTuningSpec {
            command: "sethollercost",
            min_len: 13,
            min: 5 * POWERSCALE,
            max: 20 * POWERSCALE,
            field: |runtime| &mut runtime.holler_cost,
            success: "Holler cost changed from {old} to {new} endurance points",
            invalid: "Invalid value. Please specify a cost between 5 and 20 endurance points",
            display_divisor: POWERSCALE,
        },
        CommunicationTuningSpec {
            command: "setshoutcost",
            min_len: 12,
            min: 2 * POWERSCALE,
            max: 10 * POWERSCALE,
            field: |runtime| &mut runtime.shout_cost,
            success: "Shout cost changed from {old} to {new} endurance points",
            invalid: "Invalid value. Please specify a cost between 2 and 10 endurance points",
            display_divisor: POWERSCALE,
        },
    ];

    for spec in specs {
        if lower.len() < spec.min_len || !spec.command.starts_with(lower) {
            continue;
        }
        let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if (spec.min..=spec.max).contains(&value) {
            let field = (spec.field)(runtime);
            let old = *field;
            *field = value;
            return Some(KeyringCommandResult {
                messages: vec![spec
                    .success
                    .replace("{old}", &(old / spec.display_divisor).to_string())
                    .replace("{new}", &(value / spec.display_divisor).to_string())],
                ..Default::default()
            });
        }

        return Some(KeyringCommandResult {
            messages: vec![spec
                .invalid
                .replace("{min}", &(spec.min / spec.display_divisor).to_string())
                .replace("{max}", &(spec.max / spec.display_divisor).to_string())],
            ..Default::default()
        });
    }

    None
}

/// Shared helper for the `GameSettings`-backed `/set*` tuning knobs ported
/// below (C `command.c`'s "Tool/Mine/Dungeon/Brannington/Pentagram/Clan/
/// Drop probability settings" blocks, `command.c:7113-8191`). Each C
/// handler is `cmdcmp(ptr, "<full name>", <minlen>)` gated on `CF_GOD`,
/// reads one `atoi(ptr)`, range-checks it, and on success both `log_char`s
/// a player-visible message and `xlog`s a server-log line; only the
/// `log_char` message is client-visible so that's the only one ported (the
/// same convention as the pre-existing `apply_legacy_tick_tuning_command`/
/// `apply_legacy_communication_tuning_command` above). `min_len` is copied
/// digit-for-digit from each `cmdcmp` call, including the cases where it is
/// far shorter than the full command name (an intentional legacy
/// abbreviation) - ordering of the `if let ... return` chain in the caller
/// must match the C `if` chain's source order exactly, since a short
/// abbreviation can be ambiguous across several of these names and C's
/// first-match-wins semantics depend on declaration order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_int_range_setting(
    world: &mut World,
    lower: &str,
    rest: &str,
    command: &str,
    min_len: usize,
    min: i32,
    max: i32,
    field: impl FnOnce(&mut GameSettings) -> &mut i32,
    success_template: &str,
    invalid: &str,
) -> Option<KeyringCommandResult> {
    if lower.len() < min_len || !command.starts_with(lower) {
        return None;
    }
    let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    if (min..=max).contains(&value) {
        let field_ref = field(&mut world.settings);
        let old = *field_ref;
        *field_ref = value;
        return Some(KeyringCommandResult {
            messages: vec![success_template
                .replace("{old}", &old.to_string())
                .replace("{new}", &value.to_string())],
            ..Default::default()
        });
    }
    Some(KeyringCommandResult {
        messages: vec![invalid.to_string()],
        ..Default::default()
    })
}

/// Same as `try_int_range_setting` but for the `atof(ptr)`-parsed `double`
/// knobs (`%.2f` old/new feedback in C).
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_f64_range_setting(
    world: &mut World,
    lower: &str,
    rest: &str,
    command: &str,
    min_len: usize,
    min: f64,
    max: f64,
    field: impl FnOnce(&mut GameSettings) -> &mut f64,
    success_template: &str,
    invalid: &str,
) -> Option<KeyringCommandResult> {
    if lower.len() < min_len || !command.starts_with(lower) {
        return None;
    }
    let value = legacy_atof_prefix(rest);
    if value >= min && value <= max {
        let field_ref = field(&mut world.settings);
        let old = *field_ref;
        *field_ref = value;
        return Some(KeyringCommandResult {
            messages: vec![success_template
                .replace("{old}", &format!("{old:.2}"))
                .replace("{new}", &format!("{value:.2}"))],
            ..Default::default()
        });
    }
    Some(KeyringCommandResult {
        messages: vec![invalid.to_string()],
        ..Default::default()
    })
}

/// C `command.c`'s "Remaining `/` and `#` text commands" `set*` knob
/// family that reads/writes `world.settings` (`ugaris_core::GameSettings`)
/// scalars already backed by `World`. Wired into `apply_admin_character_command`
/// only for `CF_GOD` callers, mirroring `apply_legacy_tick_tuning_command`.
pub(crate) fn apply_legacy_game_settings_tuning_command(
    world: &mut World,
    lower: &str,
    rest: &str,
) -> Option<KeyringCommandResult> {
    // command.c:7113
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setsplots",
        9,
        1000,
        10000,
        |s| &mut s.sp_lots,
        "Special item probability 'lots' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 1000 and 10000",
    ) {
        return Some(r);
    }
    // command.c:7133
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setspmany",
        9,
        500,
        5000,
        |s| &mut s.sp_many,
        "Special item probability 'many' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 500 and 5000",
    ) {
        return Some(r);
    }
    // command.c:7153
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setspsome",
        9,
        100,
        1000,
        |s| &mut s.sp_some,
        "Special item probability 'some' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 100 and 1000",
    ) {
        return Some(r);
    }
    // command.c:7173
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setspfew",
        8,
        10,
        200,
        |s| &mut s.sp_few,
        "Special item probability 'few' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 10 and 200",
    ) {
        return Some(r);
    }
    // command.c:7193
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setsprare",
        9,
        2,
        50,
        |s| &mut s.sp_rare,
        "Special item probability 'rare' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 2 and 50",
    ) {
        return Some(r);
    }
    // command.c:7213
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setspultra",
        10,
        1,
        10,
        |s| &mut s.sp_ultra,
        "Special item probability 'ultra' category changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10",
    ) {
        return Some(r);
    }
    // command.c:7234
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setorbrespawndays",
        17,
        1,
        90,
        |s| &mut s.base_orb_respawn_time_days,
        "Orb respawn time changed from {old} to {new} days",
        "Invalid value. Please specify a value between 1 and 90 days",
    ) {
        return Some(r);
    }
    // command.c:7255
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setmaxjewelcount",
        16,
        1,
        5,
        |s| &mut s.max_jewel_count,
        "Maximum jewel count changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 5",
    ) {
        return Some(r);
    }
    // command.c:7275
    if let Some(r) = try_f64_range_setting(
        world,
        lower,
        rest,
        "settunnelexpdivider",
        19,
        1.0,
        10.0,
        |s| &mut s.tunnel_exp_base_value_divider,
        "Tunnel experience base value divider changed from {old} to {new}",
        "Invalid value. Please specify a value between 1.0 and 10.0",
    ) {
        return Some(r);
    }
    // command.c:7296
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "settunnelmillexp",
        16,
        50,
        500,
        |s| &mut s.tunnel_mill_exp_base_value,
        "Tunnel mill experience base value changed from {old} to {new}",
        "Invalid value. Please specify a value between 50 and 500",
    ) {
        return Some(r);
    }
    // command.c:7317
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setraregolemchance",
        18,
        5,
        100,
        |s| &mut s.rare_golem_chance,
        "Rare golem chance changed from {old} to {new}",
        "Invalid value. Please specify a value between 5 and 100",
    ) {
        return Some(r);
    }
    // command.c:7337
    {
        let ticks = TICKS_PER_SECOND as i32;
        let min_time = ticks * 30 * 60;
        let max_time = ticks * 120 * 60;
        if lower.len() >= 14 && "setdungeontime".starts_with(lower) {
            let value =
                legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            if (min_time..=max_time).contains(&value) {
                let old = world.settings.dungeon_time;
                world.settings.dungeon_time = value;
                return Some(KeyringCommandResult {
                    messages: vec![format!(
                        "Dungeon time limit changed from {old} to {value} ticks ({} to {} minutes)",
                        old / (ticks * 60),
                        value / (ticks * 60)
                    )],
                    ..Default::default()
                });
            }
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Invalid value. Please specify a time between {min_time} and {max_time} ticks (30-120 minutes)"
                )],
                ..Default::default()
            });
        }
    }
    // command.c:7362
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setbranfoexpbase",
        16,
        5000,
        20000,
        |s| &mut s.branfo_exp_base,
        "Brannington Forest experience base changed from {old} to {new}",
        "Invalid value. Please specify a value between 5000 and 20000",
    ) {
        return Some(r);
    }
    // command.c:7382
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setbranexpbase",
        14,
        10000,
        30000,
        |s| &mut s.bran_exp_base,
        "Brannington experience base changed from {old} to {new}",
        "Invalid value. Please specify a value between 10000 and 30000",
    ) {
        return Some(r);
    }
    // command.c:7403
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentvismaxpents",
        18,
        5,
        20,
        |s| &mut s.max_visible_pents,
        "Maximum visible pentagrams changed from {old} to {new}",
        "Invalid value. Please specify a value between 5 and 20",
    ) {
        return Some(r);
    }
    // command.c:7423
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentmaxpower",
        15,
        1000,
        5000,
        |s| &mut s.max_power_level,
        "Maximum pentagram power level changed from {old} to {new}",
        "Invalid value. Please specify a value between 1000 and 5000",
    ) {
        return Some(r);
    }
    // command.c:7610
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setmaxsilvergolemtype",
        6,
        1,
        20,
        |s| &mut s.max_silver_golem_type,
        "Max silver golem type changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 20",
    ) {
        return Some(r);
    }
    // command.c:7630
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setnormaldropchance",
        6,
        5,
        100,
        |s| &mut s.normal_drop_chance,
        "Normal drop chance changed from {old} to {new}",
        "Invalid value. Please specify a value between 5 and 100",
    ) {
        return Some(r);
    }
    // command.c:7650
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setraredropchance",
        6,
        20,
        200,
        |s| &mut s.rare_drop_chance,
        "Rare drop chance changed from {old} to {new}",
        "Invalid value. Please specify a value between 20 and 200",
    ) {
        return Some(r);
    }
    // command.c:7669 (C `float`, not `double` - kept as f32 to match the
    // pre-existing `rare_drop_multiplier: f32` field).
    if lower.len() >= 6 && "setraredropmultiplier".starts_with(lower) {
        let value = legacy_atof_prefix(rest) as f32;
        if (1.0..=3.0).contains(&value) {
            let old = world.settings.rare_drop_multiplier;
            world.settings.rare_drop_multiplier = value;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Rare drop multiplier changed from {old:.2} to {value:.2}"
                )],
                ..Default::default()
            });
        }
        return Some(KeyringCommandResult {
            messages: vec!["Invalid value. Please specify a value between 1.0 and 3.0".to_string()],
            ..Default::default()
        });
    }
    // command.c:7689
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setbasedropmultiplier",
        6,
        1,
        20,
        |s| &mut s.base_drop_multiplier,
        "Base drop multiplier changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 20",
    ) {
        return Some(r);
    }
    // command.c:7709
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setleveldivisor",
        9,
        1,
        20,
        |s| &mut s.level_divisor,
        "Level divisor changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 20",
    ) {
        return Some(r);
    }
    // command.c:7728
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setraregolemboost",
        6,
        1,
        5,
        |s| &mut s.rare_golem_level_boost,
        "Rare golem level boost changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 5",
    ) {
        return Some(r);
    }
    // command.c:7748
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setgolemhpmultiplier",
        6,
        1,
        5,
        |s| &mut s.rare_golem_hp_multiplier,
        "Rare golem HP multiplier changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 5",
    ) {
        return Some(r);
    }
    // command.c:7769
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setdemonlordaccess",
        6,
        30,
        300,
        |s| &mut s.demon_lord_door_after_solve_access_time,
        "Demon lord door access time changed from {old} to {new} seconds",
        "Invalid value. Please specify a value between 30 and 300 seconds",
    ) {
        return Some(r);
    }
    // command.c:7790
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setsolvemaxdivisor",
        6,
        1,
        10,
        |s| &mut s.solve_max_divisor,
        "Solve max divisor changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10",
    ) {
        return Some(r);
    }
    // command.c:7809
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setdemonpowerdeduction",
        6,
        100,
        2000,
        |s| &mut s.demon_power_deduction,
        "Demon power deduction changed from {old} to {new}",
        "Invalid value. Please specify a value between 100 and 2000",
    ) {
        return Some(r);
    }
    // command.c:7829
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentvaluemultiplier",
        6,
        10,
        100,
        |s| &mut s.pentagram_value_multiplier,
        "Pentagram value multiplier changed from {old} to {new}",
        "Invalid value. Please specify a value between 10 and 100",
    ) {
        return Some(r);
    }
    // command.c:7849
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentworthdivisor",
        6,
        2,
        20,
        |s| &mut s.pentagram_worth_divisor,
        "Pentagram worth divisor changed from {old} to {new}",
        "Invalid value. Please specify a value between 2 and 20",
    ) {
        return Some(r);
    }
    // command.c:7869
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setluckypentchance",
        6,
        1,
        1000,
        |s| &mut s.lucky_pentagram_chance,
        "Lucky pentagram chance changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 1000",
    ) {
        return Some(r);
    }
    // command.c:7889
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpowerincrement",
        6,
        5,
        50,
        |s| &mut s.power_increment,
        "Power increment changed from {old} to {new}",
        "Invalid value. Please specify a value between 5 and 50",
    ) {
        return Some(r);
    }
    // command.c:7908
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentmaxtraining",
        6,
        500,
        3000,
        |s| &mut s.max_training_power,
        "Maximum pentagram training power changed from {old} to {new}",
        "Invalid value. Please specify a value between 500 and 3000",
    ) {
        return Some(r);
    }
    // command.c:7928
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentrandomspawn",
        6,
        1,
        50,
        |s| &mut s.random_spawn_chance,
        "Pentagram random spawn chance changed from {old} to {new} percent",
        "Invalid value. Please specify a value between 1 and 50 percent",
    ) {
        return Some(r);
    }
    // command.c:7948
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setpentspawncount",
        6,
        1,
        10,
        |s| &mut s.activation_spawn_count,
        "Pentagram activation spawn count changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10",
    ) {
        return Some(r);
    }
    // command.c:7968
    if let Some(r) = try_f64_range_setting(
        world,
        lower,
        rest,
        "setexpsolve",
        6,
        0.1,
        2.0,
        |s| &mut s.exp_solve_multiplier,
        "Experience solve multiplier changed from {old} to {new}",
        "Invalid value. Please specify a value between 0.1 and 2.0",
    ) {
        return Some(r);
    }
    // command.c:7988
    if let Some(r) = try_f64_range_setting(
        world,
        lower,
        rest,
        "setclanreflection",
        6,
        0.1,
        1.0,
        |s| &mut s.exp_clan_reflection_multiplier,
        "Clan reflection multiplier changed from {old} to {new}",
        "Invalid value. Please specify a value between 0.1 and 1.0",
    ) {
        return Some(r);
    }
    // command.c:8008
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setmaxclanbonus",
        6,
        5,
        50,
        |s| &mut s.max_clan_bonus_percent,
        "Maximum clan bonus percentage changed from {old} to {new} percent",
        "Invalid value. Please specify a value between 5 and 50 percent",
    ) {
        return Some(r);
    }
    // command.c:8030 - "int x = atoi(ptr); while(isdigit(*ptr)) ptr++; ..."
    // triple-token parse; only the positive-x/y/area success path can be
    // reached in practice (the isdigit-skip has a real quirk of not
    // stepping over a leading '-' but that only matters for negative
    // inputs, which always fail the `x > 0 && y > 0 && area > 0` guard
    // anyway, so it is not reproduced here).
    if lower.len() >= 6 && "setjaillocation".starts_with(lower) {
        let (x, y, area) = parse_legacy_xyz_triple(rest);
        if x > 0 && y > 0 && area > 0 {
            let (old_x, old_y, old_area) = (
                world.settings.jail_x,
                world.settings.jail_y,
                world.settings.jail_area,
            );
            world.settings.jail_x = x;
            world.settings.jail_y = y;
            world.settings.jail_area = area;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Jail location changed from {old_x},{old_y} (area {old_area}) to {x},{y} (area {area})"
                )],
                ..Default::default()
            });
        }
        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid coordinates or area. Format: /setjaillocation x y area".to_string(),
            ],
            ..Default::default()
        });
    }
    // command.c:8070
    if lower.len() >= 6 && "setastonlocation".starts_with(lower) {
        let (x, y, area) = parse_legacy_xyz_triple(rest);
        if x > 0 && y > 0 && area > 0 {
            let (old_x, old_y, old_area) = (
                world.settings.aston_x,
                world.settings.aston_y,
                world.settings.aston_area,
            );
            world.settings.aston_x = x;
            world.settings.aston_y = y;
            world.settings.aston_area = area;
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Aston location changed from {old_x},{old_y} (area {old_area}) to {x},{y} (area {area})"
                )],
                ..Default::default()
            });
        }
        return Some(KeyringCommandResult {
            messages: vec![
                "Invalid coordinates or area. Format: /setastonlocation x y area".to_string(),
            ],
            ..Default::default()
        });
    }
    // command.c:8112 - C stores `get_special_item_drop_multiplier()`
    // (a `double`) into an `int old_value` before printing it with `%d`
    // (a real truncating-assignment quirk in the C source), then prints
    // the new value with a bare `%f` (default 6 fractional digits).
    if lower.len() >= 15 && "setspecialdropmult".starts_with(lower) {
        let value = legacy_atoi_prefix(rest).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        if (1..=10000).contains(&value) {
            let old = world.settings.special_item_drop_multiplier as i32;
            world.settings.special_item_drop_multiplier = f64::from(value);
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Special item drop multiplier changed from {old} to {:.6}",
                    world.settings.special_item_drop_multiplier
                )],
                ..Default::default()
            });
        }
        return Some(KeyringCommandResult {
            messages: vec!["Invalid value. Please specify a value between 1 and 10000".to_string()],
            ..Default::default()
        });
    }
    // command.c:8132
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setdropproblow",
        13,
        1,
        10000,
        |s| &mut s.drop_prob_low_level,
        "Drop probability (low level) changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10000",
    ) {
        return Some(r);
    }
    // command.c:8152
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setdropprobmid",
        13,
        1,
        10000,
        |s| &mut s.drop_prob_mid_level,
        "Drop probability (mid level) changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10000",
    ) {
        return Some(r);
    }
    // command.c:8172
    if let Some(r) = try_int_range_setting(
        world,
        lower,
        rest,
        "setdropprobhigh",
        13,
        1,
        10000,
        |s| &mut s.drop_prob_high_level,
        "Drop probability (high level) changed from {old} to {new}",
        "Invalid value. Please specify a value between 1 and 10000",
    ) {
        return Some(r);
    }
    // command.c:8192 - `loot_reload()` clears the registry and rescans
    // `LOOT_DATA_DIR` (`resolve_loot_root`/`load_loot_tables`, the same
    // pair the startup path in `main.rs` uses), returning the resulting
    // table count; C's `n < 0` failure branch has no direct Rust
    // equivalent (per-file parse failures are warned-and-skipped, not a
    // hard failure) so a missing loot-data root is the closest analogue.
    if lower.len() >= 10 && "reloadloot".starts_with(lower) {
        world.loot_registry.clear_tables();
        if let Some(loot_root) = resolve_loot_root(None) {
            load_loot_tables(&mut world.loot_registry, &loot_root);
            let n = world.loot_registry.table_count();
            let mut message = COL_LIGHT_GREEN.to_vec();
            message.extend_from_slice(b"Loot tables reloaded:");
            message.extend_from_slice(COL_RESET);
            message.extend_from_slice(format!(" {n} active").as_bytes());
            return Some(KeyringCommandResult {
                message_bytes: vec![message],
                ..Default::default()
            });
        }
        let mut message = COL_LIGHT_RED.to_vec();
        message.extend_from_slice("Loot reload failed — check server log.".as_bytes());
        message.extend_from_slice(COL_RESET);
        return Some(KeyringCommandResult {
            message_bytes: vec![message],
            ..Default::default()
        });
    }
    // command.c:8203 - `#setlootmod <name> <value>`: `name` is the first
    // whitespace-delimited token (up to 63 bytes, C `modname[64]`), value
    // is `atof(ptr)` on whatever follows after skipping whitespace. C
    // rejects an empty name or a negative value with a usage message
    // without touching `loot_set_modifier` at all (the function itself
    // has no range check - see `GameSettings::set_loot_modifier`).
    if lower.len() >= 10 && "setlootmod".starts_with(lower) {
        let ptr = rest.trim_start();
        let (raw_name, remainder) = ptr.split_once(char::is_whitespace).unwrap_or((ptr, ""));
        let modname: String = raw_name.chars().take(63).collect();
        let modval = legacy_atof_prefix(remainder.trim_start());
        if modname.is_empty() || modval < 0.0 {
            return Some(KeyringCommandResult {
                messages: vec!["Usage: #setlootmod <name> <value>".to_string()],
                ..Default::default()
            });
        }
        world.settings.set_loot_modifier(&modname, modval);
        let mut message = COL_LIGHT_GREEN.to_vec();
        message.extend_from_slice(b"Loot modifier");
        message.extend_from_slice(COL_RESET);
        message.extend_from_slice(format!(" {modname} = {modval:.3}").as_bytes());
        return Some(KeyringCommandResult {
            message_bytes: vec![message],
            ..Default::default()
        });
    }

    None
}

/// C `/global` (`command.c:8226-8322`), `cmdcmp(ptr, "global", 2)` +
/// `CF_GOD`-gated: dumps every tunable `GameSettings` value as a multi-line
/// admin report. Read-only (no field is mutated), so it only needs `&World`.
/// Every line, section header, and format quirk (including the C source's
/// own inconsistent spacing on the three "Drop probability" lines - a space
/// before the dash for "low level" but not for "mid"/"high" level) is
/// transcribed digit-for-digit / letter-for-letter rather than "fixed".
pub(crate) fn apply_global_settings_command(
    world: &World,
    lower: &str,
) -> Option<KeyringCommandResult> {
    if !(lower.len() >= 2 && "global".starts_with(lower)) {
        return None;
    }

    let s = &world.settings;
    let ticks = TICKS_PER_SECOND as i32;
    let messages = vec![
        "=== Current Global Settings ===".to_string(),
        "--- Core Server Settings ---".to_string(),
        format!(
            "Item decay time: {} ticks ({} minutes)",
            s.item_decay_time,
            s.item_decay_time / (60 * ticks)
        ),
        format!(
            "Player body decay time: {} ticks ({} minutes)",
            s.player_body_decay_time,
            s.player_body_decay_time / (60 * ticks)
        ),
        format!(
            "NPC body decay time: {} ticks ({} minutes)",
            s.npc_body_decay_time,
            s.npc_body_decay_time / (60 * ticks)
        ),
        format!(
            "NPC body decay time (area 32): {} ticks ({} minutes)",
            s.npc_body_decay_time_area32,
            s.npc_body_decay_time_area32 / (60 * ticks)
        ),
        format!(
            "Respawn time: {} ticks ({} minutes)",
            s.npc_respawn_timer,
            s.npc_respawn_timer / (60 * ticks)
        ),
        format!(
            "Lagout time: {} ticks ({} minutes)",
            s.lagout_time,
            s.lagout_time / (60 * ticks)
        ),
        format!("Regen time: {} ticks", s.regen_time),
        format!(
            "Sewer item respawn time: {} hours",
            s.sewer_item_respawn_time / 3600
        ),
        "--- Experience Modifiers ---".to_string(),
        format!("Global EXP modifier: {:.2}", s.exp_modifier),
        format!(
            "Hardcore military EXP bonus: {:.2}",
            s.hardcore_military_exp_bonus
        ),
        format!("Hardcore EXP bonus: {:.2}", s.hardcore_exp_bonus),
        format!("Hardcore kill EXP bonus: {:.2}", s.hardcore_kill_exp_bonus),
        "--- Communication Settings ---".to_string(),
        format!(
            "Holler distance: {} tiles, Cost: {}",
            s.holler_dist,
            s.holler_cost / POWERSCALE
        ),
        format!(
            "Shout distance: {} tiles, Cost: {}",
            s.shout_dist,
            s.shout_cost / POWERSCALE
        ),
        format!("Say distance: {} tiles", s.say_dist),
        format!("Emote distance: {} tiles", s.emote_dist),
        format!("Quiet say distance: {} tiles", s.quietsay_dist),
        format!("Whisper distance: {} tiles", s.whisper_dist),
        "--- Tool Settings ---".to_string(),
        format!("Special item probability - Lots: {}", s.sp_lots),
        format!("Special item probability - Many: {}", s.sp_many),
        format!("Special item probability - Some: {}", s.sp_some),
        format!("Special item probability - Few: {}", s.sp_few),
        format!("Special item probability - Rare: {}", s.sp_rare),
        format!("Special item probability - Ultra: {}", s.sp_ultra),
        "--- Location Settings ---".to_string(),
        format!(
            "Jail location: {},{} (area {})",
            s.jail_x, s.jail_y, s.jail_area
        ),
        format!(
            "Aston location: {},{} (area {})",
            s.aston_x, s.aston_y, s.aston_area
        ),
        format!("Orb respawn time: {} days", s.base_orb_respawn_time_days),
        "--- Clan Settings ---".to_string(),
        format!("Maximum jewel count: {}", s.max_jewel_count),
        format!(
            "Max clan bonus percent: {}%",
            s.get_max_clan_bonus_percent()
        ),
        format!(
            "Clan reflection multiplier: {:.2}",
            s.get_exp_clan_reflection_multiplier()
        ),
        "--- Tunnel Settings ---".to_string(),
        format!(
            "Tunnel exp base value divider: {:.2}",
            s.tunnel_exp_base_value_divider
        ),
        format!(
            "Tunnel mill exp base value: {}",
            s.tunnel_mill_exp_base_value
        ),
        "--- Mine Settings ---".to_string(),
        format!("Rare golem chance: {}", s.get_rare_golem_chance()),
        format!("Max silver golem type: {}", s.get_max_silver_golem_type()),
        format!("Normal drop chance: {}", s.get_normal_drop_chance()),
        format!("Rare drop chance: {}", s.get_rare_drop_chance()),
        format!("Rare drop multiplier: {:.2}", s.get_rare_drop_multiplier()),
        format!("Base drop multiplier: {}", s.get_base_drop_multiplier()),
        format!("Level divisor: {}", s.get_level_divisor()),
        format!("Rare golem level boost: {}", s.get_rare_golem_level_boost()),
        format!(
            "Rare golem HP multiplier: {}",
            s.get_rare_golem_hp_multiplier()
        ),
        "--- Dungeon Settings ---".to_string(),
        format!(
            "Dungeon time: {} ticks ({} minutes)",
            s.get_dungeon_time(),
            s.get_dungeon_time() / (ticks * 60)
        ),
        "--- Brannington Settings ---".to_string(),
        format!("Brannington Forest exp base: {}", s.get_branfo_exp_base()),
        format!("Brannington exp base: {}", s.get_bran_exp_base()),
        "--- Pentagram Settings ---".to_string(),
        format!("Max visible pentagrams: {}", s.get_max_visible_pents()),
        format!("Max power level: {}", s.get_max_power_level()),
        format!("Max training power: {}", s.get_max_training_power()),
        format!("Demon power deduction: {}", s.get_demon_power_deduction()),
        format!("Random spawn chance: {}%", s.get_random_spawn_chance()),
        format!("Activation spawn count: {}", s.get_activation_spawn_count()),
        format!(
            "Pentagram value multiplier: {}",
            s.get_pentagram_value_multiplier()
        ),
        format!(
            "Pentagram worth divisor: {}",
            s.get_pentagram_worth_divisor()
        ),
        format!("Lucky pentagram chance: {}", s.get_lucky_pentagram_chance()),
        format!("Power increment: {}", s.get_power_increment()),
        format!("Solve max divisor: {}", s.get_solve_max_divisor()),
        format!(
            "Demon lord door access: {} seconds",
            s.get_demon_lord_door_after_solve_access_time()
        ),
        format!(
            "Experience solve multiplier: {:.2}",
            s.get_exp_solve_multiplier()
        ),
        "--- Drop Probability Settings ---".to_string(),
        format!(
            "Drop probability (low level): {} - (default 1700)",
            s.get_drop_prob_low_level()
        ),
        format!(
            "Drop probability (mid level): {}- (default 800)",
            s.get_drop_prob_mid_level()
        ),
        format!(
            "Drop probability (high level): {}- (default 532)",
            s.get_drop_prob_high_level()
        ),
    ];

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}
