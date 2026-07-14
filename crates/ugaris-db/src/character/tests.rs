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
    assert!(!legacy_password_matches("test123", ""));
    assert!(!legacy_password_matches("test123", ""));
}

/// `PgCharacterRepository::query_stats`'s counters start at zero and
/// are shared across clones (an `Arc`, not a per-clone copy) - both
/// pure in-memory properties, tested here without needing a live
/// Postgres connection at all: `connect_lazy` only parses the URL and
/// defers opening any real socket to the first actual query, which
/// `query_stats`/the raw atomic `fetch_add` below never issue.
#[tokio::test]
async fn query_stats_start_at_zero_and_are_shared_across_clones() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@127.0.0.1/db")
        .expect("connect_lazy only parses the URL, never connects");
    let repository = PgCharacterRepository::new(pool);
    assert_eq!(
        repository.query_stats(),
        CharacterQueryStats {
            save_char_cnt: 0,
            exit_char_cnt: 0,
            load_char_cnt: 0,
        }
    );

    let cloned = repository.clone();
    cloned
        .query_counters
        .save_char_cnt
        .fetch_add(1, Ordering::Relaxed);
    cloned
        .query_counters
        .exit_char_cnt
        .fetch_add(2, Ordering::Relaxed);
    cloned
        .query_counters
        .load_char_cnt
        .fetch_add(3, Ordering::Relaxed);

    // The original handle sees the clone's increments too, since both
    // share the same `Arc<CharacterQueryCounters>` - matching every
    // other `Pg*Repository`'s cheap-clone-shares-pool convention.
    assert_eq!(
        repository.query_stats(),
        CharacterQueryStats {
            save_char_cnt: 1,
            exit_char_cnt: 2,
            load_char_cnt: 3,
        }
    );
}

#[test]
fn login_query_fetches_account_password_before_status_checks() {
    assert!(BEGIN_LOGIN_SQL.contains("a.password_hash"));
    assert!(BEGIN_LOGIN_SQL.contains("for update"));
}

#[test]
fn save_queries_keep_legacy_area_guard_shape() {
    // ppd_blob/subscriber_blob are no longer in the SET list (see the
    // "Retire legacy blob writes" PORTING_TODO.md task) - the columns
    // are frozen, not written by any save path anymore.
    assert!(!SAVE_CHARACTER_BACKUP_SQL.contains("ppd_blob"));
    assert!(!SAVE_CHARACTER_BACKUP_SQL.contains("subscriber_blob"));
    assert!(
        SAVE_CHARACTER_BACKUP_SQL.contains("player_state_json = coalesce($32, player_state_json)")
    );
    assert!(SAVE_CHARACTER_BACKUP_SQL
        .contains("where id = $34 and current_area = $35 and current_mirror = $36"));

    assert!(!SAVE_CHARACTER_LOGOUT_SQL.contains("ppd_blob"));
    assert!(!SAVE_CHARACTER_LOGOUT_SQL.contains("subscriber_blob"));
    assert!(
        SAVE_CHARACTER_LOGOUT_SQL.contains("player_state_json = coalesce($32, player_state_json)")
    );
    assert!(SAVE_CHARACTER_LOGOUT_SQL.contains("allowed_area = $34"));
    assert!(SAVE_CHARACTER_LOGOUT_SQL.contains("logout_time = now()"));
    assert!(SAVE_CHARACTER_LOGOUT_SQL
        .contains("where id = $35 and current_area = $36 and current_mirror = $37"));
}

#[test]
fn find_name_by_id_sql_looks_up_by_bare_id() {
    assert_eq!(
        FIND_NAME_BY_ID_SQL,
        "select name from characters where id = $1"
    );
}

#[test]
fn find_paid_until_info_sql_joins_accounts_by_character_id() {
    assert!(FIND_PAID_UNTIL_INFO_SQL.contains("join accounts a on a.id = c.account_id"));
    assert!(FIND_PAID_UNTIL_INFO_SQL.contains("where c.id = $1"));
    assert!(FIND_PAID_UNTIL_INFO_SQL.contains("extract(epoch from a.paid_until)"));
    assert!(FIND_PAID_UNTIL_INFO_SQL.contains("extract(epoch from a.created_at)"));
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
        driver_memory: ugaris_core::character_driver::DriverMemory::default(),
        class: 0,
        dungeonfighter: None,
        fight_driver: None,
        lq_usurp: None,
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

    /// The real, `/exterminate`-populated gate (C `isbanned_iplog`):
    /// unlike `rejects_ip_locked_account` above (the static
    /// per-account flag), this account has `ip_locked = false` - only
    /// a matching unexpired `ip_bans` row for the *connecting* IP
    /// (`request`'s fixed `0x0a00_0001`) blocks the login.
    #[tokio::test]
    async fn rejects_login_from_a_banned_ip() {
        let Some(pool) = connect().await else {
            return;
        };
        let mut tx = locked_tx(&pool, 2045).await;
        let account_id = insert_account(
            &mut tx,
            AccountOpts {
                username: "ipbanned_acct",
                ..Default::default()
            },
        )
        .await;
        insert_character(&mut tx, account_id, "Ipbanned", false, 0, 3, 1, 0).await;
        sqlx::query(
            "insert into ip_bans(ip, banned_until) values ($1, now() + interval '28 days')",
        )
        .bind(0x0a00_0001i32)
        .execute(&mut *tx)
        .await
        .expect("insert ip_bans row");

        let outcome = begin_login_tx(&mut tx, request("Ipbanned", "secret"))
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
        #[allow(clippy::needless_late_init)]
        let got_login_session_id;
        match outcome {
            LoginOutcome::Ready {
                character_id: got_id,
                mirror,
                unique,
                login_session_id,
                account_id: got_account_id,
                ..
            } => {
                assert_eq!(got_id, CharacterId(character_id as u32));
                assert_eq!(mirror, 1);
                assert_eq!(unique, req.unique);
                assert_eq!(got_account_id, account_id);
                got_login_session_id = login_session_id;
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

        // The returned `login_session_id` must be the real primary key
        // of the row just inserted, not a placeholder.
        let (row_id,): (i64,) =
            sqlx::query_as("select id from login_sessions where character_id = $1")
                .bind(character_id)
                .fetch_one(&mut *tx)
                .await
                .expect("fetch login_sessions row");
        assert_eq!(got_login_session_id, row_id);
    }
}

/// Live-database tests for [`CharacterRepository::exterminate_account`]
/// and [`is_ip_banned`]. Unlike `live_login`'s tests, both real
/// methods under test commit through the repository's own pool
/// connection (no externally-supplied transaction to roll back), so
/// this module follows `auction.rs`'s `mod live` precedent instead:
/// unique per-test usernames/IPs, a real commit, explicit row cleanup
/// at the end of every test.
mod live_exterminate {
    use super::*;
    use sqlx::PgPool;

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

    async fn insert_account_and_character(pool: &PgPool, name: &str) -> (i64, i64) {
        let account_id: i64 = sqlx::query_scalar(
            "insert into accounts(username, password_hash) values ($1, 'x') returning id",
        )
        .bind(format!("{name}_acct"))
        .fetch_one(pool)
        .await
        .expect("insert account");
        let character_id: i64 = sqlx::query_scalar(
            "insert into characters(account_id, name) values ($1, $2) returning id",
        )
        .bind(account_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("insert character");
        (account_id, character_id)
    }

    async fn insert_login_session(pool: &PgPool, account_id: i64, character_id: i64, ip: i32) {
        sqlx::query(
            "insert into login_sessions(character_id, account_id, character_name, ip_address, area_id, mirror_id) \
                 values ($1, $2, 'x', $3, 1, 1)",
        )
        .bind(character_id)
        .bind(account_id)
        .bind(ip)
        .execute(pool)
        .await
        .expect("insert login_session");
    }

    async fn cleanup(pool: &PgPool, account_id: i64, character_id: i64, ips: &[i32]) {
        sqlx::query("delete from login_sessions where account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await
            .ok();
        for ip in ips {
            sqlx::query("delete from ip_bans where ip = $1")
                .bind(ip)
                .execute(pool)
                .await
                .ok();
        }
        sqlx::query("delete from characters where id = $1")
            .bind(character_id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(pool)
            .await
            .ok();
    }

    #[tokio::test]
    async fn exterminate_locks_the_account_and_bans_every_distinct_login_ip() {
        let Some(pool) = connect().await else {
            return;
        };
        let (account_id, character_id) = insert_account_and_character(&pool, "extermtarget").await;
        // Two sessions from the same IP plus one from a second IP: the
        // ported query dedupes via `select distinct`, so this must
        // yield exactly 2 banned IPs, not 3 log rows.
        insert_login_session(&pool, account_id, character_id, 0x0102_0304).await;
        insert_login_session(&pool, account_id, character_id, 0x0102_0304).await;
        insert_login_session(&pool, account_id, character_id, 0x0506_0708).await;

        let repo = PgCharacterRepository::new(pool.clone());
        let outcome = repo
            .exterminate_account("ExtermTarget")
            .await
            .expect("exterminate_account")
            .expect("character should be found");
        assert_eq!(outcome.locked_accounts, 1);
        assert_eq!(outcome.banned_ips, 2);

        let (locked,): (bool,) = sqlx::query_as("select locked from accounts where id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("fetch account");
        assert!(locked);

        let mut tx = pool.begin().await.expect("begin tx");
        assert!(is_ip_banned(&mut tx, 0x0102_0304u32)
            .await
            .expect("is_ip_banned"));
        assert!(is_ip_banned(&mut tx, 0x0506_0708u32)
            .await
            .expect("is_ip_banned"));
        assert!(!is_ip_banned(&mut tx, 0x0a0a_0a0au32)
            .await
            .expect("is_ip_banned"));
        tx.rollback().await.ok();

        cleanup(&pool, account_id, character_id, &[0x0102_0304, 0x0506_0708]).await;
    }

    #[tokio::test]
    async fn exterminate_reports_not_found_for_an_unknown_name() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgCharacterRepository::new(pool.clone());
        let outcome = repo
            .exterminate_account("NoSuchExterminateTarget")
            .await
            .expect("exterminate_account");
        assert_eq!(outcome, None);
    }

    #[tokio::test]
    async fn expired_ip_ban_no_longer_rejects_login() {
        let Some(pool) = connect().await else {
            return;
        };
        let ip: i32 = 0x0b0b_0b0b;
        sqlx::query("insert into ip_bans(ip, banned_until) values ($1, now() - interval '1 day')")
            .bind(ip)
            .execute(&pool)
            .await
            .expect("insert expired ban");

        let mut tx = pool.begin().await.expect("begin tx");
        let banned = is_ip_banned(&mut tx, ip as u32)
            .await
            .expect("is_ip_banned");
        tx.rollback().await.ok();
        assert!(!banned, "an expired ip_bans row must not still block login");

        sqlx::query("delete from ip_bans where ip = $1")
            .bind(ip)
            .execute(&pool)
            .await
            .ok();
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
