//! `/complain <name> [reason...]` text command's offline name-lookup half
//! (C `cmd_complain`, `src/system/command.c:2281-2352`, dispatched from
//! `command.c:8769-8776`'s `cmdcmp(ptr, "complain", 4)`).
//!
//! Like `world/lastseen.rs`, `World` has no DB handle, so a validly-shaped
//! target name is queued as [`ComplainLookup`] and resolved against
//! Postgres in `ugaris-server`'s `world_events.rs::apply_complain_events`,
//! which delivers the reply via [`World::queue_system_text`].
//!
//! Every other branch of `cmd_complain` (the empty-argument message, the
//! one-time disclaimer, the per-minute rate limit, and the `"lag"`/
//! `"laggy"`/`"bug"`/`"why"`/`"the"`/`"too"`/`"this"`/`"can"` name
//! blocklist) needs only the caller's own `PlayerRuntime`/`Character`
//! state, not the world, and is handled directly in `ugaris-server`'s
//! `apply_complain_command` instead of here - see that function's doc
//! comment. C's `write_scrollback` (emailing the complaint to
//! `game@ugaris.com`) has no Rust equivalent (no email/CURL infra exists
//! in this codebase, the same established omission as `/kick`'s `dlog`).
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplainLookup {
    pub requester_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// Queues the DB-backed name resolution half of `cmd_complain`
    /// (`command.c:2320-2331`). C's own extra bound on the parsed name -
    /// `if (n < 3 || n > 40) ret = -n;` (tighter than `lookup_name`'s own
    /// `2..=38` gate, checked *before* ever calling it) - is folded in
    /// here as a synchronous fast path, matching `world/lastseen.rs`'s
    /// precedent of resolving statically-known failures without a DB
    /// round trip.
    pub fn queue_complain_lookup(&mut self, requester_id: CharacterId, target_name: &str) {
        if !(3..=40).contains(&target_name.len()) {
            self.queue_system_text(
                requester_id,
                format!("Sorry, no player by the name '{target_name}' found."),
            );
            return;
        }
        self.pending_complain_lookups.push(ComplainLookup {
            requester_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_complain_lookups(&mut self) -> Vec<ComplainLookup> {
        self.pending_complain_lookups.drain(..).collect()
    }
}
