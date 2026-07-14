use super::*;

/// Scoped-down port of C's `/querystats` counters (`command.c:6588-6618`)
/// - see `ugaris-core`'s `world/querystats.rs` module doc comment for
///   exactly which C globals are (and aren't) tracked here and why. Three
///   atomics rather than plain `u64` fields since `PgCharacterRepository`'s
///   methods all take `&self` (shared across every clone via the `Arc`
///   below, matching every other `Pg*Repository`'s cheap-clone-shares-pool
///   convention).
#[derive(Debug, Default)]
pub(super) struct CharacterQueryCounters {
    /// C `save_char_cnt` (`database_character.c:221`), incremented inside
    /// `save_char`'s `area_number <= 0` ("just a data backup") branch -
    /// maps onto `save_character_snapshot`'s `CharacterSaveMode::Backup`.
    pub(super) save_char_cnt: AtomicU64,
    /// C `exit_char_cnt` (`database_character.c:243`), incremented inside
    /// `save_char`'s `area_number > 0` ("logout") branch - maps onto
    /// `save_character_snapshot`'s `CharacterSaveMode::Logout`.
    pub(super) exit_char_cnt: AtomicU64,
    /// C `load_char_cnt` (`database_character.c:1102`), incremented right
    /// before `load_char`'s "mark character as online" `UPDATE chars ...`
    /// query - maps onto `begin_login`'s equivalent `update characters
    /// set current_area = ...` query on the `LoginOutcome::Ready` path
    /// (the only path that runs it).
    pub(super) load_char_cnt: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct PgCharacterRepository {
    pool: PgPool,
    pub(super) query_counters: Arc<CharacterQueryCounters>,
}

impl PgCharacterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            query_counters: Arc::new(CharacterQueryCounters::default()),
        }
    }

    /// C `/querystats`'s scoped-down Rust equivalent - see
    /// `CharacterQueryStats`'s field docs and `ugaris-core`'s
    /// `world/querystats.rs` module doc comment.
    pub fn query_stats(&self) -> CharacterQueryStats {
        CharacterQueryStats {
            save_char_cnt: self.query_counters.save_char_cnt.load(Ordering::Relaxed),
            exit_char_cnt: self.query_counters.exit_char_cnt.load(Ordering::Relaxed),
            load_char_cnt: self.query_counters.load_char_cnt.load(Ordering::Relaxed),
        }
    }
}

#[async_trait]
impl CharacterRepository for PgCharacterRepository {
    async fn find_login_target(&self, name: &str) -> anyhow::Result<Option<CharacterSummary>> {
        let row = sqlx::query_as::<_, (i64, String, i32, i32)>(
            "select id, name, allowed_area, mirror from characters where lower(name) = lower($1)",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, name, area_id, mirror_id)| CharacterSummary {
            id: CharacterId(id as u32),
            name,
            area_id,
            mirror_id,
        }))
    }

    async fn find_last_seen(&self, name: &str) -> anyhow::Result<Option<LastSeenInfo>> {
        let row = sqlx::query_as::<_, (String, String, i64, i64, i64)>(FIND_LAST_SEEN_SQL)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(
            row.map(|(name, flags_bits, login_time, logout_time, created_at)| {
                let flags = CharacterFlags::from_bits_truncate(flags_bits.parse().unwrap_or(0));
                LastSeenInfo {
                    name,
                    is_god: flags.contains(CharacterFlags::GOD),
                    last_activity_unix: login_time.max(logout_time).max(created_at),
                }
            }),
        )
    }

    async fn begin_login(&self, request: LoginRequest) -> anyhow::Result<LoginOutcome> {
        if !request.name.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Ok(LoginOutcome::WrongPassword);
        }
        if request.no_login {
            return Ok(LoginOutcome::Shutdown);
        }
        // Matches C `load_char`'s `is_badpass_ip(login.ip)` guard
        // (`database_character.c:781-786`), checked before the row lookup
        // even begins.
        if is_ip_rate_limited(&self.pool, request.ip).await? {
            return Ok(LoginOutcome::TooManyBadPasswords);
        }

        let mut tx = self.pool.begin().await?;
        let outcome = begin_login_tx(&mut tx, request).await?;
        tx.commit().await?;
        // C's `load_char_cnt` increments right before the "mark character
        // as online" `UPDATE chars ...` query inside `load_char`
        // (`database_character.c:1099-1102`) - `begin_login_tx`'s
        // equivalent `update characters set current_area = ...` query
        // only ever runs on the `Ready` path (every other outcome returns
        // earlier without touching that row), so gating the increment on
        // `Ready` here reproduces the same "one increment per query
        // actually issued" behavior without threading the counter through
        // the free `begin_login_tx` function itself.
        if matches!(outcome, LoginOutcome::Ready { .. }) {
            self.query_counters
                .load_char_cnt
                .fetch_add(1, Ordering::Relaxed);
        }
        Ok(outcome)
    }

    async fn save_character_snapshot(&self, request: CharacterSaveRequest) -> anyhow::Result<bool> {
        // C increments `save_char_cnt`/`exit_char_cnt` unconditionally
        // right before issuing the query, regardless of whether it later
        // succeeds (`database_character.c:210-243`) - matched here by
        // incrementing before the transaction begins, not gated on
        // `saved` below.
        match request.mode {
            CharacterSaveMode::Backup { .. } => {
                self.query_counters
                    .save_char_cnt
                    .fetch_add(1, Ordering::Relaxed);
            }
            CharacterSaveMode::Logout { .. } => {
                self.query_counters
                    .exit_char_cnt
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        let mut tx = self.pool.begin().await?;
        let saved = save_character_snapshot_tx(&mut tx, &request).await?;
        if saved {
            replace_character_items_tx(&mut tx, &request.character, &request.items).await?;
            tx.commit().await?;
        }
        Ok(saved)
    }

    async fn load_character_snapshot(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Option<CharacterSnapshot>> {
        let Some((
            character,
            ppd_blob,
            subscriber_blob,
            player_state_json,
            current_area,
            current_mirror,
            allowed_area,
            mirror,
        )) = sqlx::query_as::<
            _,
            (
                Option<Json<Character>>,
                Vec<u8>,
                Vec<u8>,
                Option<serde_json::Value>,
                i32,
                i32,
                i32,
                i32,
            ),
        >(LOAD_CHARACTER_SNAPSHOT_SQL)
        .bind(character_id.0 as i64)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let Some(Json(character)) = character else {
            return Ok(None);
        };

        let item_rows = sqlx::query_as::<_, (Json<Item>,)>(LOAD_CHARACTER_ITEMS_SQL)
            .bind(character_id.0 as i64)
            .fetch_all(&self.pool)
            .await?;
        let items = item_rows.into_iter().map(|(Json(item),)| item).collect();

        Ok(Some(CharacterSnapshot {
            character,
            items,
            ppd_blob,
            subscriber_blob,
            player_state_json,
            current_area,
            current_mirror,
            allowed_area,
            mirror,
        }))
    }

    async fn release_character(&self, character_id: CharacterId) -> anyhow::Result<()> {
        sqlx::query("update characters set current_area = 0, current_mirror = 0 where id = $1")
            .bind(character_id.0 as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn rename_character(&self, from: &str, to: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("update characters set name = $1 where lower(name) = lower($2)")
            .bind(to)
            .bind(from)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn lock_name(&self, name: &str) -> anyhow::Result<bool> {
        let result =
            sqlx::query("insert into locked_names(name) values ($1) on conflict (name) do nothing")
                .bind(name)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn unlock_name(&self, name: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("delete from locked_names where name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_character_locked(
        &self,
        character_id: CharacterId,
        locked: bool,
    ) -> anyhow::Result<()> {
        sqlx::query("update characters set locked = $1 where id = $2")
            .bind(locked)
            .bind(character_id.0 as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_name_by_id(&self, character_id: CharacterId) -> anyhow::Result<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(FIND_NAME_BY_ID_SQL)
            .bind(character_id.0 as i64)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(name,)| name))
    }

    async fn find_paid_until_info(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Option<PaidUntilInfo>> {
        let row = sqlx::query_as::<_, (Option<i64>, i64)>(FIND_PAID_UNTIL_INFO_SQL)
            .bind(character_id.0 as i64)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(
            |(raw_paid_until_unix, account_created_at_unix)| PaidUntilInfo {
                raw_paid_until_unix,
                account_created_at_unix,
            },
        ))
    }

    async fn exterminate_account(&self, name: &str) -> anyhow::Result<Option<ExterminateOutcome>> {
        let mut tx = self.pool.begin().await?;
        let account_id: Option<(i64,)> =
            sqlx::query_as("select account_id from characters where lower(name) = lower($1)")
                .bind(name)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((account_id,)) = account_id else {
            return Ok(None);
        };

        let locked = sqlx::query("update accounts set locked = true where id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        // C's own `INSERT ipban SELECT ... FROM iplog` inserts one row
        // per matching log entry (duplicates included); `distinct` here
        // avoids piling up redundant rows for a repeat visitor from the
        // same address, a deliberate simplification (documented, not
        // silent) since this codebase has no other consumer counting
        // `ip_bans` rows the way C's own admin tooling might count
        // `ipban` rows.
        let banned_ips = sqlx::query(
            "insert into ip_bans(ip, banned_until) \
             select distinct ip_address, now() + interval '28 days' \
             from login_sessions where account_id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;
        Ok(Some(ExterminateOutcome {
            locked_accounts: locked,
            banned_ips,
        }))
    }

    async fn find_legacy_blob_only_characters(&self) -> anyhow::Result<Vec<LegacyBlobRow>> {
        let rows = sqlx::query_as::<_, (i64, Vec<u8>, Vec<u8>)>(FIND_LEGACY_BLOB_ONLY_SQL)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(id, ppd_blob, subscriber_blob)| LegacyBlobRow {
                character_id: CharacterId(id as u32),
                ppd_blob,
                subscriber_blob,
            })
            .collect())
    }

    async fn backfill_player_state_json(
        &self,
        character_id: CharacterId,
        player_state_json: serde_json::Value,
    ) -> anyhow::Result<()> {
        sqlx::query(BACKFILL_PLAYER_STATE_JSON_SQL)
            .bind(player_state_json)
            .bind(character_id.0 as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub(super) const FIND_NAME_BY_ID_SQL: &str = "select name from characters where id = $1";

/// See [`CharacterRepository::find_legacy_blob_only_characters`].
pub(super) const FIND_LEGACY_BLOB_ONLY_SQL: &str =
    "select id, ppd_blob, subscriber_blob from characters \
where player_state_json is null and (ppd_blob <> ''::bytea or subscriber_blob <> ''::bytea)";

/// See [`CharacterRepository::backfill_player_state_json`].
pub(super) const BACKFILL_PLAYER_STATE_JSON_SQL: &str =
    "update characters set player_state_json = $1 where id = $2 and player_state_json is null";

/// See [`CharacterRepository::find_paid_until_info`].
pub(super) const FIND_PAID_UNTIL_INFO_SQL: &str =
    "select extract(epoch from a.paid_until)::bigint, \
extract(epoch from a.created_at)::bigint \
from characters c join accounts a on a.id = c.account_id where c.id = $1";

// C: ppd_blob/subscriber_blob are intentionally absent from this SET list -
// migration 0020's player_state_json is the sole write target now (see the
// "Retire legacy blob writes" PORTING_TODO.md task); the columns are frozen
// at whatever value they held before the retirement and remain readable as
// a fallback (LOAD_CHARACTER_SNAPSHOT_SQL) for rows saved before 0020.
pub(super) const SAVE_CHARACTER_BACKUP_SQL: &str = "update characters set \
name = $1, description = $2, flags_bits = $3, speed_mode = $4, \
x = $5, y = $6, rest_area = $7, rest_x = $8, rest_y = $9, tox = $10, toy = $11, \
dir = $12, action = $13, duration = $14, step = $15, act1 = $16, act2 = $17, \
hp = $18, mana = $19, endurance = $20, lifeshield = $21, level = $22, exp = $23, \
exp_used = $24, gold = $25, cursor_item_id = $26, current_container_item_id = $27, \
values_json = $28, professions_json = $29, inventory_json = $30, \
character_json = $31, player_state_json = coalesce($32, player_state_json), \
mirror = $33, updated_at = now() \
where id = $34 and current_area = $35 and current_mirror = $36";

pub(super) const SAVE_CHARACTER_LOGOUT_SQL: &str = "update characters set \
name = $1, description = $2, flags_bits = $3, speed_mode = $4, \
x = $5, y = $6, rest_area = $7, rest_x = $8, rest_y = $9, tox = $10, toy = $11, \
dir = $12, action = $13, duration = $14, step = $15, act1 = $16, act2 = $17, \
hp = $18, mana = $19, endurance = $20, lifeshield = $21, level = $22, exp = $23, \
exp_used = $24, gold = $25, cursor_item_id = $26, current_container_item_id = $27, \
values_json = $28, professions_json = $29, inventory_json = $30, \
character_json = $31, player_state_json = coalesce($32, player_state_json), \
mirror = $33, \
current_area = 0, current_mirror = 0, allowed_area = $34, logout_time = now(), updated_at = now() \
where id = $35 and current_area = $36 and current_mirror = $37";

pub(super) const INSERT_CHARACTER_ITEM_SQL: &str = "insert into character_items(\
character_id, item_id, inventory_slot, is_cursor, item_json, name, description, flags_bits, \
sprite, item_value, min_level, max_level, needs_class, owner_id, modifier_index, modifier_value, \
x, y, carried_by, contained_in, content_id, driver, driver_data, serial, updated_at) \
values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, \
$16, $17, $18, $19, $20, $21, $22, $23, $24, now())";

pub(super) const LOAD_CHARACTER_SNAPSHOT_SQL: &str =
    "select character_json, ppd_blob, subscriber_blob, \
player_state_json, current_area, current_mirror, allowed_area, mirror \
from characters where id = $1";

/// C `db_lastseen` (`database_notes.c:352-390`): `login_time`/
/// `logout_time` are nullable (never-logged-in-since-this-column-existed
/// rows), coalesced to `0` matching C's plain `int` columns defaulting to
/// `0` rather than NULL; `created_at` always exists.
pub(super) const FIND_LAST_SEEN_SQL: &str = "select name, flags_bits, \
coalesce(extract(epoch from login_time)::bigint, 0), \
coalesce(extract(epoch from logout_time)::bigint, 0), \
extract(epoch from created_at)::bigint \
from characters where lower(name) = lower($1)";

pub(super) const LOAD_CHARACTER_ITEMS_SQL: &str = "select item_json from character_items \
where character_id = $1 order by coalesce(inventory_slot, 2147483647), \
case when is_cursor then 1 else 0 end, item_id";

pub(super) async fn save_character_snapshot_tx(
    tx: &mut Transaction<'_, Postgres>,
    request: &CharacterSaveRequest,
) -> anyhow::Result<bool> {
    let result = match request.mode {
        CharacterSaveMode::Backup {
            expected_current_area,
            expected_current_mirror,
            mirror,
        } => {
            bind_character_snapshot(sqlx::query(SAVE_CHARACTER_BACKUP_SQL), request)?
                .bind(mirror)
                .bind(request.character.id.0 as i64)
                .bind(expected_current_area)
                .bind(expected_current_mirror)
                .execute(&mut **tx)
                .await?
        }
        CharacterSaveMode::Logout {
            expected_current_area,
            expected_current_mirror,
            allowed_area,
            mirror,
        } => {
            bind_character_snapshot(sqlx::query(SAVE_CHARACTER_LOGOUT_SQL), request)?
                .bind(mirror)
                .bind(allowed_area)
                .bind(request.character.id.0 as i64)
                .bind(expected_current_area)
                .bind(expected_current_mirror)
                .execute(&mut **tx)
                .await?
        }
    };

    Ok(result.rows_affected() == 1)
}

pub(super) fn bind_character_snapshot<'q>(
    query: Query<'q, Postgres, PgArguments>,
    request: &CharacterSaveRequest,
) -> anyhow::Result<Query<'q, Postgres, PgArguments>> {
    let character = &request.character;
    Ok(query
        .bind(character.name.clone())
        .bind(character.description.clone())
        .bind(character.flags.bits().to_string())
        .bind(character.speed_mode as i32)
        .bind(character.x as i32)
        .bind(character.y as i32)
        .bind(character.rest_area as i32)
        .bind(character.rest_x as i32)
        .bind(character.rest_y as i32)
        .bind(character.tox as i32)
        .bind(character.toy as i32)
        .bind(character.dir as i32)
        .bind(character.action as i32)
        .bind(character.duration)
        .bind(character.step)
        .bind(character.act1)
        .bind(character.act2)
        .bind(character.hp)
        .bind(character.mana)
        .bind(character.endurance)
        .bind(character.lifeshield)
        .bind(character.level as i32)
        .bind(character.exp as i64)
        .bind(character.exp_used as i64)
        .bind(character.gold as i64)
        .bind(optional_item_id_to_i64(character.cursor_item))
        .bind(optional_item_id_to_i64(character.current_container))
        .bind(Json(character.values.clone()))
        .bind(Json(character.professions.clone()))
        .bind(Json(character.inventory.clone()))
        .bind(Json(character.clone()))
        .bind(request.player_state_json.clone()))
}

pub(super) async fn replace_character_items_tx(
    tx: &mut Transaction<'_, Postgres>,
    character: &Character,
    items: &[Item],
) -> anyhow::Result<()> {
    sqlx::query("delete from character_items where character_id = $1")
        .bind(character.id.0 as i64)
        .execute(&mut **tx)
        .await?;

    for (item, key) in character_item_storage_rows(character, items) {
        sqlx::query(INSERT_CHARACTER_ITEM_SQL)
            .bind(character.id.0 as i64)
            .bind(item.id.0 as i64)
            .bind(key.inventory_slot)
            .bind(key.is_cursor)
            .bind(Json(item.clone()))
            .bind(item.name.clone())
            .bind(item.description.clone())
            .bind(item.flags.bits().to_string())
            .bind(item.sprite)
            .bind(item.value as i64)
            .bind(item.min_level as i32)
            .bind(item.max_level as i32)
            .bind(item.needs_class as i32)
            .bind(item.owner_id)
            .bind(item.modifier_index.to_vec())
            .bind(item.modifier_value.to_vec())
            .bind(item.x as i32)
            .bind(item.y as i32)
            .bind(optional_character_id_to_i64(item.carried_by))
            .bind(optional_item_id_to_i64(item.contained_in))
            .bind(item.content_id as i32)
            .bind(item.driver as i32)
            .bind(item.driver_data.clone())
            .bind(item.serial as i64)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CharacterItemStorageKey {
    pub(super) item_id: ItemId,
    pub(super) inventory_slot: Option<i32>,
    pub(super) is_cursor: bool,
}

pub(super) fn character_item_storage_rows<'a>(
    character: &Character,
    items: &'a [Item],
) -> Vec<(&'a Item, CharacterItemStorageKey)> {
    let slots = inventory_item_slots(character);
    items
        .iter()
        .filter_map(|item| {
            let inventory_slot = slots.get(&item.id).copied();
            let is_cursor = character.cursor_item == Some(item.id);
            if inventory_slot.is_some() || is_cursor {
                Some((
                    item,
                    CharacterItemStorageKey {
                        item_id: item.id,
                        inventory_slot,
                        is_cursor,
                    },
                ))
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn inventory_item_slots(character: &Character) -> HashMap<ItemId, i32> {
    let mut slots = HashMap::new();
    for (slot, item_id) in character.inventory.iter().enumerate().take(INVENTORY_SIZE) {
        if let Some(item_id) = item_id {
            slots.entry(*item_id).or_insert(slot as i32);
        }
    }
    slots
}

pub(super) fn optional_character_id_to_i64(id: Option<CharacterId>) -> Option<i64> {
    id.map(|id| id.0 as i64)
}

pub(super) fn optional_item_id_to_i64(id: Option<ItemId>) -> Option<i64> {
    id.map(|id| id.0 as i64)
}

pub(super) async fn begin_login_tx(
    tx: &mut Transaction<'_, Postgres>,
    request: LoginRequest,
) -> anyhow::Result<LoginOutcome> {
    let row = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            String,
            String,
            bool,
            bool,
            bool,
            bool,
            Option<i32>,
            i32,
            i32,
            i32,
            i32,
        ),
    >(BEGIN_LOGIN_SQL)
    .bind(&request.name)
    .fetch_optional(&mut **tx)
    .await?;

    let Some((
        id,
        account_id,
        name,
        password_hash,
        character_locked,
        account_locked,
        ip_locked,
        fixed,
        paid_until,
        current_area,
        allowed_area,
        mirror,
        current_mirror,
    )) = row
    else {
        return Ok(LoginOutcome::WrongPassword);
    };

    if !legacy_password_matches(&request.password, &password_hash) {
        // Matches C `load_char_pwd` returning 1 (wrong password) ->
        // `login_passwd(); add_badpass_ip(login.ip);`
        // (`database_character.c:876-877`). An unknown character name never
        // reaches this branch (handled by the `row` being `None` above),
        // matching C's anti-enumeration behavior of only rate-limiting
        // actual wrong-password attempts against an existing account.
        record_bad_password_attempt(tx, request.ip).await?;
        return Ok(LoginOutcome::WrongPassword);
    }

    if character_locked || account_locked {
        return Ok(LoginOutcome::Locked);
    }
    // C `isbanned_iplog(login.ip) && (!row[5] || row[5][0] != 'N')`
    // (`database_character.c:660`): the real mechanism is keyed on the
    // *connecting* IP address, independent of which account is logging
    // in - `ip_locked` (the pre-existing static per-account flag) only
    // approximated this; `is_ip_banned` below checks the genuine
    // `ip_bans` table `/exterminate` populates (see that migration's doc
    // comment), so either gate rejects.
    if ip_locked || is_ip_banned(tx, request.ip).await? {
        return Ok(LoginOutcome::IpLocked);
    }
    if !fixed {
        return Ok(LoginOutcome::AccountNotFixed);
    }
    if paid_until.is_none() {
        return Ok(LoginOutcome::NotPaid);
    }
    // Matches C `load_char_dup` (`database_character.c:731-753`): another
    // character on the same account (`sID`) is already online
    // (`current_area != 0`) -> `login_dup()`. `account_id == 1` is exempt,
    // mirroring C's `if (sID == 1) return 1; // hack for easier testing`.
    if account_id != 1 {
        let duplicate_count: i64 = sqlx::query_scalar(BEGIN_LOGIN_TX_DUPLICATE_SQL)
            .bind(account_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
        if duplicate_count > 0 {
            return Ok(LoginOutcome::Duplicate);
        }
    }
    if allowed_area <= 0 {
        return Ok(LoginOutcome::InternalError);
    }

    let mirror = if mirror == 0 { 1 } else { mirror };
    if allowed_area != request.area_id {
        update_mirror_if_needed(tx, id, mirror).await?;
        return Ok(LoginOutcome::NewArea {
            character_id: CharacterId(id as u32),
            area_id: if current_area != 0 {
                current_area
            } else {
                allowed_area
            },
            mirror: if current_mirror != 0 {
                current_mirror
            } else {
                mirror
            },
            unique: request.unique,
        });
    }

    sqlx::query(
        "update characters set current_area = $1, current_mirror = $2, allowed_area = $1, login_time = now(), updated_at = now() where id = $3",
    )
    .bind(request.area_id)
    .bind(request.mirror_id)
    .bind(id)
    .execute(&mut **tx)
    .await?;

    let (login_session_id,) = sqlx::query_as::<_, (i64,)>(
        "insert into login_sessions(character_id, account_id, character_name, ip_address, area_id, mirror_id, client_vendor, unique_id) \
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning id",
    )
    .bind(id)
    .bind(account_id)
    .bind(name)
    .bind(request.ip as i32)
    .bind(request.area_id)
    .bind(request.mirror_id)
    .bind(request.vendor as i32)
    .bind(request.unique as i32)
    .fetch_one(&mut **tx)
    .await?;

    Ok(LoginOutcome::Ready {
        character_id: CharacterId(id as u32),
        character_number: 0,
        mirror,
        unique: request.unique,
        login_session_id,
        account_id,
    })
}

pub(super) fn legacy_password_matches(password: &str, stored_password: &str) -> bool {
    password == stored_password
}

pub(super) const IS_BADPASS_IP_SQL: &str = "select \
    count(*) filter (where created_at >= now() - interval '60 seconds'), \
    count(*) filter (where created_at >= now() - interval '3600 seconds'), \
    count(*) filter (where created_at >= now() - interval '86400 seconds') \
 from bad_passwords where ip = $1";

/// Matches C `is_badpass_ip` (`src/system/badip.c:56-72`): an IP is
/// rate-limited if it has more than 3 recorded bad-password attempts in
/// the last 60 seconds, more than 8 in the last hour, or more than 25 in
/// the last 24 hours.
pub(super) async fn is_ip_rate_limited(pool: &PgPool, ip: u32) -> anyhow::Result<bool> {
    let (recent_minute, recent_hour, recent_day): (i64, i64, i64) =
        sqlx::query_as(IS_BADPASS_IP_SQL)
            .bind(ip as i32)
            .fetch_one(pool)
            .await?;

    Ok(is_badpass_counts_rate_limited(
        recent_minute,
        recent_hour,
        recent_day,
    ))
}

/// Pure decision extracted from `is_ip_rate_limited` so the exact legacy
/// thresholds (`badip.c:59-70`: `>3` per minute, `>8` per hour, `>25` per
/// day) can be unit-tested without a live database.
pub(super) fn is_badpass_counts_rate_limited(
    recent_minute: i64,
    recent_hour: i64,
    recent_day: i64,
) -> bool {
    recent_minute > 3 || recent_hour > 8 || recent_day > 25
}

/// Matches C `isbanned_iplog` (`database_character.c:1821-1830`): an IP
/// is banned if it has an unexpired row in `ip_bans` (populated by
/// `/exterminate`, see `migrations/0019_ip_bans.sql`). C treats a query
/// failure as "assume banned" (`database_character.c:1828`); this codebase
/// instead surfaces the error via `?` like every other query here, so a
/// DB hiccup rejects the whole login attempt (`begin_login_tx` propagates
/// the error) rather than silently failing open.
pub(super) async fn is_ip_banned(
    tx: &mut Transaction<'_, Postgres>,
    ip: u32,
) -> anyhow::Result<bool> {
    let banned: bool = sqlx::query_scalar(
        "select exists(select 1 from ip_bans where ip = $1 and banned_until > now())",
    )
    .bind(ip as i32)
    .fetch_one(&mut **tx)
    .await?;
    Ok(banned)
}

/// Matches C `add_badpass_ip` (`src/system/badip.c:78-85`): records one
/// failed-password attempt for the given IP.
pub(super) async fn record_bad_password_attempt(
    tx: &mut Transaction<'_, Postgres>,
    ip: u32,
) -> anyhow::Result<()> {
    sqlx::query("insert into bad_passwords(ip) values ($1)")
        .bind(ip as i32)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) const BEGIN_LOGIN_SQL: &str = "select c.id, c.account_id, c.name, a.password_hash, c.locked, a.locked, a.ip_locked, a.fixed, \
         extract(epoch from a.paid_until)::int, c.current_area, c.allowed_area, c.mirror, c.current_mirror \
          from characters c join accounts a on a.id = c.account_id \
          where lower(c.name) = lower($1) for update";

/// Matches C `load_char_dup` (`database_character.c:731-753`): counts other
/// characters on the same account currently online (`current_area != 0`).
pub(super) const BEGIN_LOGIN_TX_DUPLICATE_SQL: &str =
    "select count(*) from characters where account_id = $1 and id != $2 and current_area != 0";

pub(super) async fn update_mirror_if_needed(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
    mirror: i32,
) -> anyhow::Result<()> {
    sqlx::query("update characters set mirror = $1 where id = $2 and mirror = 0")
        .bind(mirror)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
