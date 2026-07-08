pub mod achievement;
pub mod anticheat;
pub mod area;
pub mod auction;
pub mod character;
pub mod clan;
pub mod clan_log;
pub mod merchant;
pub mod military;
pub mod notes;
pub mod pentagram;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub use achievement::{AchievementRepository, PgAchievementRepository};
pub use anticheat::{
    AntiCheatCounters, AntiCheatEvent, AntiCheatFingerprint, AntiCheatHighRiskRow,
    AntiCheatPlayerStatsRow, AntiCheatRepository, AntiCheatSessionCreate,
    AntiCheatSessionHistoryRow, AntiCheatSessionInfo, AntiCheatSharedHwRow, AntiCheatSharedIpRow,
    AntiCheatSignatureRow, AntiCheatSubscriberLookup, AntiCheatViolationRow, PgAntiCheatRepository,
};
pub use area::{AreaRepository, AreaServerRecord, PgAreaRepository};
pub use auction::{
    AuctionDelivery, AuctionFilter, AuctionRecord, AuctionRepository, AuctionSearchResult,
    AuctionSortBy, AuctionStatus, DeliveryReason, DeliverySummary, NewAuction, NewDelivery,
    PgAuctionRepository, MAX_SEARCH_RESULTS,
};
pub use character::{
    CharacterQueryStats, CharacterRepository, CharacterSaveMode, CharacterSaveRequest,
    CharacterSnapshot, LastSeenInfo, LegacyBlobRow, LoginOutcome, LoginRequest,
    PgCharacterRepository,
};
pub use clan::{ClanRegistryRepository, PgClanRegistryRepository};
pub use clan_log::{
    ClanLogEntry, ClanLogFilter, ClanLogRepository, PgClanLogRepository, CLAN_LOG_DISPLAY_LIMIT,
    CLAN_LOG_FETCH_LIMIT,
};
pub use merchant::{
    MerchantRepository, MerchantStoreSnapshot, MerchantWareSnapshot, PgMerchantRepository,
};
pub use military::{
    MilitaryAdvisorStorageRepository, MilitaryMasterStorageRepository,
    PgMilitaryAdvisorStorageRepository, PgMilitaryMasterStorageRepository,
};
pub use notes::{NotesRepository, PgNotesRepository};
pub use pentagram::{PentagramRecordRepository, PentagramRecordRow, PgPentagramRecordRepository};

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

    /// Apply every SQL file under the workspace `migrations/` directory
    /// (embedded at compile time). All migrations are written idempotently
    /// (`create table if not exists` / `add column if not exists`), so
    /// running against a database that predates the `_sqlx_migrations`
    /// bookkeeping table is safe.
    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
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

    pub fn merchants(&self) -> PgMerchantRepository {
        PgMerchantRepository::new(self.pool.clone())
    }

    pub fn auctions(&self) -> PgAuctionRepository {
        PgAuctionRepository::new(self.pool.clone())
    }

    pub fn achievements(&self) -> PgAchievementRepository {
        PgAchievementRepository::new(self.pool.clone())
    }

    pub fn clans(&self) -> PgClanRegistryRepository {
        PgClanRegistryRepository::new(self.pool.clone())
    }

    pub fn clan_log(&self) -> PgClanLogRepository {
        PgClanLogRepository::new(self.pool.clone())
    }

    pub fn military_master_storage(&self) -> PgMilitaryMasterStorageRepository {
        PgMilitaryMasterStorageRepository::new(self.pool.clone())
    }

    pub fn military_advisor_storage(&self) -> PgMilitaryAdvisorStorageRepository {
        PgMilitaryAdvisorStorageRepository::new(self.pool.clone())
    }

    pub fn notes(&self) -> PgNotesRepository {
        PgNotesRepository::new(self.pool.clone())
    }

    pub fn pentagram_record(&self) -> PgPentagramRecordRepository {
        PgPentagramRecordRepository::new(self.pool.clone())
    }
}
