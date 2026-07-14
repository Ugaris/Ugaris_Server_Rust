use super::*;

/// `/jail`/`/unjail`'s async DB round trip (C `lookup_name`,
/// `system/lookup.c:42-98` + `system/database/database_lookup.c:57-83`):
/// resolves every `World::drain_pending_jail_lookups` entry (queued by a
/// validly-shaped `/jail`/`/unjail <name>` argument - see `World::
/// queue_jail_lookup`'s and `apply_admin_character_command`'s doc
/// comments) against the DB.
///
/// - no DB row -> "No character by the name %s." (C's dispatcher-level
///   `lookup_name == -1` branch, `command.c:9041`-equivalent for
///   `jail`/`unjail`).
/// - a row found -> hands off to `World::resolve_jail_lookup`, which
///   reproduces `cmd_jail_player`/`cmd_unjail_player`'s own separate
///   online-only `CF_PLAYER` name scan and, on a match, applies the
///   jail/unjail mutation (no match -> "No player by that name.", the
///   exact text both C functions share).
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_jail_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_jail_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(_)) => {
                world.resolve_jail_lookup(lookup.caller_id, &lookup.target_name, lookup.action);
            }
            Ok(None) => {
                world.queue_system_text(
                    lookup.caller_id,
                    format!("No character by the name {}.", lookup.target_name),
                );
            }
            Err(_) => continue,
        }
        applied += 1;
    }
    applied
}

/// `/jail`/`/unjail`'s cross-area hand-off (C `change_area(cn, resta,
/// restx, resty)`, `src/system/tool.c:4392-4425`'s tail): resolves every
/// `World::drain_pending_jail_cross_area_transfers` entry (queued by
/// `World::apply_jail_action` when the jail/aston destination area
/// differs from this area server's own `area_id` - see `world/jail.rs`'s
/// module doc comment) via the shared `attempt_cross_area_transfer`
/// helper, same as the `TransportTravel`/`ClanSpawnExit`/`MineGateway`/
/// `/office`+`/goto` call sites. The destination mirror always equals
/// this process's own `mirror_id`: neither jail nor aston locations carry
/// a mirror field of their own (matching C's `change_area` reading
/// `ch[cn].mirror`, i.e. the target character's *own current* mirror,
/// which under this codebase's single-process-per-area-mirror stance is
/// always this process's `mirror_id`).
pub(crate) async fn apply_jail_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_jail_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.target_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.queue_system_text(
                transfer.caller_id,
                "Nothing happens - target area server is down.".to_string(),
            );
        }
        applied += 1;
    }
    applied
}

/// The Macro Daemon's cross-server "challenge room" hand-off (C
/// `change_area`, `src/module/base.c:1110` for the suspicion-triggered
/// banishment, `848-850` for the correct-answer return trip): resolves
/// every `World::drain_pending_macro_cross_area_transfers` entry (queued
/// by `ugaris-server/src/macro_daemon.rs` when the challenge-room/
/// original-area destination differs from this area server's own
/// `area_id` - see `world/macro_npc.rs`'s module doc comment) via the
/// shared `attempt_cross_area_transfer` helper, same as every other
/// cross-area call site. Like C's own `change_area` call sites here, a
/// failed hand-off is not specially handled - C never checks `change_
/// area`'s return value at either call site either, so a down target
/// area server simply leaves the character in place with no message
/// (weaker than `apply_dungeon_eviction_transfers`'s "system-triggered,
/// no caller to notify" precedent, which at least falls back to
/// `remove_character` - not needed here since `attempt_cross_area_
/// transfer` itself already guarantees no despawn happened on a lookup
/// failure, so "leave the character exactly where they were" is already
/// the correct fallback with no extra code).
pub(crate) async fn apply_macro_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_macro_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        applied += 1;
    }
    applied
}

/// `build_remove_tile`'s evicted-player cross-area rescue (C
/// `change_area(cn, ch[cn].resta, ch[cn].restx, ch[cn].resty)`,
/// `src/area/13/dungeon.c:754`'s tail): resolves every `World::
/// drain_pending_dungeon_eviction_transfers` entry (queued by
/// `World::build_remove_tile` when the evicted player's own `rest_area`
/// differs from this area server's own `area_id` - see
/// `world/dungeon_master.rs`'s module doc comment) via the shared
/// `attempt_cross_area_transfer` helper, same as every other cross-area
/// call site. The destination mirror always equals this process's own
/// `mirror_id` (rest points carry no mirror field of their own, matching
/// C's `change_area` reading `ch[cn].mirror`). Unlike every other
/// call site, C's own fallback on failure is `exit_char(cn)` (no
/// message - the character has no "down" feedback path here since
/// `exit_char` disconnects them entirely), so a failed hand-off calls
/// `World::remove_character` instead of queuing a system text.
pub(crate) async fn apply_dungeon_eviction_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_dungeon_eviction_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.remove_character(transfer.character_id);
        }
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod jail_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        world.queue_jail_lookup(
            CharacterId(7),
            "Godmode",
            ugaris_core::world::JailAction::Jail,
        );

        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_jail_lookups().is_empty());
    }
}

#[cfg(test)]
mod jail_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_the_shared_down_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so the caller gets the legacy
        // "Nothing happens - target area server is down." text - the
        // exact fallback `World::apply_jail_action` used to send
        // eagerly before this hand-off was deferred.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1; // current server is NOT the jail area
        world.settings.jail_x = 186;
        world.settings.jail_y = 234;
        world.settings.jail_area = 3;
        let login = LoginBlock {
            name: "Godmode".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));
        let mut target_login = login.clone();
        target_login.name = "Baddie".to_string();
        world.add_character(login_character(CharacterId(2), &target_login, 1, 50, 50));
        world.resolve_jail_lookup(
            CharacterId(1),
            "Baddie",
            ugaris_core::world::JailAction::Jail,
        );
        // The synchronous jail/unjail messages (`You have jailed
        // .../You have been jailed by ...`) are not this hand-off's
        // concern - drain them so only the transfer's own feedback
        // remains below.
        world.drain_pending_system_texts();

        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        let texts = world.drain_pending_system_texts();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].character_id, CharacterId(1));
        assert_eq!(
            texts[0].message,
            "Nothing happens - target area server is down."
        );
        assert!(world.drain_pending_jail_cross_area_transfers().is_empty());
    }
}

#[cfg(test)]
mod dungeon_eviction_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_removing_the_character() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so - unlike every other cross-area
        // call site, which sends "Nothing happens - target area server
        // is down." - this one mirrors C's `exit_char(cn)` fallback and
        // removes the character outright instead (see
        // `world/dungeon_master.rs`'s module doc comment).
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 13;
        let login = LoginBlock {
            name: "Raider".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        let mut raider = login_character(CharacterId(1), &login, 13, 10, 10);
        raider.rest_area = 3; // a different area - queues a cross-area transfer
        raider.rest_x = 50;
        raider.rest_y = 60;
        assert!(world.spawn_character(raider, 10, 10));
        for (x, y) in [(245, 250), (240, 250), (235, 250), (230, 250)] {
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = (x + dx) as usize;
                    let ty = (y + dy) as usize;
                    world.map.tile_mut(tx, ty).unwrap().flags |=
                        ugaris_core::map::MapFlags::MOVEBLOCK;
                }
            }
        }
        world.build_remove_tile(10, 10);
        world.drain_pending_system_texts();

        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 13, 0).await;
        assert_eq!(applied, 1);
        assert!(!world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_dungeon_eviction_transfers().is_empty());
    }
}

#[cfg(test)]
mod macro_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_leaves_the_character_in_place_with_no_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target and never despawns the character - C
        // never checks `change_area`'s return value at either macro-
        // daemon call site either, so this hand-off has no "target area
        // server is down" message to send and no fallback action beyond
        // leaving the character exactly where it already was.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1;
        let login = LoginBlock {
            name: "Victim".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        assert!(world.spawn_character(login_character(CharacterId(1), &login, 1, 10, 10), 10, 10));
        world.queue_macro_cross_area_transfer(CharacterId(1), 3, 178, 248);

        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        assert!(world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_macro_cross_area_transfers().is_empty());
    }
}
