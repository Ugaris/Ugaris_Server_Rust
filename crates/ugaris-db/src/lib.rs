pub mod anticheat;
pub mod area;
pub mod character;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub use anticheat::{
    AntiCheatCounters, AntiCheatEvent, AntiCheatFingerprint, AntiCheatRepository,
    AntiCheatSessionCreate, PgAntiCheatRepository,
};
pub use area::{AreaRepository, PgAreaRepository};
pub use character::{
    CharacterRepository, CharacterSaveMode, CharacterSaveRequest, CharacterSnapshot, LoginOutcome,
    LoginRequest, PgCharacterRepository,
};

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(url: &str, max_connections: u32) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        sqlx::query("select 1").execute(&self.pool).await?;
        Ok(())
    }

    pub fn characters(&self) -> PgCharacterRepository {
        PgCharacterRepository::new(self.pool.clone())
    }

    pub fn areas(&self) -> PgAreaRepository {
        PgAreaRepository::new(self.pool.clone())
    }

    pub fn anticheat(&self) -> PgAntiCheatRepository {
        PgAntiCheatRepository::new(self.pool.clone())
    }
}
