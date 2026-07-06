//! `/lockname <name>` and `/unlockname <name>` admin text commands (C
//! `command.c:6528-6543` dispatch -> `cmd_lockname`/`cmd_unlockname`,
//! `command.c:2679-2701` -> `do_lockname`/`do_unlockname` ->
//! `db_lockname`/`db_unlockname`, `src/system/database/database_admin.c:
//! 357-434`), both `CF_GOD`-gated, full-word only (`cmdcmp`'s `minlen` is
//! 8/10, the full word length, no abbreviation accepted).
//!
//! Unlike `/rename`'s alpha-first-then-length validation order (see
//! `world/rename.rs`'s module doc comment), `db_lockname`/`db_unlockname`
//! check length *first* (`3..=35`, "Name too long or too short."), then
//! lowercase + alpha-validate every character ("Illegal name." on the
//! first non-alphabetic one) - the exact reverse order. Every reply
//! message uses the *original*, un-lowercased argument (C's `name`
//! parameter, not its own `lowercase_name` scratch buffer) even though
//! the DB row itself is keyed by the lowercased form.
//!
//! Neither command takes any online/offline distinction into account -
//! like `/rename`, there is no `lookup_name` call at all, so `World`
//! queues the validated request directly as a [`LockNameLookup`]/
//! [`UnlockNameLookup`] and `ugaris-server`'s `world_events.rs::
//! apply_lockname_events`/`apply_unlockname_events` resolve it against a
//! new `locked_names` Postgres table (this codebase's equivalent of C's
//! `badname` table - see `migrations/0012_locked_names.sql` for why that
//! table has no other consumer). Both commands exist purely as the
//! admin-facing audit/record-keeping actions C itself performs, matching
//! scope.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockNameLookup {
    pub requester_id: CharacterId,
    pub original_name: String,
    pub lookup_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockNameLookup {
    pub requester_id: CharacterId,
    pub original_name: String,
    pub lookup_name: String,
}

impl World {
    /// Shared `db_lockname`/`db_unlockname` validation (`database_admin.c:
    /// 365-374,388-397` / `436-445,459-466`): length bound first, then
    /// the lowercase + alpha-validate loop. Returns the lowercased query
    /// key on success.
    fn validate_lock_name(&mut self, requester_id: CharacterId, name: &str) -> Option<String> {
        if !(3..=35).contains(&name.len()) {
            self.queue_system_text(requester_id, "Name too long or too short.".to_string());
            return None;
        }
        let mut lowercase = String::with_capacity(name.len());
        for ch in name.chars() {
            if !ch.is_ascii_alphabetic() {
                self.queue_system_text(requester_id, "Illegal name.".to_string());
                return None;
            }
            lowercase.push(ch.to_ascii_lowercase());
        }
        Some(lowercase)
    }

    pub fn queue_lockname_command(&mut self, requester_id: CharacterId, name: &str) {
        let Some(lowercase) = self.validate_lock_name(requester_id, name) else {
            return;
        };
        self.pending_lockname_lookups.push(LockNameLookup {
            requester_id,
            original_name: name.to_string(),
            lookup_name: lowercase,
        });
    }

    pub fn queue_unlockname_command(&mut self, requester_id: CharacterId, name: &str) {
        let Some(lowercase) = self.validate_lock_name(requester_id, name) else {
            return;
        };
        self.pending_unlockname_lookups.push(UnlockNameLookup {
            requester_id,
            original_name: name.to_string(),
            lookup_name: lowercase,
        });
    }

    pub fn drain_pending_lockname_lookups(&mut self) -> Vec<LockNameLookup> {
        self.pending_lockname_lookups.drain(..).collect()
    }

    pub fn drain_pending_unlockname_lookups(&mut self) -> Vec<UnlockNameLookup> {
        self.pending_unlockname_lookups.drain(..).collect()
    }
}
