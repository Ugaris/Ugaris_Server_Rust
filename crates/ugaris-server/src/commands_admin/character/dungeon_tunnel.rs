use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_dungeon_tunnel(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
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
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (target_id, target_name, rd_number) =
            parse_exp_command_target(world, character_id, rest);
        if !world.characters.contains_key(&target_id) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {target_name} around.")],
                ..Default::default()
            }));
        }
        if !(10..=99).contains(&rd_number) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["RD number must be between 10 and 99.".to_string()],
                ..Default::default()
            }));
        }
        let rd_number = rd_number as u32;

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            }));
        };

        let message = match lower {
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

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![message],
            ..Default::default()
        }));
    }

    // C `/changetunnel` (`command.c:2045-2085`, `CF_GOD`-gated): sets an
    // online target's `tunnel_ppd::clevel` directly, no self-fallback -
    // an empty/unmatched name always reports "no one by the name".
    if lower == "changetunnel" || lower == "settunnel" || lower == "cleartunnel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, remainder) = take_legacy_alpha_name(rest);
        let mut tokens = remainder.split_whitespace();
        let level = tokens.next().map(legacy_atoi_prefix).unwrap_or(0) as i32;
        let amount = tokens.next().map(legacy_atoi_prefix).unwrap_or(0) as i32;

        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };

        if !(MIN_TUNNEL_LEVEL..=MAX_TUNNEL_LEVEL).contains(&level) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Invalid tunnel level. Must be between {MIN_TUNNEL_LEVEL} and {MAX_TUNNEL_LEVEL}."
                )],
                ..Default::default()
            }));
        }

        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Failed to get player data.".to_string()],
                ..Default::default()
            }));
        };

        let (caller_message, target_message) = match lower {
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
        return ControlFlow::Break(Some(result));
    }

    // C `/solvetunnel` (`command.c:2199-2222`, `CF_GOD`-gated, self
    // only): C's own reward call (`give_reward(cn, ppd, door_type)`) is
    // commented out in the oracle itself, so this is a message-only
    // no-op there too - nothing to mutate here either.
    if lower == "solvetunnel" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let exptype = legacy_atoi_prefix(rest.trim_start());
        if exptype != 0 && exptype != 1 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid exp type. Must be 0 (exp) or 1 (military exp).".to_string()],
                ..Default::default()
            }));
        }

        let reward_name = if exptype == 0 {
            "experience"
        } else {
            "military experience"
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Solved current tunnel and granted {reward_name} reward."
            )],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}
