//! Shared area-22 lab reward machinery (`src/system/lab.c`), used by all
//! five `src/area/22/lab*.c` master-kill death hooks. See `ugaris-core`'s
//! `world::lab` module doc comment for the `World`-side queueing half
//! this drains.

use super::*;

/// C `create_lab_exit(cn, level)` (`src/system/lab.c:137-158`): drops a
/// fresh `"labexit"` gate at the killer's own position (`drop_item_
/// extended(in, ch[cn].x, ch[cn].y, 4)`), tagging it with the killer's
/// character ID (`drdata[0..4]`, C's `*(unsigned int *)it[in].drdata =
/// ch[cn].ID`) and the lab level (`drdata[4]`, C's `it[in].drdata[4] =
/// level`) so `IDR_LABEXIT`'s use-side (`ItemDriverOutcome::LabExitUse`,
/// `tick_item_use_lab.rs`) can later verify ownership and call
/// `set_solved_lab`. C's own failure path (`destroy_item(in)` plus an
/// `xlog` warning) is reproduced as a `tracing::warn!` - there is no
/// player-facing message either way in C.
pub(crate) fn create_lab_exit(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    killer_id: CharacterId,
    level: u8,
) -> bool {
    let Some((x, y)) = world
        .characters
        .get(&killer_id)
        .map(|character| (usize::from(character.x), usize::from(character.y)))
    else {
        return false;
    };
    let Ok(mut item) = zone_loader.instantiate_item_template("labexit", None) else {
        return false;
    };
    if !world.map.drop_item_extended(&mut item, x, y, 4) {
        tracing::warn!(
            killer_id = killer_id.0,
            level,
            x,
            y,
            "could not drop lab exit gate"
        );
        return false;
    }
    ensure_drdata_len(&mut item, 5);
    item.driver_data[0..4].copy_from_slice(&killer_id.0.to_le_bytes());
    item.driver_data[4] = level;
    world.add_item(item);
    true
}

/// Drains every `create_lab_exit` request queued this tick by an area-22
/// lab master's own death hook (`World::queue_lab_exit_spawn`) - deferred
/// here since `World` alone can't reach `ZoneLoader`.
pub(crate) fn apply_pending_lab_exit_spawns(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
) -> usize {
    let requests = world.drain_pending_lab_exit_spawns();
    let mut applied = 0;
    for request in requests {
        if create_lab_exit(world, zone_loader, request.killer_id, request.level) {
            applied += 1;
        }
    }
    applied
}
