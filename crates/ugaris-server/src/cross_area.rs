//! Mid-game cross-area transfer (C `change_area`, `src/system/database/
//! database_character.c:343-364`, called from every teleport-style call
//! site listed in the "Cross-area transfer" `PORTING_TODO.md` task).
//!
//! C's cross-area transfer is entirely client-driven reconnect, not a
//! server-to-server handoff (see the design plan in `PORTING_LEDGER.md`'s
//! "Continuation Handoff" section): the sending server looks up the
//! target via `get_area`, saves the character to DB with the
//! *destination* area/coordinates already written (`kick_char`'s
//! `save_char(cn, save_area)`), despawns it locally, then sends the
//! `SV_SERVER` redirect packet (C `player_to_server`) and disconnects the
//! session. The client reconnects to the new address with the same
//! credentials; the receiving area server's ordinary login flow finds the
//! DB row's `allowed_area` already matching its own `area_id` and proceeds
//! as a normal `Ready` login - no in-process multi-area mode or new
//! inter-server protocol is needed.
//!
//! This module ports that sequence as a single reusable helper so every
//! `TransportTravelResult::CrossArea`/`area_id != config.area_id` call
//! site (transport points here; `/office`+`/goto`, the clan-spawn exit,
//! the mine gateway, `/jail`+`/unjail`, and the dungeon-master rescue
//! still fall back to the "target area server is down" message and are
//! left for a follow-up slice - see the `PORTING_TODO.md` task's
//! Progress Log) can call the same save-then-redirect sequence instead of
//! duplicating it.

use super::*;

/// Attempts the full `change_area`/`kick_char`/`player_to_server`
/// sequence for `character_id`: look up the target area server, save the
/// character to DB with the destination area/coordinates/mirror, despawn
/// it from this process's live `World`, then send the `SV_SERVER`
/// redirect and disconnect every session attached to it. Returns `true`
/// only once the redirect packet was actually sent (matching C's own
/// `change_area` returning `1` only when `get_area` resolves - a save or
/// DB-repository failure still logs a warning but, like C's `kick_char`
/// (which never checks `save_char`'s return value before proceeding to
/// `destroy_char`), does not by itself block the transfer; the *lookup*
/// failing is the only thing this port treats as a hard "stay put"
/// case, matching the DB-guard convention every other cross-area
/// call site already uses for the "down" fallback). Callers should send
/// today's C `"Nothing happens - target area server is down."` message
/// whenever this returns `false`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn attempt_cross_area_transfer(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
    character_id: CharacterId,
    target_area: u16,
    target_mirror: u32,
    target_x: u16,
    target_y: u16,
) -> bool {
    let (Some(area_repo), Some(character_repo)) = (area_repository, character_repository) else {
        return false;
    };

    let record = match area_repo
        .get_area(i32::from(target_area), target_mirror as i32)
        .await
    {
        Ok(record) => record,
        Err(err) => {
            warn!(
                character_id = character_id.0,
                area = target_area,
                mirror = target_mirror,
                error = %err,
                "failed to look up target area server for cross-area transfer"
            );
            return false;
        }
    };
    let Some(payload) = area_redirect_payload(record.as_ref()) else {
        return false;
    };

    let Some(player) = runtime.player_for_character(character_id) else {
        return false;
    };
    let Some(character) = world.characters.get(&character_id) else {
        return false;
    };
    let account_depot = runtime.account_depots.get(&character_id).cloned();
    let request = character_area_transfer_save_request(
        world,
        player,
        character,
        account_depot.as_ref(),
        area_id,
        mirror_id,
        target_area,
        target_mirror as u16,
        target_x,
        target_y,
    );
    match character_repo.save_character_snapshot(request).await {
        Ok(true) => {}
        Ok(false) => {
            warn!(
                character_id = character_id.0,
                area = target_area,
                mirror = target_mirror,
                "cross-area transfer save was skipped by area guard (stale current_area/mirror)"
            );
        }
        Err(err) => {
            warn!(
                character_id = character_id.0,
                area = target_area,
                mirror = target_mirror,
                error = %err,
                "failed to save DB-backed character snapshot for cross-area transfer"
            );
        }
    }

    runtime.account_depots.remove(&character_id);
    world.remove_character(character_id);
    for (session_id, _) in runtime.sessions_for_character(character_id) {
        runtime.send_to_session(session_id, payload.clone());
        runtime.flush_session(session_id);
        if let Some(commands) = runtime.sessions.get(&session_id) {
            let _ = commands.try_send(SessionCommand::Disconnect);
        }
    }
    info!(
        character_id = character_id.0,
        area = target_area,
        mirror = target_mirror,
        "redirected character to target area server for cross-area transfer"
    );
    true
}
