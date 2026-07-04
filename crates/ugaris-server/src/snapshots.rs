use super::*;

#[derive(Debug, Clone)]
pub(crate) struct CharacterSnapshotApplyResult {
    pub(crate) loaded: bool,
    pub(crate) account_depot: Option<AccountDepotState>,
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
        ..
    } = snapshot;

    player.ppd_blob = ppd_blob;
    player.subscriber_blob = subscriber_blob;
    let account_depot = decode_legacy_account_depot_subscriber_blob(&player.subscriber_blob);
    if let Some(data) = decode_legacy_achievement_data_subscriber_blob(&player.subscriber_blob) {
        player.achievement_data = data;
    }
    if let Some(stats) = decode_legacy_achievement_stats_subscriber_blob(&player.subscriber_blob) {
        player.achievement_stats = stats;
    }
    let ppd_blob = player.ppd_blob.clone();
    if !ppd_blob.is_empty() && !player.decode_legacy_ppd_blob(&ppd_blob) {
        warn!(
            character_id = character.id.0,
            "failed to decode legacy PPD blob for DB character"
        );
    }
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
                || character
                    .inventory
                    .iter()
                    .any(|slot| *slot == Some(item.id))
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
        ppd_blob: player.encode_legacy_ppd_blob(&player.ppd_blob),
        subscriber_blob: encode_legacy_achievement_stats_subscriber_blob(
            &encode_legacy_achievement_data_subscriber_blob(
                &encode_legacy_account_depot_subscriber_blob(
                    &player.subscriber_blob,
                    account_depot,
                ),
                &player.achievement_data,
            ),
            &player.achievement_stats,
        ),
        mode: CharacterSaveMode::Logout {
            expected_current_area: i32::from(area_id),
            expected_current_mirror: i32::from(mirror_id),
            allowed_area: i32::from(area_id),
            mirror: i32::from(save_mirror_id),
        },
    }
}
