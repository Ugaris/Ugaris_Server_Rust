//! `/exterminate <name>` admin text command (C `command.c:9657-9662`
//! dispatch -> `cmd_exterminate`, `command.c:2639-2651`), `CF_STAFF|
//! CF_GOD`-gated, full-word only (`cmdcmp`'s `minlen` is 11, the full
//! length of "exterminate", no abbreviation accepted).
//!
//! `cmd_exterminate` itself does no name validation beyond the parse
//! loop (`isalpha(*ptr)`, capped at 79 characters, same as every other
//! admin name-token command in this codebase) - unlike `/punish`, there
//! is no `lookup_name`/`is_valid_lookup_name` synchronous pre-check, and
//! unlike `/lockname` there is no length/charset re-validation either.
//! The whole thing (`exterminate`/`db_exterminate`,
//! `src/system/database/database_admin.c:29-95,503-507`) is a **direct
//! DB mutation**, not a `server_chat`-relayed cross-area operation - it
//! looks up the target's owning account by name, locks that account
//! (blocking every character on it from logging in, already enforced by
//! `begin_login_tx`'s `account_locked` gate), and bans every IP that
//! account has ever logged in from (see `migrations/0019_ip_bans.sql`
//! and `CharacterRepository::exterminate_account`'s doc comment for the
//! `iplog`/`ipban` mapping). None of this depends on whether the target
//! is currently online or which area server it last played on, so
//! `World` (which has no DB handle) just queues the parsed name exactly
//! like `/lockname`/`/rename` - see `ugaris-server`'s `world_events.rs::
//! apply_exterminate_events` for the DB round trip and reply text this
//! drives.
//!
//! Deliberately skipped (documented, not silent), matching this
//! codebase's established skip-untracked-C-side-effect convention (see
//! `/kick`'s `PORTING_TODO.md` Progress Log entry):
//! - `sendmail`'s exterminate notification email
//!   (`database_admin.c:87-89`).
//! - `server_chat(31, ...)`'s cross-area staff broadcast
//!   (`command.c:2650`) - this codebase has no multi-process chat relay
//!   (see the "Cross-area transfer" `PORTING_TODO.md` entry's gap (3));
//!   only the caller receives the local acknowledgement C also sends via
//!   `xlog`+`tell_chat`.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExterminateRequest {
    pub caller_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `cmd_exterminate` (`command.c:2639-2651`). `target_name` is the
    /// already-parsed alphabetic name token (empty when the argument
    /// began with a non-alphabetic character, matching C's own
    /// zero-iteration scan loop - the DB lookup then simply finds
    /// nothing, reproducing C's "Player '%s' not found." for an empty
    /// name too).
    pub fn queue_exterminate_command(&mut self, caller_id: CharacterId, target_name: &str) {
        self.pending_exterminate_requests.push(ExterminateRequest {
            caller_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_exterminate_requests(&mut self) -> Vec<ExterminateRequest> {
        self.pending_exterminate_requests.drain(..).collect()
    }
}
