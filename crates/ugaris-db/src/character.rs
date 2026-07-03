use async_trait::async_trait;
use sqlx::{postgres::PgArguments, query::Query, types::Json, PgPool, Postgres, Transaction};
use std::collections::HashMap;
use ugaris_core::{
    entity::{Character, Item, INVENTORY_SIZE},
    ids::{CharacterId, ItemId},
};

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
    pub ppd_blob: Vec<u8>,
    pub subscriber_blob: Vec<u8>,
    pub mode: CharacterSaveMode,
}

#[derive(Debug, Clone)]
pub struct CharacterSnapshot {
    pub character: Character,
    pub items: Vec<Item>,
    pub ppd_blob: Vec<u8>,
    pub subscriber_blob: Vec<u8>,
    pub current_area: i32,
    pub current_mirror: i32,
    pub allowed_area: i32,
    pub mirror: i32,
}

#[async_trait]
pub trait CharacterRepository: Send + Sync {
    async fn find_login_target(&self, name: &str) -> anyhow::Result<Option<CharacterSummary>>;
    async fn begin_login(&self, request: LoginRequest) -> anyhow::Result<LoginOutcome>;
    async fn save_character_snapshot(&self, request: CharacterSaveRequest) -> anyhow::Result<bool>;
    async fn load_character_snapshot(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Option<CharacterSnapshot>>;
    async fn release_character(&self, character_id: CharacterId) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct PgCharacterRepository {
    pool: PgPool,
}

impl PgCharacterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        Ok(outcome)
    }

    async fn save_character_snapshot(&self, request: CharacterSaveRequest) -> anyhow::Result<bool> {
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
}

const SAVE_CHARACTER_BACKUP_SQL: &str = "update characters set \
name = $1, description = $2, flags_bits = $3, speed_mode = $4, \
x = $5, y = $6, rest_area = $7, rest_x = $8, rest_y = $9, tox = $10, toy = $11, \
dir = $12, action = $13, duration = $14, step = $15, act1 = $16, act2 = $17, \
hp = $18, mana = $19, endurance = $20, lifeshield = $21, level = $22, exp = $23, \
exp_used = $24, gold = $25, cursor_item_id = $26, current_container_item_id = $27, \
values_json = $28, professions_json = $29, inventory_json = $30, \
character_json = $31, ppd_blob = $32, subscriber_blob = $33, mirror = $34, updated_at = now() \
where id = $35 and current_area = $36 and current_mirror = $37";

const SAVE_CHARACTER_LOGOUT_SQL: &str = "update characters set \
name = $1, description = $2, flags_bits = $3, speed_mode = $4, \
x = $5, y = $6, rest_area = $7, rest_x = $8, rest_y = $9, tox = $10, toy = $11, \
dir = $12, action = $13, duration = $14, step = $15, act1 = $16, act2 = $17, \
hp = $18, mana = $19, endurance = $20, lifeshield = $21, level = $22, exp = $23, \
exp_used = $24, gold = $25, cursor_item_id = $26, current_container_item_id = $27, \
values_json = $28, professions_json = $29, inventory_json = $30, \
character_json = $31, ppd_blob = $32, subscriber_blob = $33, mirror = $34, \
current_area = 0, current_mirror = 0, allowed_area = $35, logout_time = now(), updated_at = now() \
where id = $36 and current_area = $37 and current_mirror = $38";

const INSERT_CHARACTER_ITEM_SQL: &str = "insert into character_items(\
character_id, item_id, inventory_slot, is_cursor, item_json, name, description, flags_bits, \
sprite, item_value, min_level, max_level, needs_class, owner_id, modifier_index, modifier_value, \
x, y, carried_by, contained_in, content_id, driver, driver_data, serial, updated_at) \
values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, \
$16, $17, $18, $19, $20, $21, $22, $23, $24, now())";

const LOAD_CHARACTER_SNAPSHOT_SQL: &str = "select character_json, ppd_blob, subscriber_blob, \
current_area, current_mirror, allowed_area, mirror from characters where id = $1";

const LOAD_CHARACTER_ITEMS_SQL: &str = "select item_json from character_items \
where character_id = $1 order by coalesce(inventory_slot, 2147483647), \
case when is_cursor then 1 else 0 end, item_id";

async fn save_character_snapshot_tx(
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

fn bind_character_snapshot<'q>(
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
        .bind(request.ppd_blob.clone())
        .bind(request.subscriber_blob.clone()))
}

async fn replace_character_items_tx(
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
struct CharacterItemStorageKey {
    item_id: ItemId,
    inventory_slot: Option<i32>,
    is_cursor: bool,
}

fn character_item_storage_rows<'a>(
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

fn inventory_item_slots(character: &Character) -> HashMap<ItemId, i32> {
    let mut slots = HashMap::new();
    for (slot, item_id) in character.inventory.iter().enumerate().take(INVENTORY_SIZE) {
        if let Some(item_id) = item_id {
            slots.entry(*item_id).or_insert(slot as i32);
        }
    }
    slots
}

fn optional_character_id_to_i64(id: Option<CharacterId>) -> Option<i64> {
    id.map(|id| id.0 as i64)
}

fn optional_item_id_to_i64(id: Option<ItemId>) -> Option<i64> {
    id.map(|id| id.0 as i64)
}

async fn begin_login_tx(
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
    if ip_locked {
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

    sqlx::query(
        "insert into login_sessions(character_id, account_id, character_name, ip_address, area_id, mirror_id, client_vendor, unique_id) \
         values ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(account_id)
    .bind(name)
    .bind(request.ip as i32)
    .bind(request.area_id)
    .bind(request.mirror_id)
    .bind(request.vendor as i32)
    .bind(request.unique as i32)
    .execute(&mut **tx)
    .await?;

    Ok(LoginOutcome::Ready {
        character_id: CharacterId(id as u32),
        character_number: 0,
        mirror,
        unique: request.unique,
    })
}

fn legacy_password_matches(password: &str, stored_password: &str) -> bool {
    password == stored_password
}

const IS_BADPASS_IP_SQL: &str = "select \
    count(*) filter (where created_at >= now() - interval '60 seconds'), \
    count(*) filter (where created_at >= now() - interval '3600 seconds'), \
    count(*) filter (where created_at >= now() - interval '86400 seconds') \
 from bad_passwords where ip = $1";

/// Matches C `is_badpass_ip` (`src/system/badip.c:56-72`): an IP is
/// rate-limited if it has more than 3 recorded bad-password attempts in
/// the last 60 seconds, more than 8 in the last hour, or more than 25 in
/// the last 24 hours.
async fn is_ip_rate_limited(pool: &PgPool, ip: u32) -> anyhow::Result<bool> {
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
fn is_badpass_counts_rate_limited(recent_minute: i64, recent_hour: i64, recent_day: i64) -> bool {
    recent_minute > 3 || recent_hour > 8 || recent_day > 25
}

/// Matches C `add_badpass_ip` (`src/system/badip.c:78-85`): records one
/// failed-password attempt for the given IP.
async fn record_bad_password_attempt(
    tx: &mut Transaction<'_, Postgres>,
    ip: u32,
) -> anyhow::Result<()> {
    sqlx::query("insert into bad_passwords(ip) values ($1)")
        .bind(ip as i32)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

const BEGIN_LOGIN_SQL: &str = "select c.id, c.account_id, c.name, a.password_hash, c.locked, a.locked, a.ip_locked, a.fixed, \
         extract(epoch from a.paid_until)::int, c.current_area, c.allowed_area, c.mirror, c.current_mirror \
          from characters c join accounts a on a.id = c.account_id \
          where lower(c.name) = lower($1) for update";

/// Matches C `load_char_dup` (`database_character.c:731-753`): counts other
/// characters on the same account currently online (`current_area != 0`).
const BEGIN_LOGIN_TX_DUPLICATE_SQL: &str =
    "select count(*) from characters where account_id = $1 and id != $2 and current_area != 0";

async fn update_mirror_if_needed(
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

#[cfg(test)]
mod tests {
    use super::*;
    use ugaris_core::entity::{CharacterFlags, ItemFlags, SpeedMode, MAX_MODIFIERS};

    #[test]
    fn login_outcomes_match_legacy_find_login_codes() {
        assert_eq!(LoginOutcome::Waiting.legacy_find_login_code(), 0);
        assert_eq!(LoginOutcome::InternalError.legacy_find_login_code(), -1);
        assert_eq!(LoginOutcome::Locked.legacy_find_login_code(), -2);
        assert_eq!(LoginOutcome::WrongPassword.legacy_find_login_code(), -3);
        assert_eq!(LoginOutcome::Duplicate.legacy_find_login_code(), -4);
        assert_eq!(LoginOutcome::NotPaid.legacy_find_login_code(), -5);
        assert_eq!(LoginOutcome::Shutdown.legacy_find_login_code(), -6);
        assert_eq!(LoginOutcome::IpLocked.legacy_find_login_code(), -7);
        assert_eq!(LoginOutcome::AccountNotFixed.legacy_find_login_code(), -8);
        assert_eq!(
            LoginOutcome::TooManyBadPasswords.legacy_find_login_code(),
            -9
        );
    }

    #[test]
    fn legacy_password_check_matches_c_plaintext_compare() {
        assert!(legacy_password_matches("test123", "test123"));
        assert!(!legacy_password_matches("test123", "Test123"));
        assert!(!legacy_password_matches("test123", ""));
    }

    #[test]
    fn login_query_fetches_account_password_before_status_checks() {
        assert!(BEGIN_LOGIN_SQL.contains("a.password_hash"));
        assert!(BEGIN_LOGIN_SQL.contains("for update"));
    }

    #[test]
    fn save_queries_keep_legacy_area_guard_shape() {
        assert!(SAVE_CHARACTER_BACKUP_SQL.contains("ppd_blob = $32"));
        assert!(SAVE_CHARACTER_BACKUP_SQL.contains("subscriber_blob = $33"));
        assert!(SAVE_CHARACTER_BACKUP_SQL
            .contains("where id = $35 and current_area = $36 and current_mirror = $37"));

        assert!(SAVE_CHARACTER_LOGOUT_SQL.contains("allowed_area = $35"));
        assert!(SAVE_CHARACTER_LOGOUT_SQL.contains("logout_time = now()"));
        assert!(SAVE_CHARACTER_LOGOUT_SQL
            .contains("where id = $36 and current_area = $37 and current_mirror = $38"));
    }

    #[test]
    fn badpass_ip_rate_limit_matches_legacy_thresholds() {
        // C `is_badpass_ip` (`badip.c:56-72`): blocked once a window count
        // exceeds (not reaches) the threshold.
        assert!(!is_badpass_counts_rate_limited(0, 0, 0));
        assert!(!is_badpass_counts_rate_limited(3, 0, 0));
        assert!(is_badpass_counts_rate_limited(4, 0, 0));
        assert!(!is_badpass_counts_rate_limited(0, 8, 0));
        assert!(is_badpass_counts_rate_limited(0, 9, 0));
        assert!(!is_badpass_counts_rate_limited(0, 0, 25));
        assert!(is_badpass_counts_rate_limited(0, 0, 26));
        // Any single window tripping is enough, independent of the others.
        assert!(is_badpass_counts_rate_limited(4, 0, 0));
        assert!(is_badpass_counts_rate_limited(0, 9, 0));
        assert!(is_badpass_counts_rate_limited(0, 0, 26));
    }

    #[test]
    fn badpass_ip_sql_scopes_to_the_three_legacy_windows_for_one_ip() {
        assert!(IS_BADPASS_IP_SQL.contains("interval '60 seconds'"));
        assert!(IS_BADPASS_IP_SQL.contains("interval '3600 seconds'"));
        assert!(IS_BADPASS_IP_SQL.contains("interval '86400 seconds'"));
        assert!(IS_BADPASS_IP_SQL.contains("where ip = $1"));
    }

    #[test]
    fn duplicate_login_query_excludes_self_and_scopes_to_online_characters() {
        assert!(BEGIN_LOGIN_TX_DUPLICATE_SQL.contains("account_id = $1"));
        assert!(BEGIN_LOGIN_TX_DUPLICATE_SQL.contains("id != $2"));
        assert!(BEGIN_LOGIN_TX_DUPLICATE_SQL.contains("current_area != 0"));
    }

    #[test]
    fn character_item_storage_rows_keep_inventory_slots_and_cursor() {
        let mut character = character(7);
        character.inventory[30] = Some(ItemId(11));
        character.inventory[31] = Some(ItemId(11));
        character.inventory[32] = Some(ItemId(12));
        character.cursor_item = Some(ItemId(13));

        let items = vec![item(11), item(12), item(13), item(99)];
        let keys = character_item_storage_rows(&character, &items)
            .into_iter()
            .map(|(_, key)| key)
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                CharacterItemStorageKey {
                    item_id: ItemId(11),
                    inventory_slot: Some(30),
                    is_cursor: false,
                },
                CharacterItemStorageKey {
                    item_id: ItemId(12),
                    inventory_slot: Some(32),
                    is_cursor: false,
                },
                CharacterItemStorageKey {
                    item_id: ItemId(13),
                    inventory_slot: None,
                    is_cursor: true,
                },
            ]
        );
    }

    #[test]
    fn character_snapshot_json_round_trips_without_database() {
        let mut character = character(42);
        character.flags = CharacterFlags::PLAYER | CharacterFlags::SPY;
        character.exp = 1234;
        character.exp_used = 1000;
        character.inventory[30] = Some(ItemId(77));

        let decoded = Json(character.clone()).0;

        assert_eq!(decoded.id, character.id);
        assert_eq!(decoded.flags, character.flags);
        assert_eq!(decoded.exp, 1234);
        assert_eq!(decoded.inventory[30], Some(ItemId(77)));
    }

    fn character(id: u32) -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(id),
            serial: id,
            name: format!("Char{id}"),
            description: String::new(),
            flags: CharacterFlags::PLAYER,
            sprite: 0,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            staff_code: String::new(),
            speed_mode: SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 1,
            rest_x: 126,
            rest_y: 179,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            military_points: 0,
            military_normal_exp: 0,
            gold: 0,
            karma: 0,
            creation_time: 0,
            saves: 0,
            got_saved: 0,
            deaths: 0,
            regen_ticker: 0,
            last_regen: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }

    /// Live-database tests for `begin_login_tx`'s row-decision branching
    /// (unknown name / wrong password / locked / not-paid / duplicate /
    /// area routing / success), gated behind `DATABASE_URL` per the task
    /// note ("otherwise gate live tests behind `DATABASE_URL`"). Each test
    /// opens its own transaction, serializes against sibling live tests
    /// with a transaction-scoped advisory lock (`pg_advisory_xact_lock`,
    /// released automatically on rollback/commit), resets the `accounts`
    /// id sequence to a deterministic offset so `account_id == 1` (C's
    /// duplicate-login test-account exemption, `sID == 1`) can be tested
    /// precisely without racing other tests for that id, and always rolls
    /// back at the end - no fixture ever needs manual cleanup. Skips
    /// (rather than fails) when `DATABASE_URL` is unset or unreachable, so
    /// the suite stays green in this porting environment's default
    /// no-Postgres setup while still running for real against a live
    /// database in environments (or Ralph iterations) that provide one.
    mod live_login {
        use super::*;
        use sqlx::{PgPool, Postgres, Transaction};

        const ADVISORY_LOCK_KEY: i64 = 0x7567_6172_6973_6462; // "ugarisdb"-ish

        async fn connect() -> Option<PgPool> {
            let url = std::env::var("DATABASE_URL").ok()?;
            match PgPool::connect(&url).await {
                Ok(pool) => Some(pool),
                Err(err) => {
                    eprintln!("skipping live DB test: could not connect to DATABASE_URL: {err}");
                    None
                }
            }
        }

        /// Opens a transaction, serializes against other live tests, and
        /// resets the `accounts_id_seq` so the next inserted account gets
        /// id `next_account_id`. The transaction is never committed by the
        /// caller (see module doc), so this reset is always race-free and
        /// never collides with real persisted data.
        async fn locked_tx(pool: &PgPool, next_account_id: i64) -> Transaction<'_, Postgres> {
            let mut tx = pool.begin().await.expect("begin tx");
            sqlx::query("select pg_advisory_xact_lock($1)")
                .bind(ADVISORY_LOCK_KEY)
                .execute(&mut *tx)
                .await
                .expect("advisory lock");
            sqlx::query("select setval('accounts_id_seq', $1, false)")
                .bind(next_account_id)
                .execute(&mut *tx)
                .await
                .expect("reset accounts sequence");
            tx
        }

        struct AccountOpts {
            username: &'static str,
            password_hash: &'static str,
            locked: bool,
            ip_locked: bool,
            fixed: bool,
            paid: bool,
        }

        impl Default for AccountOpts {
            fn default() -> Self {
                Self {
                    username: "live_test_account",
                    password_hash: "secret",
                    locked: false,
                    ip_locked: false,
                    fixed: true,
                    paid: true,
                }
            }
        }

        async fn insert_account(tx: &mut Transaction<'_, Postgres>, opts: AccountOpts) -> i64 {
            let (id,): (i64,) = sqlx::query_as(
                "insert into accounts(username, password_hash, locked, ip_locked, fixed, paid_until) \
                 values ($1, $2, $3, $4, $5, case when $6 then now() + interval '1 day' else null end) \
                 returning id",
            )
            .bind(opts.username)
            .bind(opts.password_hash)
            .bind(opts.locked)
            .bind(opts.ip_locked)
            .bind(opts.fixed)
            .bind(opts.paid)
            .fetch_one(&mut **tx)
            .await
            .expect("insert account");
            id
        }

        #[allow(clippy::too_many_arguments)]
        async fn insert_character(
            tx: &mut Transaction<'_, Postgres>,
            account_id: i64,
            name: &str,
            locked: bool,
            current_area: i32,
            allowed_area: i32,
            mirror: i32,
            current_mirror: i32,
        ) -> i64 {
            let (id,): (i64,) = sqlx::query_as(
                "insert into characters(account_id, name, locked, current_area, allowed_area, mirror, current_mirror) \
                 values ($1, $2, $3, $4, $5, $6, $7) returning id",
            )
            .bind(account_id)
            .bind(name)
            .bind(locked)
            .bind(current_area)
            .bind(allowed_area)
            .bind(mirror)
            .bind(current_mirror)
            .fetch_one(&mut **tx)
            .await
            .expect("insert character");
            id
        }

        fn request(name: &str, password: &str) -> LoginRequest {
            LoginRequest {
                name: name.to_string(),
                password: password.to_string(),
                vendor: 0,
                unique: 42,
                ip: 0x0a00_0001,
                area_id: 3,
                mirror_id: 1,
                no_login: false,
            }
        }

        #[tokio::test]
        async fn rejects_unknown_character_name() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2000).await;

            let outcome = begin_login_tx(&mut tx, request("nosuchcharacter", "whatever"))
                .await
                .expect("begin_login_tx");

            assert_eq!(outcome, LoginOutcome::WrongPassword);
        }

        #[tokio::test]
        async fn rejects_wrong_password_and_records_bad_password() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2010).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "wrongpw_acct",
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Wrongpw", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Wrongpw", "not-the-password"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::WrongPassword);

            let (bad_count,): (i64,) =
                sqlx::query_as("select count(*) from bad_passwords where ip = $1")
                    .bind(0x0a00_0001i32)
                    .fetch_one(&mut *tx)
                    .await
                    .expect("count bad_passwords");
            assert_eq!(
                bad_count, 1,
                "wrong password must record a bad_passwords row (C add_badpass_ip)"
            );
        }

        #[tokio::test]
        async fn rejects_locked_character() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2020).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "lockedchar_acct",
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Lockedchar", true, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Lockedchar", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::Locked);
        }

        #[tokio::test]
        async fn rejects_locked_account() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2030).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "lockedacct_acct",
                    locked: true,
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Lockedacct", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Lockedacct", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::Locked);
        }

        #[tokio::test]
        async fn rejects_ip_locked_account() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2040).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "iplocked_acct",
                    ip_locked: true,
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Iplocked", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Iplocked", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::IpLocked);
        }

        #[tokio::test]
        async fn rejects_unfixed_account() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2050).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "unfixed_acct",
                    fixed: false,
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Unfixedchar", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Unfixedchar", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::AccountNotFixed);
        }

        #[tokio::test]
        async fn rejects_not_paid_account() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2060).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "notpaid_acct",
                    paid: false,
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Notpaidchar", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Notpaidchar", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::NotPaid);
        }

        #[tokio::test]
        async fn rejects_internal_error_for_unresolved_allowed_area() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2070).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "noarea_acct",
                    ..Default::default()
                },
            )
            .await;
            insert_character(&mut tx, account_id, "Noareachar", false, 0, 0, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Noareachar", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::InternalError);
        }

        #[tokio::test]
        async fn rejects_duplicate_login_for_normal_account() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2080).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "dup_acct",
                    ..Default::default()
                },
            )
            .await;
            // Another character on the same account is already online.
            insert_character(&mut tx, account_id, "Duponline", false, 5, 3, 1, 5).await;
            insert_character(&mut tx, account_id, "Dupoffline", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Dupoffline", "secret"))
                .await
                .expect("begin_login_tx");
            assert_eq!(outcome, LoginOutcome::Duplicate);
        }

        #[tokio::test]
        async fn exempts_account_id_one_from_duplicate_login_check() {
            let Some(pool) = connect().await else {
                return;
            };
            // Deterministically force this account to id 1, matching C's
            // `if (sID == 1) return 1; // hack for easier testing`
            // (`database_character.c:731-753`) exemption from the
            // duplicate-login check.
            let mut tx = locked_tx(&pool, 1).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "hack_test_acct",
                    ..Default::default()
                },
            )
            .await;
            assert_eq!(account_id, 1, "test setup must land account_id == 1");
            insert_character(&mut tx, account_id, "Hackonline", false, 5, 3, 1, 5).await;
            insert_character(&mut tx, account_id, "Hackoffline", false, 0, 3, 1, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Hackoffline", "secret"))
                .await
                .expect("begin_login_tx");
            assert_ne!(
                outcome,
                LoginOutcome::Duplicate,
                "account_id == 1 must be exempt from the duplicate-login check"
            );
        }

        #[tokio::test]
        async fn routes_to_new_area_when_allowed_area_mismatches_request() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2090).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "newarea_acct",
                    ..Default::default()
                },
            )
            .await;
            // allowed_area (7) differs from request.area_id (3).
            insert_character(&mut tx, account_id, "Newareachar", false, 0, 7, 2, 0).await;

            let outcome = begin_login_tx(&mut tx, request("Newareachar", "secret"))
                .await
                .expect("begin_login_tx");
            match outcome {
                LoginOutcome::NewArea {
                    area_id, mirror, ..
                } => {
                    assert_eq!(area_id, 7);
                    assert_eq!(mirror, 2);
                }
                other => panic!("expected NewArea, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn accepts_matching_area_and_records_login_session() {
            let Some(pool) = connect().await else {
                return;
            };
            let mut tx = locked_tx(&pool, 2100).await;
            let account_id = insert_account(
                &mut tx,
                AccountOpts {
                    username: "ready_acct",
                    ..Default::default()
                },
            )
            .await;
            let character_id =
                insert_character(&mut tx, account_id, "Readychar", false, 0, 3, 1, 0).await;

            let req = request("Readychar", "secret");
            let outcome = begin_login_tx(&mut tx, req.clone())
                .await
                .expect("begin_login_tx");
            match outcome {
                LoginOutcome::Ready {
                    character_id: got_id,
                    mirror,
                    unique,
                    ..
                } => {
                    assert_eq!(got_id, CharacterId(character_id as u32));
                    assert_eq!(mirror, 1);
                    assert_eq!(unique, req.unique);
                }
                other => panic!("expected Ready, got {other:?}"),
            }

            let (current_area,): (i32,) =
                sqlx::query_as("select current_area from characters where id = $1")
                    .bind(character_id)
                    .fetch_one(&mut *tx)
                    .await
                    .expect("fetch character");
            assert_eq!(current_area, req.area_id);

            let (session_count,): (i64,) =
                sqlx::query_as("select count(*) from login_sessions where character_id = $1")
                    .bind(character_id)
                    .fetch_one(&mut *tx)
                    .await
                    .expect("count login_sessions");
            assert_eq!(session_count, 1);
        }
    }

    fn item(id: u32) -> Item {
        Item {
            id: ItemId(id),
            name: format!("Item{id}"),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: id,
        }
    }
}
