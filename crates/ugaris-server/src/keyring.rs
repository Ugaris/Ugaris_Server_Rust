use super::*;

pub(crate) const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;

pub(crate) const IID_PLACEHOLDER_KEY: u32 = (59 << 24) | 0x000004;

#[cfg(test)]
pub(crate) const IID_AREA1_SKELKEY1: u32 = (1 << 24) | 0x000002;

pub(crate) const INVENTORY_KEY_START_SLOT: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyringAddApplyResult {
    Added { key_name: String },
    Duplicate,
    Full,
    NotAKey,
    MissingPlayer,
    MissingCursorItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyringAutoAddPickupResult {
    Added { key_name: String },
    Duplicate { key_name: String },
    Full { key_name: String },
    MissingPlayer,
    MissingCursorItem,
}

pub(crate) fn keyring_show_messages(player: Option<&PlayerRuntime>) -> Vec<String> {
    player
        .map(PlayerRuntime::keyring_display_lines)
        .unwrap_or_else(|| vec!["Your keyring is empty.".to_string()])
}

pub(crate) fn cursor_holds_keyring(world: &World, character: &Character) -> bool {
    character
        .cursor_item
        .and_then(|item_id| world.items.get(&item_id))
        .is_some_and(|item| item.template_id == IID_KEY_RING || item.driver == IDR_KEY_RING)
}

pub(crate) fn is_runtime_keyring_candidate(item: &Item) -> bool {
    is_registered_key(item.template_id)
}

pub(crate) fn keyring_entry_to_item(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    entry: &ugaris_core::player::KeyringEntry,
) -> Item {
    if let Some(item) =
        loader.instantiate_item_template_by_id(entry.template_id, Some(character_id))
    {
        return item;
    }

    if let Some(mut item) =
        loader.instantiate_item_template_by_id(IID_PLACEHOLDER_KEY, Some(character_id))
    {
        item.template_id = entry.template_id;
        item.name = entry.name.clone();
        item.description = entry.description.clone();
        item.sprite = entry.sprite;
        item.flags = ItemFlags::from_bits_retain(entry.flags) | ItemFlags::USED;
        item.value = entry.value;
        item.driver = entry.driver;
        item.driver_data = entry.driver_data.clone();
        item.serial = entry.expire_serial;
        return item;
    }

    Item {
        id: loader.allocate_item_id(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        flags: ItemFlags::from_bits_retain(entry.flags) | ItemFlags::USED,
        sprite: entry.sprite,
        value: entry.value,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: entry.template_id,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(character_id),
        contained_in: None,
        content_id: 0,
        driver: entry.driver,
        driver_data: entry.driver_data.clone(),
        serial: entry.expire_serial,
    }
}

pub(crate) fn give_removed_keyring_entry(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    entry: &ugaris_core::player::KeyringEntry,
) -> Result<(), &'static str> {
    let mut item = keyring_entry_to_item(loader, character_id, entry);
    let item_id = item.id;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return Err("Failed to access keyring data.");
    };
    match give_item_to_character(character, &mut item, GiveItemFlags::NONE) {
        GiveItemResult::Ok => {
            world.add_item(item);
            Ok(())
        }
        GiveItemResult::Full => Err("Your inventory is full."),
        _ => {
            world.items.remove(&item_id);
            Err("Cannot remove this key here. Return to where you found it.")
        }
    }
}

pub(crate) fn apply_keyring_command(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    command: &str,
) -> Option<KeyringCommandResult> {
    let lower = command.to_ascii_lowercase();
    let rest = if lower.starts_with("#keyring") || lower.starts_with("/keyring") {
        &command[8..]
    } else {
        return None;
    };
    let rest = rest.trim();

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if !cursor_holds_keyring(world, character) {
        return Some(KeyringCommandResult {
            messages: vec![
                "You need to hold a keyring on your cursor to use this command.".to_string(),
            ],
            inventory_changed: false,
            ..Default::default()
        });
    }

    if rest.is_empty() {
        let mut messages = player.keyring_display_lines();
        if player.keyring.is_empty() {
            messages.push("Use '#keyring addall' to add all keys from inventory.".to_string());
        }
        return Some(KeyringCommandResult {
            messages,
            inventory_changed: false,
            ..Default::default()
        });
    }

    let mut words = rest.split_whitespace();
    match words.next().unwrap_or_default().to_ascii_lowercase().as_str() {
        "remove" => {
            let Some(number) = words.next().and_then(|word| word.parse::<usize>().ok()) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Usage: #keyring remove <number>".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(index) = number.checked_sub(1) else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            let Some(entry) = player.keyring.get(index).cloned() else {
                return Some(KeyringCommandResult {
                    messages: vec!["Invalid key number. Use #keyring to see the list.".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            };
            if let Err(message) = give_removed_keyring_entry(world, loader, character_id, &entry) {
                return Some(KeyringCommandResult {
                    messages: vec![message.to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            }
            player.remove_keyring_key_at(index);
            Some(KeyringCommandResult {
                messages: vec![format!("Removed {} from your keyring.", entry.name)],
                inventory_changed: true,
                ..Default::default()
            })
        }
        "addall" => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            let mut added_count = 0;
            for slot in INVENTORY_KEY_START_SLOT..character.inventory.len() {
                let Some(item_id) = character.inventory[slot] else {
                    continue;
                };
                let Some(item) = world.items.get(&item_id).cloned() else {
                    continue;
                };
                if !is_runtime_keyring_candidate(&item) {
                    continue;
                }
                if player.add_keyring_item(&item) == KeyringAddResult::Added {
                    character.inventory[slot] = None;
                    character.flags.insert(CharacterFlags::ITEMS);
                    world.items.remove(&item_id);
                    added_count += 1;
                }
            }
            let messages = if added_count > 0 {
                vec![format!("Added {added_count} keys to your keyring.")]
            } else {
                vec!["No keys found to add.".to_string()]
            };
            Some(KeyringCommandResult {
                messages,
                inventory_changed: added_count > 0,
                ..Default::default()
            })
        }
        "addallkeys" => {
            let Some(character) = world.characters.get(&character_id) else {
                return Some(KeyringCommandResult::default());
            };
            if !character
                .flags
                .intersects(CharacterFlags::GOD | CharacterFlags::STAFF)
            {
                return Some(KeyringCommandResult {
                    messages: vec!["This command requires staff privileges.".to_string()],
                    inventory_changed: false,
                    ..Default::default()
                });
            }

            let mut added_count = 0;
            for template_id in REGISTERED_KEY_IDS {
                let Some(item) = loader.instantiate_item_template_by_id(*template_id, Some(character_id))
                else {
                    continue;
                };
                if player.add_keyring_item(&item) == KeyringAddResult::Added {
                    added_count += 1;
                }
                if player.keyring.len() >= ugaris_core::player::KEYRING_MAX_KEYS {
                    break;
                }
            }

            Some(KeyringCommandResult {
                messages: vec![
                    "Adding all registered keys to keyring...".to_string(),
                    format!(
                        "Added {added_count} keys to your keyring (total: {}/{}).",
                        player.keyring.len(),
                        ugaris_core::player::KEYRING_MAX_KEYS
                    ),
                ],
                inventory_changed: false,
                ..Default::default()
            })
        }
        "auto" => {
            let enabled = !player.keyring_auto_add();
            player.set_keyring_auto_add(enabled);
            let message = if enabled {
                "Auto-add keys enabled. Keys will be automatically added to your keyring when picked up."
            } else {
                "Auto-add keys disabled. Keys will go to your inventory as normal."
            };
            Some(KeyringCommandResult {
                messages: vec![message.to_string()],
                inventory_changed: false,
                ..Default::default()
            })
        }
        _ => Some(KeyringCommandResult {
            messages: vec!["Unknown keyring subcommand. Use: #keyring, #keyring remove <n>, #keyring addall, #keyring auto".to_string()],
            inventory_changed: false,
            ..Default::default()
        }),
    }
}

pub(crate) fn apply_keyring_add_cursor_item(
    world: &mut World,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    key_item_id: ItemId,
) -> KeyringAddApplyResult {
    let Some(player) = player else {
        return KeyringAddApplyResult::MissingPlayer;
    };
    let Some(key_item) = world.items.get(&key_item_id) else {
        return KeyringAddApplyResult::MissingCursorItem;
    };
    if !is_runtime_keyring_candidate(key_item) {
        return KeyringAddApplyResult::NotAKey;
    }
    let key_snapshot: Item = key_item.clone();

    match player.add_keyring_item(&key_snapshot) {
        KeyringAddResult::Added => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return KeyringAddApplyResult::MissingPlayer;
            };
            let Some(key_item) = world.items.get_mut(&key_item_id) else {
                return KeyringAddApplyResult::MissingCursorItem;
            };
            if character.cursor_item != Some(key_item_id) || !consume_item(character, key_item) {
                return KeyringAddApplyResult::MissingCursorItem;
            }
            KeyringAddApplyResult::Added {
                key_name: key_snapshot.name,
            }
        }
        KeyringAddResult::Duplicate => KeyringAddApplyResult::Duplicate,
        KeyringAddResult::Full => KeyringAddApplyResult::Full,
    }
}

pub(crate) fn apply_keyring_auto_add_pickup(
    world: &mut World,
    player: Option<&mut PlayerRuntime>,
    character_id: CharacterId,
    key_item_id: ItemId,
) -> Option<KeyringAutoAddPickupResult> {
    let player = player?;
    if !player.keyring_auto_add() {
        return None;
    }
    let key_item = world.items.get(&key_item_id)?;
    if !is_runtime_keyring_candidate(key_item) {
        return None;
    }
    let key_snapshot: Item = key_item.clone();

    Some(match player.add_keyring_item(&key_snapshot) {
        KeyringAddResult::Added => {
            let Some(character) = world.characters.get_mut(&character_id) else {
                return Some(KeyringAutoAddPickupResult::MissingPlayer);
            };
            let Some(key_item) = world.items.get_mut(&key_item_id) else {
                return Some(KeyringAutoAddPickupResult::MissingCursorItem);
            };
            if character.cursor_item != Some(key_item_id) || !consume_item(character, key_item) {
                return Some(KeyringAutoAddPickupResult::MissingCursorItem);
            }
            KeyringAutoAddPickupResult::Added {
                key_name: key_snapshot.name,
            }
        }
        KeyringAddResult::Duplicate => KeyringAutoAddPickupResult::Duplicate {
            key_name: key_snapshot.name,
        },
        KeyringAddResult::Full => KeyringAutoAddPickupResult::Full {
            key_name: key_snapshot.name,
        },
    })
}
