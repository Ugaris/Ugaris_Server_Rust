use super::*;

pub(crate) fn read_driver_data_u16(driver_data: &[u8], offset: usize) -> Option<u16> {
    let bytes = driver_data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn write_driver_data_u16(driver_data: &mut Vec<u8>, offset: usize, value: u16) {
    if driver_data.len() < offset + 2 {
        driver_data.resize(offset + 2, 0);
    }
    driver_data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn offset_u16(value: u16, delta: i16) -> Option<u16> {
    let value = i32::from(value) + i32::from(delta);
    (0..=u16::MAX as i32)
        .contains(&value)
        .then_some(value as u16)
}

pub(crate) fn valid_map_coords(x: i32, y: i32) -> Option<(usize, usize)> {
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    Some((x, y))
}

pub(crate) fn adjacent_direction(
    from_x: u16,
    from_y: u16,
    to_x: usize,
    to_y: usize,
) -> Option<Direction> {
    match (
        to_x as i32 - i32::from(from_x),
        to_y as i32 - i32::from(from_y),
    ) {
        (1, 0) => Some(Direction::Right),
        (0, 1) => Some(Direction::Down),
        (-1, 0) => Some(Direction::Left),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

pub(crate) fn adjacent_use_direction(
    from_x: u16,
    from_y: u16,
    to_x: usize,
    to_y: usize,
    front_wall: bool,
) -> Option<Direction> {
    match (
        to_x as i32 - i32::from(from_x),
        to_y as i32 - i32::from(from_y),
    ) {
        (1, 0) if !front_wall => Some(Direction::Right),
        (0, 1) if !front_wall => Some(Direction::Down),
        (-1, 0) => Some(Direction::Left),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

pub(crate) fn item_in_facing_direction(
    character: &Character,
    map: &MapGrid,
) -> Option<(ItemId, Direction)> {
    let direction = Direction::try_from(character.dir).ok()?;
    let (dx, dy) = direction.delta();
    let x = offset_coordinate(usize::from(character.x), dx)?;
    let y = offset_coordinate(usize::from(character.y), dy)?;
    let item_id = map.tile(x, y)?.item;
    (item_id != 0).then_some((ItemId(item_id), direction))
}

pub(crate) fn offset_to_direction(
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
) -> Option<Direction> {
    let mut dx = to_x as i32 - from_x as i32;
    let mut dy = to_y as i32 - from_y as i32;

    if dx.abs() / 2 > dy.abs() {
        dy = 0;
    }
    if dy.abs() / 2 > dx.abs() {
        dx = 0;
    }

    match (dx.signum(), dy.signum()) {
        (1, 1) => Some(Direction::RightDown),
        (1, -1) => Some(Direction::RightUp),
        (1, 0) => Some(Direction::Right),
        (-1, 1) => Some(Direction::LeftDown),
        (-1, -1) => Some(Direction::LeftUp),
        (-1, 0) => Some(Direction::Left),
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

pub(crate) fn offset_coordinate(value: usize, offset: i16) -> Option<usize> {
    if offset.is_negative() {
        value.checked_sub(offset.unsigned_abs() as usize)
    } else {
        value.checked_add(offset as usize)
    }
}

pub(crate) fn clamp_world_coordinate(value: i32) -> u16 {
    value.clamp(0, (MAX_MAP - 1) as i32) as u16
}

pub(crate) fn diagonal_slide_alternates(direction: u8) -> Option<(Direction, Direction)> {
    match Direction::try_from(direction).ok()? {
        Direction::LeftUp => Some((Direction::Left, Direction::Up)),
        Direction::RightUp => Some((Direction::Right, Direction::Up)),
        Direction::LeftDown => Some((Direction::Left, Direction::Down)),
        Direction::RightDown => Some((Direction::Right, Direction::Down)),
        _ => None,
    }
}

pub(crate) fn write_driver_data_u32(item: &mut Item, offset: usize, value: u32) {
    item.driver_data.resize(offset + 4, 0);
    item.driver_data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn read_u32_le_prefix(bytes: &[u8]) -> u32 {
    read_u32_le_at(bytes, 0)
}

pub(crate) fn read_u32_le_at(bytes: &[u8], offset: usize) -> u32 {
    let mut raw = [0; 4];
    if offset < bytes.len() {
        let len = (bytes.len() - offset).min(raw.len());
        raw[..len].copy_from_slice(&bytes[offset..offset + len]);
    }
    u32::from_le_bytes(raw)
}

pub(crate) fn write_u32_le_prefix(bytes: &mut Vec<u8>, value: u32) {
    if bytes.len() < 4 {
        bytes.resize(4, 0);
    }
    bytes[..4].copy_from_slice(&value.to_le_bytes());
}
