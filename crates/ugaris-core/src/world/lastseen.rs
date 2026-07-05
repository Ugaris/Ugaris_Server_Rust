//! `/lastseen` text command (C `command.c:9027-9046`, dispatching to
//! `lastseen`/`db_lastseen` in `src/system/database/database_lookup.c:
//! 142-157` + `src/system/database/database_notes.c:352-390`).
//!
//! Like `world/clanmaster.rs`'s `rank:`/`fire:` offline-name fallback,
//! `World` has no DB handle, so a validly-shaped target name is queued as
//! [`LastSeenLookup`] and resolved synchronously (from this codebase's
//! point of view - genuinely asynchronously from the DB's point of view)
//! against Postgres in `ugaris-server`'s `world_events.rs::
//! apply_lastseen_events`, which delivers the reply via
//! [`World::queue_system_text`] - see that function's doc comment for the
//! full message-shape breakdown (matching `db_lastseen` exactly).
//!
//! C's `lookup_name` (`lookup.c:42-59`) has its own early-return validity
//! gate - empty, any non-alphabetic byte, or length outside `2..=38` all
//! resolve to `-1` ("no such name") *before* ever touching the database -
//! and the command dispatcher's `ID == -1` branch and `db_lastseen`'s
//! "no DB row" branch print the exact same "No character by the name %s."
//! text, so both cases are handled here: an invalid shape is answered
//! immediately (no DB round-trip needed, matching C's synchronous `-1`),
//! while a valid shape is queued for the DB lookup.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSeenLookup {
    pub requester_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `command.c`'s `lastseen:` handler (`command.c:9027-9046`): trims
    /// leading whitespace off the argument (`while (isspace(*ptr))
    /// ptr++;`), then calls `lookup_name(ptr, name)`. See the module doc
    /// comment for why the validity gate is folded in here rather than
    /// deferred to the DB round-trip.
    pub fn queue_lastseen_lookup(&mut self, requester_id: CharacterId, target_name: &str) {
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(
                requester_id,
                format!("No character by the name {target_name}."),
            );
            return;
        }
        self.pending_lastseen_lookups.push(LastSeenLookup {
            requester_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_lastseen_lookups(&mut self) -> Vec<LastSeenLookup> {
        self.pending_lastseen_lookups.drain(..).collect()
    }
}

/// C `lookup_name`'s validity gate (`lookup.c:44-59`): non-empty, every
/// byte alphabetic (`isalpha`), and length in `2..=38`. `pub(super)`
/// (rather than private) so sibling modules with their own
/// `lookup_name`-gated offline fallback (e.g. `world/admin_flag.rs`'s
/// `cmd_flag` port) can reuse it instead of duplicating the gate.
pub(super) fn is_valid_lookup_name(name: &str) -> bool {
    let len = name.len();
    (2..=38).contains(&len) && name.bytes().all(|b| b.is_ascii_alphabetic())
}
