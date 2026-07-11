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
use ugaris_core::world::{
    AiEguardSpawnPlan, AiWorkerSpawnPlan, StrategySpawnerSpawnPlan, StrategySpawnerUseOutcome,
};

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

/// C `ai_main`'s "create new workers" character-creation tail
/// (`spawner_sub`'s own `create_char`/`item_drop_char` half,
/// `strategy.c:1259-1279`, reached via `:2648`'s call), fed from
/// [`World::ai_plan_worker_spawn`]'s returned plan - same shape as
/// [`spawn_strategy_worker`] (drop near the AI's own spawner item, apply
/// the `value[1]` warcry/endurance/speed bonuses, `update_char`, restore
/// hp/endurance/mana to max, stamp dir/sprite/group), just without the
/// player-facing "No space..." feedback message (this is AI-side; C's own
/// `ai_main` simply lets the "create new workers" `while` loop `break`
/// with no message at all when `spawner_sub` returns `0`). Returns the
/// freshly spawned worker's `(CharacterId, x, y)` on success - the caller
/// still owes C's own roster-registration tail (`ai_main`'s inline
/// "add new npc to list" loop, `:2654-2668`, already ported as
/// `AiData::register_new_worker`) - or `None` on drop failure (no free
/// adjacent tile), matching that `break`; the `NPCPRICE` Platinum was
/// already deducted by `ai_plan_worker_spawn` and is deliberately NOT
/// refunded here (see that method's own doc comment). Not yet reachable
/// live - `ai_main` itself isn't assembled into one real call site yet
/// (see `crate::world::strategy_ai`'s module doc comment) - so this is
/// exercised directly by tests only, same precedent as several `strategy_
/// ai`/`strategy_ai_tasks` slices before their own live call site landed
/// (exercised directly by `tests::strategy`, hence `#[allow(dead_code)]`
/// - same precedent as `dungeon.rs`/`snapshots.rs`/`depot.rs`/
/// `events.rs`'s pre-wired-but-not-yet-called code).
#[allow(dead_code)]
pub(crate) fn spawn_ai_worker(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    plan: AiWorkerSpawnPlan,
) -> Option<(CharacterId, u16, u16)> {
    let character_id = runtime.allocate_character_id();
    let (mut worker, inventory_items) = zone_loader
        .instantiate_character_template("strategy_npc", character_id)
        .ok()?;

    worker.values[1][CharacterValue::Warcry as usize] =
        worker.values[1][CharacterValue::Warcry as usize].saturating_add(plan.warcry as i16);
    worker.values[1][CharacterValue::Endurance as usize] =
        worker.values[1][CharacterValue::Endurance as usize].saturating_add(plan.endurance as i16);
    worker.values[1][CharacterValue::Speed as usize] =
        worker.values[1][CharacterValue::Speed as usize].saturating_add(plan.speed as i16);

    worker.dir = Direction::RightDown as u8;
    worker.sprite = 353 + plan.npc_color;
    worker.group = plan.group;

    let (x, y) = world.spawn_character_from_item_drop(worker, plan.spawner_id)?;
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
    Some((character_id, x, y))
}

/// C `create_eguard`'s own `create_char`/`drop_char` tail
/// (`strategy.c:2991-3023`), fed from [`World::ai_plan_eguard_spawn`]'s
/// returned plan - drops the fresh worker directly at `(plan.x, plan.y)`
/// (`World::spawn_character`, C `drop_char`) rather than near a spawner
/// item, and additionally stamps the fixed `level`/`WIS`/`INT`/`AGI`/
/// `STR` values `create_eguard` sets directly (unlike a recruited worker,
/// which keeps whatever level its `"strategy_npc"` template already has)
/// via [`World::finish_ai_eguard_spawn`] instead of [`World::
/// finish_strategy_worker_spawn`]. Returns the freshly spawned eternal
/// guard's `CharacterId` on success - the caller still owes C's own
/// roster-registration tail (already ported as `AiData::
/// register_new_eguard`) - or `None` on drop failure (no free tile at
/// `(x, y)`), same "just don't register a live character, don't refund
/// the already-spent cost" precedent as [`spawn_ai_worker`]. Not yet
/// reachable live, same reason as `spawn_ai_worker` - also
/// `#[allow(dead_code)]`'d for the same reason.
#[allow(dead_code)]
pub(crate) fn spawn_ai_eguard(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    plan: AiEguardSpawnPlan,
) -> Option<CharacterId> {
    let character_id = runtime.allocate_character_id();
    let (mut worker, inventory_items) = zone_loader
        .instantiate_character_template("strategy_npc", character_id)
        .ok()?;

    worker.level = plan.level as u32;
    worker.values[1][CharacterValue::Wisdom as usize] = plan.level as i16;
    worker.values[1][CharacterValue::Intelligence as usize] = plan.level as i16;
    worker.values[1][CharacterValue::Agility as usize] = plan.level as i16;
    worker.values[1][CharacterValue::Strength as usize] = plan.level as i16;

    worker.values[1][CharacterValue::Warcry as usize] =
        worker.values[1][CharacterValue::Warcry as usize].saturating_add(plan.warcry as i16);
    worker.values[1][CharacterValue::Endurance as usize] =
        worker.values[1][CharacterValue::Endurance as usize].saturating_add(plan.endurance as i16);
    worker.values[1][CharacterValue::Speed as usize] =
        worker.values[1][CharacterValue::Speed as usize].saturating_add(plan.speed as i16);

    worker.dir = Direction::RightDown as u8;
    worker.sprite = 353 + plan.npc_color;
    worker.group = plan.group;

    if !world.spawn_character(worker, usize::from(plan.x), usize::from(plan.y)) {
        return None;
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

    world.finish_ai_eguard_spawn(character_id, plan.owner_name, plan.x, plan.y);
    Some(character_id)
}
