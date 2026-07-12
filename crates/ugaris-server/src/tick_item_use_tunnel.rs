//! Completed-action-outcome handling: the Long Tunnels (`src/area/33/
//! tunnel.c`) `IDR_TUNNELDOOR` exit-pillar family
//! (`TunnelDoorExitReward`). Split out of the giant `match outcome { ... }`
//! block per the same P0.5 "Finish main() phase decomposition" precedent
//! as every other `tick_item_use_*` sibling.
//!
//! `TunnelRewardFacts`/`TunnelRewardOutcome` (`ugaris_core::world::tunnel`)
//! carry the `PlayerRuntime` snapshot in and the `PlayerRuntime` writes/
//! feedback lines back out, the same split `area33.rs`'s Gorwin wiring
//! (`GorwinPlayerFacts`/`GorwinOutcomeEvent`) already established for this
//! area.

use super::*;

pub(crate) async fn dispatch_tunnel_outcome(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    area_id: u16,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    let ugaris_core::item_driver::ItemDriverOutcome::TunnelDoorExitReward {
        character_id,
        door_type,
        ..
    } = outcome
    else {
        return;
    };

    // C `if (teleport_char_driver(cn, 250, 250)) { give_reward(...); ppd->
    // clevel = MIN_TUNNEL_LEVEL; }` (`tunnel.c:631-634`): both the reward
    // and the `clevel` reset only happen when the teleport actually moves
    // the player (C's own `teleport_char_driver` returns `0`/no-op when
    // already within 1 tile of the target).
    if !world.teleport_char_driver(character_id, 250, 250) {
        *blocked += 1;
        return;
    }

    let Some(facts) = runtime
        .player_for_character(character_id)
        .map(|player| TunnelRewardFacts {
            reward_level: player.gorwin_tunnel_level(),
            tunnel_used: (0..=MAX_TUNNEL_LEVEL)
                .map(|level| player.tunnel_used(level))
                .collect(),
        })
    else {
        *executed += 1;
        return;
    };

    let result = world.apply_tunnel_reward(character_id, &facts, door_type, u32::from(area_id));

    if let Some(player) = runtime.player_for_character_mut(character_id) {
        if let Some((level, used)) = result.new_used_count {
            player.set_tunnel_used(level, used);
        }
        if let Some(next) = result.promote_gorwin_to {
            player.set_gorwin_tunnel_level(next);
        }
        player.set_tunnel_clevel(MIN_TUNNEL_LEVEL);
    }

    for message in result.messages {
        feedback.push((character_id, message));
    }

    if result.award_achievement {
        award_tunnel_level_achievement(world, runtime, achievement_repository, character_id).await;
    }

    *executed += 1;
}
