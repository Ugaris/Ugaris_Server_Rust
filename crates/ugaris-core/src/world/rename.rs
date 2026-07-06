//! `/rename <from> <to>` admin text command (C `command.c:6517-6524`
//! dispatch -> `cmd_rename`, `command.c:2657-2676` -> `do_rename`/
//! `db_rename`, `src/system/database/database_admin.c:291-355`),
//! `CF_GOD`-gated, full-word only (`cmdcmp`'s `minlen` is 6, the full
//! length of "rename", no abbreviation accepted).
//!
//! Unlike `/jail`/`/lastseen`/`cmd_flag`'s offline fallback, `cmd_rename`
//! itself never calls `lookup_name` at all - `do_rename`/`db_rename` run
//! directly against the `chars` table by name with no online/offline
//! distinction, so there is no synchronous validity gate here beyond
//! `to`'s own case-normalization/alpha/length checks (mirrored exactly
//! from `db_rename`'s own validation order, `database_admin.c:305-320`:
//! alpha-check every character *while* capitalizing the first and
//! lowercasing the rest, bailing with "Illegal name." on the first
//! non-alphabetic character; only once the whole string passes is the
//! *length* checked, `3..=35`, "Name too long or too short." - a
//! genuinely different check order from `/lockname`'s length-first, see
//! `world/lockname.rs`'s module doc comment). `from` gets no validation
//! at all in C (an empty or malformed `from` just fails to match any row
//! later - reported as the "not found" case below).
//!
//! `World` has no DB handle, so a validly-shaped rename is queued as
//! [`RenameLookup`] and resolved against Postgres in `ugaris-server`'s
//! `world_events.rs::apply_rename_events`, which reproduces `db_rename`'s
//! three-way reply: a query error -> "Failed to change name."; no row
//! matched `from` -> "Didn't work, most probable cause: %s not found.";
//! success -> "Changed %s to %s. The change will be visible after the
//! next login." (the exact reason this rewrite never refreshes a live
//! in-memory `Character.name`, even if the caller renamed themselves).
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameLookup {
    pub requester_id: CharacterId,
    pub from_name: String,
    pub to_name: String,
}

impl World {
    /// C `cmd_rename`'s `to`-name validation (`do_rename`'s own
    /// capitalize/lowercase + alpha loop, `database_admin.c:305-317`,
    /// followed by its `len` bounds check, `:318-320`). `from` is passed
    /// through unvalidated, matching C.
    pub fn queue_rename_command(
        &mut self,
        requester_id: CharacterId,
        from_name: &str,
        to_name: &str,
    ) {
        let mut normalized_to = String::with_capacity(to_name.len());
        for (index, ch) in to_name.chars().enumerate() {
            if !ch.is_ascii_alphabetic() {
                self.queue_system_text(requester_id, "Illegal name.".to_string());
                return;
            }
            normalized_to.push(if index == 0 {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            });
        }
        if !(3..=35).contains(&normalized_to.len()) {
            self.queue_system_text(requester_id, "Name too long or too short.".to_string());
            return;
        }
        self.pending_rename_lookups.push(RenameLookup {
            requester_id,
            from_name: from_name.to_string(),
            to_name: normalized_to,
        });
    }

    pub fn drain_pending_rename_lookups(&mut self) -> Vec<RenameLookup> {
        self.pending_rename_lookups.drain(..).collect()
    }
}
