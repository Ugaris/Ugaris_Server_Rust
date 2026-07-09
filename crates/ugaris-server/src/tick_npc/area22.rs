//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn pass_0(
    world: &mut World,
    _runtime: &mut ServerRuntime,
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
    let npc_message_characters: Vec<_> = world
        .characters
        .iter()
        .filter_map(|(&character_id, character)| {
            (!character.driver_messages.is_empty()
                && (character.driver == CDR_SIMPLEBADDY
                    || character.driver == CDR_LAB2UNDEAD
                    || matches!(
                        character.driver_state.as_ref(),
                        Some(
                            CharacterDriverState::SimpleBaddy(_)
                                | CharacterDriverState::Lab2Undead(_)
                        )
                    )))
            .then_some(character_id)
        })
        .collect();
    if !npc_message_characters.is_empty() {
        let mut simple_baddy_outcomes = 0;
        let mut lab2_undead_outcomes = 0;
        for character_id in npc_message_characters {
            let driver_state = world
                .characters
                .get(&character_id)
                .and_then(|character| character.driver_state.as_ref())
                .cloned();
            match driver_state {
                Some(CharacterDriverState::SimpleBaddy(_)) => {
                    simple_baddy_outcomes += world
                        .process_simple_baddy_message_actions(character_id, config.area_id)
                        .len();
                }
                Some(CharacterDriverState::Lab2Undead(_)) => {
                    lab2_undead_outcomes += world.process_lab2_undead_message_actions(character_id);
                }
                _ => {
                    if world
                        .characters
                        .get(&character_id)
                        .is_some_and(|character| character.driver == CDR_SIMPLEBADDY)
                    {
                        simple_baddy_outcomes += world
                            .process_simple_baddy_message_actions(character_id, config.area_id)
                            .len();
                    } else if world
                        .characters
                        .get(&character_id)
                        .is_some_and(|character| character.driver == CDR_LAB2UNDEAD)
                    {
                        lab2_undead_outcomes +=
                            world.process_lab2_undead_message_actions(character_id);
                    }
                }
            }
        }
        info!(
            simple_baddy_outcomes,
            lab2_undead_outcomes,
            tick = world.tick.0,
            "processed NPC driver messages"
        );
    }

    let simple_baddy_attacks = world
        .process_simple_baddy_attack_actions_with_random(config.area_id, |limit| {
            runtime_random_below(limit as i32).max(0) as u32
        });
    if simple_baddy_attacks != 0 {
        info!(
            simple_baddy_attacks,
            tick = world.tick.0,
            "queued simple-baddy attack actions"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lostcon_driver_4(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    completed_actions: &[WorldActionCompletion],
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
    // C `lostcon_driver`'s full per-tick body
    // (`src/module/lostcon.c:117-220`) for every character
    // currently lingering under `CDR_LOSTCON`: the message
    // loop, the low-hp-heal/low-mana-potion/low-shield-
    // magicshield pre-cascade, `fight_driver_update(cn); if
    // (fight_driver_attack_visible(cn, ppd->nomove)) return; if
    // (!ppd->nomove && fight_driver_follow_invisible(cn))
    // return;`, the bless/magicshield/heal post-cascade
    // fallback, and finally `do_idle(cn, TICKS)` if none of the
    // above did anything this tick.
    let lostcon_character_ids: Vec<CharacterId> = runtime.lostcon_players.keys().copied().collect();
    let mut lostcon_attacks = 0;
    let mut lostcon_idles = 0;
    for character_id in lostcon_character_ids {
        world.process_lostcon_messages(character_id);
        let (fight_suppressions, self_care_suppressions) = runtime
            .lostcon_players
            .get(&character_id)
            .map(|player| {
                (
                    player.fight_driver_suppressions(),
                    player.lostcon_self_care_suppressions(),
                )
            })
            .unwrap_or_default();
        world.process_lostcon_self_care_precascade(
            character_id,
            config.area_id,
            self_care_suppressions,
        );
        let attacked = world.process_lostcon_attack_action_with_random(
            character_id,
            config.area_id,
            fight_suppressions,
            |limit| runtime_random_below(limit as i32).max(0) as u32,
        );
        if attacked {
            lostcon_attacks += 1;
            continue;
        }
        if world.process_lostcon_self_care_postcascade(character_id, self_care_suppressions) {
            continue;
        }
        if world.queue_lostcon_idle(character_id) {
            lostcon_idles += 1;
        }
    }
    if lostcon_attacks != 0 || lostcon_idles != 0 {
        info!(
            lostcon_attacks,
            lostcon_idles,
            tick = world.tick.0,
            "queued lostcon self-defense/idle actions"
        );
    }

    let simple_baddy_noncombat = world.process_simple_baddy_noncombat_actions_with_completions(
        config.area_id,
        &completed_actions,
    );
    if simple_baddy_noncombat != 0 {
        info!(
            simple_baddy_noncombat,
            tick = world.tick.0,
            "queued simple-baddy noncombat actions"
        );
    }

    let lab2_undead_cathedral = world.process_lab2_undead_cathedral_self_destructions();
    if lab2_undead_cathedral != 0 {
        info!(
            lab2_undead_cathedral,
            tick = world.tick.0,
            "processed Lab 2 undead cathedral self-destruction"
        );
    }

    let lab2_undead_crypt_doors = world.process_lab2_undead_crypt_door_actions();
    if lab2_undead_crypt_doors != 0 {
        info!(
            lab2_undead_crypt_doors,
            tick = world.tick.0,
            "processed Lab 2 undead crypt door closures"
        );
    }

    let lab2_undead_patrol = world.process_lab2_undead_patrol_actions(config.area_id);
    if lab2_undead_patrol != 0 {
        info!(
            lab2_undead_patrol,
            tick = world.tick.0,
            "queued Lab 2 undead patrol actions"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gate_fight_driver_51(
    world: &mut World,
    _runtime: &mut ServerRuntime,
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
    // C `gate_fight_driver`: the private-room duel opponent
    // spawned by `EnterTestReady` (`src/system/gatekeeper.c`).
    // Its `gate_fight_dead` death-reward tail is wired via
    // `apply_gate_fight_death_from_hurt_event`, called from
    // `apply_pk_hate_from_hurt_events` below.
    let gate_fight_acted = world.process_gate_fight_actions(config.area_id);
    if gate_fight_acted != 0 {
        info!(
            gate_fight_acted,
            tick = world.tick.0,
            "processed gate-fight opponent actions"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn labgnome_driver_108(
    world: &mut World,
    _runtime: &mut ServerRuntime,
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
    // C `labgnome_driver`: the area-22 Lab 1 torch-gnome triad (guard/
    // fighter/immortal master).
    let labgnome_acted = world.process_labgnome_actions(config.area_id);
    if labgnome_acted != 0 {
        info!(
            labgnome_acted,
            tick = world.tick.0,
            "processed lab-1 torch-gnome actions"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lab2herald_driver_109(
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
    // C `lab2_herald_driver`: the area-22 Lab 2 graveyard chapel keeper.
    let player_facts = crate::area22::lab2_herald_player_facts(runtime);
    let events = world.process_lab2_herald_actions(&player_facts, config.area_id);
    if !events.is_empty() {
        let applied = crate::area22::apply_lab2_herald_events(runtime, events);
        if applied != 0 {
            info!(
                applied,
                tick = world.tick.0,
                "processed lab-2 herald dialogue actions"
            );
        }
    }
}
