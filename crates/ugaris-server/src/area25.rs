//! Server-side wiring for area 25's Warpmaster NPC (`CDR_WARPMASTER`,
//! `ugaris_core::world::npc::area25::warpmaster::process_warpmaster_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area19.rs`: [`apply_warpmaster_events`] applies the returned
//! [`WarpmasterOutcomeEvent`]s, some of which need `PlayerRuntime` (the
//! `warped_ppd` reset) or `ZoneLoader` (creating `warped_door_key` items),
//! neither of which `World` can see.

use super::*;
use ugaris_core::world::npc::area25::WarpmasterOutcomeEvent;

/// Applies each [`WarpmasterOutcomeEvent`] queued by `World::
/// process_warpmaster_actions`.
pub(crate) fn apply_warpmaster_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    events: Vec<WarpmasterOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            // C `warpmaster`'s `NT_TEXT` "reset" branch (`warped.c:1045-
            // 1052`): `ppd->points = 0; for (n..MAXWARPBONUS)
            // ppd->bonuslast_used[n] = 0; ppd->nostepexp = 1;`.
            WarpmasterOutcomeEvent::ResetWarpPpd { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.warp_points = 0;
                player.warp_bonus_last_used.fill(0);
                player.warp_nostepexp = 1;
                applied += 1;
            }
            // C `warpmaster`'s `NT_GIVE` alchemy-stone trade
            // (`warped.c:1061-1133`): create `count` `warped_door_key`
            // items and give them to `player_id`, preserving C's `flag ||
            // !give_char_item(co, in)` fallback - the ingredient item is
            // destroyed if at least one key was given, else handed back
            // to the giver (destroyed only if that also fails).
            WarpmasterOutcomeEvent::GiveKeys {
                warpmaster_id: _,
                player_id,
                ingredient_item_id,
                count,
            } => {
                let mut gave_any = false;
                for _ in 0..count {
                    let Ok(item) = zone_loader.instantiate_item_template("warped_door_key", None)
                    else {
                        continue;
                    };
                    let item_id = item.id;
                    world.add_item(item);
                    if world.give_char_item(player_id, item_id) {
                        gave_any = true;
                    } else {
                        world.destroy_item(item_id);
                    }
                }
                // Identical branches on purpose: mirrors C's `flag ||
                // !give_char_item(co, in)` fallback, where the else arm's
                // give attempt is the observable difference.
                #[allow(clippy::if_same_then_else)]
                if gave_any {
                    world.destroy_item(ingredient_item_id);
                } else if !world.give_char_item(player_id, ingredient_item_id) {
                    world.destroy_item(ingredient_item_id);
                }
                applied += 1;
            }
        }
    }
    applied
}
