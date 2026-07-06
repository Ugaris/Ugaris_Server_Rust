//! `/jail` and `/unjail` admin text commands (C `command.c:8861-8882`
//! dispatch -> `cmd_jail_player`/`cmd_unjail_player`, `command.c:2022-2041`/
//! `2230-2249` -> `jail_player`/`unjail_player`, `src/system/tool.c:
//! 4392-4425`), both `CF_STAFF|CF_GOD`-gated, full-word only (`cmdcmp`'s
//! `minlen` equals the full word length for both, so no abbreviation is
//! accepted).
//!
//! C's dispatcher gates the whole call on `lookup_name(ptr, name)`
//! succeeding first (`ID == 0` -> silent no-op this tick, matching every
//! other not-yet-cached async lookup in this codebase; `ID == -1` ->
//! "No character by the name %s.") *before* `cmd_jail_player`/
//! `cmd_unjail_player` run their own, entirely separate online-only
//! `CF_PLAYER` name scan (a real double-gate quirk: the target must both
//! be a known DB account *and* currently online for anything to happen -
//! an offline-but-real account resolves to `cmd_jail_player`'s own "No
//! player by that name." instead). Like `world/lastseen.rs`'s `/lastseen`
//! and `world/admin_flag.rs`'s `cmd_flag` offline fallback, `World` has no
//! DB handle, so a validly-shaped target name is queued as [`JailLookup`]
//! and resolved against Postgres in `ugaris-server`'s `world_events.rs::
//! apply_jail_events`, which then calls back into [`World::
//! resolve_jail_lookup`] for the online-scan/mutation half.
//!
//! `jail_player`/`unjail_player` (`tool.c:4392-4425`) themselves are
//! otherwise fully self-contained: unconditionally set the target's
//! respawn point (`restx`/`resty`/`resta`, plus `CF_RESPAWN` for jail
//! only - a real asymmetry with unjail, preserved as-is) to the
//! `GameSettings`-backed jail/aston location (already wired by
//! `/setjaillocation`/`/setastonlocation`), message both parties, then
//! either `teleport_char_driver` locally (when this area server's
//! `area_id` already equals the jail/aston area) or queue a
//! [`JailCrossAreaTransfer`] for `ugaris-server`'s `world_events.rs::
//! apply_jail_cross_area_transfers` to hand off to the shared
//! `attempt_cross_area_transfer` helper (`World` has no DB handle or
//! `ServerRuntime` of its own, same reason the lookup itself is deferred)
//! - matching C's `change_area(cn, resta, restx, resty)` call exactly;
//! the caller is only told "Nothing happens - target area server is
//! down." if that hand-off itself fails.
use super::lastseen::is_valid_lookup_name;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JailAction {
    Jail,
    Unjail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JailLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub action: JailAction,
}

/// A `/jail`/`/unjail` mutation whose destination area differs from this
/// area server's own `area_id` - queued for `ugaris-server`'s
/// `world_events.rs::apply_jail_cross_area_transfers` since `World` has
/// no DB handle or `ServerRuntime` to perform the `change_area` hand-off
/// itself. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JailCrossAreaTransfer {
    pub caller_id: CharacterId,
    pub target_id: CharacterId,
    pub target_area: u16,
    pub target_x: u16,
    pub target_y: u16,
}

impl World {
    /// C `/jail`/`/unjail`'s dispatcher-level `lookup_name` gate
    /// (`command.c:8848-8858`/`8839-8849`... see module doc comment): an
    /// invalidly-shaped name resolves immediately (C's synchronous `-1`
    /// case); a validly-shaped one is queued for the DB round trip.
    pub fn queue_jail_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: &str,
        action: JailAction,
    ) {
        let target_name = target_name.trim();
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(
                caller_id,
                format!("No character by the name {target_name}."),
            );
            return;
        }
        self.pending_jail_lookups.push(JailLookup {
            caller_id,
            target_name: target_name.to_string(),
            action,
        });
    }

    pub fn drain_pending_jail_lookups(&mut self) -> Vec<JailLookup> {
        self.pending_jail_lookups.drain(..).collect()
    }

    /// Drains every cross-area `/jail`/`/unjail` hand-off queued this
    /// tick - see [`JailCrossAreaTransfer`].
    pub fn drain_pending_jail_cross_area_transfers(&mut self) -> Vec<JailCrossAreaTransfer> {
        self.pending_jail_cross_area_transfers.drain(..).collect()
    }

    /// Called once the DB has confirmed `target_name` is a real account
    /// (C's `lookup_name` returning a positive ID): reproduces
    /// `cmd_jail_player`/`cmd_unjail_player`'s own separate online-only
    /// `CF_PLAYER` name scan (`command.c:2022-2032`/`2230-2240`) and, on a
    /// match, applies the jail/unjail mutation. No match -> "No player by
    /// that name." (the exact text both `cmd_*_player` functions share).
    pub fn resolve_jail_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: &str,
        action: JailAction,
    ) {
        let Some(target_id) = self.find_online_player_by_name(target_name) else {
            self.queue_system_text(caller_id, "No player by that name.".to_string());
            return;
        };
        self.apply_jail_action(caller_id, target_id, action);
    }

    /// C `jail_player`/`unjail_player` (`tool.c:4392-4425`) - see the
    /// module doc comment for the full behavior breakdown.
    fn apply_jail_action(
        &mut self,
        caller_id: CharacterId,
        target_id: CharacterId,
        action: JailAction,
    ) {
        let caller_name = self
            .characters
            .get(&caller_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();

        let (rest_x, rest_y, rest_area, set_respawn_flag, verb) = match action {
            JailAction::Jail => (
                self.settings.jail_x as u16,
                self.settings.jail_y as u16,
                self.settings.jail_area as u16,
                true,
                "jailed",
            ),
            JailAction::Unjail => (
                self.settings.aston_x as u16,
                self.settings.aston_y as u16,
                self.settings.aston_area as u16,
                false,
                "unjailed",
            ),
        };

        let Some(target) = self.characters.get_mut(&target_id) else {
            return;
        };
        target.rest_x = rest_x;
        target.rest_y = rest_y;
        target.rest_area = rest_area;
        if set_respawn_flag {
            target.flags.insert(CharacterFlags::RESPAWN);
        }
        let target_name = target.name.clone();

        self.queue_system_text(target_id, format!("You have been {verb} by {caller_name}."));
        self.queue_system_text(caller_id, format!("You have {verb} {target_name}."));

        if self.area_id == rest_area {
            self.teleport_char_driver(target_id, rest_x, rest_y);
        } else {
            self.pending_jail_cross_area_transfers
                .push(JailCrossAreaTransfer {
                    caller_id,
                    target_id,
                    target_area: rest_area,
                    target_x: rest_x,
                    target_y: rest_y,
                });
        }
    }
}
