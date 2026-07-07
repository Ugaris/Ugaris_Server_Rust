//! Completed-action-outcome handling: the transport-point family
//! (`src/system/transport.c`) of `ItemDriverOutcome` variants (open the
//! transport-point selector, travel via a selected point). Split out of
//! the giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). Warp, chests, dungeon, ice/
//! palace, Teufel, skel-raise, and Edemon/Fdemon were sliced first; this
//! is the eighth family slice. The rest of the match (clan-spawn, lq,
//! arena, shrines, xmas, swamp, burndown, key-assembly) is still inline
//! in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_transport_outcome(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    config: &ServerConfig,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::TransportOpen {
            character_id,
            point,
            ..
        } => {
            let Some(player) = runtime.player_for_character_mut(character_id) else {
                *failed += 1;
                return;
            };
            let newly_seen = if point == ugaris_core::item_driver::LEGACY_TRANSPORT_CLAN_EXIT {
                false
            } else {
                player.touch_transport(point)
            };
            let seen = player.transport_seen;
            if newly_seen {
                feedback.push((
                    character_id,
                    "You have reached a new transportation point.".to_string(),
                ));
            }
            let clan_access = transport_clan_access(world, character_id);
            let payload =
                bytes::BytesMut::from(&ugaris_protocol::packet::transport(seen, clan_access)[..]);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TransportInvalid {
            character_id,
            point,
            ..
        } => {
            feedback.push((character_id, format!("Nothing happens - BUG ({point},#1).")));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TransportTravel {
            character_id, spec, ..
        } => {
            let Some(player) = runtime.player_for_character(character_id) else {
                *failed += 1;
                return;
            };
            match apply_transport_travel(world, player, character_id, config.area_id, spec) {
                TransportTravelResult::SameArea { mirror, .. } => {
                    if let Some(player) = runtime.player_for_character_mut(character_id) {
                        player.set_current_mirror(mirror);
                    }
                    let mut builder = PacketBuilder::new();
                    builder.mirror(mirror);
                    let payload = builder.into_payload();
                    for (session_id, _) in runtime.sessions_for_character(character_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    *executed += 1;
                }
                TransportTravelResult::CrossArea { area, x, y, mirror } => {
                    let transferred = attempt_cross_area_transfer(
                        world,
                        runtime,
                        character_repository,
                        area_repository,
                        config.area_id,
                        config.mirror_id,
                        character_id,
                        area,
                        mirror,
                        x,
                        y,
                    )
                    .await;
                    if transferred {
                        *executed += 1;
                    } else {
                        feedback.push((
                            character_id,
                            "Nothing happens - target area server is down.".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
                TransportTravelResult::Busy => {
                    feedback.push((
                        character_id,
                        "Please try again soon. Target is busy".to_string(),
                    ));
                    *blocked += 1;
                }
                TransportTravelResult::Blocked(message) | TransportTravelResult::Bug(message) => {
                    feedback.push((character_id, message));
                    *blocked += 1;
                }
            }
        }
        _ => {}
    }
}
