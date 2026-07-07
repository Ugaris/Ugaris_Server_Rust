//! Live PostgreSQL integration tests using disposable containers.
//!
//! Requires a working Docker daemon. Excluded from the default test run:
//!
//! ```bash
//! cargo test -p ugaris-db -- --ignored
//! ```
//!
//! For a persistent local development database instead, use the workspace
//! `compose.yaml` (`docker compose up -d`) and point `DATABASE_URL` at it.

use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use ugaris_core::entity::{Character, CharacterFlags, SpeedMode};
use ugaris_core::ids::CharacterId;
use ugaris_db::{CharacterRepository, CharacterSaveMode, CharacterSaveRequest, Database};

fn minimal_character(id: u32, name: &str) -> Character {
    let mut c = character(id);
    c.name = name.to_string();
    c.flags = CharacterFlags::USED | CharacterFlags::PLAYER | CharacterFlags::ALIVE;
    c.level = 3;
    c
}

// Full literal fixture copied from ugaris-db's unit tests.
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
    }
}

#[tokio::test]
#[ignore = "requires docker (cargo test -p ugaris-db -- --ignored)"]
async fn character_snapshot_round_trips_player_state_json() -> anyhow::Result<()> {
    let container = Postgres::default().start().await?;
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container.get_host_port_ipv4(5432).await?
    );

    let db = Database::connect(&url, 2).await?;
    db.run_migrations().await?;

    // Seed the minimal account/character rows the save path updates.
    sqlx::query("insert into accounts(id, username, password_hash) values (1, 'it-test', 'x')")
        .execute(db.pool())
        .await?;
    sqlx::query(
        "insert into characters(id, account_id, name, current_area, current_mirror) \
         values (42, 1, 'RoundTrip', 1, 1)",
    )
    .execute(db.pool())
    .await?;

    let repository = db.characters();
    let state = serde_json::json!({
        "player": {"keyring": [{"template_id": 7, "name": "Test Key"}]},
        "account_depot": null,
    });
    let saved = repository
        .save_character_snapshot(CharacterSaveRequest {
            character: minimal_character(42, "RoundTrip"),
            items: Vec::new(),
            player_state_json: Some(state.clone()),
            mode: CharacterSaveMode::Logout {
                expected_current_area: 1,
                expected_current_mirror: 1,
                allowed_area: 1,
                mirror: 1,
            },
        })
        .await?;
    assert!(saved, "logout save matches the area/mirror guard");

    let snapshot = repository
        .load_character_snapshot(CharacterId(42))
        .await?
        .expect("saved snapshot loads");
    assert_eq!(snapshot.player_state_json, Some(state));
    assert_eq!(snapshot.character.name, "RoundTrip");

    // Offline saves bind None and must preserve the stored document.
    sqlx::query("update characters set current_area = 1, current_mirror = 1 where id = 42")
        .execute(db.pool())
        .await?;
    repository
        .save_character_snapshot(CharacterSaveRequest {
            character: minimal_character(42, "RoundTrip"),
            items: Vec::new(),
            player_state_json: None,
            mode: CharacterSaveMode::Backup {
                expected_current_area: 1,
                expected_current_mirror: 1,
                mirror: 1,
            },
        })
        .await?;
    let snapshot = repository
        .load_character_snapshot(CharacterId(42))
        .await?
        .expect("snapshot still loads");
    assert!(
        snapshot.player_state_json.is_some(),
        "coalesce keeps the JSON document when an offline save binds None"
    );
    Ok(())
}
