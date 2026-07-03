use super::*;

/// Field-level cache of one client map cell, mirroring the C server's
/// per-player `cmap` model. Cached values are position independent so the
/// cache can shift on scroll exactly like the client shifts its map. The
/// default value (all zero, no character) is a dark/black cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CellTile {
    pub gsprite: u32,
    pub fsprite: u32,
    pub isprite: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellCharacter {
    pub id: u16,
    pub sprite: u32,
    pub action: u8,
    pub duration: u8,
    pub step: u8,
    pub dir: u8,
    pub health: u8,
    pub mana: u8,
    pub shield: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct VisibleMapCell {
    pub effects: [u16; 4],
    pub tile: CellTile,
    pub character: Option<CellCharacter>,
    pub character_name_packet: Option<Vec<u8>>,
}

/// C `trans_light` from `src/system/player.c`: quantize a 0..255 light value
/// into the client's 0..14 light-level bits (0 = brightest).
pub(crate) fn trans_light(light: i32) -> u16 {
    if light > 52 {
        0
    } else if light > 40 {
        1
    } else if light > 32 {
        2
    } else if light > 28 {
        3
    } else if light > 24 {
        4
    } else if light > 20 {
        5
    } else if light > 16 {
        6
    } else if light > 12 {
        7
    } else if light > 10 {
        8
    } else if light > 8 {
        9
    } else if light > 6 {
        10
    } else if light > 4 {
        11
    } else if light > 2 {
        12
    } else if light > 1 {
        13
    } else {
        14
    }
}

/// C `plr_map_update` per-tile visibility gate: combines tile light plus
/// scaled daylight, infrared/infravision boosts, the always-lit 3x3 center,
/// and line of sight. Returns the base client flags (`CMF_VISIBLE` plus the
/// light level) or `None` when the tile is dark for this viewer.
pub(crate) fn tile_visibility(
    world: &World,
    viewer: &Character,
    map_x: usize,
    map_y: usize,
) -> Option<u16> {
    let tile = world.map.tile(map_x, map_y)?;

    // C: light = max(map[m].light, check_dlightm(m));
    let mut light = ugaris_core::see::check_light(tile, world.date.daylight);
    let mut flags: u16 = 0;

    if viewer
        .flags
        .contains(ugaris_core::entity::CharacterFlags::INFRAVISION)
        && light < 4
    {
        light = 4;
        flags = ugaris_protocol::packet::CMF_INFRA;
    } else if viewer
        .flags
        .contains(ugaris_core::entity::CharacterFlags::INFRARED)
    {
        light = light.max(32);
    }

    if light < 1 {
        // C: the tiles right around the character stay visible.
        let center_dx = i32::from(viewer.x) - map_x as i32;
        let center_dy = i32::from(viewer.y) - map_y as i32;
        if center_dx.abs() < 2 && center_dy.abs() < 2 {
            light = 1;
        } else {
            // Dark/Light profession night sight (>= 30) is not ported yet.
            return None;
        }
    }

    if !world.map.can_see(
        usize::from(viewer.x),
        usize::from(viewer.y),
        map_x,
        map_y,
        ugaris_core::legacy::DIST_MAX,
    ) {
        return None;
    }

    Some(flags | ugaris_protocol::packet::CMF_VISIBLE | trans_light(light))
}

/// Build the cache cell for one diamond position, applying the C
/// visibility gates. Dark or out-of-map cells are the default (black) cell.
fn build_cell(
    world: &World,
    viewer: &Character,
    pk_relations: &PkRelationSnapshot,
    coords: Option<(usize, usize)>,
    known_character_names: &mut HashMap<u16, Vec<u8>>,
) -> VisibleMapCell {
    let Some((map_x, map_y)) = coords else {
        return VisibleMapCell::default();
    };
    let Some(base_flags) = tile_visibility(world, viewer, map_x, map_y) else {
        return VisibleMapCell::default();
    };
    let Some(tile) = world.map.tile(map_x, map_y) else {
        return VisibleMapCell::default();
    };

    // C: items are gated by char_see_item on visible tiles.
    let (item_sprite, item_flags) = (tile.item != 0)
        .then_some(tile.item)
        .and_then(|id| world.items.get(&ugaris_core::ids::ItemId(id)))
        .filter(|item| {
            ugaris_core::see::char_see_item(viewer, item, &world.map, world.date.daylight)
        })
        .map(|item| {
            let mut sprite = item.sprite.max(0) as u32;
            if item.flags.contains(ItemFlags::PLAYERBODY) {
                sprite |= 0x8000_0000;
            }
            (sprite, item.flags)
        })
        .unwrap_or((0, ItemFlags::empty()));

    let character = tile_character(world, viewer, tile).map(|visible| {
        let packet = character_name_packet_for_viewer(pk_relations, viewer, visible).to_vec();
        known_character_names.insert(client_character_id(visible), packet);
        cell_character_record(visible)
    });
    let character_name_packet = tile_character(world, viewer, tile)
        .map(|visible| character_name_packet_for_viewer(pk_relations, viewer, visible).to_vec());

    VisibleMapCell {
        effects: tile.effects,
        tile: CellTile {
            gsprite: tile.ground_sprite,
            fsprite: tile.foreground_sprite,
            isprite: item_sprite,
            flags: client_map_flags(tile, item_flags, base_flags),
        },
        character,
        character_name_packet,
    }
}

pub(crate) fn cell_character_record(character: &Character) -> CellCharacter {
    CellCharacter {
        id: client_character_id(character),
        sprite: character.sprite.max(0) as u32,
        action: character.action.min(u16::from(u8::MAX)) as u8,
        duration: character.duration.clamp(0, i32::from(u8::MAX)) as u8,
        step: character.step.clamp(0, i32::from(u8::MAX)) as u8,
        dir: character.dir,
        health: resource_percent(
            character.hp,
            character.values[0][CharacterValue::Hp as usize],
        ),
        mana: resource_percent(
            character.mana,
            character.values[0][CharacterValue::Mana as usize],
        ),
        shield: resource_percent(
            character.lifeshield,
            character.values[0][CharacterValue::MagicShield as usize]
                .max(character.values[0][CharacterValue::Rage as usize]),
        ),
    }
}

fn cell_character_packet(record: &CellCharacter, client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_character_basic(
        MapPosition::Absolute(client_pos),
        record.sprite,
        record.id,
        CharacterMapAction {
            action: record.action,
            duration: record.duration,
            step: record.step,
        },
        CharacterMapStatus {
            dir: record.dir,
            health: record.health,
            mana: record.mana,
            shield: record.shield,
        },
    )
    .expect("fixed character map field mask is valid")
}

fn cell_tile_packet(tile: &CellTile, client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_tile_basic(
        MapPosition::Absolute(client_pos),
        tile.gsprite,
        tile.fsprite,
        tile.isprite,
        tile.flags,
    )
    .expect("fixed tile map field mask is valid")
}

fn cell_effect_packet(effects: [u16; 4], client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_effects_basic(MapPosition::Absolute(client_pos), effects)
        .expect("fixed effect map field mask is valid")
}

/// Emit the delta packets for one cell. `previous = None` means the client
/// state is unknown (login, teleport, scrolled-in cell): everything is sent,
/// including an explicit character clear to stomp stale data.
#[allow(clippy::too_many_arguments)]
fn emit_cell_update(
    previous: Option<&VisibleMapCell>,
    next: &VisibleMapCell,
    client_pos: u16,
    known_character_names: &HashMap<u16, Vec<u8>>,
    payloads: &mut Vec<bytes::BytesMut>,
    current: &mut bytes::BytesMut,
) {
    if previous.is_none_or(|previous| previous.effects != next.effects) {
        append_map_packet(
            payloads,
            current,
            cell_effect_packet(next.effects, client_pos),
        );
    }
    if previous.is_none_or(|previous| previous.tile != next.tile) {
        append_map_packet(payloads, current, cell_tile_packet(&next.tile, client_pos));
    }
    if let (Some(record), Some(name_packet)) =
        (next.character.as_ref(), next.character_name_packet.as_ref())
    {
        if known_character_names.get(&record.id) != Some(name_packet) {
            append_map_packet(payloads, current, bytes::BytesMut::from(&name_packet[..]));
        }
    }
    match (previous.map(|cell| cell.character), next.character) {
        (Some(Some(previous_char)), Some(next_char)) if previous_char == next_char => {}
        (_, Some(next_char)) => {
            append_map_packet(
                payloads,
                current,
                cell_character_packet(&next_char, client_pos),
            );
        }
        (Some(Some(_)), None) | (None, None) => {
            append_map_packet(payloads, current, map_character_clear_packet(client_pos));
        }
        (Some(None), None) => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibleMapCache {
    pub(crate) center_x: u16,
    pub(crate) center_y: u16,
    pub(crate) view_distance: usize,
    pub(crate) cells: HashMap<u16, VisibleMapCell>,
    pub(crate) known_character_names: HashMap<u16, Vec<u8>>,
}

pub(crate) fn map_refresh_payloads(
    world: &World,
    character: &Character,
    pk_relations: &PkRelationSnapshot,
    view_distance: usize,
) -> Vec<bytes::BytesMut> {
    let mut builder = PacketBuilder::new();
    builder.origin(character.x, character.y);

    let mut payloads = vec![builder.into_payload()];
    payloads.extend(initial_map_payloads(
        world,
        character,
        pk_relations,
        view_distance,
    ));
    payloads
}

pub(crate) fn visible_map_cache(
    world: &World,
    character: &Character,
    pk_relations: &PkRelationSnapshot,
    view_distance: usize,
) -> VisibleMapCache {
    let mut known_character_names = HashMap::new();
    let cells = diamond_cells(character, view_distance)
        .map(|(client_pos, coords)| {
            (
                client_pos,
                build_cell(
                    world,
                    character,
                    pk_relations,
                    coords,
                    &mut known_character_names,
                ),
            )
        })
        .collect();

    VisibleMapCache {
        center_x: character.x,
        center_y: character.y,
        view_distance,
        cells,
        known_character_names,
    }
}

pub(crate) fn map_diff_payloads(
    world: &World,
    character: &Character,
    pk_relations: &PkRelationSnapshot,
    view_distance: usize,
    cache: &mut VisibleMapCache,
) -> Vec<bytes::BytesMut> {
    if cache.center_x != character.x
        || cache.center_y != character.y
        || cache.view_distance != view_distance
    {
        *cache = visible_map_cache(world, character, pk_relations, view_distance);
        return map_refresh_payloads(world, character, pk_relations, view_distance);
    }

    let next_cache = visible_map_cache(world, character, pk_relations, view_distance);
    let mut payloads = Vec::new();
    let mut current = bytes::BytesMut::new();

    for (client_pos, next_cell) in &next_cache.cells {
        emit_cell_update(
            cache.cells.get(client_pos),
            next_cell,
            *client_pos,
            &cache.known_character_names,
            &mut payloads,
            &mut current,
        );
    }

    if !current.is_empty() {
        payloads.push(current);
    }
    *cache = next_cache;
    payloads
}

pub(crate) fn queue_periodic_player_frames(
    runtime: &mut ServerRuntime,
    world: &World,
) -> (usize, usize) {
    let pk_relations = PkRelationSnapshot::from_runtime(runtime);
    let sessions: Vec<_> = runtime
        .players
        .iter()
        .filter_map(|(&session_id, player)| {
            if player.state != PlayerConnectionState::Normal {
                return None;
            }
            Some((session_id, player.character_id?, player.view_distance))
        })
        .collect();

    let mut diff_sessions = 0;
    let mut empty_frames = 0;
    for (session_id, character_id, view_distance) in sessions {
        let Some(character) = world.characters.get(&character_id) else {
            continue;
        };
        let mut payloads = match runtime.map_caches.get_mut(&session_id) {
            Some(cache) => map_diff_payloads(world, character, &pk_relations, view_distance, cache),
            None => {
                let payloads = map_refresh_payloads(world, character, &pk_relations, view_distance);
                runtime.map_caches.insert(
                    session_id,
                    visible_map_cache(world, character, &pk_relations, view_distance),
                );
                payloads
            }
        };
        payloads.extend(client_effect_payloads(
            world,
            character,
            view_distance,
            runtime.effect_caches.entry(session_id).or_default(),
        ));

        if payloads.is_empty() {
            // The end-of-tick flush sends the empty legacy tick frame.
            empty_frames += 1;
        } else if runtime.send_many_to_session(session_id, payloads) {
            diff_sessions += 1;
        }
    }

    (diff_sessions, empty_frames)
}

pub(crate) fn look_map_payloads(
    world: &World,
    area_id: u16,
    request: LookMapRequest,
) -> Vec<bytes::BytesMut> {
    if !request.visible {
        return vec![ugaris_protocol::packet::system_text(
            "Too far away or hidden.",
        )];
    }

    let mut messages = Vec::new();
    messages.push(section_look_text(
        area_id,
        request.x,
        request.y,
        request.character_level,
    ));

    if let Some(tile) = world.map.tile(request.x, request.y) {
        if tile.flags.contains(MapFlags::RESTAREA) {
            messages.push("This place is a rest area.".to_string());
        }
        if tile.flags.contains(MapFlags::CLAN) {
            messages.push("This is a clan area.".to_string());
        }
        if tile.flags.contains(MapFlags::ARENA) {
            messages.push("This place is an arena.".to_string());
        }
        if tile.flags.contains(MapFlags::PEACE) {
            messages.push("This place is a peaceful zone.".to_string());
        }
    }

    messages
        .into_iter()
        .map(|message| ugaris_protocol::packet::system_text(&message))
        .collect()
}

pub(crate) fn walk_section_payload(
    area_id: u16,
    player: &mut PlayerRuntime,
    character: &Character,
) -> Option<bytes::BytesMut> {
    let next_section = section_at(area_id, usize::from(character.x), usize::from(character.y));
    let next_section_id = next_section.map_or(0, |section| section.id);
    if next_section_id == player.current_section_id {
        return None;
    }

    let message = if let Some(section) = next_section {
        format!("Now entering {}.", section.name)
    } else if let Some(name) = section_name_by_id(player.current_section_id) {
        format!("Now leaving {name}.")
    } else {
        player.current_section_id = next_section_id;
        return None;
    };

    player.current_section_id = next_section_id;
    let mut bytes = Vec::with_capacity(COL_DARK_GRAY.len() + message.len());
    bytes.extend_from_slice(COL_DARK_GRAY);
    bytes.extend_from_slice(message.as_bytes());
    let mut payload = ugaris_protocol::packet::system_text_bytes(&bytes);
    if let Some(section) = next_section {
        if let Some(music) = section_music_special(section.id) {
            payload.extend_from_slice(&ugaris_protocol::packet::special(music, u32::MAX, 0));
        }
    }
    Some(payload)
}

pub(crate) fn section_music_special(section_id: u16) -> Option<u32> {
    match section_id {
        4 | 17 | 18 | 19 | 29..=44 | 46..=48 | 50 => Some(1003),
        57 | 59 => Some(1010),
        58 | 68..=70 => Some(1004),
        60..=66 => Some(1002),
        _ => None,
    }
}

pub(crate) fn area_sound_payload(
    area_id: u16,
    character: &Character,
    hour: i64,
    random_seed: u64,
) -> Option<[u8; 13]> {
    let section = section_at(area_id, usize::from(character.x), usize::from(character.y))?;
    let sound = area_sound_special(
        section.id,
        hour,
        legacy_random(random_seed, 100),
        legacy_random(random_seed.wrapping_add(1), 1000),
        legacy_random(random_seed.wrapping_add(2), 10000),
    )?;
    Some(ugaris_protocol::packet::special(
        sound.special_type,
        sound.opt1 as u32,
        sound.opt2 as u32,
    ))
}

/// One-step walk update: send `SV_SCROLL_*` + `SV_ORIGIN` plus the walker's
/// character clear/update, and shift the session cache exactly like the
/// client shifts its own map. The per-tick diff pass then fills in fringe
/// tiles and any line-of-sight changes against the shifted cache.
pub(crate) fn movement_scroll_payload(
    character: &Character,
    old_x: u16,
    old_y: u16,
    view_distance: usize,
    cache: &mut VisibleMapCache,
) -> Option<bytes::BytesMut> {
    let scroll = scroll_command(old_x, old_y, character.x, character.y)?;
    let mut builder = PacketBuilder::new();
    builder.scroll(scroll).origin(character.x, character.y);

    let dx = i32::from(character.x) - i32::from(old_x);
    let dy = i32::from(character.y) - i32::from(old_y);
    cache.shift(dx, dy, character.x, character.y);

    if let Some(old_pos) =
        old_relative_client_position(old_x, old_y, character.x, character.y, view_distance)
    {
        builder.raw(&map_character_clear_packet(old_pos));
        cache.clear_character(old_pos);
    }
    let center_pos = client_center_map_position(view_distance);
    let record = cell_character_record(character);
    builder.raw(&cell_character_packet(&record, center_pos));
    cache.set_character(center_pos, record);

    Some(builder.into_payload())
}

impl VisibleMapCache {
    /// Replicate the client's flat `memmove` scroll on the cached cells.
    /// Cells that scroll in from untracked positions are removed so the
    /// next diff pass resends them unconditionally (the client holds stale
    /// duplicated-edge data there, exactly like the legacy client).
    pub(crate) fn shift(&mut self, dx: i32, dy: i32, new_center_x: u16, new_center_y: u16) {
        let side = (self.view_distance * 2 + 1) as i32;
        let offset = dx + dy * side;
        let mut shifted: HashMap<u16, VisibleMapCell> = HashMap::new();
        for (client_pos, cell) in self.cells.drain() {
            // The client moves cmap[pos + offset] into cmap[pos]; our cell
            // for `pos` therefore lands at `pos - offset`.
            let target = i32::from(client_pos) - offset;
            if target >= 0 && target < side * side {
                shifted.insert(target as u16, cell);
            }
        }
        self.cells = shifted;
        self.center_x = new_center_x;
        self.center_y = new_center_y;
    }

    pub(crate) fn clear_character(&mut self, client_pos: u16) {
        if let Some(cell) = self.cells.get_mut(&client_pos) {
            cell.character = None;
            cell.character_name_packet = None;
        }
    }

    pub(crate) fn set_character(&mut self, client_pos: u16, record: CellCharacter) {
        if let Some(cell) = self.cells.get_mut(&client_pos) {
            cell.character = Some(record);
        }
    }
}

pub(crate) fn map_position_in_diamond(
    map_x: usize,
    map_y: usize,
    center_x: u16,
    center_y: u16,
    view_distance: usize,
) -> bool {
    let dx = map_x as i32 - i32::from(center_x);
    let dy = map_y as i32 - i32::from(center_y);
    dx.abs() + dy.abs() <= view_distance as i32
}

pub(crate) fn scroll_command(old_x: u16, old_y: u16, new_x: u16, new_y: u16) -> Option<u8> {
    match (
        i32::from(new_x) - i32::from(old_x),
        i32::from(new_y) - i32::from(old_y),
    ) {
        (1, 0) => Some(SV_SCROLL_RIGHT),
        (-1, 0) => Some(SV_SCROLL_LEFT),
        (0, 1) => Some(SV_SCROLL_DOWN),
        (0, -1) => Some(SV_SCROLL_UP),
        (-1, -1) => Some(SV_SCROLL_LEFTUP),
        (-1, 1) => Some(SV_SCROLL_LEFTDOWN),
        (1, -1) => Some(SV_SCROLL_RIGHTUP),
        (1, 1) => Some(SV_SCROLL_RIGHTDOWN),
        _ => None,
    }
}

pub(crate) fn old_relative_client_position(
    old_x: u16,
    old_y: u16,
    new_x: u16,
    new_y: u16,
    view_distance: usize,
) -> Option<u16> {
    let side = view_distance.checked_mul(2)?.checked_add(1)?;
    let client_x = i32::from(old_x) - i32::from(new_x) + view_distance as i32;
    let client_y = i32::from(old_y) - i32::from(new_y) + view_distance as i32;
    if client_x < 0 || client_y < 0 || client_x >= side as i32 || client_y >= side as i32 {
        return None;
    }
    Some((client_x as usize + client_y as usize * side).min(u16::MAX as usize) as u16)
}

pub(crate) fn map_character_clear_packet(client_pos: u16) -> bytes::BytesMut {
    ugaris_protocol::packet::map_delta(
        MapLayer::Character,
        MapPosition::Absolute(client_pos),
        MAP_CHARACTER_CLEAR,
        &[],
    )
    .expect("fixed character clear map field mask is valid")
}

pub(crate) fn initial_map_payloads(
    world: &World,
    viewer: &Character,
    pk_relations: &PkRelationSnapshot,
    view_distance: usize,
) -> Vec<bytes::BytesMut> {
    let mut payloads = Vec::new();
    let mut current = bytes::BytesMut::new();
    let mut known_character_names = HashMap::new();

    for (client_pos, coords) in diamond_cells(viewer, view_distance) {
        let next = build_cell(
            world,
            viewer,
            pk_relations,
            coords,
            &mut known_character_names,
        );
        // Unknown previous state: stomp everything, including stale
        // characters, because the client never clears its map on origin
        // changes.
        emit_cell_update(
            None,
            &next,
            client_pos,
            &HashMap::new(),
            &mut payloads,
            &mut current,
        );
    }

    if !current.is_empty() {
        payloads.push(current);
    }
    payloads
}

pub(crate) fn append_map_packet(
    payloads: &mut Vec<bytes::BytesMut>,
    current: &mut bytes::BytesMut,
    packet: bytes::BytesMut,
) {
    if !current.is_empty() && current.len() + packet.len() > MAP_BOOTSTRAP_CHUNK_TARGET {
        payloads.push(std::mem::take(current));
    }
    current.extend_from_slice(&packet);
}

pub(crate) fn client_map_flags(tile: &MapTile, item_flags: ItemFlags, base_flags: u16) -> u16 {
    let mut flags = base_flags;
    if tile.flags.contains(MapFlags::UNDERWATER) {
        flags |= CMF_UNDERWATER;
    }
    if tile.flags.contains(MapFlags::SINK_ANKLE) {
        flags |= CMF_SINK_ANKLE;
    }
    if tile.flags.contains(MapFlags::SINK_KNEE) {
        flags |= CMF_SINK_KNEE;
    }
    if tile.flags.contains(MapFlags::SINK_BELLY) {
        flags |= CMF_SINK_BELLY;
    }
    if tile.flags.contains(MapFlags::SINK_CHEST) {
        flags |= CMF_SINK_CHEST;
    }
    if item_flags.contains(ItemFlags::TAKE) {
        flags |= CMF_TAKE;
    }
    if item_flags.contains(ItemFlags::USE) {
        flags |= CMF_USE;
    }
    flags
}

pub(crate) fn tile_character<'a>(
    world: &'a World,
    viewer: &Character,
    tile: &MapTile,
) -> Option<&'a Character> {
    // C: characters are gated by char_see_char even on visible tiles
    // (stealth, invisibility, and per-target light checks).
    (tile.character != 0)
        .then_some(CharacterId(u32::from(tile.character)))
        .and_then(|id| world.characters.get(&id))
        .filter(|target| {
            ugaris_core::see::char_see_char(viewer, target, &world.map, world.date.daylight)
        })
}

pub(crate) fn client_character_id(character: &Character) -> u16 {
    character.id.0 as u16
}

pub(crate) fn character_name_packet(character: &Character) -> bytes::BytesMut {
    character_name_packet_with_relation(character, 0)
}

pub(crate) fn character_name_packet_for_viewer(
    pk_relations: &PkRelationSnapshot,
    viewer: &Character,
    character: &Character,
) -> bytes::BytesMut {
    character_name_packet_with_relation(
        character,
        pk_relation_for_viewer(pk_relations, viewer, character),
    )
}

pub(crate) fn pk_relation_for_viewer(
    pk_relations: &PkRelationSnapshot,
    viewer: &Character,
    character: &Character,
) -> u8 {
    if !character.flags.contains(CharacterFlags::PK) {
        return 0;
    }

    let him = pk_relations.has_hate(character.id, viewer.id);
    let me = pk_relations.has_hate(viewer.id, character.id);
    match (him, me) {
        (true, true) => 5,
        (true, false) => 4,
        (false, true) => 3,
        (false, false) if pk_hate_prerequisites(viewer, character) => 2,
        (false, false) => 1,
    }
}

pub(crate) fn character_name_packet_with_relation(
    character: &Character,
    pk_relation: u8,
) -> bytes::BytesMut {
    let name = if character.flags.contains(CharacterFlags::WON) {
        if character.flags.contains(CharacterFlags::FEMALE) {
            format!("Lady {}", character.name)
        } else {
            format!("Sir {}", character.name)
        }
    } else {
        character.name.clone()
    };
    let colors = if character.sprite == 27 {
        [0, 0, 0]
    } else {
        [character.c1, character.c2, character.c3]
    };

    ugaris_protocol::packet::character_name(
        client_character_id(character),
        character.level.min(u32::from(u8::MAX)) as u8,
        colors,
        character.clan.min(u16::from(u8::MAX)) as u8,
        pk_relation,
        &name,
    )
}

/// All diamond client positions with their map coordinates; out-of-map
/// positions yield `None` so callers can emit dark cells for them.
pub(crate) fn diamond_cells(
    viewer: &Character,
    view_distance: usize,
) -> impl Iterator<Item = (u16, Option<(usize, usize)>)> {
    let xoff = i32::from(viewer.x) - view_distance as i32;
    let yoff = i32::from(viewer.y) - view_distance as i32;
    let side = view_distance * 2 + 1;

    (0..=view_distance * 2).flat_map(move |y| {
        let (xs, xe) = if y < view_distance {
            (view_distance - y, view_distance + y)
        } else {
            (y - view_distance, view_distance * 3 - y)
        };

        (xs..=xe).map(move |x| {
            let map_x = xoff + x as i32;
            let map_y = yoff + y as i32;
            let coords = (map_x >= 1
                && map_y >= 1
                && (map_x as usize) < ugaris_core::legacy::MAX_MAP
                && (map_y as usize) < ugaris_core::legacy::MAX_MAP)
                .then_some((map_x as usize, map_y as usize));
            ((x + y * side) as u16, coords)
        })
    })
}

pub(crate) fn client_center_map_position(view_distance: usize) -> u16 {
    let side = view_distance.saturating_mul(2).saturating_add(1);
    view_distance
        .saturating_add(view_distance.saturating_mul(side))
        .min(u16::MAX as usize) as u16
}

pub(crate) fn resource_percent(current: i32, max_value: i16) -> u8 {
    let max_value = i32::from(max_value).max(1);
    let percent = (current / (POWERSCALE / 100)) / max_value;
    percent.clamp(0, i32::from(u8::MAX)) as u8
}
