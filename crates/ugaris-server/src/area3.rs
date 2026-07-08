//! Server-side wiring for area 3's crypt-entrance/crypt-quest NPCs
//! (`CDR_THOMAS`/`ugaris_core::world::thomas::process_thomas_actions`,
//! `CDR_SIRJONES`/`ugaris_core::world::sir_jones::
//! process_sir_jones_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established in
//! `area1.rs`: [`thomas_player_facts`]/[`sir_jones_player_facts`] snapshot
//! the per-player `area3_ppd`/`quest_log` facts each NPC's dialogue needs
//! before the tick, and [`apply_thomas_events`]/[`apply_sir_jones_events`]
//! apply the returned events afterward. Sir Jones's crypt-quest reward
//! (`SirJonesOutcomeEvent::GoldEarned`) needs `ZoneLoader::
//! instantiate_item_template` - C's `create_money_item` + plain
//! `give_char_item` (not the auto-gold-converting `give_char_item_smart`)
//! - so [`apply_sir_jones_events`] takes a `&mut ZoneLoader` parameter,
//! same precedent as `area1.rs`'s `apply_lydia_events`/
//! `apply_logain_events`.

use super::*;
use ugaris_core::world::{
    SirJonesOutcomeEvent, SirJonesPlayerFacts, ThomasOutcomeEvent, ThomasPlayerFacts,
};

pub(crate) fn thomas_player_facts(
    world: &World,
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ThomasPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let level = world
                .characters
                .get(&character_id)
                .map(|character| character.level)
                .unwrap_or_default();
            Some((
                character_id,
                ThomasPlayerFacts {
                    crypt_state: player.area3_crypt_state(),
                    level,
                },
            ))
        })
        .collect()
}

/// Applies each [`ThomasOutcomeEvent`] queued by
/// `World::process_thomas_actions`.
pub(crate) fn apply_thomas_events(
    runtime: &mut ServerRuntime,
    events: Vec<ThomasOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ThomasOutcomeEvent::UpdateCryptState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_state(new_state);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn sir_jones_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, SirJonesPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                SirJonesPlayerFacts {
                    crypt_state: player.area3_crypt_state(),
                    crypt_bonus: player.area3_crypt_bonus(),
                    quest18_count: player.quest_log.count(18),
                    quest19_done: player.quest_log.is_done(19),
                },
            ))
        })
        .collect()
}

/// Applies each [`SirJonesOutcomeEvent`] queued by
/// `World::process_sir_jones_actions`. See the module doc comment for why
/// this needs `loader`.
pub(crate) async fn apply_sir_jones_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<SirJonesOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            SirJonesOutcomeEvent::UpdateCryptState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_state(new_state);
                applied += 1;
            }
            // C `ppd->crypt_bonus = 1;` (`area3.c:2012`).
            SirJonesOutcomeEvent::SetCryptBonus { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_bonus(1);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            SirJonesOutcomeEvent::QuestOpen { player_id, quest } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(quest);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `create_money_item(MONEY_AREA3_VAMPIRE1)` + plain
            // `give_char_item(co, in)` (`area3.c:1946-1951`) - see the
            // module doc comment for why this stays a literal carried
            // item instead of an instant gold credit.
            SirJonesOutcomeEvent::GoldEarned { player_id, amount } => {
                if let Ok(mut item) = loader.instantiate_item_template("money", None) {
                    item.value = amount;
                    item.sprite = create_money_item_sprite(amount);
                    item.description = format!("{:.2}G.", f64::from(amount) / 100.0);
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}
