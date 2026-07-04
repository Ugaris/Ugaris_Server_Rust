use super::*;

pub(crate) fn login_character(
    character_id: CharacterId,
    login: &LoginBlock,
    area_id: u16,
    spawn_x: usize,
    spawn_y: usize,
) -> Character {
    let mut values = Character::empty_values();
    set_character_value(&mut values, CharacterValue::Hp, 50);
    set_character_value(&mut values, CharacterValue::Endurance, 50);
    set_character_value(&mut values, CharacterValue::Mana, 50);
    set_character_value(&mut values, CharacterValue::Speed, 50);

    Character {
        id: character_id,
        serial: character_id.0,
        name: login.name.clone(),
        description: String::new(),
        template_key: String::new(),
        respawn_ticks: 0,
        merchant: None,
        flags: CharacterFlags::USED | CharacterFlags::PLAYER | CharacterFlags::ALIVE,
        sprite: 1,
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
        staff_code: String::new(),
        speed_mode: SpeedMode::Normal,
        x: 0,
        y: 0,
        rest_area: area_id,
        rest_x: spawn_x as u16,
        rest_y: spawn_y as u16,
        tox: 0,
        toy: 0,
        dir: 0,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 50 * POWERSCALE,
        mana: 50 * POWERSCALE,
        endurance: 50 * POWERSCALE,
        lifeshield: 0,
        level: 1,
        exp: 0,
        exp_used: 0,
        military_points: 0,
        military_normal_exp: 0,
        gold: 0,
        karma: 0,
        creation_time: 0,
        saves: 0,
        got_saved: 0,
        deaths: 0,
        regen_ticker: 0,
        last_regen: 0,
        cursor_item: None,
        current_container: None,
        values,
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
        driver_memory: ugaris_core::character_driver::DriverMemory::default(),
        class: 0,
    }
}

pub(crate) fn login_character_from_template(
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    login: &LoginBlock,
    area_id: u16,
    spawn_x: usize,
    spawn_y: usize,
) -> Result<(Character, Vec<ugaris_core::entity::Item>), ugaris_core::zone::ZoneError> {
    let (mut character, items) =
        loader.instantiate_character_template(DEFAULT_PLAYER_TEMPLATE, character_id)?;
    character.name = login.name.clone();
    character.description.clear();
    character
        .flags
        .insert(CharacterFlags::USED | CharacterFlags::PLAYER | CharacterFlags::ALIVE);
    if DEFAULT_PLAYER_TEMPLATE.starts_with("seyan") {
        character
            .flags
            .insert(CharacterFlags::WARRIOR | CharacterFlags::MAGE);
    }
    character.rest_area = area_id;
    character.rest_x = spawn_x as u16;
    character.rest_y = spawn_y as u16;
    character.level = character.level.max(1);
    Ok((character, items))
}

pub(crate) fn legacy_questlog_payload(player: &PlayerRuntime) -> bytes::BytesMut {
    let mut quest_bytes = Vec::with_capacity(ugaris_protocol::packet::QUESTLOG_QUEST_COUNT);
    for entry in player
        .quest_log
        .entries()
        .iter()
        .take(ugaris_protocol::packet::QUESTLOG_QUEST_COUNT)
    {
        quest_bytes.push((entry.done & 0x3f) | ((entry.flags & 0x03) << 6));
    }

    ugaris_protocol::packet::questlog(&quest_bytes, &player.encode_legacy_randomshrine_ppd())
}

pub(crate) fn set_character_value(values: &mut [Vec<i16>], value: CharacterValue, amount: i16) {
    let index = value as usize;
    values[0][index] = amount;
    values[1][index] = amount;
}

pub(crate) fn login_payload(
    world: &World,
    character: &Character,
    mirror_id: u16,
    tick: u64,
) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder
        .login_done()
        .ticker(tick.saturating_sub(1) as u32)
        .mirror(u32::from(mirror_id))
        .protocol(ugaris_protocol::packet::SERVER_PROTOCOL_VERSION)
        .origin(character.x, character.y);

    for value in 0..ugaris_core::entity::CHARACTER_VALUE_COUNT {
        builder.set_value0(value as u8, character.values[0][value]);
        builder.set_value1(value as u8, character.values[1][value]);
    }

    builder
        .set_hp((character.hp / POWERSCALE) as u16)
        .set_endurance((character.endurance / POWERSCALE) as u16)
        .set_mana((character.mana / POWERSCALE) as u16)
        .set_lifeshield((character.lifeshield / POWERSCALE) as u16)
        .exp(character.exp)
        .exp_used(character.exp_used)
        .gold(character.gold);

    let (cursor_sprite, cursor_flags) = character
        .cursor_item
        .and_then(|item_id| item_packet_fields(world, item_id))
        .unwrap_or((0, 0));
    builder.set_cursor_item(cursor_sprite, cursor_flags);

    for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
        let (sprite, flags) = character.inventory[slot]
            .and_then(|item_id| item_packet_fields(world, item_id))
            .unwrap_or((0, 0));
        builder.set_item(slot as u8, sprite, flags);
    }

    builder.system_text(LOGIN_ACCEPTED_MESSAGE);
    builder.into_payload()
}

/// C `read_login` (`src/system/player.c:396-444`): maps a non-`Ready`
/// `find_login` outcome to the exact reject text sent via
/// `player_client_exit`. Returns `None` only for `Ready`/`Waiting`, which do
/// not reject the connection here (`Waiting` means "still awaiting the DB",
/// which cannot occur once `begin_login` has already resolved to a value).
pub(crate) fn login_reject_message(outcome: &LoginOutcome) -> Option<&'static str> {
    match outcome {
        LoginOutcome::Ready { .. } | LoginOutcome::Waiting => None,
        // Cross-area transfer (C `player_to_server`) is not implemented yet
        // (tracked separately in `PORTING_TODO.md`'s "Cross-area transfer"
        // task); until then, treat a target-area redirect like C's
        // target-area-server-down fallback instead of silently spawning a
        // scaffold character in the wrong area.
        LoginOutcome::NewArea { .. } => Some(
            "Target area server is down. Your character is being transfered to a different area. Please try again.",
        ),
        LoginOutcome::InternalError => Some(LOGIN_REJECT_INTERNAL_ERROR),
        LoginOutcome::Locked => Some(LOGIN_REJECT_LOCKED),
        LoginOutcome::WrongPassword => Some(LOGIN_REJECT_WRONG_PASSWORD),
        LoginOutcome::Duplicate => Some(LOGIN_REJECT_DUPLICATE),
        LoginOutcome::NotPaid => Some(LOGIN_REJECT_NOT_PAID),
        LoginOutcome::Shutdown => Some(LOGIN_REJECT_SHUTDOWN),
        LoginOutcome::IpLocked => Some(LOGIN_REJECT_IP_LOCKED),
        LoginOutcome::AccountNotFixed => Some(LOGIN_REJECT_ACCOUNT_NOT_FIXED),
        LoginOutcome::TooManyBadPasswords => Some(LOGIN_REJECT_TOO_MANY_BAD_PASSWORDS),
    }
}

pub(crate) fn login_bootstrap_payloads(
    world: &World,
    character: &Character,
    pk_relations: &PkRelationSnapshot,
    mirror_id: u16,
    tick: u64,
    view_distance: usize,
    effect_cache: &mut ClientEffectCache,
) -> Vec<bytes::BytesMut> {
    let mut payloads = vec![login_payload(world, character, mirror_id, tick)];
    payloads.extend(initial_map_payloads(
        world,
        character,
        pk_relations,
        view_distance,
    ));
    payloads.extend(client_effect_payloads(
        world,
        character,
        view_distance,
        effect_cache,
    ));
    payloads
}
