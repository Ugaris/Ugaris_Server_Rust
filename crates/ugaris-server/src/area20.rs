//! Area 20 (Live Quest) server-side glue needing `ZoneLoader`/
//! `PlayerRuntime` - see `ugaris_core::world::npc::area20::lqnpc` for the
//! pure per-tick driver this applies events from.

use super::*;
use ugaris_core::world::{make_lq_item_template_id, LqItemSpec, LqNpcOutcomeEvent};

/// C `create_lq_item` (`src/area/20/lq.c:199-217`): instantiate the
/// `"lq_<base>"` template, apply the admin-authored name/description
/// override, and stamp the `MAKE_ITEMID(DEV_ID_LQ, keyID)` identity plus
/// `IF_LABITEM` - the only place in this port that actually creates one of
/// these items (both `spawn_lq_npc_character`'s `carry_item` and
/// `apply_lqnpc_events`'s `GiveRewardItem` call through here).
pub(crate) fn create_lq_item(
    loader: &mut ZoneLoader,
    world: &mut World,
    owner: Option<CharacterId>,
    spec: &LqItemSpec,
) -> Option<ItemId> {
    let template = format!("lq_{}", spec.base);
    let mut item = loader.instantiate_item_template(&template, owner).ok()?;
    if !spec.name.is_empty() {
        item.name = spec.name.clone();
    }
    if !spec.description.is_empty() {
        item.description = spec.description.clone();
    }
    item.template_id = make_lq_item_template_id(spec.key_id);
    item.flags.insert(ItemFlags::LABITEM);
    let item_id = item.id;
    world.items.insert(item_id, item);
    Some(item_id)
}

/// Applies [`LqNpcOutcomeEvent`]s from `World::process_lqnpc_actions`:
/// player quest-mark writes (`PlayerRuntime::set_lq_mark`) and quest-item
/// turn-in rewards (`create_lq_item` + `World::give_char_item`). Returns
/// the number of events applied.
pub(crate) fn apply_lqnpc_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<LqNpcOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            LqNpcOutcomeEvent::SetPlayerMark { player_id, mark_id } => {
                if let Some(player) = runtime.player_for_character_mut(player_id) {
                    player.set_lq_mark(mark_id);
                    applied += 1;
                }
            }
            LqNpcOutcomeEvent::GiveRewardItem { receiver_id, item } => {
                if let Some(item_id) = create_lq_item(loader, world, Some(receiver_id), &item) {
                    if !world.give_char_item(receiver_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}
