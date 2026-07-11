//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn spiritbran_driver_133(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `spirit_brannington_driver`: the ghost NPC in Brannington who
    // explains the necromancer plot and runs "The Brannington Holy Relic"
    // (quest 44) (`src/area/29/brannington.c`).
    let spiritbran_facts = spiritbran_player_facts(runtime);
    let spiritbran_events = world.process_spiritbran_actions(&spiritbran_facts, config.area_id);
    let spiritbran_events_applied = apply_spiritbran_events(world, runtime, spiritbran_events);
    if spiritbran_events_applied != 0 {
        info!(
            spiritbran_events_applied,
            tick = world.tick.0,
            "applied spiritbran dialogue events"
        );
    }
}
