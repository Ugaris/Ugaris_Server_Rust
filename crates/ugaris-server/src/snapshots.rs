use super::*;

#[derive(Debug, Clone)]
pub(crate) struct CharacterSnapshotApplyResult {
    pub(crate) loaded: bool,
    pub(crate) account_depot: Option<AccountDepotState>,
}

/// Restores the fixed C-array lengths (`ch.value[2][V_MAX]`,
/// `ch.item[INVENTORY_SIZE]`, `ch.profession[P_MAX]`) on a character
/// deserialized from `character_json`. Downstream per-tick/per-packet code
/// indexes these collections directly (matching the C fixed arrays), so a
/// hand-edited or corrupted row must never be allowed to violate the shape
/// invariant - bad persisted data must not crash the server.
pub(crate) fn normalize_character_shape(character: &mut Character) {
    character
        .values
        .resize(2, vec![0; ugaris_core::entity::CHARACTER_VALUE_COUNT]);
    for values in &mut character.values {
        values.resize(ugaris_core::entity::CHARACTER_VALUE_COUNT, 0);
    }
    character
        .professions
        .resize(ugaris_core::entity::PROFESSION_COUNT, 0);
    character
        .inventory
        .resize(ugaris_core::entity::INVENTORY_SIZE, None);
}

pub(crate) fn apply_character_snapshot(
    world: &mut World,
    player: &mut PlayerRuntime,
    snapshot: CharacterSnapshot,
    fallback_x: usize,
    fallback_y: usize,
    realtime_seconds: u64,
) -> CharacterSnapshotApplyResult {
    let CharacterSnapshot {
        mut character,
        items,
        ppd_blob,
        subscriber_blob,
        player_state_json,
        ..
    } = snapshot;

    normalize_character_shape(&mut character);

    // Typed JSON state (migration 0020) is authoritative when present; the
    // legacy PPD/subscriber blobs remain a read fallback for rows saved
    // before the column existed.
    let restored_from_json = player_state_json.and_then(|value| {
        match serde_json::from_value::<PersistedPlayerState>(value) {
            Ok(persisted) => Some(restore_player_from_persisted(player, persisted)),
            Err(err) => {
                warn!(
                    character_id = character.id.0,
                    error = %err,
                    "failed to deserialize player state document; falling back to legacy blobs"
                );
                None
            }
        }
    });

    // These decoders are `#[deprecated]` (migration 0020's `player_state_json`
    // is authoritative now) but remain the only path that can hydrate a
    // pre-0020 row that has never been backfilled - see the "Retire legacy
    // blob writes" `PORTING_TODO.md` task.
    #[allow(deprecated)]
    let account_depot = if let Some(account_depot) = restored_from_json {
        account_depot
    } else {
        player.ppd_blob = ppd_blob;
        player.subscriber_blob = subscriber_blob;
        let account_depot = decode_legacy_account_depot_subscriber_blob(&player.subscriber_blob);
        if let Some(data) = decode_legacy_achievement_data_subscriber_blob(&player.subscriber_blob)
        {
            player.achievement_data = data;
        }
        if let Some(stats) =
            decode_legacy_achievement_stats_subscriber_blob(&player.subscriber_blob)
        {
            player.achievement_stats = stats;
        }
        let ppd_blob = player.ppd_blob.clone();
        if !ppd_blob.is_empty() && !player.decode_legacy_ppd_blob(&ppd_blob) {
            warn!(
                character_id = character.id.0,
                "failed to decode legacy PPD blob for DB character"
            );
        }
        account_depot
    };
    if player.shutup_until_seconds > realtime_seconds {
        character.flags.insert(CharacterFlags::SHUTUP);
    } else {
        player.shutup_until_seconds = 0;
        character.flags.remove(CharacterFlags::SHUTUP);
    }

    let character_id = character.id;
    let spawn_x = usize::from(character.x).max(1);
    let spawn_y = usize::from(character.y).max(1);
    if !world.spawn_character(character.clone(), spawn_x, spawn_y) {
        character.x = 0;
        character.y = 0;
        if !world.spawn_character(character, fallback_x, fallback_y) {
            return CharacterSnapshotApplyResult {
                loaded: false,
                account_depot: None,
            };
        }
    }

    for item in items {
        world.add_item(item);
    }
    // C `login_ok` (`src/system/database/database_character.c:1512`):
    // `update_char(cn)` once the loaded character (equipment included) is
    // fully in place, before the newbie HP/endurance/mana-to-max branch
    // that follows in C.
    world.update_character(character_id);
    CharacterSnapshotApplyResult {
        loaded: true,
        account_depot,
    }
}

pub(crate) fn character_snapshot_items(world: &World, character: &Character) -> Vec<Item> {
    world
        .items
        .values()
        .filter(|item| {
            item.carried_by == Some(character.id)
                || character.cursor_item == Some(item.id)
                || character.inventory.contains(&Some(item.id))
        })
        .cloned()
        .collect()
}

pub(crate) fn is_logout_vanishing_item(item: &Item) -> bool {
    item.driver == IDR_ARKHATA && item.driver_data.first().copied().unwrap_or_default() == 1
}

#[allow(dead_code)]
pub(crate) fn remove_area_leave_vanishing_items(
    world: &mut World,
    character_id: CharacterId,
) -> Vec<ItemId> {
    let item_ids: Vec<ItemId> = world
        .items
        .values()
        .filter(|item| item.carried_by == Some(character_id) && is_logout_vanishing_item(item))
        .map(|item| item.id)
        .collect();

    for item_id in &item_ids {
        world.destroy_item(*item_id);
    }

    if let Some(character) = world.characters.get_mut(&character_id) {
        if character
            .current_container
            .is_some_and(|item_id| item_ids.contains(&item_id))
        {
            character.current_container = None;
        }
    }

    item_ids
}

pub(crate) fn character_logout_snapshot(
    world: &World,
    character: &Character,
) -> (Character, Vec<Item>) {
    let mut snapshot_character = character.clone();
    let mut vanished_items = HashSet::new();
    let items: Vec<Item> = character_snapshot_items(world, character)
        .into_iter()
        .filter(|item| {
            if is_logout_vanishing_item(item) {
                vanished_items.insert(item.id);
                false
            } else {
                true
            }
        })
        .collect();

    if !vanished_items.is_empty() {
        if snapshot_character
            .cursor_item
            .is_some_and(|item_id| vanished_items.contains(&item_id))
        {
            snapshot_character.cursor_item = None;
        }
        for slot in &mut snapshot_character.inventory {
            if slot.is_some_and(|item_id| vanished_items.contains(&item_id)) {
                *slot = None;
            }
        }
        if snapshot_character
            .current_container
            .is_some_and(|item_id| vanished_items.contains(&item_id))
        {
            snapshot_character.current_container = None;
        }
    }

    (snapshot_character, items)
}

/// Authoritative typed persistence document (migration 0020,
/// `characters.player_state_json`). The full serde `PlayerRuntime` is the
/// payload on purpose: every `#[serde(default)]` field added to it in the
/// future persists automatically, and the JSON section names double as the
/// public read schema for website/launcher/bot integration
/// (`player_state_json->'player'->'keyring'`, `->'quest_log'`, ...). The
/// legacy PPD/subscriber blobs are still written during the transition but
/// are no longer read when this document is present.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PersistedPlayerState {
    pub(crate) player: PlayerRuntime,
    #[serde(default)]
    pub(crate) account_depot: Option<AccountDepotState>,
}

pub(crate) fn persisted_player_state_json(
    player: &PlayerRuntime,
    account_depot: Option<&AccountDepotState>,
) -> Option<serde_json::Value> {
    match serde_json::to_value(PersistedPlayerState {
        player: player.clone(),
        account_depot: account_depot.cloned(),
    }) {
        Ok(value) => Some(value),
        Err(err) => {
            warn!(error = %err, "failed to serialize player state document; falling back to legacy blobs");
            None
        }
    }
}

/// Restore a freshly deserialized [`PersistedPlayerState`] into the live
/// session runtime, preserving the connection/handshake identity of the
/// current session (the persisted document was written by a previous
/// session). Returns the persisted account depot, if any.
pub(crate) fn restore_player_from_persisted(
    player: &mut PlayerRuntime,
    persisted: PersistedPlayerState,
) -> Option<AccountDepotState> {
    let mut restored = persisted.player;
    restored.session_id = player.session_id;
    restored.state = player.state;
    restored.client_version = player.client_version;
    restored.view_distance = player.view_distance;
    restored.login_tick = player.login_tick;
    restored.last_command_tick = player.last_command_tick;
    restored.character_id = player.character_id;
    restored.character_number = player.character_number;
    restored.anticheat_session_id = player.anticheat_session_id;
    restored.ac_watch_enabled = player.ac_watch_enabled;
    restored.current_mirror_id = player.current_mirror_id;
    restored.action = Default::default();
    restored.queue.clear();
    restored.scrollback.clear();
    *player = restored;
    persisted.account_depot
}

pub(crate) fn character_save_request(
    world: &World,
    player: &PlayerRuntime,
    character: &Character,
    account_depot: Option<&AccountDepotState>,
    area_id: u16,
    mirror_id: u16,
) -> CharacterSaveRequest {
    let save_mirror_id = if player.current_mirror_id == 0 {
        mirror_id
    } else {
        player.current_mirror_id
    };
    let (snapshot_character, snapshot_items) = character_logout_snapshot(world, character);
    CharacterSaveRequest {
        character: snapshot_character,
        items: snapshot_items,
        player_state_json: persisted_player_state_json(player, account_depot),
        mode: CharacterSaveMode::Logout {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            allowed_area: i32::from(area_id),
            mirror: i32::from(save_mirror_id),
        },
    }
}

/// C `change_area`'s `kick_char`->`save_char(cn, save_area)` half
/// (`database_character.c:343-364` combined with `player.c:115-149`):
/// saves the character exactly like a normal logout
/// (`character_save_request`), except `allowed_area`/`mirror` are set to
/// the *destination* area/mirror instead of this server's own
/// `area_id`/`mirror_id`, and the snapshot's `x`/`y` are overwritten
/// with the destination coordinates. C models this via separate
/// `tmpx`/`tmpy`/`tmpa` fields set by `change_area` before `kick_char`
/// runs and consumed by the *receiving* server's `tick_login`
/// (`drop_char_extended(cn, ch[cn].tmpx, ch[cn].tmpy, 6)`) - this
/// codebase's `Character` has no separate tmp-position fields (see the
/// "Cross-area transfer" `PORTING_TODO.md` task), so the destination
/// coordinates are written directly into the saved `x`/`y` snapshot
/// fields the receiving server's own `apply_character_snapshot` already
/// spawns from, achieving the same net effect with one less field pair.
#[allow(clippy::too_many_arguments)]
pub(crate) fn character_area_transfer_save_request(
    world: &World,
    player: &PlayerRuntime,
    character: &Character,
    account_depot: Option<&AccountDepotState>,
    area_id: u16,
    mirror_id: u16,
    target_area_id: u16,
    target_mirror_id: u16,
    target_x: u16,
    target_y: u16,
) -> CharacterSaveRequest {
    let (mut snapshot_character, snapshot_items) = character_logout_snapshot(world, character);
    snapshot_character.x = target_x;
    snapshot_character.y = target_y;
    CharacterSaveRequest {
        character: snapshot_character,
        items: snapshot_items,
        player_state_json: persisted_player_state_json(player, account_depot),
        mode: CharacterSaveMode::Logout {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            allowed_area: i32::from(target_area_id),
            mirror: i32::from(target_mirror_id),
        },
    }
}

/// C `save_char(cn, 0)` (`database_character.c:95-...`, the "backup" mode
/// used by both `backup_players` and `/saveall`): unlike a logout save,
/// this serializes the character's *entire* live state exactly as-is,
/// including the currently-held cursor item (C's `if ((in = ch[cn].
/// citem)) *itmp++ = it[in];`) and without running any of the item-
/// vanishing logic `character_logout_snapshot` applies - the character
/// stays online and unmoved, so nothing should disappear.
pub(crate) fn character_backup_save_request(
    world: &World,
    player: &PlayerRuntime,
    character: &Character,
    account_depot: Option<&AccountDepotState>,
    area_id: u16,
    mirror_id: u16,
) -> CharacterSaveRequest {
    let save_mirror_id = if player.current_mirror_id == 0 {
        mirror_id
    } else {
        player.current_mirror_id
    };
    CharacterSaveRequest {
        character: character.clone(),
        items: character_snapshot_items(world, character),
        player_state_json: persisted_player_state_json(player, account_depot),
        mode: CharacterSaveMode::Backup {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            mirror: i32::from(save_mirror_id),
        },
    }
}
