use crate::direction::Direction;
use crate::entity::Character;

/// C `offset2dx(frx, fry, tox, toy)` (`src/system/tool.c:309-349`): the
/// 8-way direction pointing from `(frx,fry)` toward `(tox,toy)`, snapping
/// near-diagonal offsets to a cardinal direction when one axis dominates
/// the other by more than 2x. Returns `None` for `(0, 0)` (C returns `0`,
/// not a valid `DX_*` value).
pub fn offset2dx(frx: i32, fry: i32, tox: i32, toy: i32) -> Option<Direction> {
    let mut dx = tox - frx;
    let mut dy = toy - fry;

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

pub fn map_dist(fx: u16, fy: u16, tx: u16, ty: u16) -> i32 {
    let dx = fx.abs_diff(tx) as i32;
    let dy = fy.abs_diff(ty) as i32;

    if dx > dy {
        (dx << 1) + dy
    } else {
        (dy << 1) + dx
    }
}

pub fn char_dist(from: &Character, to: &Character) -> i32 {
    map_dist(from.x, from.y, to.x, to.y)
}

pub fn tile_char_dist(from: &Character, to: &Character) -> u16 {
    from.x.abs_diff(to.x).max(from.y.abs_diff(to.y))
}

pub fn step_char_dist(from: &Character, to: &Character) -> u16 {
    let (tx, ty) = if to.tox != 0 {
        (to.tox, to.toy)
    } else {
        (to.x, to.y)
    };

    from.x.abs_diff(tx) + from.y.abs_diff(ty)
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{CharacterFlags, SpeedMode},
        ids::CharacterId,
    };

    use super::*;

    #[test]
    fn offset2dx_matches_legacy_eight_way_snapping() {
        assert_eq!(offset2dx(10, 10, 20, 10), Some(Direction::Right));
        assert_eq!(offset2dx(10, 10, 0, 10), Some(Direction::Left));
        assert_eq!(offset2dx(10, 10, 10, 20), Some(Direction::Down));
        assert_eq!(offset2dx(10, 10, 10, 0), Some(Direction::Up));
        assert_eq!(offset2dx(10, 10, 20, 20), Some(Direction::RightDown));
        assert_eq!(offset2dx(10, 10, 20, 0), Some(Direction::RightUp));
        assert_eq!(offset2dx(10, 10, 0, 20), Some(Direction::LeftDown));
        assert_eq!(offset2dx(10, 10, 0, 0), Some(Direction::LeftUp));
        // dy small relative to dx snaps to a pure horizontal direction.
        assert_eq!(offset2dx(10, 10, 20, 12), Some(Direction::Right));
        // dx small relative to dy snaps to a pure vertical direction.
        assert_eq!(offset2dx(10, 10, 12, 20), Some(Direction::Down));
        assert_eq!(offset2dx(10, 10, 10, 10), None);
    }

    #[test]
    fn map_dist_matches_legacy_two_three_cost_estimate() {
        assert_eq!(map_dist(10, 10, 13, 10), 6);
        assert_eq!(map_dist(10, 10, 13, 12), 8);
        assert_eq!(map_dist(10, 10, 11, 14), 9);
    }

    #[test]
    fn character_distance_helpers_match_drvlib() {
        let from = character(1, 10, 10);
        let mut to = character(2, 13, 12);

        assert_eq!(char_dist(&from, &to), 8);
        assert_eq!(tile_char_dist(&from, &to), 3);
        assert_eq!(step_char_dist(&from, &to), 5);

        to.tox = 11;
        to.toy = 10;
        assert_eq!(step_char_dist(&from, &to), 1);
    }

    fn character(id: u32, x: u16, y: u16) -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(id),
            serial: id,
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
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
            x,
            y,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
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
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
            driver_memory: crate::character_driver::DriverMemory::default(),
            class: 0,
            dungeonfighter: None,
            fight_driver: None,
            lq_usurp: None,
        }
    }
}
