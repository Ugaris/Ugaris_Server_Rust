use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_diagnostics(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    if lower.len() >= 4 && "prof".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec!["--- Profile ---".to_string(), "---------------".to_string()],
            ..Default::default()
        }));
    }

    // C `/profinfo` (`command.c:7496-7500`, `cmdcmp(ptr, "profinfo", 5)`,
    // `CF_GOD`-gated). Distinct from `/prof`/`cmd_show_prof` above: C's
    // `profinfo` sends one header line to the player and then calls
    // `show_prof()` (`server.c:934-986`), which is entirely `xlog()`
    // console-only output - the caller never receives the actual
    // cycle-profiler dump. A faithful port is therefore just the header
    // line; there is also no Rust equivalent of the underlying `proftab`
    // rdtsc-cycle profiler to port even if C's player-facing behavior
    // were different.
    if lower.len() >= 5 && "profinfo".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec!["Profiling Information:".to_string()],
            ..Default::default()
        }));
    }

    // C `/poolstats` (`command.c:7503-7506`, `cmdcmp(ptr, "poolstats", 5)`,
    // `CF_GOD`-gated). Same pattern as `/profinfo`: C sends one header
    // line to the player, then `log_connection_pool_state()`
    // (`database_connection_pool.c:23-37`) writes the actual pool
    // occupancy/request-counter data to the console via `xlog()` only -
    // the caller never sees it. C's connection pool is also a hand-rolled
    // fixed-size MySQL connection array with its own counters, not
    // analogous to sqlx's `PgPool` internals, so even a "richer than C"
    // version would need new instrumentation. Faithful port: header line
    // only.
    if lower.len() >= 5 && "poolstats".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec!["Connection Pool Statistics:".to_string()],
            ..Default::default()
        }));
    }

    // C `/memstats` (`command.c:7476-7493`, `cmdcmp(ptr, "memstats", 5)`,
    // `CF_GOD`-gated). Unlike `/profinfo`/`/poolstats` above, C's
    // `memstats` sends every data line to the player via `log_char`, so a
    // faithful port needs real numbers, not just the header. C reports
    // live occupancy against fixed-capacity C arrays (`used_chars` of
    // `MAXCHARS`, `used_items` of `MAXITEM`, `used_effects` of
    // `MAXEFFECT`, `used_containers` of `MAXCONTAINER` - the first three
    // are runtime-configurable globals in the C oracle, not even C
    // compile-time constants), plus a heap-allocation byte counter
    // (`mem_usage`) and a pending-notify-message counter (`used_msgs`).
    // Rust's `World` has no fixed-capacity arrays at all (its character/
    // item/effect stores are unbounded `HashMap`s - see `world/mod.rs`),
    // so there is no "/MAX" denominator to report; the three occupancy
    // counts are reported here as plain live counts instead. `mem_usage`
    // and `used_msgs` have no Rust analogue whatsoever (no allocation-
    // tracking, no persistent notify-queue-depth concept - pending
    // notifications are drained to packets every tick, not held in a
    // countable queue), so both are reported as a fixed `0`, matching the
    // established "no real Rust equivalent -> always report the harmless
    // constant" convention (e.g. `#accleanup`'s always-`0` heartbeat-log
    // count, `world_events.rs`). `used_containers` has no dedicated Rust
    // collection either - `world/consistency.rs`'s doc comment: "is this
    // item a container" is derived from `Item.content_id != 0`, not a
    // separate store - so it is computed here the same way.
    if lower.len() >= 5 && "memstats".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let containers = world
            .items
            .values()
            .filter(|item| item.content_id != 0)
            .count();
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Memory Usage Statistics:".to_string(),
                "Total memory usage: 0 KB".to_string(),
                format!("Characters: {} used", world.characters.len()),
                format!("Items: {} used", world.items.len()),
                format!("Effects: {} used", world.effects.len()),
                format!("Containers: {containers} used"),
                "Messages: 0 used".to_string(),
            ],
            ..Default::default()
        }));
    }

    // C `/querystats` (`command.c:6588-6618`, `cmdcmp(ptr, "querystats",
    // 5)`, `CF_GOD`-gated). Unlike `/profinfo`/`/poolstats`/`/memstats`
    // above, this reply needs a live `PgCharacterRepository` read, which
    // this dispatcher has no access to - see `ugaris-core`'s
    // `world/querystats.rs` module doc comment for the full scoping
    // rationale (only `save_char_cnt`/`exit_char_cnt`/`load_char_cnt` are
    // tracked; every other C counter this command reads has no Rust
    // instrumentation) - so this just queues the lookup for
    // `apply_querystats_events` to resolve and reply via
    // `World::queue_system_text` once drained.
    if lower.len() >= 5 && "querystats".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        world.queue_querystats_lookup(character_id);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    if lower.len() >= 6 && "staffcode".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, code_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };

        let mut letters = code_text.trim_start().chars();
        let first = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let second = letters
            .next()
            .filter(char::is_ascii_alphabetic)
            .map(|ch| ch.to_ascii_uppercase())
            .unwrap_or('A');
        let code = format!("{first}{second}");
        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let target_name = target.name.clone();
        target.staff_code = code.clone();
        runtime.staff_codes.insert(target_id, code.clone());
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!("Set {target_name}'s staff code to {code}.")],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}
