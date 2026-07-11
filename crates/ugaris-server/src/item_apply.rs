use super::*;

pub(crate) fn arkhata_stopwatch_feedback(player: &PlayerRuntime, realtime_seconds: u64) -> String {
    if player.arkhata_clerk_state() != 5 {
        return "#92 ".to_string();
    }

    let diff = ARKHATA_CLERK_TIME_SECONDS - realtime_seconds.min(i32::MAX as u64) as i32
        + player.arkhata_clerk_time_seconds();
    if diff > 0 {
        format!("#91 Time: {} Astonian Minutes", diff / 5)
    } else {
        "#92 YOU FAILED!".to_string()
    }
}

pub(crate) fn special_potion_fun_message(
    world: &World,
    character_id: CharacterId,
    kind: u8,
) -> Option<String> {
    let character = world.characters.get(&character_id);
    let name = character
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    let his = legacy_hisname(character);
    let him = legacy_himname(character);

    match kind {
        8 => Some(format!(
            "You see {name} hit {him}self on the head with a mug."
        )),
        9 => Some(format!(
            "{name} suddenly starts singing in a slurred tongue... Dogs start howling..."
        )),
        10 => Some(format!(
            "{name}'s hair suddenly shoots up as if hit by electricity."
        )),
        11 => Some(format!("{name} seems to be enjoying a fine ale.")),
        12 => Some(format!("{name} drinks a delicious apple juice.")),
        13 => Some(format!("{name} feels refreshed.")),
        14 => Some(format!("{name} cracks {his} strong knuckles.")),
        15 => Some(format!("{name} starts frothing at the mouth.")),
        _ => None,
    }
}

pub(crate) fn legacy_hisname(character: Option<&Character>) -> &'static str {
    match character.map(|character| character.flags) {
        Some(flags) if flags.contains(CharacterFlags::MALE) => "his",
        Some(flags) if flags.contains(CharacterFlags::FEMALE) => "her",
        _ => "its",
    }
}

pub(crate) fn legacy_himname(character: Option<&Character>) -> &'static str {
    match character.map(|character| character.flags) {
        Some(flags) if flags.contains(CharacterFlags::MALE) => "him",
        Some(flags) if flags.contains(CharacterFlags::FEMALE) => "her",
        _ => "it",
    }
}

pub(crate) fn lollipop_area_message(world: &World, character_id: CharacterId) -> String {
    let name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    format!("{name} licks a lollipop.")
}

pub(crate) fn potion_area_message(world: &World, character_id: CharacterId) -> String {
    let name = world
        .characters
        .get(&character_id)
        .map(|character| character.name.as_str())
        .unwrap_or("Someone");
    format!("{name} drinks a potion.")
}

pub(crate) fn apply_empty_potion_drink(
    world: &mut World,
    loader: &mut ZoneLoader,
    item_id: ItemId,
    character_id: CharacterId,
    empty_kind: u8,
) -> bool {
    let template = format!("empty_potion{empty_kind}");
    let Ok(mut empty_item) = loader.instantiate_item_template(&template, Some(character_id)) else {
        return false;
    };

    let Some(mut potion) = world.items.remove(&item_id) else {
        return false;
    };
    let Some(character) = world.characters.get_mut(&character_id) else {
        world.items.insert(item_id, potion);
        return false;
    };
    if potion.carried_by != Some(character_id) {
        world.items.insert(item_id, potion);
        return false;
    }

    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;

    character.hp = capped_resource(
        character.hp,
        potion.driver_data.get(1).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Hp),
    );
    character.mana = capped_resource(
        character.mana,
        potion.driver_data.get(2).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Mana),
    );
    character.endurance = capped_resource(
        character.endurance,
        potion.driver_data.get(3).copied().unwrap_or_default(),
        max_character_value(character, CharacterValue::Endurance),
    );

    if !replace_item_in_character(character, &mut potion, &mut empty_item) {
        character.hp = old_hp;
        character.mana = old_mana;
        character.endurance = old_endurance;
        world.items.insert(item_id, potion);
        return false;
    }
    world.add_item(empty_item);
    true
}

pub(crate) fn capped_resource(current: i32, added_units: u8, max_units: i32) -> i32 {
    (current + i32::from(added_units) * POWERSCALE).min(max_units * POWERSCALE)
}

pub(crate) fn max_character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn christmas_pop_inspection_messages() -> [&'static str; 4] {
    [
        "You notice a tiny inscription on the magical lollipop. It reads:",
        "\"Place me under a Christmas tree to receive a special gift from the gods.\"",
        "In shimmering letters below, you see:",
        "\"Each tree grants but one wish per adventurer.\"",
    ]
}

pub(crate) fn is_torch_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_TORCH)
}

pub(crate) fn is_timed_potion_source_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| matches!(item.driver, IDR_BEYONDPOTION | IDR_FLASK))
}

pub(crate) fn is_no_potion_area_blocked_item(world: &World, item_id: ItemId) -> bool {
    world.items.get(&item_id).is_some_and(|item| {
        matches!(
            item.driver,
            ugaris_core::item_driver::IDR_POTION
                | IDR_BEYONDPOTION
                | IDR_SPECIAL_POTION
                | IDR_FLASK
        )
    })
}

pub(crate) fn is_demonshrine_item(world: &World, item_id: ItemId) -> bool {
    world
        .items
        .get(&item_id)
        .is_some_and(|item| item.driver == IDR_DEMONSHRINE)
}

pub(crate) fn character_has_active_beyond_potion(world: &World, character_id: CharacterId) -> bool {
    world
        .characters
        .get(&character_id)
        .is_some_and(|character| {
            character.inventory[12..30].iter().any(|item_id| {
                item_id
                    .and_then(|item_id| world.items.get(&item_id))
                    .is_some_and(|item| item.driver == ugaris_core::spell::IDR_POTION_SP)
            })
        })
}

pub(crate) fn timer_outcome_feedback(
    outcomes: &[ugaris_core::item_driver::ItemDriverOutcome],
) -> Vec<(CharacterId, String)> {
    outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater {
                character_id,
                ..
            } => Some((*character_id, TORCH_HISS_MESSAGE.to_string())),
            ugaris_core::item_driver::ItemDriverOutcome::TorchExpired {
                character_id,
                item_name,
                ..
            } => Some((
                *character_id,
                format!("Your {} expired.", outcome_item_name_text(item_name)),
            )),
            _ => None,
        })
        .collect()
}

pub(crate) fn outcome_item_name_text(bytes: &[u8]) -> String {
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

pub(crate) fn item_driver_context_for_request(
    world: &World,
    player: Option<&PlayerRuntime>,
    request: &ugaris_core::item_driver::ItemDriverRequest,
) -> ugaris_core::item_driver::ItemDriverContext {
    let ugaris_core::item_driver::ItemDriverRequest::Driver {
        driver,
        item_id,
        character_id,
        ..
    } = request
    else {
        return ugaris_core::item_driver::ItemDriverContext::default();
    };
    if *driver == ugaris_core::item_driver::IDR_ASSEMBLE
        || *driver == ugaris_core::item_driver::IDR_PALACEKEY
        || *driver == ugaris_core::item_driver::IDR_FLASK
        || *driver == ugaris_core::item_driver::IDR_ARKHATA
    {
        let cursor_item = world
            .characters
            .get(character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| world.items.get(&cursor_item_id));
        return ugaris_core::item_driver::ItemDriverContext {
            door_key: None,
            cursor_template_id: cursor_item.map(|item| item.template_id),
            cursor_driver: cursor_item.map(|item| item.driver),
            cursor_sprite: cursor_item.map(|item| item.sprite),
            cursor_drdata0: cursor_item.and_then(|item| item.driver_data.first().copied()),
            hour: world.date.hour as u8,
            fullmoon: world.date.fullmoon,
            newmoon: world.date.newmoon,
            solstice: world.date.solstice,
            equinox: world.date.equinox,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_BONEHINT {
        let needs_init = world
            .items
            .get(item_id)
            .map(|item| item.driver_data.get(1).copied().unwrap_or_default() == 0)
            .unwrap_or(false);
        if needs_init {
            return ugaris_core::item_driver::ItemDriverContext {
                bone_hint_nr: Some((runtime_random_below(25) as f64).sqrt() as u8),
                bone_hint_pos: Some(runtime_random_below(3).max(0) as u8),
                ..ugaris_core::item_driver::ItemDriverContext::default()
            };
        }
        return ugaris_core::item_driver::ItemDriverContext::default();
    }
    if *driver == ugaris_core::item_driver::IDR_LABENTRANCE {
        return ugaris_core::item_driver::ItemDriverContext {
            lab_solved_bits: player
                .map(|player| player.lab_solved_bits)
                .unwrap_or_default(),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_STAFFER {
        return ugaris_core::item_driver::ItemDriverContext {
            rouven_state: Some(
                player
                    .map(|player| player.staffer_rouven_state())
                    .unwrap_or_default(),
            ),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_LAB3_SPECIAL {
        return ugaris_core::item_driver::ItemDriverContext {
            lab3_guard_talkstep: Some(
                player
                    .map(|player| player.legacy_lab3_guard_talkstep())
                    .unwrap_or_default(),
            ),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_WARPKEYDOOR {
        let area25_door_key = world.characters.get(character_id).and_then(|character| {
            character.inventory.iter().flatten().find_map(|item_id| {
                world.items.get(item_id).and_then(|item| {
                    (item.template_id == IID_AREA25_DOORKEY).then(|| (*item_id, item.name.clone()))
                })
            })
        });
        return ugaris_core::item_driver::ItemDriverContext {
            area25_door_key,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == IDR_WARPBONUS {
        let cursor_item = world
            .characters
            .get(character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| world.items.get(&cursor_item_id));
        let location_id = world
            .items
            .get(item_id)
            .map(|item| i32::from(item.x) + (i32::from(item.y) << 8) + (25_i32 << 16));
        let (warp_bonus_base, warp_bonus_points, warp_bonus_used_at_base) = player
            .map(|player| {
                let base = if player.warp_base > 0 {
                    player.warp_base as u32
                } else {
                    40
                };
                let used = location_id.and_then(|location_id| {
                    player
                        .warp_bonus_ids
                        .iter()
                        .position(|stored| *stored == location_id)
                        .and_then(|index| player.warp_bonus_last_used.get(index).copied())
                        .map(|used| used.max(0) as u32)
                });
                (Some(base), player.warp_points.max(0) as u32, used)
            })
            .unwrap_or((Some(40), 0, None));
        return ugaris_core::item_driver::ItemDriverContext {
            cursor_template_id: cursor_item.map(|item| item.template_id),
            cursor_drdata0: cursor_item.and_then(|item| item.driver_data.first().copied()),
            warp_bonus_base,
            warp_bonus_points,
            warp_bonus_used_at_base,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == IDR_BOOKCASE || *driver == IDR_PICKCHEST || *driver == IDR_PICKDOOR {
        let (has_area17_library_key, has_area17_lockpick, has_area17_cursor_lockpick) = world
            .characters
            .get(character_id)
            .map(|character| {
                let has_library_key = character.inventory.iter().flatten().any(|item_id| {
                    world
                        .items
                        .get(item_id)
                        .is_some_and(|item| item.template_id == IID_AREA17_LIBRARYKEY)
                });
                let has_lockpick = character.inventory.iter().flatten().any(|item_id| {
                    world
                        .items
                        .get(item_id)
                        .is_some_and(|item| item.template_id == IID_AREA17_LOCKPICK)
                });
                let has_cursor_lockpick = character.cursor_item.is_some_and(|item_id| {
                    world
                        .items
                        .get(&item_id)
                        .is_some_and(|item| item.template_id == IID_AREA17_LOCKPICK)
                });
                (has_library_key, has_lockpick, has_cursor_lockpick)
            })
            .unwrap_or_default();
        return ugaris_core::item_driver::ItemDriverContext {
            has_area17_library_key,
            has_area17_lockpick,
            has_area17_cursor_lockpick,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_RANDOMSHRINE {
        let shrine_type = world
            .items
            .get(item_id)
            .and_then(|item| item.driver_data.first().copied())
            .unwrap_or(0);
        return ugaris_core::item_driver::ItemDriverContext {
            random_shrine_already_used: shrine_type != 255
                && player.is_some_and(|player| player.has_used_random_shrine(shrine_type)),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == IDR_ISLENADOOR {
        let (islena_room_has_player, islena_present, islena_resting) =
            islena_door_room_context(world);
        return ugaris_core::item_driver::ItemDriverContext {
            islena_room_has_player,
            islena_present,
            islena_resting,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_TEUFELARENA {
        return ugaris_core::item_driver::ItemDriverContext {
            teufel_arena_roll: Some(runtime_random_below(8).max(0) as u8),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_TEUFELRATNEST {
        let teufel_ratnest_guard_active = world.items.get(item_id).is_some_and(|item| {
            (0..5).any(|slot| {
                let id_offset = 10 + slot * 2;
                let serial_offset = 20 + slot * 4;
                let character_id = item
                    .driver_data
                    .get(id_offset..id_offset + 2)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .unwrap_or_default();
                let serial = item
                    .driver_data
                    .get(serial_offset..serial_offset + 4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .unwrap_or_default();
                character_id != 0
                    && world
                        .characters
                        .get(&CharacterId(u32::from(character_id)))
                        .is_some_and(|character| {
                            character.flags.contains(CharacterFlags::USED)
                                && character.serial == serial
                        })
            })
        });
        return ugaris_core::item_driver::ItemDriverContext {
            teufel_ratnest_guard_active,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == IDR_WARMFIRE {
        let has_curse_spell = world
            .characters
            .get(character_id)
            .map(|character| {
                character.inventory.iter().flatten().any(|item_id| {
                    world
                        .items
                        .get(item_id)
                        .is_some_and(|item| item.driver == IDR_CURSE)
                })
            })
            .unwrap_or(false);
        return ugaris_core::item_driver::ItemDriverContext {
            has_curse_spell,
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver == ugaris_core::item_driver::IDR_LAB5_ITEM {
        // C `has_potion(cn)` (`lab5.c:245-259`): only meaningful for
        // `drdata[0]==11`, but harmless (and simpler) to compute
        // unconditionally, same precedent as `has_curse_spell` above.
        let has_potion = world.characters.get(character_id).is_some_and(|character| {
            let carries_potion = |item_id: &ItemId| {
                world
                    .items
                    .get(item_id)
                    .is_some_and(|item| item.driver == ugaris_core::item_driver::IDR_POTION)
            };
            character
                .inventory
                .iter()
                .skip(30)
                .flatten()
                .any(carries_potion)
                || character.cursor_item.as_ref().is_some_and(carries_potion)
        });
        let lab5_chestbox_already_opened =
            player.is_some_and(|player| player.lab5_chestbox_already_opened(item_id.0));
        return ugaris_core::item_driver::ItemDriverContext {
            has_potion,
            lab5_chestbox_already_opened,
            lab5_ritual_daemon: Some(player.map(|player| player.lab5_ritual_daemon).unwrap_or(0)),
            lab5_ritual_state: Some(player.map(|player| player.lab5_ritual_state).unwrap_or(0)),
            ..ugaris_core::item_driver::ItemDriverContext::default()
        };
    }
    if *driver != ugaris_core::item_driver::IDR_DOOR
        && *driver != ugaris_core::item_driver::IDR_EDEMONDOOR
        && *driver != ugaris_core::item_driver::IDR_INFINITE_CHEST
    {
        return ugaris_core::item_driver::ItemDriverContext::default();
    }
    let required_key_id = world
        .items
        .get(item_id)
        .map(chest_required_key_id)
        .unwrap_or_default();
    if required_key_id == 0 {
        return ugaris_core::item_driver::ItemDriverContext::default();
    }

    let door_key = if *driver == ugaris_core::item_driver::IDR_INFINITE_CHEST {
        infinite_chest_key_access(world, *character_id, required_key_id)
    } else if *driver == ugaris_core::item_driver::IDR_EDEMONDOOR {
        exact_carried_door_key_access(world, *character_id, required_key_id)
    } else {
        door_key_access(world, player, *character_id, required_key_id)
    };

    ugaris_core::item_driver::ItemDriverContext {
        door_key,
        cursor_template_id: None,
        ..ugaris_core::item_driver::ItemDriverContext::default()
    }
}

pub(crate) fn islena_door_room_context(world: &World) -> (bool, bool, bool) {
    let mut room_has_player = false;
    let mut islena_present = false;
    let mut islena_resting = false;

    for character in world.characters.values() {
        if !(138..147).contains(&character.x) || !(49..58).contains(&character.y) {
            continue;
        }
        if character.flags.contains(CharacterFlags::PLAYER) {
            room_has_player = true;
        }
        if character.driver == CDR_PALACEISLENA {
            islena_present = true;
            let max_hp = character
                .values
                .first()
                .and_then(|values| values.get(CharacterValue::Hp as usize))
                .copied()
                .unwrap_or_default() as i32
                * POWERSCALE;
            let max_mana = character
                .values
                .first()
                .and_then(|values| values.get(CharacterValue::Mana as usize))
                .copied()
                .unwrap_or_default() as i32
                * POWERSCALE;
            if character.hp < max_hp || character.mana < max_mana {
                islena_resting = true;
            }
        }
    }

    (room_has_player, islena_present, islena_resting)
}

pub(crate) fn exact_carried_door_key_access(
    world: &World,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let character = world.characters.get(&character_id)?;
    for item_id in character.inventory.iter().skip(30).flatten().copied() {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    character
        .cursor_item
        .and_then(|item_id| carried_door_key_access(world, item_id, required_key_id))
}

pub(crate) fn door_key_access(
    world: &World,
    player: Option<&PlayerRuntime>,
    character_id: CharacterId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let character = world.characters.get(&character_id)?;
    let inventory_items = character.inventory.iter().skip(30).flatten().copied();

    for item_id in inventory_items.clone() {
        if let Some(access) = carried_door_key_access(world, item_id, IID_SKELETON_KEY) {
            return Some(access);
        }
    }
    if let Some(item_id) = character.cursor_item {
        if let Some(access) = carried_door_key_access(world, item_id, IID_SKELETON_KEY) {
            return Some(access);
        }
    }
    for item_id in inventory_items {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    if let Some(item_id) = character.cursor_item {
        if let Some(access) = carried_door_key_access(world, item_id, required_key_id) {
            return Some(access);
        }
    }
    player
        .and_then(|player| player.keyring_key_name(required_key_id))
        .map(|name| ugaris_core::item_driver::DoorKeyAccess {
            key_id: required_key_id,
            name: name.to_string(),
            source: ugaris_core::item_driver::DoorKeySource::Keyring,
        })
}

pub(crate) fn carried_door_key_access(
    world: &World,
    item_id: ItemId,
    required_key_id: u32,
) -> Option<ugaris_core::item_driver::DoorKeyAccess> {
    let item = world.items.get(&item_id)?;
    (item.template_id == required_key_id).then(|| ugaris_core::item_driver::DoorKeyAccess {
        key_id: item.template_id,
        name: item.name.clone(),
        source: ugaris_core::item_driver::DoorKeySource::Carried,
    })
}

pub(crate) fn grant_money_to_cursor(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    amount: u32,
) -> bool {
    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return false;
    }
    let item_id = loader.allocate_item_id();
    let item = ugaris_core::entity::Item {
        id: item_id,
        name: "Money".to_string(),
        description: String::new(),
        flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::MONEY,
        sprite: 0,
        value: amount,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(character_id),
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: Vec::new(),
        serial: amount,
    };
    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    if character.cursor_item.is_some() {
        return false;
    }
    character.cursor_item = Some(item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);
    true
}

pub(crate) fn deposit_cursor_money_to_gold(world: &mut World, character_id: CharacterId) -> bool {
    let Some(item_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item)
    else {
        return false;
    };
    let Some(item) = world.items.get(&item_id) else {
        return false;
    };
    if !item.flags.contains(ItemFlags::MONEY) {
        return false;
    }

    let amount = item.value;
    world.items.remove(&item_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = None;
        character.gold = character.gold.saturating_add(amount);
        character.flags.insert(CharacterFlags::ITEMS);
    }
    true
}

pub(crate) fn apply_gold_client_action(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    action: &ClientAction,
) -> bool {
    match action {
        ClientAction::TakeGold { amount } => {
            if world
                .characters
                .get(&character_id)
                .and_then(|character| character.cursor_item)
                .is_some()
                && !deposit_cursor_money_to_gold(world, character_id)
            {
                return false;
            }
            let Some(character) = world.characters.get(&character_id) else {
                return false;
            };
            if *amount < 1 || *amount > character.gold {
                return true;
            }
            if grant_money_to_cursor(world, loader, character_id, *amount) {
                if let Some(character) = world.characters.get_mut(&character_id) {
                    character.gold = character.gold.saturating_sub(*amount);
                    character.flags.insert(CharacterFlags::ITEMS);
                }
            }
            true
        }
        ClientAction::DropGold => deposit_cursor_money_to_gold(world, character_id),
        _ => false,
    }
}

/// C `cl_junk_item` (`src/system/player.c:1325`): destroys the item on the
/// cursor (`ch[cn].citem`) unless it carries `IF_NOJUNK`. Clears the cursor
/// and sets `CF_ITEMS` (mirrored here by `world.destroy_item` clearing
/// `cursor_item` and inserting `CharacterFlags::ITEMS`). No response packet
/// is sent directly - the client sees the cleared cursor via the normal
/// `CF_ITEMS`-driven inventory refresh.
pub(crate) fn apply_junk_item_client_action(world: &mut World, character_id: CharacterId) -> bool {
    let Some(item_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item)
    else {
        return false;
    };
    let Some(item) = world.items.get(&item_id) else {
        return false;
    };
    if item.flags.contains(ItemFlags::NOJUNK) {
        return false;
    }

    world.destroy_item(item_id)
}
