use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_exp_military(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    if lower == "setskill" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (name, pos, val) = parse_setskill_args(rest);
        let Some(target_id) = find_online_character_by_name(world, &name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        if pos < 0 || pos >= CHARACTER_VALUE_NAMES.len() as i64 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Position out of bounds.".to_string()],
                ..Default::default()
            }));
        }
        if !(0..=255).contains(&val) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Value out of bounds.".to_string()],
                ..Default::default()
            }));
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
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
        return ControlFlow::Break(Some(KeyringCommandResult {
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
        }));
    }

    if lower == "setlevel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
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
        return ControlFlow::Break(Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        }));
    }

    if lower.len() >= 3 && "exp".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            }));
        }
        if exp != 0 {
            give_exp_with_runtime_modifiers(world, target_id, exp, area_id);
            let Some(target) = world.characters.get(&target_id) else {
                return ControlFlow::Break(Some(KeyringCommandResult::default()));
            };
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            }));
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        }));
    }

    if lower == "milexp" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (target_id, target_name, exp) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            }));
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
                return ControlFlow::Break(Some(KeyringCommandResult::default()));
            };
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Gave {} {} military exp.", target.name, exp)],
                inventory_changed: true,
                ..Default::default()
            }));
        }

        let target = world
            .characters
            .get(&target_id)
            .expect("target just checked");
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("{} has {} exp.", target.name, target.exp)],
            ..Default::default()
        }));
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
    if lower.len() >= 8 && "labsolved".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (target_id, target_name, val) = parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            }));
        }

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get lab data for {target_name}.")],
                ..Default::default()
            }));
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
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_milinfo` (`command.c:5071-5160`, `CF_GOD`-gated,
    // `command.c:10085-10091`): full-word `/milinfo [name]`, self if no
    // name given.
    if lower == "milinfo" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return ControlFlow::Break(Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    }));
                }
            }
        };
        let Some(target) = world.characters.get(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        let target_name = target.name.clone();
        let military_points = target.military_points;
        let military_normal_exp = target.military_normal_exp;
        let current_yday = world.date.yday + 1;

        let Some(player) = runtime.player_for_character(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            }));
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

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_milpref` (`command.c:5169-5249`, `CF_GOD`-gated): sets a
    // player's mission type/difficulty preference. Name is required
    // (unlike `milinfo`/`milreset`/`milsolve`'s self-fallback) - C prints
    // the 3-line usage block instead.
    if lower == "milpref" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /milpref <character> <type> <difficulty>".to_string(),
                    "Types: 0=none, 1=demon, 2=ratling, 3=silver".to_string(),
                    "Difficulties: 0=easy, 1=normal, 2=hard, 3=impossible, 4=insane, -1=none"
                        .to_string(),
                ],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
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
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            }));
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

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_milreset` (`command.c:5258-5304`, `CF_GOD`-gated): resets
    // mission/advisor cooldowns, self if no name given.
    if lower == "milreset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, _) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return ControlFlow::Break(Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    }));
                }
            }
        };
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            }));
        };

        player.set_mission_yday(0);
        player.set_military_solved_yday(0);
        player.set_military_took_mission(0);
        player.set_military_reroll_yday(0);
        for advisor in 0..MILITARY_PPD_MAXADVISOR {
            player.set_military_advisor_last(advisor, 0);
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset all mission and advisor cooldowns for {target_name}"
            )],
            ..Default::default()
        }));
    }

    // C `cmd_milpoints` (`command.c:5313-5384`, `CF_GOD`-gated): grants
    // raw military points to a named player. Name is required (no self
    // fallback). Deliberately does NOT call `World::give_military_pts`
    // (see the promotion-block comment below for why).
    if lower == "milpoints" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /milpoints <character> <points>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Please specify number of points to grant.".to_string()],
                ..Default::default()
            }));
        }

        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
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

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_milrec` (`command.c:5393-5446`, `CF_GOD`-gated): grants
    // "recommendation points" (`ppd->current_pts`) to a named player.
    if lower == "milrec" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Usage: /milrec <character> <points>".to_string()],
                ..Default::default()
            }));
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };

        let points = legacy_atoi_prefix(remainder.trim_start()) as i32;
        if points == 0 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![
                    "Please specify number of recommendation points to grant.".to_string()
                ],
                ..Default::default()
            }));
        }

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let Some(player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            }));
        };
        let new_total = player.military_current_pts().saturating_add(points);
        player.set_military_current_pts(new_total);

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Granted {points} recommendation points to {target_name} (total: {new_total})"
            )],
            ..Default::default()
        }));
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
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec!["Could not find Military Master NPC.".to_string()],
            ..Default::default()
        }));
    }

    // C `cmd_milsolve` (`command.c:5498-5613`, `CF_GOD`-gated): force-
    // completes a player's active mission, self if no name given, with an
    // optional trailing `announce` flag that also broadcasts a high-rank
    // promotion and notifies the target player directly.
    if lower == "milsolve" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let target_id = if name.is_empty() {
            character_id
        } else {
            match find_online_character_by_name(world, name) {
                Some(id) => id,
                None => {
                    return ControlFlow::Break(Some(KeyringCommandResult {
                        messages: vec![format!("Sorry, no one by the name {name} around.")],
                        ..Default::default()
                    }));
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
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Could not get military data for {target_name}.")],
                ..Default::default()
            }));
        };

        let took_mission = player.military_took_mission();
        if took_mission == 0 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("{target_name} does not have an active mission.")],
                ..Default::default()
            }));
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
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
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

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

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
