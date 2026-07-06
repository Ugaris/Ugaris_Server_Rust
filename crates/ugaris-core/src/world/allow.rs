//! `/allow <name>` text command (C `command.c:8371-8378` dispatch ->
//! `allow_body`, `src/system/death.c:1013-1029`, plus its `server_chat`
//! body `allow_body_db`, `death.c:1045-1067`). No permission gate (any
//! player can use it), `cmdcmp(ptr, "allow", 3)`'s `minlen` is 3 - "all"
//! up to "allow" all match (`commands_admin.rs`'s dispatch site uses the
//! same `starts_with` abbreviation idiom already used for `/showvalues`/
//! `/showattack`).
//!
//! `allow_body` resolves the argument name (C `lookup_name`) and, once
//! resolved, grants that character third-party access to every grave
//! container the *caller* currently owns (`con[ct].owner ==
//! charID(cn)`) - i.e. "let `<name>` loot my own corpse(s)", never the
//! caller's kills (those are already governed by the separate `killer`
//! ACL slot `/allow` never touches - see `world/death.rs`'s
//! `set_grave_acl`). `allow_body_db`'s `con[n].access = coID ? ... : 0`
//! ternary means granting access to a new character silently overwrites
//! whatever character was previously granted access to that grave (a
//! grave has only one grantable access slot, matching this port's
//! `grant_grave_access`).
//!
//! C's real `allow_body_db` runs via a `server_chat(1026, ...)`
//! cross-area broadcast so every area server currently holding one of
//! the caller's graves grants access locally and reports its own count
//! back via `tell_chat`. This codebase has no cross-process chat relay
//! yet (see the "Cross-area transfer" `PORTING_TODO.md` entry's gap
//! (3)), so this is the documented single-process-only slice: the
//! target name is resolved via `find_login_target` (C's synchronous
//! `lookup_name`), then the grant applies to every grave this process's
//! own `World` currently holds for the caller - matching every other
//! documented cross-area gap in this codebase.
use super::lastseen::is_valid_lookup_name;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowRequest {
    pub caller_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `/allow <name>`'s inline handler (`command.c:8371-8378`): trims
    /// leading whitespace, then calls `allow_body(cn, ptr)` on the
    /// entire (untokenized) remainder - like `/showvalues`, no
    /// alpha-only prefix extraction happens first. `allow_body` itself
    /// does no validation of its own beyond `lookup_name`'s own gate; an
    /// invalid shape resolves to C's `coID == -1` branch, "No player by
    /// that name." - the same text as a DB-confirmed-missing name.
    pub fn queue_allow_command(&mut self, caller_id: CharacterId, target_name: &str) {
        let target_name = target_name.trim_start();
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(caller_id, "No player by that name.".to_string());
            return;
        }
        self.pending_allow_requests.push(AllowRequest {
            caller_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_allow_requests(&mut self) -> Vec<AllowRequest> {
        self.pending_allow_requests.drain(..).collect()
    }

    /// C `allow_body_db` (`death.c:1045-1067`): grants `target_id`
    /// access to every grave container `caller_id` owns in this
    /// process's `World`, returning the number of graves updated (C's
    /// `cnt`, reported back to the caller by the `ugaris-server` async
    /// drain via `tell_chat(0, cnID, 1, "Area %d: Allowed access to %d
    /// corpses.", areaID, cnt)`).
    pub fn grant_grave_access_to(&mut self, caller_id: CharacterId, target_id: CharacterId) -> u32 {
        let mut count = 0u32;
        for item in self.items.values_mut() {
            if crate::item_driver::grave_owner_id(item) == caller_id.0 {
                crate::item_driver::grant_grave_access(item, Some(target_id));
                count += 1;
            }
        }
        count
    }
}
