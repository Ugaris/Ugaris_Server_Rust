//! Session-side wiring for the `CDR_LOSTCON` disconnect linger
//! (`src/system/player.c::kick_player`, `src/module/lostcon.c`).
//!
//! `ugaris_core::world::lostcon` owns the `World`-side driver/deadline
//! state; this module owns the session bookkeeping around it: stashing the
//! disconnecting session's `PlayerRuntime` instead of dropping it, restoring
//! it on a reclaiming reconnect, and collecting expired characters (with
//! their stashed runtime + account depot) for the tick loop to save and
//! despawn.

use super::*;

/// C `kick_player` (`src/system/player.c:174`): detach the disconnecting
/// session from its character and arm the `CDR_LOSTCON` linger instead of
/// despawning immediately. Returns `false` (caller should fall back to an
/// immediate save+despawn) when there is no live world character to linger,
/// mirroring C's `if (player[nr]->state == ST_NORMAL)` guard.
pub(crate) fn enter_lostcon_on_disconnect(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    player: PlayerRuntime,
    account_depot: Option<AccountDepotState>,
    current_tick: u64,
    lagout_time: i32,
) -> Option<PlayerRuntime> {
    let deadline = current_tick.saturating_add(lagout_time.max(0) as u64);
    if !world.enter_lostcon(character_id, deadline) {
        return Some(player);
    }
    if let Some(account_depot) = account_depot {
        runtime.account_depots.insert(character_id, account_depot);
    }
    runtime.lostcon_players.insert(character_id, player);
    None
}

/// C `tick_login()` (`src/system/database/database_character.c:1164`) /
/// `read_login()` (`src/system/player.c:493`): reclaim a character still
/// lingering under `CDR_LOSTCON` in place instead of re-reading it from the
/// database or spawning a fresh one. Restores the stashed `PlayerRuntime`
/// (PPD-backed state) onto the new session and clears the linger driver.
pub(crate) fn reclaim_lostcon_on_login(
    world: &mut World,
    runtime: &mut ServerRuntime,
    session_id: u64,
    character_id: CharacterId,
    current_tick: u64,
) -> bool {
    if !world.is_lostcon(character_id) {
        return false;
    }
    if let Some(stashed) = runtime.lostcon_players.remove(&character_id) {
        let restored = stashed.reclaim_for_session(session_id, current_tick);
        runtime.players.insert(session_id, restored);
    }
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
        if player.character_number == 0 {
            player.character_number = character_id.0;
        }
    }
    world.reclaim_lostcon(character_id);
    true
}

/// C `lostcon_driver`'s timeout branch plus `exit_char`/`kick_char`:
/// characters whose lagout linger expired without being reclaimed. Returns
/// the stashed `PlayerRuntime`/account depot for each so the caller can
/// build the same `character_save_request` a normal disconnect would have
/// used, then call `world.remove_character`.
pub(crate) fn take_expired_lostcon_characters(
    world: &World,
    runtime: &mut ServerRuntime,
    current_tick: u64,
) -> Vec<(CharacterId, PlayerRuntime, Option<AccountDepotState>)> {
    world
        .expired_lostcon_characters(current_tick)
        .into_iter()
        .filter_map(|character_id| {
            let player = runtime.lostcon_players.remove(&character_id)?;
            let account_depot = runtime.account_depots.remove(&character_id);
            Some((character_id, player, account_depot))
        })
        .collect()
}

/// C `lostcon_driver`'s early-exit gauntlet (`lostcon_early_exit_
/// characters`'s doc comment has the full list: rest-area/arena tiles,
/// the karma cutoff) - characters that leave at once regardless of the
/// ordinary lagout timeout. Same stashed-runtime/account-depot contract as
/// `take_expired_lostcon_characters`; callers should merge both lists into
/// the same save+despawn loop.
pub(crate) fn take_lostcon_early_exit_characters(
    world: &World,
    runtime: &mut ServerRuntime,
    area_id: u16,
) -> Vec<(CharacterId, PlayerRuntime, Option<AccountDepotState>)> {
    world
        .lostcon_early_exit_characters(area_id)
        .into_iter()
        .filter_map(|character_id| {
            let player = runtime.lostcon_players.remove(&character_id)?;
            let account_depot = runtime.account_depots.remove(&character_id);
            Some((character_id, player, account_depot))
        })
        .collect()
}
