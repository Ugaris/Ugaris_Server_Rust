//! Completed-action-outcome handling: the `IDR_STR_MINE`/`IDR_STR_STORAGE`/
//! `IDR_STR_DEPOT` player-"look" family of `ItemDriverOutcome` variants
//! (`src/area/23_24/strategy.c`'s `mine`/`storage`/`depot`, `CF_PLAYER`
//! branch only - see `ugaris_core::item_driver::area23_24`'s module doc
//! comment for the two documented gaps every one of these three still
//! has), plus `IDR_STR_SPAWNER`'s player-facing worker-recruit request
//! (`StrSpawnerUse`, C `spawner`/`spawner_sub`, `strategy.c:1244-1381`) -
//! the one variant in this family that needs `ZoneLoader`/`ServerRuntime`
//! to actually build the fresh worker character, same "needs more than
//! `World`" precedent as `tick_item_use_minewall`'s `MineWallDig`. Split
//! out of the giant `match outcome { ... }` block that still lives inline
//! in `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition"), same precedent as every other `tick_item_use_*`
//! family dispatcher.

use super::*;
use ugaris_core::world::{StrategySpawnerSpawnPlan, StrategySpawnerUseOutcome};

pub(crate) fn dispatch_strategy_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::StrMineLook {
            character_id,
            platinum,
            ..
        } => {
            feedback.push((
                character_id,
                format!("There are {platinum} units of Platinum left."),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrDepotLook {
            character_id,
            platinum,
            ..
        } => {
            feedback.push((
                character_id,
                format!("This depot contains {platinum} units of Platinum."),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrStorageInteract {
            character_id,
            conversion,
            platinum,
            ..
        } => {
            match conversion {
                ugaris_core::item_driver::StrStorageConversion::Converted { added, .. } => {
                    feedback.push((
                        character_id,
                        format!("Converted to {added} units of Platinum and added to storage."),
                    ));
                }
                ugaris_core::item_driver::StrStorageConversion::WrongKind => {
                    feedback.push((
                        character_id,
                        "You can only add mined gold or silver. The exchange rate is 5 to 1 \
                         for gold and 50 to 1 for silver."
                            .to_string(),
                    ));
                }
                ugaris_core::item_driver::StrStorageConversion::None => {}
            }
            feedback.push((
                character_id,
                format!("This storage contains {platinum} units of Platinum."),
            ));
            *executed += 1;
        }
        // Worker-only outcomes (`StrMineWorkerDig`/`StrBuildingWorkerTransfer`/
        // `StrDepotWorkerTakeover`): the state mutation already happened in
        // `World::apply_item_driver_outcome`, and there is no player to send
        // feedback text to (no `CDR_STRATEGY` worker can be spawned yet - see
        // `world::npc::area23_24::worker`'s module doc comment) - just count
        // the completion.
        ugaris_core::item_driver::ItemDriverOutcome::StrMineWorkerDig { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StrBuildingWorkerTransfer { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StrDepotWorkerTakeover { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrSpawnerUse {
            item_id,
            character_id,
        } => {
            let Some(player) = runtime.player_for_character_mut(character_id) else {
                return;
            };
            let dispatch =
                world.try_dispatch_strategy_spawner_use(character_id, item_id, &player.strategy);
            if let StrategySpawnerUseOutcome::Ready(plan) = dispatch {
                spawn_strategy_worker(world, zone_loader, runtime, character_id, plan);
            }
            *executed += 1;
        }
        _ => {}
    }
}

/// C `spawner_sub`'s tail (`strategy.c:1259-1286`): builds the fresh
/// `"strategy_npc"` worker, drops it near the spawner
/// (`World::spawn_character_from_item_drop`, C `item_drop_char`), applies
/// the four player-upgrade `value[1]` bonuses before `update_char`
/// (`World::update_character`) recomputes `value[0]`, restores hp/
/// endurance/mana to the new max, then stamps dir/sprite/group and hands
/// off to `World::finish_strategy_worker_spawn` for the driver-state
/// stamp. C's `ch[co].tmpx`/`tmpy` scratch fields (never read anywhere in
/// `strategy.c` itself) are not modeled - same class of omission as every
/// other unread scratch field in this codebase.
///
/// On drop failure (no free adjacent tile), queues C's own "No space to
/// drop char or max worker reached." message - the `NPCPRICE` Platinum
/// was already spent by `World::try_dispatch_strategy_spawner_use` and is
/// deliberately NOT refunded here, matching the real C quirk that
/// method's own doc comment documents.
fn spawn_strategy_worker(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    requester_id: CharacterId,
    plan: StrategySpawnerSpawnPlan,
) {
    const NO_SPACE_MESSAGE: &str = "No space to drop char or max worker reached.";

    let character_id = runtime.allocate_character_id();
    let Ok((mut worker, inventory_items)) =
        zone_loader.instantiate_character_template("strategy_npc", character_id)
    else {
        world.queue_system_text(requester_id, NO_SPACE_MESSAGE.to_string());
        return;
    };

    worker.values[1][CharacterValue::Warcry as usize] =
        worker.values[1][CharacterValue::Warcry as usize].saturating_add(plan.warcry as i16);
    worker.values[1][CharacterValue::Endurance as usize] =
        worker.values[1][CharacterValue::Endurance as usize].saturating_add(plan.endurance as i16);
    worker.values[1][CharacterValue::Speed as usize] =
        worker.values[1][CharacterValue::Speed as usize].saturating_add(plan.speed as i16);

    worker.dir = Direction::RightDown as u8;
    worker.sprite = 353 + plan.npc_color;
    worker.group = plan.group;

    if world
        .spawn_character_from_item_drop(worker, plan.spawner_id)
        .is_none()
    {
        world.queue_system_text(requester_id, NO_SPACE_MESSAGE.to_string());
        return;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    world.update_character(character_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    }

    world.finish_strategy_worker_spawn(
        character_id,
        plan.owner_name,
        plan.trainspeed,
        plan.max_level,
    );
}
