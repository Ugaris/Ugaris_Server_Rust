use super::*;

pub(crate) fn legacy_achievement_colored_line(color: &[u8], text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(color.len() + text.len() + COL_RESET.len());
    out.extend_from_slice(color);
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(COL_RESET);
    out
}

/// `strftime(..., "%Y-%m-%d", localtime(&timestamp))` (`achievement.c:1416`)
/// approximated in UTC (no `chrono`/timezone database dependency in this
/// workspace); `timestamp` is always a real unlock time (`0` is filtered
/// out by the caller before this runs), so the u64 cast is safe.
fn legacy_achievement_date(timestamp: i64) -> String {
    let (year, month, day) = civil_from_unix_seconds(timestamp.max(0) as u64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// C `achievement_list` (`achievement.c:1421-1452`).
pub(crate) fn legacy_achievement_list_lines(data: &AccountAchievements) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    lines.push(legacy_achievement_colored_line(
        COL_ORANGE,
        "=== Your Achievements ===",
    ));

    let mut unlocked_count = 0usize;
    for ty in AchievementType::ALL {
        let achievement = &data.achievements[ty as usize];
        if achievement.timestamp == 0 {
            continue;
        }
        let def = achievement_def(ty);
        let date = legacy_achievement_date(achievement.timestamp);
        let mut line = Vec::new();
        line.extend_from_slice(COL_LIGHT_GREEN);
        line.extend_from_slice(format!("[+] {}", def.name).as_bytes());
        line.extend_from_slice(COL_RESET);
        line.extend_from_slice(
            format!(
                " - {} ({date} by {})",
                def.description, achievement.achieved_by
            )
            .as_bytes(),
        );
        lines.push(line);
        unlocked_count += 1;
    }

    if unlocked_count == 0 {
        lines.push(b"You haven't unlocked any achievements yet. Keep playing!".to_vec());
    } else {
        lines.push(legacy_achievement_colored_line(
            COL_DARK_GRAY,
            &format!("Unlocked: {unlocked_count}/{ACHIEVEMENT_TYPE_COUNT} achievements"),
        ));
    }
    lines
}

/// C `achievement_show_stats` (`achievement.c:1453-1499`).
pub(crate) fn legacy_achievement_stats_lines(stats: &AchievementStats) -> Vec<Vec<u8>> {
    let heading = |text: &str| legacy_achievement_colored_line(COL_ORANGE, text);
    let plain = |text: String| text.into_bytes();
    vec![
        heading("=== Achievement Statistics ==="),
        heading("Gathering:"),
        plain(format!("  Flowers picked: {}", stats.flowers_picked)),
        plain(format!("  Mushrooms picked: {}", stats.mushrooms_picked)),
        plain(format!("  Berries picked: {}", stats.berries_picked)),
        plain(format!("  Potions brewed: {}", stats.potions_brewed)),
        heading("Combat:"),
        plain(format!("  Enemies killed: {}", stats.enemies_killed)),
        plain(format!("  Demons defeated: {}", stats.demons_defeated)),
        plain(format!(
            "    Earth: {}, Fire: {}, Ice: {}, Hell: {}",
            stats.demons_per_area[PentArea::Earth as usize],
            stats.demons_per_area[PentArea::Fire as usize],
            stats.demons_per_area[PentArea::Ice as usize],
            stats.demons_per_area[PentArea::Hell as usize],
        )),
        plain(format!("  PvP kills: {}", stats.pvp_kills)),
        heading("Pentagram:"),
        plain(format!("  Pents solved: {}", stats.pents_solved)),
        plain(format!(
            "    Earth: {}, Fire: {}, Ice: {}, Hell: {}",
            stats.pents_per_area[PentArea::Earth as usize],
            stats.pents_per_area[PentArea::Fire as usize],
            stats.pents_per_area[PentArea::Ice as usize],
            stats.pents_per_area[PentArea::Hell as usize],
        )),
        heading("Collection:"),
        plain(format!("  Chests opened: {}", stats.chests_opened)),
        plain(format!(
            "  Stones: Earth {}, Fire {}, Ice {}",
            stats.earth_stones, stats.fire_stones, stats.ice_stones,
        )),
        heading("Mining:"),
        plain(format!("  Silver mined: {}", stats.silver_mined)),
        plain(format!("  Gold mined: {}", stats.gold_mined)),
        heading("Missions:"),
        plain(format!("  Military missions: {}", stats.military_missions)),
        plain(format!("  Tunnel levels: {}", stats.tunnel_levels)),
    ]
}

/// C `achievement_award`'s congratulations text (`achievement.c:621-624`),
/// only ever shown with `show_congrats = 1`, which in the whole codebase
/// is exclusively the `/achgive` admin path (`command.c:9126`) - every
/// other C call site either passes `0` (silent, e.g. `achievement_fix_
/// all`) or is one of the not-yet-wired gameplay call sites tracked by
/// this task's "REMAINING" note. `achievement_send_to_client`'s
/// `SV_ACH_UNLOCK` packet (`achievement_unlock_payload` here) is sent
/// unconditionally in C regardless of `show_congrats` and is built by the
/// caller separately.
fn legacy_achievement_unlock_congrats_lines(name: &str, description: &str) -> [Vec<u8>; 2] {
    let mut line1 = Vec::new();
    line1.extend_from_slice(COL_LIGHT_GREEN);
    line1.extend_from_slice(b"Achievement Unlocked: ");
    line1.extend_from_slice(COL_ORANGE);
    line1.extend_from_slice(name.as_bytes());
    line1.extend_from_slice(COL_RESET);
    let line2 = legacy_achievement_colored_line(COL_DARK_GRAY, description);
    [line1, line2]
}

/// C `achievement_list`/`achievement_show_stats`/`achievement_fix_all`/
/// `achievement_clear_all`/`achievement_sync_all` plus the `/achgive`
/// admin-only give command (`achievement.c:1421-1810`, dispatched from
/// `command.c:9076-9227`). `/achievements`/`/achstats` are player-
/// accessible and always operate on the caller (no name argument in C);
/// the remaining four verbs are `CF_GOD`-gated and accept an optional
/// target name defaulting to the caller, mirroring `/reset`'s pattern
/// (`commands_admin.rs`'s `apply_admin_character_command`, `lower ==
/// "reset"` branch).
pub(crate) async fn apply_achievement_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    command: &str,
    now: i64,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if lower.len() >= 6 && "achievements".starts_with(&lower) {
        let player = runtime.player_for_character(character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: legacy_achievement_list_lines(&player.achievement_data),
            ..Default::default()
        });
    }

    if lower.len() >= 8 && "achstats".starts_with(&lower) {
        let player = runtime.player_for_character(character_id)?;
        return Some(KeyringCommandResult {
            message_bytes: legacy_achievement_stats_lines(&player.achievement_stats),
            ..Default::default()
        });
    }

    let is_give = lower.len() >= 7 && "achgive".starts_with(&lower);
    let is_fix = lower.len() >= 6 && "achfix".starts_with(&lower);
    let is_clear = lower.len() >= 8 && "achclear".starts_with(&lower);
    let is_sync = lower.len() >= 7 && "achsync".starts_with(&lower);
    if !is_give && !is_fix && !is_clear && !is_sync {
        return None;
    }
    let caller = world.characters.get(&character_id)?;
    if !caller.flags.contains(CharacterFlags::GOD) {
        return None;
    }

    if is_give {
        let rest = rest.trim_start();
        let (name, id_text) = take_legacy_alpha_name(rest);
        if name.is_empty() {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Usage: /achgive <name> <achievement_id>".to_string(),
                    format!("Achievement IDs: 0-{}", ACHIEVEMENT_TYPE_COUNT - 1),
                ],
                ..Default::default()
            });
        }
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found.")],
                ..Default::default()
            });
        };
        let ach_id = legacy_atoi_prefix(id_text.trim_start());
        if ach_id < 0 || ach_id as usize >= ACHIEVEMENT_TYPE_COUNT {
            return Some(KeyringCommandResult {
                messages: vec![format!(
                    "Invalid achievement ID. Range: 0-{}",
                    ACHIEVEMENT_TYPE_COUNT - 1
                )],
                ..Default::default()
            });
        }
        let ty = AchievementType::ALL[ach_id as usize];
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let newly_unlocked =
            runtime
                .player_for_character_mut(target_id)
                .is_some_and(|target_player| {
                    target_player.achievement_data.award(ty, &target_name, now)
                });
        if newly_unlocked {
            let def = achievement_def(ty);
            let mut payloads = vec![achievement_unlock_payload(ty, now)];
            for line in legacy_achievement_unlock_congrats_lines(def.name, def.description) {
                payloads.push(ugaris_protocol::packet::system_text_bytes(&line));
            }
            send_raw_payloads_to_character(runtime, target_id, &payloads);
            record_achievement_firsts_and_announce(
                world,
                repository,
                target_id,
                &target_name,
                &[ty],
            )
            .await;
        }
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievement {ach_id} awarded to {target_name}.")],
            ..Default::default()
        });
    }

    // achfix / achclear / achsync share the "optional name, defaults to
    // self" target-resolution idiom (`command.c:9135-9227`).
    let name = rest.trim();
    let target_id = if name.is_empty() {
        character_id
    } else {
        match find_online_character_by_name(world, name) {
            Some(id) => id,
            None => {
                return Some(KeyringCommandResult {
                    messages: vec![format!("Player '{name}' not found.")],
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

    if is_fix {
        let Some(target_character) = world.characters.get(&target_id) else {
            return Some(KeyringCommandResult {
                messages: vec![format!("Player '{name}' not found.")],
                ..Default::default()
            });
        };
        let level = target_character.level as i32;
        let is_hardcore = target_character.flags.contains(CharacterFlags::HARDCORE);
        let is_won = target_character.flags.contains(CharacterFlags::WON);
        let professions = target_character.professions.clone();
        let mut unlocked = Vec::new();
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            unlocked.extend(check_level(
                &mut target_player.achievement_data,
                level,
                is_hardcore,
                &target_name,
                now,
            ));
            if is_won
                && target_player.achievement_data.award(
                    AchievementType::Ladykiller,
                    &target_name,
                    now,
                )
            {
                unlocked.push(AchievementType::Ladykiller);
            }
            unlocked.extend(fix_all_stat_thresholds(
                &mut target_player.achievement_data,
                &target_player.achievement_stats,
                &target_name,
                now,
            ));
            for prof_type in 0..=10i32 {
                if let Some(&prof_level) = professions.get(prof_type as usize) {
                    if prof_level > 0 {
                        unlocked.extend(check_profession(
                            &mut target_player.achievement_data,
                            prof_type,
                            prof_level as i32,
                            &target_name,
                            now,
                        ));
                    }
                }
            }
        }
        let payloads: Vec<_> = unlocked
            .iter()
            .map(|&ty| achievement_unlock_payload(ty, now))
            .collect();
        send_raw_payloads_to_character(runtime, target_id, &payloads);
        record_achievement_firsts_and_announce(
            world,
            repository,
            target_id,
            &target_name,
            &unlocked,
        )
        .await;
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievements fixed for {target_name}.")],
            ..Default::default()
        });
    }

    if is_clear {
        if let Some(target_player) = runtime.player_for_character_mut(target_id) {
            clear_all(
                &mut target_player.achievement_data,
                &mut target_player.achievement_stats,
            );
        }
        return Some(KeyringCommandResult {
            messages: vec![format!("Achievements cleared for {target_name}.")],
            ..Default::default()
        });
    }

    // is_sync
    let payloads = runtime
        .player_for_character(target_id)
        .map(|target_player| {
            achievement_sync_payloads(
                &target_player.achievement_data,
                &target_player.achievement_stats,
            )
        })
        .unwrap_or_default();
    send_raw_payloads_to_character(runtime, target_id, &payloads);
    Some(KeyringCommandResult {
        messages: vec![format!("Achievements synced to client for {target_name}.")],
        ..Default::default()
    })
}

/// Sends already-built protocol packets (e.g. `SV_ACH_UNLOCK`/`SV_ACH_SYNC`
/// or a pre-colored `SV_TEXT` congrats line) directly to every live
/// session for `character_id`, bypassing the `command_feedback_bytes`
/// pipeline's `system_text_bytes` re-wrap (which is only correct for raw,
/// not-yet-packetized text) - mirrors the tick loop's own deferred-
/// achievement-sync send pattern (`main.rs`'s `DEFERRED_ACHIEVEMENTS`
/// sweep).
fn send_raw_payloads_to_character(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    payloads: &[bytes::BytesMut],
) {
    for payload in payloads {
        for (session_id, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
}
