use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_progression(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    if lower == "reset" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let (name, _) = take_legacy_alpha_name(rest.trim_start());
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };

        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        if target.values.len() < 2 {
            target
                .values
                .resize_with(2, || vec![0; CHARACTER_VALUE_NAMES.len()]);
        }
        if target.values[1].len() < CHARACTER_VALUE_NAMES.len() {
            target.values[1].resize(CHARACTER_VALUE_NAMES.len(), 0);
        }
        for index in 0..=CharacterValue::Immunity as usize {
            let cap = if index <= CharacterValue::Strength as usize {
                10
            } else {
                1
            };
            if target.values[1][index] > cap {
                target.values[1][index] = cap;
            }
        }
        for value in [CharacterValue::Rage, CharacterValue::Duration] {
            let index = value as usize;
            if target.values[1][index] > 1 {
                target.values[1][index] = 1;
            }
        }
        target.exp_used = 0;
        target.flags.insert(CharacterFlags::UPDATE);
        return ControlFlow::Break(Some(KeyringCommandResult {
            inventory_changed: target_id == character_id,
            name_changed: target_id == character_id,
            ..Default::default()
        }));
    }

    if lower == "resetgift" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let (name, area_text) = take_legacy_alpha_name(rest);
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Sorry, no one by the name {name} around.")],
                ..Default::default()
            }));
        };
        let area_id = legacy_atoi_prefix(area_text.trim_start());
        if !(0..=63).contains(&area_id) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid area ID. Must be between 0 and 63.".to_string()],
                ..Default::default()
            }));
        }

        let Some(target_player) = runtime.player_for_character_mut(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Could not retrieve player data.".to_string()],
                ..Default::default()
            }));
        };
        let was_set = target_player.xmas_tree_marked(area_id as u16);
        target_player.unmark_xmas_tree(area_id as u16);
        let target_name = world
            .characters
            .get(&target_id)
            .map(|character| character.name.as_str())
            .unwrap_or(name);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Reset gift flag for {} in area {} (was {}).",
                target_name,
                area_id,
                if was_set { "set" } else { "not set" }
            )],
            ..Default::default()
        }));
    }

    if lower.len() >= 5 && "questlog".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let name = rest.split_whitespace().next().unwrap_or_default();
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            }));
        };
        let Some(target_name) = world
            .characters
            .get(&target_id)
            .map(|character| character.name.clone())
        else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            }));
        };
        let Some(target_player) = runtime.player_for_character(target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Failed to get quest data for {target_name}")],
                ..Default::default()
            }));
        };

        let mut messages = vec![format!("Quest log for {target_name}:")];
        for (quest_id, entry) in target_player.quest_log.entries().iter().enumerate() {
            if entry.flags != 0 {
                messages.push(format!(
                    "Quest #{}: {}, Done level: {}",
                    quest_id,
                    if (entry.flags & QF_OPEN) != 0 {
                        "Open"
                    } else {
                        "Closed"
                    },
                    entry.done
                ));
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    if lower.len() >= 5 && "listitem".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let item_id = ItemId(legacy_atoi_prefix(rest).max(0) as u32);
        let Some(item) = world.items.get(&item_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid item number or item doesn't exist".to_string()],
                ..Default::default()
            }));
        };

        let mut messages = vec![
            format!("Item #{}: {}", item.id.0, item.name),
            format!("Description: {}", item.description),
            format!("Flags: 0x{:x}", item.flags.bits()),
            format!(
                "Driver: {}, ID: {}, Sprite: {}",
                item.driver, item.template_id, item.sprite
            ),
        ];
        if let Some(carried_by) = item.carried_by {
            let carrier_name = world
                .characters
                .get(&carried_by)
                .map(|character| character.name.as_str())
                .unwrap_or("Unknown");
            messages.push(format!("Carried by: {} ({})", carrier_name, carried_by.0));
        } else if item.x != 0 {
            messages.push(format!("Position: {},{}", item.x, item.y));
        }
        for n in 0..ugaris_core::entity::MAX_MODIFIERS {
            let modifier_index = item.modifier_index[n];
            if modifier_index != 0 {
                let skill_name = if modifier_index > 0 {
                    value_name(modifier_index)
                } else {
                    "unknown"
                };
                messages.push(format!(
                    "Mod #{}: {:+} to {}",
                    n, item.modifier_value[n], skill_name
                ));
            }
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    if lower.len() >= 5 && "setkarma".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let rest = rest.trim_start();
        let mut split = rest.splitn(2, char::is_whitespace);
        let name = split.next().unwrap_or_default();
        let karma_text = split.next().unwrap_or_default().trim_start();
        let karma =
            legacy_atoi_prefix(karma_text).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let Some(target_id) = find_online_character_by_name(world, name) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            }));
        };
        let Some(target) = world.characters.get_mut(&target_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!("Character {name} not found")],
                ..Default::default()
            }));
        };
        let old_karma = target.karma;
        target.karma = karma;
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Changed {}'s karma from {} to {}",
                target.name, old_karma, target.karma
            )],
            ..Default::default()
        }));
    }

    if lower == "setexpmod" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.exp_modifier;
            world.settings.exp_modifier = value;
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Global experience modifier changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            }));
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        }));
    }

    if lower == "sethardcoreexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            let old_value = world.settings.hardcore_exp_bonus;
            world.settings.hardcore_exp_bonus = value;
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            }));
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        }));
    }

    if lower == "sethardcoremilexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let value = legacy_atof_prefix(rest);
        if (0.1..=1000.0).contains(&value) {
            // C's `hardcore_military_exp_bonus` global is a single value
            // read directly by `give_military_pts`/`give_military_pts_no_npc`
            // (`tool.c:3249-3306`); stored on `world.settings` (like
            // `exp_modifier`/`hardcore_exp_bonus`) instead of `ServerRuntime`
            // so `World::give_military_pts` (`ugaris-core`, no `ServerRuntime`
            // access) can read the live-tunable value directly.
            let old_value = world.settings.hardcore_military_exp_bonus;
            world.settings.hardcore_military_exp_bonus = value;
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore military experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            }));
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Invalid value. Please specify a number between 0.1 and 1000.0".to_string(),
            ],
            ..Default::default()
        }));
    }

    if lower == "sethardcorekillexpbonus" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let value = legacy_atof_prefix(rest);
        if (1.0..=3.0).contains(&value) {
            let old_value = runtime.hardcore_kill_exp_bonus;
            runtime.hardcore_kill_exp_bonus = value;
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Hardcore kill experience bonus changed from {old_value:.2} to {value:.2}"
                )],
                ..Default::default()
            }));
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec!["Invalid value. Please specify a number between 1.0 and 3.0".to_string()],
            ..Default::default()
        }));
    }

    if lower.len() >= 5 && "listchars".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let mut character_ids: Vec<_> = world.characters.keys().copied().collect();
        character_ids.sort_by_key(|id| id.0);

        let mut count = 0;
        let mut players = 0;
        let mut npcs = 0;
        let mut messages = vec!["Active characters:".to_string()];
        for id in character_ids {
            let Some(character) = world.characters.get(&id) else {
                continue;
            };
            if character.flags.is_empty() {
                continue;
            }
            count += 1;
            if character.flags.contains(CharacterFlags::PLAYER) {
                players += 1;
                messages.push(format!(
                    "Player: {:3} - {} (L{})",
                    id.0, character.name, character.level
                ));
            } else {
                npcs += 1;
                if count < 50 {
                    messages.push(format!(
                        "NPC:    {:3} - {} (L{}, D:{})",
                        id.0, character.name, character.level, character.driver
                    ));
                }
            }
        }
        messages.push(format!(
            "Total: {count} characters ({players} players, {npcs} NPCs)"
        ));
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/clearmerchantstores <id>` (`command.c:7510-7538`), `CF_GOD`-gated
    // (`cmdcmp(ptr, "clearmerchantstores", 10)`). Resets an online
    // merchant's inventory to empty and its gold to the default starting
    // amount (`ch[merchant_cn].gold = 10000`), matching C's
    // "Default starting gold" comment verbatim. Unlike C, which destroys
    // each carried item entity one at a time (`remove_item_char`/
    // `destroy_item` over `it[]`), the Rust `MerchantStore.wares` slots own
    // their `Item` data directly (no separate item-table entries to free),
    // so clearing is just overwriting every slot with `None`.
    if lower.len() >= 10 && "clearmerchantstores".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let merchant_id = CharacterId(legacy_atoi_prefix(rest.trim_start()).max(0) as u32);
        let Some(merchant) = world.characters.get(&merchant_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            }));
        };
        if merchant.driver != CDR_MERCHANT {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            }));
        }
        let merchant_name = merchant.name.clone();

        world.ensure_merchant_store(merchant_id);
        let Some(store) = world.merchant_stores.get_mut(&merchant_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid merchant ID or not a merchant character".to_string()],
                ..Default::default()
            }));
        };
        store.gold = 10_000;
        for ware in store.wares.iter_mut() {
            *ware = None;
        }

        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Merchant {} (ID: {}) inventory cleared and gold reset",
                merchant_name, merchant_id.0
            )],
            clear_merchant_store_requested: Some(merchant_id),
            ..Default::default()
        }));
    }

    // C `/checksanity` (`command.c:7443-7457`), `CF_GOD`-gated
    // (`cmdcmp(ptr, "checksanity", 5)`). Runs the full self-healing
    // `consistency_check_*` sweep (`World::consistency_check`, see
    // `world/consistency.rs`'s module doc comment) and reports the same
    // four aggregate error counts C does. C's per-anomaly `elog` console
    // lines aren't reproduced (see that module's doc comment for the
    // established untracked-console-side-effect convention).
    if lower.len() >= 5 && "checksanity".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }

        let report = world.consistency_check();
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Running consistency checks...".to_string(),
                format!("Item errors: {}", report.item_errors),
                format!("Map errors: {}", report.map_errors),
                format!("Character errors: {}", report.char_errors),
                format!("Container errors: {}", report.container_errors),
                "Consistency check complete".to_string(),
            ],
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}
