//! Admin-scheduled server shutdown.
//!
//! C `/shutdown` (`system/command.c:6068-6086`, `cmdcmp(ptr, "shutdown",
//! 8)` - `CF_GOD`-gated, and note the C `minlen` equals the full word
//! length, so unlike most admin commands here no abbreviation is accepted)
//! calls `start_shutdown(diff, down)` (`command.c:541-557`), which computes
//! the absolute target time and normally broadcasts it as a
//! `server_chat(1033, ...)` control message that every connected area
//! server relays through `shutdown_bg` (`system/tool.c:3152-3164`). Since
//! this is a standalone single-area server (see `AGENTS.md`), the relay hop
//! is skipped and the scheduling mutation is applied directly - the same
//! simplification every other `server_chat`-relayed admin command already
//! ported here uses (`/setweather`/`clearweather`/`setareaweather` in
//! `weather.rs`, `global` in `commands_admin.rs`, etc).
//!
//! `shutdown_warn` (`system/tool.c:3120-3149`) is the periodic countdown
//! broadcast + actual-exit trigger, normally called every 20s from C's
//! `monitor_20s_task` (`server.c:216-222`) plus once immediately by
//! `shutdown_bg` whenever a new shutdown is scheduled or cancelled. Ported
//! as [`tick_shutdown_scheduler`], called both directly from
//! [`apply_shutdown_command`] (for the immediate broadcast) and from the
//! main tick loop at the same ~20s cadence.

use super::*;

/// C `start_shutdown`/`shutdown_bg` (`command.c:541-557`, `system/tool.c:
/// 3152-3164`), folded into one direct call (see the module doc comment for
/// why the `server_chat` relay hop is skipped). `diff_minutes`/
/// `down_minutes` are the raw `atoi()`-parsed command arguments, including
/// C's own quirk that a negative `diff` leaves `down` parsed from the exact
/// same substring (C's `while (isdigit(*ptr)) ptr++;` does not step over a
/// leading `-` sign) - callers should reproduce that quirk in argument
/// parsing rather than pre-validating it away.
pub(crate) fn apply_shutdown_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    diff_minutes: i64,
    down_minutes: i64,
) {
    if diff_minutes != 0 {
        // C `start_shutdown`: `if (!down) down = 15;`
        let down = if down_minutes == 0 { 15 } else { down_minutes };
        let now = i64::from(current_realtime_seconds());
        runtime.shutdown_at = now + diff_minutes * 60;
        runtime.shutdown_down_minutes = down;
        // C `shutdown_bg`: `shutdown_last = 999;` sentinel so the
        // immediate `shutdown_warn` call below always broadcasts.
        runtime.shutdown_warned_minutes = 999;
        tick_shutdown_scheduler(world, runtime);
    } else {
        cancel_shutdown(world, runtime);
    }
}

/// C `shutdown_bg`'s cancel branch (`t == 0`, `system/tool.c:3158-3164`):
/// only actually cancels (and only then broadcasts) if a shutdown was
/// pending or logins were already blocked - a bare `/shutdown` with no
/// scheduled shutdown is a silent no-op in C.
fn cancel_shutdown(world: &mut World, runtime: &mut ServerRuntime) {
    if runtime.shutdown_at != 0 || runtime.nologin {
        runtime.shutdown_at = 0;
        runtime.nologin = false;
        broadcast_to_online_players(world, runtime, "Shutdown has been cancelled.");
    }
}

/// C `shutdown_warn` (`system/tool.c:3120-3149`). Returns `true` once the
/// scheduled time has arrived (C sets the global `quit = 1` there; the
/// caller - the main tick loop - breaks out of the run loop instead).
pub(crate) fn tick_shutdown_scheduler(world: &mut World, runtime: &mut ServerRuntime) -> bool {
    if runtime.shutdown_at == 0 {
        return false;
    }
    let now = i64::from(current_realtime_seconds());
    let min = (runtime.shutdown_at - now + 50) / 60;
    if min != runtime.shutdown_warned_minutes {
        let message = if min > 0 {
            format!(
                "The server will go down in {min} minute{}. Expected downtime: {} minutes.",
                if min > 1 { "s" } else { "" },
                runtime.shutdown_down_minutes
            )
        } else {
            format!(
                "The server will go down NOW. Expected downtime: {} minutes.",
                runtime.shutdown_down_minutes
            )
        };
        broadcast_to_online_players(world, runtime, &message);
        runtime.shutdown_warned_minutes = min;
        if min < 3 {
            runtime.nologin = true;
        }
    }
    now >= runtime.shutdown_at
}

/// C's `for (n = 1; n < MAXCHARS; n++) if (ch[n].flags & CF_PLAYER)
/// log_char(n, ...)` loops in `shutdown_warn`/`shutdown_bg`: every
/// currently-connected player (C's in-memory `ch[]` only holds online
/// characters, so `CF_PLAYER` there already means "online"), light-red
/// colored (`COL_LIGHT_RED`) like the exact C text.
fn broadcast_to_online_players(world: &mut World, runtime: &ServerRuntime, message: &str) {
    let bytes = legacy_light_red_text_bytes(message);
    let targets: Vec<CharacterId> = runtime
        .players
        .values()
        .filter_map(|player| player.character_id)
        .collect();
    for character_id in targets {
        world.queue_system_text_bytes(character_id, bytes.clone());
    }
}
