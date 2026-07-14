//! `/punish <name> <level> <reason>` (C `command.c:6500-6507` dispatch ->
//! `cmd_punish`, `command.c:2354-2406`) and `/unpunish <name> <note id>`
//! (`command.c:6541-6547` dispatch -> `cmd_unpunish`, `command.c:2706-2731`)
//! admin text commands. `/punish` is `CF_GOD|CF_STAFF`-gated, `/unpunish`
//! is `CF_GOD`-only; both are full-word only (`cmdcmp`'s `minlen` equals
//! each word's full length, so no abbreviation is accepted).
//!
//! Both dispatch through C's async DB task-queue (`task_punish_player`/
//! `task_unpunish_player`, `src/system/task.c:171-188,358-382`, whose
//! shared `set_task` completion handler (`task.c:213-295`) reproduces the
//! exact same "online (any loaded character, no `CF_PLAYER` filter,
//! `getfirst_char`/`getnext_char` scan by resolved ID) first, else read/
//! mutate/write the persisted `chars` row directly, else silently no-op
//! if the account is logged in somewhere else" shape `world/admin_flag.rs`'s
//! `cmd_flag` offline fallback and `world/clanmaster.rs`'s `rank:`/`fire:`
//! offline fallback already established in this codebase - reused here
//! rather than reinvented. `World` has no DB handle, so a validly-shaped
//! target name is queued as [`PunishRequest`]/[`UnpunishRequest`] and
//! resolved against Postgres in `ugaris-server`'s `world_events.rs::
//! apply_punish_events`/`apply_unpunish_events`.
//!
//! `punish()`/`unpunish()` (`src/system/punish.c:41-132`) themselves are
//! ported as the pure, `World`-independent [`apply_punishment`]/
//! [`apply_unpunishment`] functions below so the exact same mutation code
//! runs whether the target is a live `World::characters` entry (online)
//! or a freshly loaded, about-to-be-saved-and-discarded DB snapshot
//! (offline) - see `apply_punish_events`'s doc comment in `ugaris-server`
//! for how the two branches share these.
//!
//! Deliberately simplified vs. C (documented, not silent):
//! - `add_note`'s punishment record (`punish.c:106`, this codebase's
//!   `kind = 1` `notes` row - see [`PunishmentNote`]) is written
//!   unconditionally by the caller in `ugaris-server` once a mutation is
//!   applied; unlike C, a note-write failure does not roll back or block
//!   the karma/exp mutation and player-facing messages (C's own
//!   `punish()` returns `add_note`'s result, which `punish_player` uses to
//!   gate *all* of its messaging - a real behavioral gap, but this
//!   codebase has no transaction spanning both the `characters` table and
//!   the `notes` table here, matching the existing precedent of not
//!   rolling back a character mutation on a secondary write's failure
//!   elsewhere in this file, e.g. `world/clanmaster.rs`'s clan-log write).
//! - `write_scrollback`/`server_chat(31, ...)` (moderation-evidence email
//!   + cross-server broadcast, `command.c:2398-2404`) have no Rust
//!     equivalent and are skipped, matching the established
//!     skip-untracked-C-side-effect convention (see `/kick`'s Progress Log
//!     entry in `PORTING_TODO.md`).
//! - C's `kick_player` (`punish_player`'s lock/kick disconnect,
//!   `player.c:174-202`) both detaches the socket *and* leaves the
//!   character lingering under `CDR_LOSTCON` for reconnect - this
//!   codebase's `ugaris-server::apply_punish_events` reproduces this
//!   exactly by sending the exit message and requesting a session
//!   disconnect (not a full `/kick`-style `exit_char` teardown), which
//!   funnels through the same `SessionEvent::Disconnected` ->
//!   `enter_lostcon_on_disconnect` machinery a real network drop already
//!   uses in this codebase.
use super::lastseen::is_valid_lookup_name;
use super::*;

/// C `struct punishment` (`src/system/punish.h:20-25`) - the exact byte
/// shape this codebase's `kind = 1` `notes` rows store (see
/// [`encode_punishment_note`]/[`decode_punishment_note`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunishmentNote {
    pub level: i32,
    pub exp: i32,
    pub karma: i32,
    pub reason: String,
}

/// C `add_note(co->ID, 1, pID, ...)` (`punish.c:106`): note kind `1` is
/// the only kind this codebase's `notes` table currently stores or reads
/// (see `crates/ugaris-db/src/notes.rs`'s module doc comment).
pub const PUNISHMENT_NOTE_KIND: i16 = 1;

/// C `struct punishment`'s in-memory layout: three little-endian `i32`s
/// (`level`, `exp`, `karma`) then a fixed 80-byte, NUL-padded `reason`
/// buffer - no compiler padding is needed since every field before the
/// trailing array is already 4-byte aligned.
pub fn encode_punishment_note(note: &PunishmentNote) -> Vec<u8> {
    let mut buf = Vec::with_capacity(92);
    buf.extend_from_slice(&note.level.to_le_bytes());
    buf.extend_from_slice(&note.exp.to_le_bytes());
    buf.extend_from_slice(&note.karma.to_le_bytes());
    let mut reason_bytes = [0u8; 80];
    let source = note.reason.as_bytes();
    let len = source.len().min(79);
    reason_bytes[..len].copy_from_slice(&source[..len]);
    buf.extend_from_slice(&reason_bytes);
    buf
}

/// Inverse of [`encode_punishment_note`]. Returns `None` for a
/// too-short/malformed blob (this codebase never reads back a `notes`
/// row it didn't write itself, but a malformed row should not panic the
/// `/unpunish` handler).
pub fn decode_punishment_note(bytes: &[u8]) -> Option<PunishmentNote> {
    if bytes.len() < 92 {
        return None;
    }
    let level = i32::from_le_bytes(bytes[0..4].try_into().ok()?);
    let exp = i32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let karma = i32::from_le_bytes(bytes[8..12].try_into().ok()?);
    let reason_bytes = &bytes[12..92];
    let nul = reason_bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(reason_bytes.len());
    let reason = String::from_utf8_lossy(&reason_bytes[..nul]).to_string();
    Some(PunishmentNote {
        level,
        exp,
        karma,
        reason,
    })
}

/// C `death_loss` (`src/system/death.c:487-489`): exp lost due to death,
/// reused verbatim by `punish()`'s per-level exp-loss formula below (a
/// separate copy from `world/death.rs`'s own death-handling exp-loss
/// arithmetic, which layers additional taper logic on top of a
/// differently-shaped caller and is not this same C function).
fn death_loss(total_exp: u32) -> u32 {
    total_exp / 25
}

/// C `punish`'s per-level `(exp, karma)` switch (`punish.c:56-89`).
fn punishment_losses(level: u8, current_exp: u32) -> (u32, i32) {
    match level {
        0 => (0, 0),
        1 => (death_loss(current_exp).div_ceil(4), 1),
        2 => (death_loss(current_exp).div_ceil(2), 2),
        3 => (death_loss(current_exp), 4),
        4 => (death_loss(current_exp) * 2, 6),
        5 => (death_loss(current_exp) * 4, 8),
        6 => (0, 12),
        _ => (0, 0),
    }
}

/// Result of applying a punishment mutation - same shape whether
/// `character` was a live `World::characters` entry or a freshly loaded
/// DB snapshot, see the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PunishmentOutcome {
    pub exp_loss: u32,
    pub karma_loss: i32,
    /// C `plock == 1` (`punish.c:99-101`): `karma <= -12`, unconditional
    /// on `CF_PAID`.
    pub lock: bool,
    /// C `pkick == 1` (`punish.c:102-104`): unpaid *and* `karma <= -5`.
    pub kick: bool,
}

/// C `punish()`'s in-place mutation half (`punish.c:41-105`), minus
/// `add_note`'s DB write - see the module doc comment for why that's
/// performed separately by the caller.
pub fn apply_punishment(character: &mut Character, level: u8) -> PunishmentOutcome {
    let (exp_loss, karma_loss) = punishment_losses(level, character.exp);
    character.exp = character.exp.saturating_sub(exp_loss);
    character.karma -= karma_loss;
    let lock = character.karma <= -12;
    let kick = !character.flags.contains(CharacterFlags::PAID) && character.karma <= -5;
    PunishmentOutcome {
        exp_loss,
        karma_loss,
        lock,
        kick,
    }
}

/// C `unpunish()`'s in-place mutation half (`punish.c:109-131`), minus
/// `db_unpunish`'s DB read+delete - see `ugaris-server`'s
/// `apply_unpunish_events` (via `NotesRepository::take_note`).
pub fn apply_unpunishment(character: &mut Character, note: &PunishmentNote) {
    character.exp = character.exp.saturating_add(note.exp.max(0) as u32);
    character.karma += note.karma;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunishRequest {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub level: u8,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnpunishRequest {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub note_id: i64,
}

impl World {
    /// C `set_task`'s online scan (`task.c:222-231`, the same shape
    /// `world/admin_flag.rs`'s `cmd_flag` port already established for
    /// its own online branch): every currently loaded character (no
    /// `CF_PLAYER` filter), by exact case-insensitive name match. A
    /// separate copy rather than a shared helper (that one is private to
    /// its own module), matching this codebase's own precedent of small
    /// per-module duplication over premature abstraction.
    fn find_loaded_punish_target_by_name(&self, name: &str) -> Option<CharacterId> {
        self.characters
            .values()
            .find(|character| character.name.eq_ignore_ascii_case(name))
            .map(|character| character.id)
    }

    /// C `cmd_punish` (`command.c:2354-2406`). `target_name` is the
    /// already-parsed alphabetic name token; `reason` is the remaining
    /// text (C's raw-byte copy loop, capped at 79 characters);
    /// `reason_overflowed` is `true` when the original input had more
    /// than 79 non-consumed bytes left over (C's `*ptr` truthy check,
    /// `command.c:2388`) - the caller (`ugaris-server::commands_admin`)
    /// computes this from the raw remaining string length since it needs
    /// the pre-truncation length, which this method's `&str` parameter
    /// alone cannot distinguish from an exact 79-byte reason.
    ///
    /// Validation order matches C exactly: name-shape (C's synchronous
    /// `lookup_name == -1` case) first, then reason-too-short, then
    /// reason-too-long, then level-out-of-bounds - all before ever
    /// queuing anything, since none of these depend on the (deferred)
    /// DB resolution.
    pub fn queue_punish_command(
        &mut self,
        caller_id: CharacterId,
        target_name: &str,
        level: i32,
        reason: &str,
        reason_overflowed: bool,
    ) -> Vec<String> {
        if !is_valid_lookup_name(target_name) {
            return vec![format!("Sorry, no player by the name {target_name}.")];
        }
        if reason.len() < 5 {
            return vec![format!("Sorry, the reason {reason} is too short.")];
        }
        if reason_overflowed {
            return vec!["Sorry, the reason is too long.".to_string()];
        }
        if !(0..=6).contains(&level) {
            return vec!["Sorry, the level is out of bounds (0-6).".to_string()];
        }
        self.pending_punish_requests.push(PunishRequest {
            caller_id,
            target_name: target_name.to_string(),
            level: level as u8,
            reason: reason.to_string(),
        });
        Vec::new()
    }

    pub fn drain_pending_punish_requests(&mut self) -> Vec<PunishRequest> {
        self.pending_punish_requests.drain(..).collect()
    }

    /// C `cmd_unpunish` (`command.c:2706-2731`). Unlike `/punish`, C's
    /// own "UnPunishment scheduled." acknowledgement is sent once `uID`
    /// resolves (`command.c:2729`, unconditional on the task's eventual
    /// success, matching `task_set_flags`'s "Update scheduled." fire-and-
    /// forget shape) - deferred here to `ugaris-server`'s
    /// `apply_unpunish_events` since this codebase's DB resolution is
    /// itself deferred (see the module doc comment).
    pub fn queue_unpunish_command(
        &mut self,
        caller_id: CharacterId,
        target_name: &str,
        note_id: i64,
    ) -> Vec<String> {
        if !is_valid_lookup_name(target_name) {
            return vec![format!("Sorry, no player by the name {target_name}.")];
        }
        self.pending_unpunish_requests.push(UnpunishRequest {
            caller_id,
            target_name: target_name.to_string(),
            note_id,
        });
        Vec::new()
    }

    pub fn drain_pending_unpunish_requests(&mut self) -> Vec<UnpunishRequest> {
        self.pending_unpunish_requests.drain(..).collect()
    }

    /// Exposed for `ugaris-server::world_events::apply_punish_events`/
    /// `apply_unpunish_events`'s online branch - see
    /// [`Self::find_loaded_punish_target_by_name`]'s doc comment.
    pub fn find_punish_target_online(&self, name: &str) -> Option<CharacterId> {
        self.find_loaded_punish_target_by_name(name)
    }
}
