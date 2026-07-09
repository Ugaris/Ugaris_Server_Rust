//! Per-tick sync phase: the part of the legacy tick loop that runs once per
//! tick after the NPC/queued-event passes (`tick_npc::run_all`) and before
//! the `events_rx` branch - PK hate updates from hurt events, the ~20s
//! shutdown-scheduler check, pending world/area/system text and channel
//! broadcast drains, resource/value sync frames, periodic map/action diff
//! frames, and the final per-tick frame flush. Extracted verbatim from
//! `main()`'s `tick.tick()` arm (P0.5 "Finish main() phase decomposition")
//! with a superset-params signature, preserving exact execution order;
//! `main.rs` must not grow when this phase changes. Returns whether the
//! scheduled shutdown time was reached this tick, so the caller can break
//! its loop exactly like the inline code did.
use super::*;

pub(crate) fn sync_phase(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
) -> bool {
    let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
    let pk_hate_updates =
        apply_pk_hate_from_hurt_events(runtime, world, realtime_seconds, &*zone_loader);
    if pk_hate_updates != 0 {
        info!(
            pk_hate_updates,
            tick = world.tick.0,
            "applied PK hate updates from hurt events"
        );
    }

    // C's five `src/area/22/lab*.c` master-kill death hooks all call
    // `create_lab_exit` directly; this port defers the actual
    // `ZoneLoader`-backed item creation to here - see `world::lab`'s
    // module doc comment.
    let lab_exit_spawns = lab::apply_pending_lab_exit_spawns(world, zone_loader);
    if lab_exit_spawns != 0 {
        info!(
            lab_exit_spawns,
            tick = world.tick.0,
            "spawned lab exit reward gates"
        );
    }

    // C `monitor_20s_task`'s `shutdown_warn()` call
    // (`server.c:216-222`), same ~20s cadence.
    let shutdown_due = if world.tick.0 % (TICKS_PER_SECOND * 20) == 0 {
        tick_shutdown_scheduler(world, runtime)
    } else {
        false
    };

    let area_text_sessions = send_pending_world_area_texts(runtime, world);
    if area_text_sessions != 0 {
        info!(
            area_text_sessions,
            tick = world.tick.0,
            "queued world area text feedback"
        );
    }

    let area_text_bytes_sessions = send_pending_world_area_text_bytes(runtime, world);
    if area_text_bytes_sessions != 0 {
        info!(
            area_text_bytes_sessions,
            tick = world.tick.0,
            "queued world area text byte feedback"
        );
    }

    let world_text_sessions = send_pending_world_system_texts(runtime, world);
    if world_text_sessions != 0 {
        info!(
            world_text_sessions,
            tick = world.tick.0,
            "queued world system text feedback"
        );
    }

    let world_text_bytes_sessions = send_pending_world_system_text_bytes(runtime, world);
    if world_text_bytes_sessions != 0 {
        info!(
            world_text_bytes_sessions,
            tick = world.tick.0,
            "queued world system text byte feedback"
        );
    }

    let world_player_special_sessions = send_pending_world_player_specials(runtime, world);
    if world_player_special_sessions != 0 {
        info!(
            world_player_special_sessions,
            tick = world.tick.0,
            "queued world player special feedback"
        );
    }

    let channel_broadcast_sessions = send_pending_world_channel_broadcasts(runtime, world);
    if channel_broadcast_sessions != 0 {
        info!(
            channel_broadcast_sessions,
            tick = world.tick.0,
            "queued world channel broadcast feedback"
        );
    }

    let resource_sync_sessions = queue_resource_sync_frames(runtime, world);
    if resource_sync_sessions != 0 {
        info!(
            resource_sync_sessions,
            tick = world.tick.0,
            "queued resource/value sync frames"
        );
    }

    let (periodic_diff_sessions, periodic_empty_frames) =
        queue_periodic_player_frames(runtime, world);
    if periodic_diff_sessions != 0 {
        info!(
            periodic_diff_sessions,
            tick = world.tick.0,
            "queued periodic map/action diffs"
        );
    }
    if periodic_empty_frames != 0 {
        tracing::trace!(
            periodic_empty_frames,
            tick = world.tick.0,
            "queued empty legacy tick frames"
        );
    }
    // Exactly one legacy tick frame per session per tick: the
    // lockstep client advances its clock per received frame.
    runtime.flush_tick_frames(true);

    // C `while (!quit)` (`server.c:612`): `shutdown_warn`
    // setting the global `quit = 1` once the scheduled time
    // arrives (`system/tool.c:3148`) ends the C main loop the
    // same way `ctrl_c` does below.
    if shutdown_due {
        info!("scheduled shutdown time reached");
    }
    shutdown_due
}
