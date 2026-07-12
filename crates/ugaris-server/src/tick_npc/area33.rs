//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gorwin_driver_158(
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
    // C `gorwin_driver`: the Tunnel Changer NPC who runs the Long Tunnels
    // (area 33) entrance lobby (`src/area/33/tunnel.c`).
    let gorwin_facts = gorwin_player_facts(runtime);
    let gorwin_events = world.process_gorwin_actions(&gorwin_facts, config.area_id);
    let gorwin_events_applied = apply_gorwin_events(runtime, gorwin_events);
    if gorwin_events_applied != 0 {
        info!(
            gorwin_events_applied,
            tick = world.tick.0,
            "applied gorwin dialogue events"
        );
    }
}
