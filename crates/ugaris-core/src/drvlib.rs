use crate::entity::Character;

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
            id: CharacterId(id),
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
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
            gold: 0,
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
        }
    }
}
