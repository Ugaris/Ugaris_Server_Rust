use super::*;

pub(crate) fn spawn_edemon_gate_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    template: &str,
    slot: usize,
    x: u16,
    y: u16,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((character, inventory_items)) =
        loader.instantiate_character_template(template, character_id)
    else {
        return false;
    };
    let serial = character.serial;
    if !world.spawn_character(character, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_edemon_gate_spawn_result(item_id, slot, character_id, serial)
}

/// C `respawn_callback` from `src/system/death.c`: recreate the template
/// character at its registered spawn tile, initialize resources, and face
/// right-down. Returns `false` when the tile is blocked so the caller can
/// schedule the legacy ten-second retry.
pub(crate) fn respawn_npc_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    request: &ugaris_core::world::NpcRespawnRequest,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(&request.template_key, character_id)
    else {
        return false;
    };
    character.dir = ugaris_core::direction::Direction::RightDown as u8;
    character.hp = i32::from(character.values[0][ugaris_core::entity::CharacterValue::Hp as usize])
        * ugaris_core::entity::POWERSCALE;
    character.endurance =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Endurance as usize])
            * ugaris_core::entity::POWERSCALE;
    character.mana =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Mana as usize])
            * ugaris_core::entity::POWERSCALE;
    character.lifeshield = character.lifeshield.max(0);
    character.rest_x = request.x;
    character.rest_y = request.y;
    if !world.spawn_character(character, usize::from(request.x), usize::from(request.y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    true
}

pub(crate) fn spawn_chestspawn_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    template: &str,
    x: u16,
    y: u16,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(template, character_id)
    else {
        return false;
    };
    character.dir = ugaris_core::direction::Direction::RightDown as u8;
    character.hp = i32::from(character.values[0][ugaris_core::entity::CharacterValue::Hp as usize])
        * ugaris_core::entity::POWERSCALE;
    character.endurance =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Endurance as usize])
            * ugaris_core::entity::POWERSCALE;
    character.mana =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Mana as usize])
            * ugaris_core::entity::POWERSCALE;
    character
        .flags
        .remove(ugaris_core::entity::CharacterFlags::RESPAWN);
    if !world.spawn_character(character, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_chestspawn_spawn_result(item_id, character_id, 0)
}

pub(crate) fn spawn_warp_trial_fighter(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    template: &str,
    x: u16,
    y: u16,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut fighter, inventory_items)) =
        loader.instantiate_character_template(template, character_id)
    else {
        return false;
    };
    fighter.dir = Direction::RightDown as u8;
    fighter.hp = i32::from(fighter.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    fighter.endurance =
        i32::from(fighter.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    fighter.mana = i32::from(fighter.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    fighter.lifeshield =
        i32::from(fighter.values[0][CharacterValue::MagicShield as usize]) * POWERSCALE;

    if !world.spawn_character(fighter, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    true
}

pub(crate) fn spawn_swampspawn_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    template: &str,
    x: u16,
    y: u16,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(template, character_id)
    else {
        return false;
    };
    character.rest_x = x;
    character.rest_y = y;
    character.dir = ugaris_core::direction::Direction::RightDown as u8;
    character.hp = i32::from(character.values[0][ugaris_core::entity::CharacterValue::Hp as usize])
        * ugaris_core::entity::POWERSCALE;
    character.endurance =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Endurance as usize])
            * ugaris_core::entity::POWERSCALE;
    character.mana =
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Mana as usize])
            * ugaris_core::entity::POWERSCALE;
    let serial = character.serial;
    if !world.spawn_character(character, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_swampspawn_spawn_result(item_id, character_id, serial)
}

pub(crate) fn spawn_teufel_ratnest_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    level: u16,
    template: &str,
) -> bool {
    let Some((_x, _y, slot, wave_increase)) = teufel_ratnest_spawn_slot(world, item_id, level)
    else {
        return false;
    };

    let character_id = runtime.allocate_character_id();
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(template, character_id)
    else {
        return false;
    };
    character.dir = Direction::RightDown as u8;
    character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    character.endurance =
        i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    character.lifeshield =
        i32::from(character.values[0][CharacterValue::MagicShield as usize]) * POWERSCALE;
    character.flags.insert(CharacterFlags::NONOTIFY);
    apply_teufel_ratnest_random_suffix(&mut character, runtime_random_below);
    let serial = character.serial;

    let Some((placed_x, placed_y)) = world.spawn_character_from_item_drop(character, item_id)
    else {
        return false;
    };
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.rest_x = placed_x;
        character.rest_y = placed_y;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    apply_teufel_ratnest_spawn_result(world, item_id, slot, character_id, serial, wave_increase)
}

pub(crate) fn teufel_ratnest_spawn_slot(
    world: &mut World,
    item_id: ItemId,
    level: u16,
) -> Option<(u16, u16, usize, bool)> {
    let (x, y) = world.items.get(&item_id).map(|item| (item.x, item.y))?;
    for slot in 0..5 {
        let (stored_id, stored_serial) = world.items.get(&item_id).map(|item| {
            let id_offset = 10 + slot * 2;
            let serial_offset = 20 + slot * 4;
            let stored_id = item
                .driver_data
                .get(id_offset..id_offset + 2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                .unwrap_or_default();
            let stored_serial = item
                .driver_data
                .get(serial_offset..serial_offset + 4)
                .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                .unwrap_or_default();
            (stored_id, stored_serial)
        })?;
        if stored_id == 0 {
            return Some((x, y, slot, true));
        }
        let character_id = CharacterId(u32::from(stored_id));
        let live = world
            .characters
            .get(&character_id)
            .is_some_and(|character| {
                character.flags.contains(CharacterFlags::USED) && character.serial == stored_serial
            });
        if !live {
            return Some((x, y, slot, true));
        }
        if world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.level > u32::from(level))
        {
            let _ = world.remove_character(character_id);
            return Some((x, y, slot, false));
        }
    }
    None
}

pub(crate) fn apply_teufel_ratnest_spawn_result(
    world: &mut World,
    item_id: ItemId,
    slot: usize,
    character_id: CharacterId,
    serial: u32,
    wave_increase: bool,
) -> bool {
    let Some(item) = world.items.get_mut(&item_id) else {
        return false;
    };
    item.driver_data.resize(40, 0);
    let id_offset = 10 + slot * 2;
    let serial_offset = 20 + slot * 4;
    item.driver_data[id_offset..id_offset + 2]
        .copy_from_slice(&(character_id.0 as u16).to_le_bytes());
    item.driver_data[serial_offset..serial_offset + 4].copy_from_slice(&serial.to_le_bytes());
    if wave_increase {
        let wave = u16::from_le_bytes([item.driver_data[0], item.driver_data[1]]);
        if wave < 50_000 {
            item.driver_data[0..2].copy_from_slice(&wave.saturating_add(10).to_le_bytes());
        }
    }
    true
}

pub(crate) fn apply_teufel_ratnest_random_suffix(
    character: &mut Character,
    mut random_below: impl FnMut(i32) -> i32,
) {
    let Some((value, name_suffix, description_suffix)) = (match random_below(20) {
        0 => Some((CharacterValue::Attack, " *A", " Increased Attack.")),
        1 => Some((CharacterValue::Parry, " *P", " Increased Parry.")),
        2 => Some((CharacterValue::Freeze, " *R", " Increased Freeze.")),
        3 => Some((CharacterValue::Flash, " *F", " Increased Flash.")),
        4 => Some((CharacterValue::Immunity, " *I", " Increased Immunity.")),
        _ => None,
    }) else {
        return;
    };

    let amount = random_below(10).clamp(0, 9) as i16 + 7;
    character.values[1][value as usize] =
        character.values[1][value as usize].saturating_add(amount);
    character.name.push_str(name_suffix);
    character.description.push_str(description_suffix);
    character.flags.insert(CharacterFlags::UPDATE);
}

pub(crate) fn spawn_fdemon_gate_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    level: u8,
    slot: usize,
    x: u16,
    y: u16,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let template = format!("fdemon{}s", level);
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    character.rest_x = x;
    character.rest_y = y;
    character.dir = ugaris_core::direction::Direction::RightDown as u8;
    let serial = character.serial;
    if !world.spawn_character(character, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_fdemon_gate_spawn_result(item_id, slot, character_id, serial)
}

pub(crate) fn spawn_lq_npc_character(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    request: &ugaris_core::world::LqNpcSpawnRequest,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let template = format!("lq_{}", request.basename);
    let Ok((mut character, mut inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    character.driver = CDR_LQNPC;
    character.rest_x = request.x;
    character.rest_y = request.y;
    character.dir = ugaris_core::direction::Direction::RightDown as u8;
    character.level = u32::from(request.level);
    if request.mode == b'n' {
        character
            .flags
            .insert(CharacterFlags::IMMORTAL | CharacterFlags::NOATTACK);
    }
    if !request.name.is_empty() {
        character.name = request.name.clone();
    }
    if !request.description.is_empty() {
        character.description = request.description.clone();
    }
    apply_lq_raise(&mut character, request.level);
    add_lq_statboost_items(&mut character, loader, &mut inventory_items);
    add_lq_equipment_items(&mut character, loader, &mut inventory_items);
    character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    character.endurance =
        i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    let serial = character.serial;
    if !world.spawn_character(character, usize::from(request.x), usize::from(request.y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.apply_lq_npc_spawn_result(request.slot, character_id, serial)
}

pub(crate) fn add_lq_statboost_items(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
) {
    let boost = (character.level / 5 + 1).min(i16::MAX as u32) as i16;
    let stat_boost = (character.level / 3 + 1).min(i16::MAX as u32) as i16;

    let weapon_skill = if lq_base_value(character, CharacterValue::Dagger) != 0 {
        CharacterValue::Dagger
    } else if lq_base_value(character, CharacterValue::Staff) != 0 {
        CharacterValue::Staff
    } else if lq_base_value(character, CharacterValue::Sword) != 0 {
        CharacterValue::Sword
    } else if lq_base_value(character, CharacterValue::TwoHand) != 0 {
        CharacterValue::TwoHand
    } else {
        CharacterValue::Hand
    };
    let mut warrior_modifiers = vec![(weapon_skill, boost)];
    if lq_base_value(character, CharacterValue::Attack) != 0 {
        warrior_modifiers.extend([
            (CharacterValue::Attack, boost),
            (CharacterValue::Parry, boost),
            (CharacterValue::Tactics, boost),
        ]);
    }
    if lq_base_value(character, CharacterValue::Warcry) != 0 {
        warrior_modifiers.push((CharacterValue::Warcry, boost));
    }
    add_lq_spell_item(character, loader, inventory_items, &warrior_modifiers);

    let mut mage_modifiers = Vec::new();
    for value in [
        CharacterValue::Bless,
        CharacterValue::Light,
        CharacterValue::Fireball,
        CharacterValue::MagicShield,
        CharacterValue::Freeze,
    ] {
        if lq_base_value(character, value) != 0 {
            mage_modifiers.push((value, boost));
        }
    }
    if !mage_modifiers.is_empty() {
        add_lq_spell_item(character, loader, inventory_items, &mage_modifiers);
    }

    add_lq_spell_item(
        character,
        loader,
        inventory_items,
        &[
            (CharacterValue::Immunity, boost),
            (CharacterValue::Wisdom, stat_boost),
            (CharacterValue::Intelligence, stat_boost),
            (CharacterValue::Agility, stat_boost),
            (CharacterValue::Strength, stat_boost),
        ],
    );
}

pub(crate) fn add_lq_equipment_items(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
) {
    let mut weapon_template = None;
    for (value, prefix) in [
        (CharacterValue::Dagger, "dagger"),
        (CharacterValue::Staff, "staff"),
        (CharacterValue::Sword, "sword"),
        (CharacterValue::TwoHand, "twohand"),
    ] {
        let base = lq_base_value(character, value);
        if base != 0 {
            weapon_template = Some(format!("{}{}q1", prefix, lq_equipment_tier(base)));
        }
    }
    if let Some(template) = weapon_template {
        set_lq_equipment_item(
            character,
            loader,
            inventory_items,
            worn_slot::RIGHT_HAND,
            &template,
        );
    }

    let armor_base = lq_base_value(character, CharacterValue::ArmorSkill);
    if armor_base != 0 {
        let tier = lq_equipment_tier(armor_base);
        for (slot, prefix) in [
            (worn_slot::HEAD, "helmet"),
            (worn_slot::BODY, "armor"),
            (worn_slot::LEGS, "leggings"),
            (worn_slot::ARMS, "sleeves"),
        ] {
            set_lq_equipment_item(
                character,
                loader,
                inventory_items,
                slot,
                &format!("{}{}q1", prefix, tier),
            );
        }
    }
}

pub(crate) fn set_lq_equipment_item(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
    slot: usize,
    template: &str,
) -> bool {
    let Ok(item) = loader.instantiate_item_template(template, Some(character.id)) else {
        return false;
    };
    if let Some(previous_id) = character.inventory[slot] {
        inventory_items.retain(|item| item.id != previous_id);
    }
    character.inventory[slot] = Some(item.id);
    inventory_items.push(item);
    true
}

pub(crate) fn lq_equipment_tier(base: i16) -> i16 {
    (base / 10 + 1).min(10)
}

pub(crate) fn apply_lq_raise(character: &mut Character, level: u16) {
    let spend = level2exp(u32::from(level) + 2).saturating_sub(1);
    let sum: i32 = character.values[1]
        .iter()
        .enumerate()
        .filter_map(|(value, &amount)| {
            (!lq_raise_skips_value(value) && amount != 0).then_some(i32::from(amount))
        })
        .sum();
    if sum <= 0 {
        character.exp = 0;
        character.exp_used = 0;
        character.level = 1;
        return;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    for value in 0..CHARACTER_VALUE_NAMES.len() {
        if lq_raise_skips_value(value) || character.values[1][value] == 0 {
            continue;
        }
        let cost =
            (f64::from(spend) / f64::from(sum) * f64::from(character.values[1][value])) as i32;
        let raised =
            legacy_cost_to_skill(value, cost, seyan).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        character.values[1][value] = raised;
        character.values[0][value] = raised;
    }
    character.exp = legacy_calc_exp_used(character);
    character.exp_used = character.exp;
    character.level = exp2level(character.exp);
}

pub(crate) fn lq_raise_skips_value(value: usize) -> bool {
    value == CharacterValue::Profession as usize
        || value == CharacterValue::Cold as usize
        || value == CharacterValue::Demon as usize
        || value == CharacterValue::Speed as usize
        || value == CharacterValue::Light as usize
        || value == CharacterValue::Weapon as usize
        || value == CharacterValue::Armor as usize
}

pub(crate) fn legacy_cost_to_skill(value: usize, cost: i32, seyan: bool) -> i32 {
    let mut sum = 0_i32;
    for n in (legacy_skill_start(value) + 1)..200 {
        sum += legacy_raise_cost(value, n, seyan) as i32;
        if sum > cost {
            return n - 1;
        }
    }
    199
}

pub(crate) fn add_lq_spell_item(
    character: &mut Character,
    loader: &mut ZoneLoader,
    inventory_items: &mut Vec<Item>,
    modifiers: &[(CharacterValue, i16)],
) -> bool {
    let Some(slot) = (12..30).find(|slot| character.inventory[*slot].is_none()) else {
        return false;
    };
    let Ok(mut item) = loader.instantiate_item_template("lqx_spell", Some(character.id)) else {
        return false;
    };
    for (index, (value, amount)) in modifiers.iter().take(5).enumerate() {
        item.modifier_index[index] = *value as i16;
        item.modifier_value[index] = *amount;
        add_lq_effective_value(character, *value, *amount);
    }
    character.inventory[slot] = Some(item.id);
    inventory_items.push(item);
    true
}

pub(crate) fn add_lq_effective_value(
    character: &mut Character,
    value: CharacterValue,
    amount: i16,
) {
    if let Some(slot) = character
        .values
        .get_mut(0)
        .and_then(|values| values.get_mut(value as usize))
    {
        *slot = slot.saturating_add(amount);
    }
}

pub(crate) fn lq_base_value(character: &Character, value: CharacterValue) -> i16 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default()
}

/// C `enter_test`'s `room_start[]` (`src/system/gatekeeper.c:317`): the
/// seven candidate private-room top-left corners, tried in order.
pub(crate) const GATE_TEST_ROOM_STARTS: [(u16, u16); 7] = [
    (186, 196),
    (194, 196),
    (202, 196),
    (178, 212),
    (186, 212),
    (194, 212),
    (202, 212),
];

/// C `enter_test`'s `switch (class)` template pick (`gatekeeper.c:
/// 245-259`): classes `7` (Arch-Seyan'Du) and `8` (Seyan'Du) share the
/// same `gatekeeper_s` opponent template.
fn gate_test_opponent_template(class: i32) -> Option<&'static str> {
    match class {
        5 => Some("gatekeeper_w"),
        6 => Some("gatekeeper_m"),
        7 | 8 => Some("gatekeeper_s"),
        _ => None,
    }
}

/// C `enter_test`'s success tail (`gatekeeper.c:392-407`) plus
/// `enter_room` (`gatekeeper.c:227-303`): `take_money`, then search the
/// seven private rooms for one that is empty, spawn the class-appropriate
/// opponent inside it (`create_char`/`drop_char`), and teleport+reset the
/// player (`World::gate_finish_enter_room`). Refunds the entry fee and
/// sends the "gatekeeper is busy" notice if every room is occupied,
/// matching C exactly. Called from `apply_gate_welcome_events` on
/// `GateWelcomeOutcomeEvent::EnterTestReady`, since (unlike the rest of
/// `world/gatekeeper.rs`) this needs `ZoneLoader::instantiate_character_template`
/// - see that module's doc comment.
pub(crate) fn gate_enter_test_spawn_room(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    player_id: CharacterId,
    class: i32,
) -> bool {
    const GATE_TEST_PRICE: u32 = 100 * 100;

    let Some(template) = gate_test_opponent_template(class) else {
        return false;
    };

    if !world.gate_take_money(player_id, GATE_TEST_PRICE) {
        world.queue_system_text(player_id, "Thou canst pay the price of 100G.");
        return false;
    }

    for (xs, ys) in GATE_TEST_ROOM_STARTS {
        if !world.gate_room_is_clear(xs, ys) {
            continue;
        }

        let character_id = runtime.allocate_character_id();
        let Ok((mut opponent, inventory_items)) =
            loader.instantiate_character_template(template, character_id)
        else {
            continue;
        };
        opponent.dir = Direction::RightDown as u8;
        opponent.hp = i32::from(opponent.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        opponent.endurance =
            i32::from(opponent.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        opponent.mana = i32::from(opponent.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
        // C `ch[co].tmpx/tmpy` (`gatekeeper.c:274-275`): the opponent's
        // "return to post" coordinates, consumed once `gate_fight_driver`
        // is ported. No dedicated `tmpx`/`tmpy` field exists on
        // `Character` yet, so `rest_x`/`rest_y` (already reused for this
        // purpose by other NPC spawns, e.g. `respawn_npc_character`) stand
        // in.
        opponent.rest_x = xs + 4;
        opponent.rest_y = ys + 13;
        // C `notify_char(co, NT_NPC, NTID_GATEKEEPER, cn, 0)`
        // (`gatekeeper.c:277`).
        opponent.push_driver_message(NT_NPC, NTID_GATEKEEPER, player_id.0 as i32, 0);

        if !world.spawn_character(opponent, usize::from(xs + 4), usize::from(ys + 13)) {
            continue;
        }
        for item in inventory_items {
            world.items.insert(item.id, item);
        }

        if world.gate_finish_enter_room(player_id, xs, ys) {
            if let Some(player) = runtime.player_for_character_mut(player_id) {
                player.gate_target_class = class;
                player.gate_step = 1;
            }
            return true;
        }

        world.remove_character(character_id);
    }

    world.gate_give_money_silent(player_id, GATE_TEST_PRICE);
    world.queue_system_text(
        player_id,
        "Sorry, the gatekeeper is busy at the moment. Please come back later.",
    );
    false
}
