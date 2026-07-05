//! Generic by-name character-flag toggle text commands (C `cmd_flag`,
//! `src/system/command.c:2870-2937`), used by `/god` (`CF_GOD`), `/setsir`
//! (`CF_WON`), `/staff` (`CF_STAFF`), `/emaster` (`CF_EVENTMASTER`),
//! `/devel` (`CF_DEVELOPER`), `/hardcore` (`CF_HARDCORE`), and `/qmaster`
//! (`CF_LQMASTER`) - all `CF_GOD`-gated (`command.c:9257-9337`).
//!
//! Unlike the self-toggle commands (`/immortal`, `/invisible`, `/xray`,
//! `/spy`, ...), `cmd_flag` always targets a *named* character (never the
//! caller implicitly): it first scans the currently loaded character
//! table (`getfirst_char`/`getnext_char`, matching any loaded character
//! regardless of `CF_PLAYER` - reproduced here by reusing
//! `commands_player::find_online_character_by_name`, which has the same
//! no-`CF_PLAYER`-filter shape) for an exact case-insensitive name match.
//! If found, the flag is toggled in memory immediately and the caller
//! gets `"Set {name} {flag_name} to {on/off}."` (`command.c:2932-2936`).
//!
//! If no loaded character matches, C falls through to `lookup_name`/
//! `task_set_flags` (`command.c:2887-2898`): a synchronous name-validity
//! check against a cached name index (`-1` -> "Sorry, no player by the
//! name %s.", sent immediately) followed by an async DB task-queue write
//! whose *own* completion handler, `set_flags` (`task.c:198-211`), always
//! toggles the flag on the persisted row (no "online somewhere else"
//! rejection message - `set_task`'s cross-check at `task.c:250-253` is a
//! silent `xlog`-only no-op) and then sends a *second*, differently
//! worded confirmation back to the caller: `"Set flag on %s to %s."`
//! (`task.c:208`, genuinely inconsistent with cmd_flag's own online-branch
//! wording, which is preserved as-is rather than "fixed").
//!
//! `World` has no DB handle and no synchronous name-index cache, so -
//! like `world/lastseen.rs`'s `/lastseen` and `world/clanmaster.rs`'s
//! `rank:`/`fire:` offline fallback - the not-loaded case defers *both*
//! of C's two messages to the single async DB round-trip performed by
//! `ugaris-server`'s `world_events.rs::apply_admin_flag_events`: a
//! missing DB row gets the "Sorry, no player by the name %s." text (C's
//! synchronous `lookup_name == -1` case), while a real row gets both the
//! "Update scheduled." acknowledgement and (assuming the character isn't
//! logged into a different area) the "Set flag on %s to %s." completion,
//! in that order.
use super::lastseen::is_valid_lookup_name;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminFlagToggle {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub flag: CharacterFlags,
}

impl World {
    /// C `cmd_flag`'s online-scan branch (`command.c:2882-2886`):
    /// `getfirst_char`/`getnext_char` walks *every* currently loaded
    /// character (no `CF_PLAYER` filter, unlike the sibling
    /// `find_online_player_by_name` in `world/clanmaster.rs`/
    /// `world/trader.rs`), matching by exact case-insensitive name.
    fn find_loaded_character_by_name(&self, name: &str) -> Option<CharacterId> {
        self.characters
            .values()
            .find(|character| character.name.eq_ignore_ascii_case(name))
            .map(|character| character.id)
    }

    /// C `cmd_flag` (`command.c:2870-2937`), the shared body of `/god`,
    /// `/setsir`, `/staff`, `/emaster`, `/devel`, `/hardcore`, and
    /// `/qmaster`. `target_name` is the already-parsed alphabetic name
    /// token (C's `isalpha`-only scan, `command.c:2874-2876` -
    /// non-alphabetic trailing text, if any, was never part of the name
    /// and is simply ignored, matching C). `flag_name` is C's `fptr`
    /// switch-case string (`command.c:2901-2930`), used only by the
    /// *online* branch's message - see the module doc comment for why
    /// the offline completion message never names the flag.
    ///
    /// Returns the caller-facing message(s), if any (empty when nothing
    /// is said immediately - the offline branch's messages are entirely
    /// deferred to `ugaris-server`'s `apply_admin_flag_events`).
    pub fn apply_cmd_flag_command(
        &mut self,
        caller_id: CharacterId,
        target_name: &str,
        flag: CharacterFlags,
        flag_name: &str,
    ) -> Vec<String> {
        if let Some(target_id) = self.find_loaded_character_by_name(target_name) {
            let Some(target) = self.characters.get_mut(&target_id) else {
                return Vec::new();
            };
            target.flags.toggle(flag);
            let state = if target.flags.contains(flag) {
                "on"
            } else {
                "off"
            };
            return vec![format!("Set {} {flag_name} to {state}.", target.name)];
        }
        if !is_valid_lookup_name(target_name) {
            // C `lookup_name`'s synchronous `-1` case (`lookup.c:44-63`),
            // reported immediately - `command.c:2893` - without ever
            // reaching `task_set_flags`.
            return vec![format!("Sorry, no player by the name {target_name}.")];
        }
        self.pending_admin_flag_toggles.push(AdminFlagToggle {
            caller_id,
            target_name: target_name.to_string(),
            flag,
        });
        Vec::new()
    }

    pub fn drain_pending_admin_flag_toggles(&mut self) -> Vec<AdminFlagToggle> {
        self.pending_admin_flag_toggles.drain(..).collect()
    }
}
