use super::*;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct KeyringCommandResult {
    pub(crate) messages: Vec<String>,
    pub(crate) message_bytes: Vec<Vec<u8>>,
    pub(crate) target_message_bytes: Vec<(CharacterId, Vec<u8>)>,
    pub(crate) inventory_changed: bool,
    pub(crate) name_changed: bool,
    pub(crate) name_refresh: Vec<CharacterId>,
    /// Set when the command moved the character to a new mirror (C
    /// `ch[cn].mirror = m` in `/goto`/`/jump`, `command.c`). The call site
    /// must send the client a `mirror` packet, matching the same-area
    /// transport-travel mirror-change path.
    pub(crate) mirror_changed: Option<u32>,
    /// Set when `/logout` was used while standing on a blue square (C
    /// `cmd_logout`, `command.c:4457-4471`, gated on `MF_RESTAREA`). The
    /// character is still fully live in the world at this point; the call
    /// site must perform the actual `exit_char`/`player_client_exit`
    /// teardown: save its snapshot (at its rest position, matching C's
    /// `tmpx/tmpy = restx/resty` swap before `kick_char`'s save), remove it
    /// from the world, then send `SV_EXIT` and disconnect every session
    /// attached to it.
    pub(crate) logout_requested: bool,
    /// Set to the target's id when `/kick` (C `command.c:8668-8698`) found
    /// an online player by name. The call site must perform the same
    /// `exit_char`/`player_client_exit` teardown as `logout_requested`,
    /// but against this target character instead of the command caller:
    /// save its snapshot at its rest position, remove it from the world,
    /// then send `SV_EXIT` (with the kick-specific reason text) and
    /// disconnect every session attached to it.
    pub(crate) kick_target: Option<CharacterId>,
    /// Set by `/setclanjewels` (C `command.c:7563-7596`) when the
    /// optional `do_log` argument is nonzero (the default): `(clan_nr,
    /// serial, prio, content)` for the call site to hand to
    /// `clan_log::write_clan_log_entry`, matching C's
    /// `add_clanlog(clan_nr, clan[clan_nr].status.serial, ch[cn].ID, 1,
    /// ...)`. The command layer has no DB handle of its own (same reason
    /// `/clanlog` itself is wired at the `main.rs` call site instead of
    /// here).
    pub(crate) clan_log_entry: Option<(u16, u32, u8, String)>,
    /// Set by `/saveall` (C `command.c:7460-7473`, `CF_GOD`-gated). The
    /// command layer has no DB handle of its own (same reason `/kick`'s
    /// save and `/setclanjewels`'s clan-log write are both deferred to the
    /// `main.rs` call site instead of here): the call site must (1) save
    /// exactly one online player snapshot, advancing the round-robin
    /// cursor (C `backup_players`'s static `n`, `player.c:3707-3721`, also
    /// driven every 85s by `maintenance_60s_task` - unlike that periodic
    /// sweep, this flag only advances the cursor once per `/saveall`
    /// invocation, matching C exactly), and (2) save every live merchant
    /// store (C `save_all_merchants`, `database_merchant.c:848-857`).
    /// The messages below are pushed unconditionally up front, matching
    /// C's own unconditional `log_char` calls (`backup_players`/
    /// `save_all_merchants` return nothing C checks). C's third pair of
    /// messages ("Forcing save of pentagram records..."/"Pentagram
    /// records saved" around `save_pentagram_record_scheduled()`) is
    /// deliberately omitted: the pentagram-record-tracking feature itself
    /// isn't ported yet (see `PORTING_TODO.md`'s `/saveall` note), so
    /// there is nothing to save and claiming otherwise would be dishonest.
    pub(crate) save_all_requested: bool,
    /// Set by `/clearmerchantstores <id>` (C `command.c:7510-7538`,
    /// `CF_GOD`-gated) to the merchant's `CharacterId` after the command
    /// layer has already reset the in-memory store (gold to 10000, every
    /// ware slot cleared). The command layer has no DB handle of its own
    /// (same reason `/saveall`'s merchant sweep is deferred to the
    /// `main.rs` call site instead of here), so the call site must persist
    /// the cleared store via `save_merchant_store_if_configured`, matching
    /// C's own `save_merchant_inventory(merchant_cn)` call right after the
    /// mutation.
    pub(crate) clear_merchant_store_requested: Option<CharacterId>,
    /// Plain-text system messages addressed to a character other than the
    /// command caller (e.g. `/changetunnel`/`/settunnel`/`/cleartunnel`,
    /// C `command.c:2045-2199`, notifying the edited target player). Kept
    /// separate from `target_message_bytes` (already-encoded packet
    /// bytes) since these are plain `log_char`-style strings, matching
    /// the convention `auction::AuctionCommandResult::other_messages`
    /// already established for the same "message a non-caller character"
    /// shape.
    pub(crate) other_messages: Vec<(CharacterId, String)>,
    /// Set by `/office` (`command.c:9670-9676`) and `/goto`/`/jump`
    /// (`finish_goto_jump`, ported from `command.c:8537-8567`/`8608-8625`)
    /// when the target is in a different area than the caller's current
    /// one: `(target_area, target_x, target_y)` for the call site to hand
    /// to `attempt_cross_area_transfer`, matching C's `change_area(cn, a,
    /// x, y)`. The command layer has no DB handle or `ServerRuntime` of
    /// its own (same reason `/kick`'s save is deferred to the `main.rs`
    /// call site instead of here); the target mirror is `mirror_changed`
    /// when the command also set one (C sets `ch[cn].mirror = m` before
    /// calling `change_area`, which then reads `ch[cn].mirror` via
    /// `get_area`), else the caller's own current area/mirror. On
    /// failure the call site must fall back to the same "Nothing happens
    /// - target area server is down." message every other cross-area
    ///   teleport site in this codebase uses.
    pub(crate) cross_area_transfer: Option<(u16, u16, u16)>,
}
