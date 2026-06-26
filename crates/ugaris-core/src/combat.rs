use serde::{Deserialize, Serialize};

pub const ATTACK_DIV: i32 = 5;
pub const FIREBALL_DAMAGE: i32 = 5;
pub const STRIKE_DAMAGE: i32 = 5;
pub const FIGHT_ENEMY_COUNT: usize = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownEnemy {
    pub character_number: u32,
    pub character_id: u32,
    pub last_x: u16,
    pub last_y: u16,
    pub visible: bool,
    pub hurt_me: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FightDriverData {
    pub enemies: [KnownEnemy; FIGHT_ENEMY_COUNT],
    pub start_dist: i32,
    pub stop_dist: i32,
    pub char_dist: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub last_hit: i32,
}

impl Default for FightDriverData {
    fn default() -> Self {
        Self {
            enemies: [KnownEnemy::default(); FIGHT_ENEMY_COUNT],
            start_dist: 0,
            stop_dist: 0,
            char_dist: 0,
            home_x: 0,
            home_y: 0,
            last_hit: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_constants_match_c_header() {
        assert_eq!(ATTACK_DIV, 5);
        assert_eq!(FIREBALL_DAMAGE, 5);
        assert_eq!(STRIKE_DAMAGE, 5);
    }
}
