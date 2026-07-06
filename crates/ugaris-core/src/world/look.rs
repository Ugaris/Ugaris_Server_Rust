//! `/look <name>` (`src/system/command.c:8990-9019`, `CF_GOD|CF_STAFF`-
//! gated) and `/klog` (`command.c:9022-9024` -> `karmalog`/`db_karmalog`,
//! `src/system/database/database_notes.c:230-275`, also `CF_GOD|CF_STAFF`
//! -gated) staff commands.
//!
//! `/look` resolves a target name (C `lookup_name`) then lists every
//! `kind = 1` (punishment) note filed against them (C `read_notes` ->
//! `db_read_notes` -> `list_punishment`, `src/system/punish.c:26-38`).
//! `/klog` takes no argument and lists every `kind = 1` note created in
//! the last 24 hours across every character (C `karmalog` ->
//! `db_karmalog` -> `karmalog_s`). Both need the reverse ID->name lookup
//! (C `lookup_ID`, `src/system/lookup.c:98-135`) for the note's creator
//! (and, for `/klog`, the note's target too) - see
//! `ugaris_db::CharacterRepository::find_name_by_id`.
//!
//! `World` has no DB handle, so both commands are queued here and
//! resolved against Postgres in `ugaris-server`'s `world_events.rs::
//! apply_look_events`/`apply_klog_events`, which deliver every reply line
//! via [`World::queue_system_text`] - see those functions' doc comments
//! for the exact message shapes (matching `db_read_notes`/`db_karmalog`
//! byte for byte, minus C's unreachable-in-this-codebase "lookup in
//! progress" intermediate state - see `queue_look_command`'s doc
//! comment for why).
use super::lastseen::is_valid_lookup_name;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookRequest {
    pub requester_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `/look`'s inline handler (`command.c:8990-9019`): trims leading
    /// whitespace, then either reports "Expected a character name." for
    /// an empty argument or calls `lookup_name(ptr, name)` on the *entire*
    /// (untokenized) remainder - unlike `/punish`'s `take_legacy_alpha_
    /// name`, no alpha-only prefix is extracted first, so any embedded
    /// space or punctuation reaches `lookup_name` and fails its own
    /// `isalpha` gate exactly the same way a too-long/too-short name does
    /// (both resolve to C's `ID == -1`, "No character by the name %s.").
    /// C's `ID == 0` ("Character lookup is in progress") branch has no
    /// analogue here: this codebase's name resolution is a single
    /// deferred-to-next-tick DB round trip, not a persistent multi-tick
    /// cache-fill, so it always eventually resolves to found-or-not-found,
    /// never stays "in progress" from the caller's point of view.
    pub fn queue_look_command(&mut self, requester_id: CharacterId, target_name: &str) {
        if target_name.is_empty() {
            self.queue_system_text(requester_id, "Expected a character name.".to_string());
            return;
        }
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(
                requester_id,
                format!("No character by the name {target_name}."),
            );
            return;
        }
        self.pending_look_requests.push(LookRequest {
            requester_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_look_requests(&mut self) -> Vec<LookRequest> {
        self.pending_look_requests.drain(..).collect()
    }

    /// C `/klog`'s inline handler (`command.c:9022-9024`): no argument at
    /// all, just `karmalog(ch[cn].ID)` - the reply always goes to the
    /// caller's own id.
    pub fn queue_klog_command(&mut self, requester_id: CharacterId) {
        self.pending_klog_requests.push(requester_id);
    }

    pub fn drain_pending_klog_requests(&mut self) -> Vec<CharacterId> {
        self.pending_klog_requests.drain(..).collect()
    }
}
