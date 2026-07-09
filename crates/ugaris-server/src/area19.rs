//! Server-side wiring for area 19's Nomad Plains NPCs (`CDR_NOMAD`/
//! `ugaris_core::world::npc::area19::nomad::process_nomad_actions`,
//! `CDR_MADHERMIT`/`...::madhermit::process_madhermit_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area17.rs`: [`nomad_player_facts`] snapshots the per-player
//! `nomad_ppd` facts the driver needs before the tick, and
//! [`apply_nomad_events`] applies the returned events afterward - some of
//! which additionally need `ZoneLoader` (salt/dice/statue item creation),
//! which `World` cannot see either.

use super::*;
use ugaris_core::world::{NomadOutcomeEvent, NomadPlayerFacts};

pub(crate) fn nomad_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, NomadPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let mut nomad_state = [0i32; 10];
            let mut nomad_win = [0i32; 10];
            for (index, (state, win)) in
                nomad_state.iter_mut().zip(nomad_win.iter_mut()).enumerate()
            {
                *state = player.nomad_state(index);
                *win = player.nomad_win(index);
            }
            Some((
                character_id,
                NomadPlayerFacts {
                    nomad_state,
                    nomad_win,
                    tribe_member: player.nomad_tribe_member(),
                    open_bet: player.nomad_open_bet(),
                    open_roll: player.nomad_open_roll(),
                },
            ))
        })
        .collect()
}

/// C `create_item("salt")` followed by `it[in2].value *= amount;
/// *(unsigned int *)(it[in2].drdata) = amount; set_salt_data(in2);` - the
/// `ZoneLoader`-needing half of `World::configure_fresh_salt_item`'s doc
/// comment.
fn create_salt_item(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    amount: u32,
) -> Option<ItemId> {
    let item = zone_loader.instantiate_item_template("salt", None).ok()?;
    let item_id = item.id;
    world.add_item(item);
    world.configure_fresh_salt_item(item_id, amount);
    Some(item_id)
}

/// Applies each [`NomadOutcomeEvent`] queued by `World::
/// process_nomad_actions`.
pub(crate) fn apply_nomad_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    events: Vec<NomadOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            NomadOutcomeEvent::UpdateNomadState {
                player_id,
                nr,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_nomad_state(nr, new_state);
                applied += 1;
            }
            // C `questlog_open(co, 32/33/34)`.
            NomadOutcomeEvent::QuestOpen {
                player_id,
                quest_id,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(quest_id as usize);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 32/33/34)`.
            NomadOutcomeEvent::QuestDone {
                player_id,
                quest_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) =
                    player
                        .quest_log
                        .complete_legacy(quest_id as usize, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `ppd->tribe_member |= TM_TRIBE1` (`nomad_1_give`).
            NomadOutcomeEvent::SetTribeMember { player_id, flag } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.add_nomad_tribe_member(flag);
                applied += 1;
            }
            // C `ppd->nomad_win[nr] += / -= dat->bet` (`nomad_roll`'s win
            // branch).
            NomadOutcomeEvent::AdjustNomadWin {
                player_id,
                nr,
                delta,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                let current = player.nomad_win(nr);
                player.set_nomad_win(nr, current + delta);
                applied += 1;
            }
            // C `ppd->open_bet`/`open_roll1/2/3` writes (`nomad_bet`/
            // `nomad_roll`).
            NomadOutcomeEvent::SetOpenBet {
                player_id,
                bet,
                roll1,
                roll2,
                roll3,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_nomad_open_bet(bet, roll1, roll2, roll3);
                applied += 1;
            }
            // C `give_exp(co, diff/10 or diff/2)` (`nomad_5_give`).
            NomadOutcomeEvent::GiveExp {
                player_id,
                base_exp,
            } => {
                world.give_exp(player_id, base_exp, u32::from(world.area_id));
                applied += 1;
            }
            // C `nomad_1_give`'s wolf/white-wolf-skin trade-in
            // (`nomad.c:710-734`).
            NomadOutcomeEvent::GiveSaltForSkin {
                nomad_id,
                player_id,
                skin_item_id,
                amount,
            } => {
                let Some(item_id) = create_salt_item(world, zone_loader, amount) else {
                    world.npc_say(nomad_id, "Oopsy");
                    continue;
                };
                if world.give_char_item(player_id, item_id) {
                    world.npc_say(
                        nomad_id,
                        &format!("Here, {amount} ounces of salt for these skins."),
                    );
                    world.destroy_item(skin_item_id);
                    applied += 1;
                } else {
                    world.destroy_item(item_id);
                    world.npc_say(nomad_id, "Oops.");
                }
            }
            // C `nomad_2_text`/`nomad_6_text`'s dice/golden-statue
            // purchase (`nomad.c:602-679`).
            NomadOutcomeEvent::BuyItemWithSalt {
                nomad_id,
                player_id,
                template,
                cost,
            } => {
                let Ok(item) = zone_loader.instantiate_item_template(template, Some(player_id))
                else {
                    continue;
                };
                let item_id = item.id;
                world.add_item(item);
                if world.give_char_item(player_id, item_id) {
                    world.remove_salt(player_id, cost);
                    let player_name = world
                        .characters
                        .get(&player_id)
                        .map(|character| character.name.clone())
                        .unwrap_or_default();
                    world.npc_say(
                        nomad_id,
                        &format!("It's a pleasure doing business with thee, {player_name}."),
                    );
                    applied += 1;
                } else {
                    world.destroy_item(item_id);
                }
            }
            // C `nomad_roll`'s loss branch (`nomad.c:904-922`).
            NomadOutcomeEvent::PaySaltWinnings {
                nomad_id,
                player_id,
                amount,
                nr,
            } => {
                let Some(item_id) = create_salt_item(world, zone_loader, amount.max(0) as u32)
                else {
                    world.npc_say(nomad_id, "Oopsy");
                    continue;
                };
                if world.give_char_item(player_id, item_id) {
                    world.npc_say(nomad_id, &format!("Here, {amount} ounces of salt."));
                    if let Some(player) = runtime.player_for_character_mut(player_id) {
                        let current = player.nomad_win(nr);
                        player.set_nomad_win(nr, current - amount);
                    }
                    applied += 1;
                } else {
                    world.destroy_item(item_id);
                    world.npc_say(nomad_id, "Oops.");
                }
            }
        }
    }
    applied
}
