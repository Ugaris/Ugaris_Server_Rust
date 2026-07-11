//! Server-side wiring for the Area 32 governor job-board NPC
//! (`CDR_MISSIONGIVE`, "Mister Jones",
//! `ugaris_core::world::npc::area32::governor::process_mission_giver_actions`).
//!
//! Mirrors `area29.rs`'s `apply_countbran_events`/`apply_daughterbran_events`
//! shape: `apply_mission_giver_events` needs `loader` (generic reward-item
//! creation) and `legacy_item_look_text` (reward preview), both
//! `ugaris-server`-only capabilities `ugaris-core`'s `World` cannot reach -
//! see `governor`'s module doc comment for the full ported/remaining slice
//! breakdown.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area32::governor::{
    MissionGiveOutcomeEvent, MissionGivePlayerFacts, MIS_REWARDS,
};

pub(crate) fn mission_giver_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, MissionGivePlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                MissionGivePlayerFacts {
                    ppd: player.governor,
                },
            ))
        })
        .collect()
}

/// Applies each [`MissionGiveOutcomeEvent`] queued by `World::
/// process_mission_giver_actions`. `UpdatePpd` is always applied first
/// within a single event batch (see that function's own doc comment on
/// why event order matters here): `GiveItemReward`'s own point deduction
/// mutates `PlayerRuntime` directly, since it isn't known whether the
/// generic item-template create/give will even succeed until this
/// function runs.
pub(crate) fn apply_mission_giver_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<MissionGiveOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            MissionGiveOutcomeEvent::UpdatePpd { player_id, ppd } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.governor = ppd;
                applied += 1;
            }
            // C `mission_show_reward`'s generic branch (`missions.c:1272-
            // 1287`): `create_item`+`look_item`+`destroy_item`, then the
            // trailing "This could be yours for..." line.
            MissionGiveOutcomeEvent::ShowItemReward {
                player_id,
                npc_id,
                reward_index,
            } => {
                let Some(reward) = MIS_REWARDS.get(reward_index) else {
                    continue;
                };
                let Some(viewer) = world.characters.get(&player_id).cloned() else {
                    continue;
                };
                let Ok(item) = loader.instantiate_item_template(reward.itmtmp, Some(player_id))
                else {
                    world.npc_quiet_say(
                        npc_id,
                        "Oops. I've run out of stock. Please choose something else.",
                    );
                    continue;
                };
                for line in legacy_item_look_text(&item, &viewer).lines() {
                    world.queue_system_text(player_id, line.to_string());
                }
                let points = runtime
                    .player_for_character(player_id)
                    .map(|player| player.governor.points)
                    .unwrap_or(0);
                world.npc_quiet_say(
                    npc_id,
                    &format!(
                        "This could be yours for {} points (you have {points} points). Say ibuy {} to buy it.",
                        reward.value, reward.code
                    ),
                );
                applied += 1;
            }
            // C `mission_give_reward`'s generic branch (`missions.c:1212-
            // 1237`): `create_item`, `IF_BONDTAKE` owner stamping,
            // `give_char_item`, and only on success the point deduction +
            // "here you go" line.
            MissionGiveOutcomeEvent::GiveItemReward {
                player_id,
                npc_id,
                reward_index,
            } => {
                let Some(reward) = MIS_REWARDS.get(reward_index) else {
                    continue;
                };
                let Ok(mut item) = loader.instantiate_item_template(reward.itmtmp, Some(player_id))
                else {
                    world.npc_quiet_say(
                        npc_id,
                        "Oops. I've run out of stock. Please choose something else.",
                    );
                    continue;
                };
                if item.flags.contains(ItemFlags::BONDTAKE) {
                    item.owner_id = player_id.0 as i32;
                }
                let item_id = item.id;
                world.add_item(item);
                if !world.give_char_item(player_id, item_id) {
                    world.destroy_item(item_id);
                    world.npc_quiet_say(
                        npc_id,
                        "Hey, sleepy head, there's no room in your hand or inventory to give you an item!",
                    );
                    continue;
                }
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.governor.points -= reward.value;
                let points_left = player.governor.points;
                let Some(character) = world.characters.get(&player_id) else {
                    continue;
                };
                let player_name = character.name.clone();
                world.npc_quiet_say(
                    npc_id,
                    &format!(
                        "Here you go, {player_name}, one {} ({}) for {} points. You now have {points_left} points left.",
                        reward.code, reward.desc, reward.value
                    ),
                );
                applied += 1;
            }
        }
    }
    applied
}
