//! `/rmdeath <name>` admin text command (C `command.c:8884-8903` dispatch
//! -> `cmd_removedeath`, `command.c:2006-2019`), `CF_GOD`-gated, full-word
//! only (`cmdcmp`'s `minlen` is 7, the full length of "rmdeath", so no
//! abbreviation is accepted).
//!
//! C's dispatcher gates the whole call on `lookup_name(ptr, name)`
//! succeeding first (`ID == 0` -> silent no-op this tick, matching every
//! other not-yet-cached async lookup in this codebase; `ID == -1` ->
//! "No character by the name %s.") *before* `cmd_removedeath` runs its own
//! `co < MAXCHARS` bounds check and mutates `ch[co].deaths` directly - in
//! C's architecture every character that has ever existed keeps a
//! permanent, always-resident `ch[]` slot (online or not), so that bounds
//! check is effectively dead code and the mutation always lands whether
//! or not the target is currently connected. `World` has no such
//! permanent record for offline characters (see `world/jail.rs`'s module
//! doc comment for the identical architecture gap), so - exactly like
//! `/jail`/`/unjail` - a validly-shaped name is queued as [`RmdeathLookup`]
//! and resolved against Postgres in `ugaris-server`'s `world_events.rs::
//! apply_rmdeath_events`, which then calls back into [`World::
//! resolve_rmdeath_lookup`] for the online-scan/mutation half: a
//! DB-confirmed but currently-offline account reports "No player by that
//! name." instead of silently mutating an absent in-memory record - the
//! same deliberate deviation `/jail`/`/unjail` already established for
//! this exact gap, not a new one.
use super::lastseen::is_valid_lookup_name;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmdeathLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `/rmdeath`'s dispatcher-level `lookup_name` gate (`command.c:
    /// 8884-8896`; see module doc comment): an invalidly-shaped name
    /// resolves immediately (C's synchronous `-1` case); a validly-shaped
    /// one is queued for the DB round trip.
    pub fn queue_rmdeath_lookup(&mut self, caller_id: CharacterId, target_name: &str) {
        let target_name = target_name.trim();
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(
                caller_id,
                format!("No character by the name {target_name}."),
            );
            return;
        }
        self.pending_rmdeath_lookups.push(RmdeathLookup {
            caller_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_rmdeath_lookups(&mut self) -> Vec<RmdeathLookup> {
        self.pending_rmdeath_lookups.drain(..).collect()
    }

    /// Called once the DB has confirmed `target_name` is a real account
    /// (C's `lookup_name` returning a positive ID): C `cmd_removedeath`
    /// (`command.c:2006-2019`) - see module doc comment for the
    /// online-only deviation. On a match, decrements the target's
    /// `deaths` counter by one (saturating at zero - C's plain `int--`
    /// would go negative, but `Character::deaths` is unsigned here) and
    /// messages the caller "Removing 1 death from %s." (C's `dlog` audit
    /// line is skipped, matching the established untracked-C-side-effect
    /// convention for this file's other admin commands).
    pub fn resolve_rmdeath_lookup(&mut self, caller_id: CharacterId, target_name: &str) {
        let Some(target_id) = self.find_online_player_by_name(target_name) else {
            self.queue_system_text(caller_id, "No player by that name.".to_string());
            return;
        };
        let Some(target) = self.characters.get_mut(&target_id) else {
            self.queue_system_text(caller_id, "No player by that name.".to_string());
            return;
        };
        target.deaths = target.deaths.saturating_sub(1);
        let target_name = target.name.clone();
        self.queue_system_text(caller_id, format!("Removing 1 death from {target_name}."));
    }
}
