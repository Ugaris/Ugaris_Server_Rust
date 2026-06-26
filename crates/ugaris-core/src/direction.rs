use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Direction {
    Right = 1,
    RightDown = 2,
    Down = 3,
    LeftDown = 4,
    Left = 5,
    LeftUp = 6,
    Up = 7,
    RightUp = 8,
}

impl Direction {
    pub fn delta(self) -> (i16, i16) {
        match self {
            Self::Right => (1, 0),
            Self::RightDown => (1, 1),
            Self::Down => (0, 1),
            Self::LeftDown => (-1, 1),
            Self::Left => (-1, 0),
            Self::LeftUp => (-1, -1),
            Self::Up => (0, -1),
            Self::RightUp => (1, -1),
        }
    }
}

impl TryFrom<u8> for Direction {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Right),
            2 => Ok(Self::RightDown),
            3 => Ok(Self::Down),
            4 => Ok(Self::LeftDown),
            5 => Ok(Self::Left),
            6 => Ok(Self::LeftUp),
            7 => Ok(Self::Up),
            8 => Ok(Self::RightUp),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_ids_match_c_header() {
        assert_eq!(Direction::Right as u8, 1);
        assert_eq!(Direction::Left as u8, 5);
        assert_eq!(Direction::RightUp as u8, 8);
    }
}
