use super::*;

#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub name: String,
    pub password: String,
    pub vendor: u32,
    pub unique: u32,
    pub ip: u32,
    pub area_id: i32,
    pub mirror_id: i32,
    pub no_login: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Waiting,
    Ready {
        character_id: CharacterId,
        character_number: u32,
        mirror: i32,
        unique: u32,
        /// `login_sessions.id` of the row just inserted for this login, for
        /// linking an anti-cheat session (`AntiCheatSessionCreate::
        /// login_session_id`) back to it (C's `ac_player_login` links
        /// against the same login-tracking record via the character/socket
        /// it holds, not a shared row id, but this codebase's `login_
        /// sessions` table gives a natural foreign key instead).
        login_session_id: i64,
        /// The account (subscriber) id, needed by the anti-cheat session
        /// bridge (C `get_subscriberId_from_character`) without a second
        /// query.
        account_id: i64,
    },
    NewArea {
        character_id: CharacterId,
        area_id: i32,
        mirror: i32,
        unique: u32,
    },
    InternalError,
    Locked,
    WrongPassword,
    Duplicate,
    NotPaid,
    Shutdown,
    IpLocked,
    AccountNotFixed,
    TooManyBadPasswords,
}

impl LoginOutcome {
    pub fn legacy_find_login_code(&self) -> i32 {
        match self {
            Self::Waiting => 0,
            Self::Ready { .. } | Self::NewArea { .. } => 1,
            Self::InternalError => -1,
            Self::Locked => -2,
            Self::WrongPassword => -3,
            Self::Duplicate => -4,
            Self::NotPaid => -5,
            Self::Shutdown => -6,
            Self::IpLocked => -7,
            Self::AccountNotFixed => -8,
            Self::TooManyBadPasswords => -9,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterSummary {
    pub id: CharacterId,
    pub name: String,
    pub area_id: i32,
    pub mirror_id: i32,
}

/// See [`CharacterRepository::find_paid_until_info`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaidUntilInfo {
    /// `accounts.paid_until` as a unix timestamp; `None` for SQL NULL
    /// (C's `subscriber.paid_till` column being unset, read as `0`).
    pub raw_paid_until_unix: Option<i64>,
    /// `accounts.created_at` as a unix timestamp (C's `subscriber.
    /// creation_time`).
    pub account_created_at_unix: i64,
}

/// C `db_lastseen`'s `charinfo` row shape (`database_notes.c:352-390`):
/// the properly-capitalized name, whether the row's `class` carries
/// `CF_GOD` (staff never gets an elapsed-time readout), and the most
/// recent of `login_time`/`logout_time`/`created_at` as a unix
/// timestamp (C's own `last_activity = max(login_time, logout_time,
/// creation_time)` chain, computed here in SQL via `greatest` semantics
/// so no chrono/timezone dependency is needed in Rust - see
/// `ugaris-server`'s `apply_lastseen_events` doc comment for the caller
/// side of this).
#[derive(Debug, Clone)]
pub struct LastSeenInfo {
    pub name: String,
    pub is_god: bool,
    pub last_activity_unix: i64,
}

/// Snapshot returned by [`PgCharacterRepository::query_stats`] - see
/// `CharacterQueryCounters`'s field docs for the exact C counter each
/// value mirrors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterQueryStats {
    pub save_char_cnt: u64,
    pub exit_char_cnt: u64,
    pub load_char_cnt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterSaveMode {
    Backup {
        expected_current_area: i32,
        expected_current_mirror: i32,
        mirror: i32,
    },
    Logout {
        expected_current_area: i32,
        expected_current_mirror: i32,
        allowed_area: i32,
        mirror: i32,
    },
}

#[derive(Debug, Clone)]
pub struct CharacterSaveRequest {
    pub character: Character,
    pub items: Vec<Item>,
    /// Typed serde state document (see migration 0020): the sole
    /// per-player persistence write target now. The `ppd_blob`/
    /// `subscriber_blob` columns are frozen (no longer written by any save
    /// path - see the "Retire legacy blob writes" `PORTING_TODO.md` task);
    /// [`CharacterSnapshot`] still reads them as a fallback for rows saved
    /// before migration 0020 existed.
    pub player_state_json: Option<serde_json::Value>,
    pub mode: CharacterSaveMode,
}

#[derive(Debug, Clone)]
pub struct CharacterSnapshot {
    pub character: Character,
    pub items: Vec<Item>,
    pub ppd_blob: Vec<u8>,
    pub subscriber_blob: Vec<u8>,
    pub player_state_json: Option<serde_json::Value>,
    pub current_area: i32,
    pub current_mirror: i32,
    pub allowed_area: i32,
    pub mirror: i32,
}

#[async_trait]
pub trait CharacterRepository: Send + Sync {
    async fn find_login_target(&self, name: &str) -> anyhow::Result<Option<CharacterSummary>>;
    async fn find_last_seen(&self, name: &str) -> anyhow::Result<Option<LastSeenInfo>>;
    async fn begin_login(&self, request: LoginRequest) -> anyhow::Result<LoginOutcome>;
    async fn save_character_snapshot(&self, request: CharacterSaveRequest) -> anyhow::Result<bool>;
    async fn load_character_snapshot(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Option<CharacterSnapshot>>;
    async fn release_character(&self, character_id: CharacterId) -> anyhow::Result<()>;
    /// C `db_rename` (`database_admin.c:296-355`): `/rename`'s DB half.
    /// Renames the `characters` row matching `from` (case-insensitively,
    /// like every other by-name lookup in this file) to `to`, returning
    /// whether a row was actually touched - see `ugaris-server`'s
    /// `world_events.rs::apply_rename_events` for the three-way reply
    /// this return value (plus a query `Err`) drives. Note this is a
    /// straight `characters` table mutation, not a character-specific
    /// operation gated on a live `Character`/`CharacterId` - it lives on
    /// this trait purely to reuse the already-threaded `PgPool`/
    /// `Option<PgCharacterRepository>` plumbing (`ugaris-server`'s
    /// `main.rs`), matching this file's own `query_stats()` precedent of
    /// non-per-character methods sharing the same repository.
    async fn rename_character(&self, from: &str, to: &str) -> anyhow::Result<bool>;
    /// C `db_lockname` (`database_admin.c:365-398`): `/lockname`'s DB
    /// half. Inserts `name` (already lowercased/alpha-validated by
    /// `ugaris-core`'s `world/lockname.rs`) into `locked_names`,
    /// returning whether a new row was actually inserted (`false` when
    /// the name is already locked, mirroring C's `affected_rows == 0`
    /// case) - see `migrations/0012_locked_names.sql` for why this
    /// table has no other consumer in this codebase yet.
    async fn lock_name(&self, name: &str) -> anyhow::Result<bool>;
    /// C `db_unlockname` (`database_admin.c:436-467`): `/unlockname`'s DB
    /// half, the mirror image of [`Self::lock_name`].
    async fn unlock_name(&self, name: &str) -> anyhow::Result<bool>;
    /// C `set_task`'s `chr`-row `locked` column write (`task.c:262-267`),
    /// the account-lock half of `/punish`/`/unpunish` (`ugaris-core`'s
    /// `world/punish.rs`): C's own SQL builds the assigned value as one of
    /// `'Y'`/`'N'`/the literal column name `locked` (a self-assignment
    /// no-op) depending on whether `plock` is `1`/`-1`/`0` - modeled here
    /// as a plain boolean setter, only ever called for the two real
    /// (non-no-op) cases. Lives on this trait for the same reason
    /// [`Self::rename_character`] does: a straight `characters` table
    /// mutation that isn't part of the `character_json`/`ppd_blob`
    /// snapshot `save_character_snapshot` writes, reusing the
    /// already-threaded `PgPool`/`Option<PgCharacterRepository>` plumbing
    /// rather than adding a whole new repository for one column.
    async fn set_character_locked(
        &self,
        character_id: CharacterId,
        locked: bool,
    ) -> anyhow::Result<()>;
    /// C `db_lookup_id`/`lookup_ID` (`src/system/database/database_lookup.c:
    /// 20-48` + `src/system/lookup.c:98-135`): reverse ID->name
    /// resolution, used by `/look`'s note-creator-name display and
    /// `/klog`'s karmalog target/creator names (`list_punishment`/
    /// `karmalog_s`, `src/system/punish.c:26-38` + `database_notes.c:
    /// 227-244`). C caches this behind a 4096-entry, 1-hour LRU
    /// (`lookup.c`'s `MAXLOOK`/`lookup_ID`); this codebase has no such
    /// in-memory cache (every lookup here is already a per-tick deferred
    /// DB round trip, see `world/lastseen.rs`'s module doc comment), so
    /// this queries `characters` directly on every call. Returns `None`
    /// for an unknown id, matching C's `lookup_ID` returning `-1` with no
    /// row found (this codebase has no analogue of C's `"*unknown*"`/
    /// `"**deleted**"` placeholder strings - the caller substitutes its
    /// own fallback text).
    async fn find_name_by_id(&self, character_id: CharacterId) -> anyhow::Result<Option<String>>;
    /// C `load_char_pwd`'s two SQL-sourced inputs to the paid-account
    /// expiration computation (`database_character.c:626-668`): the raw
    /// `subscriber.paid_till`/`subscriber.creation_time` columns, here
    /// `accounts.paid_until`/`accounts.created_at` joined by the
    /// requested character's `account_id` (same join shape as
    /// `BEGIN_LOGIN_SQL`). Keyed by character id (not name) since every
    /// caller already has a resolved `CharacterSummary`/live `Character`
    /// (`/values`, C's `look_values_bg`, `tool.c:2903-2911` - see
    /// `ugaris-core`'s `world::values::compute_paid_till` for what this
    /// feeds). Returns `None` only if the character row itself (or its
    /// `account_id` join) doesn't exist; a null `paid_until` column
    /// surfaces as `PaidUntilInfo::raw_paid_until_unix: None` (C's
    /// `row[2] ? atoi(row[2]) : 0` "never paid" case), not as an outer
    /// `None`.
    async fn find_paid_until_info(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Option<PaidUntilInfo>>;
    /// C `exterminate`/`db_exterminate` (`database_admin.c:29-95,503-507`):
    /// `/exterminate <name>`'s DB half. Resolves `name` (case-insensitive,
    /// like every other by-name lookup in this file) to its owning
    /// account, locks that account (C's `subscriber.locked = 'Y'`, already
    /// enforced by `begin_login_tx`'s `account_locked` gate) and bans
    /// every distinct IP that account has ever logged in from (C's
    /// `INSERT ipban SELECT ip FROM iplog WHERE sID = ...`, this
    /// codebase's `login_sessions.ip_address` history standing in for
    /// `iplog` - see `migrations/0019_ip_bans.sql`). Returns `None` when
    /// no character has that name (C's "Player '%s' not found." branch);
    /// `Some(ExterminateOutcome)` otherwise, whose counts drive the
    /// "Locked %d accounts and %d IP addresses." reply.
    async fn exterminate_account(&self, name: &str) -> anyhow::Result<Option<ExterminateOutcome>>;
    /// See the "Retire legacy blob writes" `PORTING_TODO.md` task: finds
    /// every `characters` row that predates migration 0020 (`player_state_
    /// json is null`) but still carries decodable legacy data (`ppd_blob`/
    /// `subscriber_blob` non-empty), for a one-off startup backfill that
    /// decodes each row through the legacy `#[deprecated]` decoders and
    /// writes the typed document back via [`Self::backfill_player_state_
    /// json`]. Decoding itself can't happen in this crate (the decoders
    /// live in `ugaris-server`, next to `PlayerRuntime`); this only does
    /// the raw row scan.
    async fn find_legacy_blob_only_characters(&self) -> anyhow::Result<Vec<LegacyBlobRow>>;
    /// Writes the decoded `player_state_json` document back for one row
    /// produced by [`Self::find_legacy_blob_only_characters`]. Guarded by
    /// `player_state_json is null` so this can never clobber a document a
    /// live session already saved since the row was read (this runs once
    /// at startup before any session exists, so that should never happen
    /// in practice - the guard is defense in depth, not a real race).
    async fn backfill_player_state_json(
        &self,
        character_id: CharacterId,
        player_state_json: serde_json::Value,
    ) -> anyhow::Result<()>;
}

/// See [`CharacterRepository::exterminate_account`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExterminateOutcome {
    pub locked_accounts: u64,
    pub banned_ips: u64,
}

/// See [`CharacterRepository::find_legacy_blob_only_characters`].
#[derive(Debug, Clone)]
pub struct LegacyBlobRow {
    pub character_id: CharacterId,
    pub ppd_blob: Vec<u8>,
    pub subscriber_blob: Vec<u8>,
}
