//! Drains for `World`-queued events into client sessions and DB tasks.
//!
//! `mod.rs` keeps the generic text/hurt-event fan-out; NPC death hooks and
//! per-system event appliers live in submodules. Anti-cheat report
//! formatting is in `anticheat_report`, admin command DB round-trips in
//! `admin_tasks`.

mod admin_events;
mod anticheat_report;
mod death_hooks;
mod events_misc;
mod npc_events;

pub(crate) use admin_events::*;
pub(crate) use anticheat_report::*;
pub(crate) use death_hooks::*;
pub(crate) use events_misc::*;
pub(crate) use npc_events::*;

use super::*;

pub(crate) fn remove_stale_pvp_hate_if_effect_check_fails(
    player: &mut PlayerRuntime,
    attacker: &Character,
    target: &Character,
    area_id: u16,
) {
    if area_id == 1 {
        return;
    }
    if !attacker.flags.contains(CharacterFlags::PLAYER)
        || !target.flags.contains(CharacterFlags::PLAYER)
        || !attacker.flags.contains(CharacterFlags::PK)
    {
        return;
    }
    if attacker.id == target.id
        || !target.flags.contains(CharacterFlags::PK)
        || attacker.level.abs_diff(target.level) > 3
    {
        player.remove_pk_hate(target.id.0);
    }
}

pub(crate) fn apply_pk_hate_from_hurt_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    realtime_seconds: u64,
    loader: &ZoneLoader,
) -> usize {
    let mut applied = 0;
    let events = world.drain_legacy_hurt_events();
    for event in &events {
        apply_player_fightback_from_hurt_event(runtime, world, *event, world.tick.0);
    }
    for event in events {
        apply_swamp_monster_death_from_hurt_event(runtime, world, event);
        apply_teufel_rat_death_from_hurt_event(runtime, world, event);
        apply_caligar_skelly_death_from_hurt_event(runtime, world, event);
        apply_lab2_undead_death_from_hurt_event(runtime, world, event);
        apply_gate_fight_death_from_hurt_event(runtime, world, event, loader);
        apply_gate_welcome_death_from_hurt_event(world, event);
        apply_dungeonmaster_death_from_hurt_event(world, event);
        apply_gorwin_death_from_hurt_event(world, event);
        apply_area1_quest_giver_death_from_hurt_event(world, event);
        apply_area3_immortal_death_from_hurt_event(world, event);
        apply_area30_clan_npc_death_from_hurt_event(world, event);
        apply_lampghost_death_from_hurt_event(world, event);
        apply_labgnome_death_from_hurt_event(world, event);
        apply_area1_monster_death_from_hurt_event(runtime, world, event);
        apply_forest_monster_death_from_hurt_event(runtime, world, event);
        apply_bredel_death_from_hurt_event(runtime, world, event);
        apply_riverbeast_death_from_hurt_event(runtime, world, event);
        apply_bigbadspider_death_from_hurt_event(runtime, world, event);
        apply_asturin_death_from_hurt_event(runtime, world, event);
        apply_shr_werewolf_death_from_hurt_event(runtime, world, event);
        apply_two_guard_death_from_hurt_event(runtime, world, event);
        apply_two_robber_death_from_hurt_event(runtime, world, event);
        apply_smugglelead_death_from_hurt_event(runtime, world, event);
        apply_robberboss_death_from_hurt_event(runtime, world, event);
        apply_centinel_death_from_hurt_event(runtime, world, event);
        apply_two_servant_death_from_hurt_event(runtime, world, event);
        apply_vampire_death_from_hurt_event(runtime, world, event);
        apply_vampire2_death_from_hurt_event(runtime, world, event);
        apply_fdemon_demon_death_from_hurt_event(runtime, world, event);
        apply_lqnpc_death_from_hurt_event(runtime, world, event);
        apply_warpfighter_death_from_hurt_event(world, event);
        apply_mission_fighter_death_from_hurt_event(runtime, world, event);
        apply_arkhata_prisoner_death_from_hurt_event(world, event);
        apply_arkhata_bookeater_death_from_hurt_event(runtime, world, event);
        apply_arkhataskelly_death_from_hurt_event(runtime, world, event);
        apply_gladiator_death_from_hurt_event(world, event);
        apply_arkhata_immortal_death_from_hurt_event(world, event);

        let eligible = match (
            world.characters.get(&event.target_id),
            world.characters.get(&event.cause_id),
        ) {
            (Some(target), Some(cause)) => {
                target.id != cause.id
                    && target
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && cause
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && target.level.abs_diff(cause.level) <= 3
            }
            _ => false,
        };
        if !eligible {
            continue;
        }
        let Some(player) = runtime.player_for_character_mut(event.target_id) else {
            continue;
        };
        let Some(target) = world.characters.get_mut(&event.target_id) else {
            continue;
        };
        player.add_pk_hate_from_hit(target, event.cause_id.0);
        applied += 1;

        if event.outcome.killed {
            if let Some(player) = runtime.player_for_character_mut(event.target_id) {
                player.add_pk_death(realtime_seconds);
            }
            if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
                player.add_pk_kill(realtime_seconds);
            }
        }
    }
    applied
}

pub(crate) fn apply_player_fightback_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &World,
    event: LegacyHurtEvent,
    current_tick: u64,
) -> bool {
    let Some((attacker_serial, legacy_distance)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, attacker)| {
            target
                .flags
                .contains(CharacterFlags::PLAYER)
                .then_some((attacker.serial, char_dist(target, attacker)))
        })
    else {
        return false;
    };
    runtime
        .player_for_character_mut(event.target_id)
        .is_some_and(|player| {
            player.apply_got_hit_fightback(
                event.cause_id,
                attacker_serial,
                legacy_distance,
                current_tick,
            )
        })
}

pub(crate) fn send_pending_world_system_texts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_system_texts() {
        let payload = ugaris_protocol::packet::system_text(&event.message);
        for (session_id, _) in runtime.sessions_for_character(event.character_id) {
            if runtime.send_to_session(session_id, payload.clone()) {
                sent += 1;
            }
        }
    }
    sent
}

/// Single-character `SV_SPECIAL` sibling of
/// [`send_pending_world_system_texts`] - see `WorldPlayerSpecial`.
pub(crate) fn send_pending_world_player_specials(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_player_specials() {
        let payload = bytes::BytesMut::from(
            &ugaris_protocol::packet::special(event.special_type, event.opt1, event.opt2)[..],
        );
        for (session_id, _) in runtime.sessions_for_character(event.character_id) {
            if runtime.send_to_session(session_id, payload.clone()) {
                sent += 1;
            }
        }
    }
    sent
}

/// Byte-payload sibling of [`send_pending_world_system_texts`] - see
/// `WorldSystemTextBytes`.
pub(crate) fn send_pending_world_system_text_bytes(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_system_text_bytes() {
        let payload = ugaris_protocol::packet::system_text_bytes(&event.message);
        for (session_id, _) in runtime.sessions_for_character(event.character_id) {
            if runtime.send_to_session(session_id, payload.clone()) {
                sent += 1;
            }
        }
    }
    sent
}

pub(crate) fn send_pending_world_area_texts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_area_texts() {
        let payload = ugaris_protocol::packet::system_text(&event.message);
        let max_distance = i32::from(event.max_distance);
        let recipients: Vec<_> = world
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                ((i32::from(character.x) - i32::from(event.x)).abs() <= max_distance
                    && (i32::from(character.y) - i32::from(event.y)).abs() <= max_distance)
                    .then_some(character_id)
            })
            .collect();
        for character_id in recipients {
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    sent += 1;
                }
            }
        }
    }
    sent
}

/// Byte-payload sibling of [`send_pending_world_area_texts`] - see
/// `WorldAreaTextBytes`.
pub(crate) fn send_pending_world_area_text_bytes(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_area_text_bytes() {
        let payload = ugaris_protocol::packet::system_text_bytes(&event.message);
        let max_distance = i32::from(event.max_distance);
        let recipients: Vec<_> = world
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                ((i32::from(character.x) - i32::from(event.x)).abs() <= max_distance
                    && (i32::from(character.y) - i32::from(event.y)).abs() <= max_distance)
                    .then_some(character_id)
            })
            .collect();
        for character_id in recipients {
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    sent += 1;
                }
            }
        }
    }
    sent
}

/// C `server_chat(channel, text)` (`src/system/chat/chat.c:827-834`),
/// consumer half: drains `World::drain_pending_channel_broadcasts` and fans
/// each message out to every connected player who has joined that channel,
/// matching the channel-bit delivery rule `apply_chat_command`
/// (`commands_chat.rs`) uses for player-authored channel messages (no
/// clan/mirror/area/ignore filters apply to channel 6 "Grats", so a plain
/// join-bit check is sufficient here).
pub(crate) fn send_pending_world_channel_broadcasts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_channel_broadcasts() {
        let payload = ugaris_protocol::packet::system_text_bytes(&event.message_bytes);
        let bit = 1_u32 << (event.channel.saturating_sub(1));
        let recipients: Vec<CharacterId> = runtime
            .players
            .values()
            .filter(|player| player.chat_channels & bit != 0)
            .filter_map(|player| player.character_id)
            .collect();
        for character_id in recipients {
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    sent += 1;
                }
            }
        }
    }
    sent
}

pub(crate) fn pk_hate_prerequisites(source: &Character, target: &Character) -> bool {
    source.id != target.id
        && source
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && target
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && source.level.abs_diff(target.level) <= 3
}
