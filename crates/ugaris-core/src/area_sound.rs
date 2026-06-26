#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSoundSpecial {
    pub special_type: u32,
    pub opt1: i32,
    pub opt2: i32,
}

pub fn area_sound_special(
    section_id: u16,
    hour: i64,
    sound_roll: u32,
    distance_roll: u32,
    pan_roll: u32,
) -> Option<AreaSoundSpecial> {
    let special_type = match area_sound_kind(section_id)? {
        AreaSoundKind::WetDungeon => match sound_roll % 100 {
            10 | 20 => 14,
            30 | 40 => 15,
            50 | 60 => 16,
            _ => return None,
        },
        AreaSoundKind::DryDungeon => match sound_roll % 100 {
            10 => 36,
            20 => 37,
            30 => 38,
            40 => 39,
            50 => 40,
            _ => return None,
        },
        AreaSoundKind::Woods => woods_sound(hour, sound_roll % 100)?,
        AreaSoundKind::Park => park_sound(hour, sound_roll % 100)?,
        AreaSoundKind::Underwater => match sound_roll % 100 {
            10 => 47,
            20 => 48,
            30 => 49,
            _ => return None,
        },
    };

    Some(AreaSoundSpecial {
        special_type,
        opt1: -((distance_roll % 1000) as i32 + 100),
        opt2: 5000 - (pan_roll % 10000) as i32,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaSoundKind {
    WetDungeon,
    DryDungeon,
    Woods,
    Park,
    Underwater,
}

fn area_sound_kind(section_id: u16) -> Option<AreaSoundKind> {
    match section_id {
        4 | 17..=19 | 29..=44 | 46..=48 | 50 | 68..=70 => Some(AreaSoundKind::WetDungeon),
        58 => Some(AreaSoundKind::Woods),
        60..=62 => Some(AreaSoundKind::Park),
        63..=66 => Some(AreaSoundKind::DryDungeon),
        114 => Some(AreaSoundKind::Underwater),
        _ => None,
    }
}

fn woods_sound(hour: i64, roll: u32) -> Option<u32> {
    if hour > 6 && hour < 22 {
        match roll {
            10 => Some(10),
            20 => Some(11),
            30 => Some(12),
            40 => Some(19),
            50 => Some(20),
            60 => Some(21),
            70 => Some(22),
            80 => Some(24),
            90 => Some(25),
            _ => None,
        }
    } else {
        match roll {
            10 => Some(17),
            20 => Some(18),
            30 => Some(26),
            40 => Some(27),
            50 => Some(28),
            60 => Some(23),
            _ => None,
        }
    }
}

fn park_sound(hour: i64, roll: u32) -> Option<u32> {
    if hour > 6 && hour < 22 {
        woods_sound(hour, roll)
    } else {
        match roll {
            30 => Some(26),
            40 => Some(27),
            50 => Some(28),
            60 => Some(23),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wet_dungeon_matches_legacy_roll_table_and_options() {
        assert_eq!(
            area_sound_special(46, 12, 10, 0, 0),
            Some(AreaSoundSpecial {
                special_type: 14,
                opt1: -100,
                opt2: 5000,
            })
        );
        assert_eq!(area_sound_special(46, 12, 70, 0, 0), None);
    }

    #[test]
    fn woods_and_park_preserve_day_night_tables() {
        assert_eq!(
            area_sound_special(58, 12, 10, 9, 10).unwrap().special_type,
            10
        );
        assert_eq!(
            area_sound_special(58, 23, 10, 9, 10).unwrap().special_type,
            17
        );
        assert_eq!(area_sound_special(60, 23, 10, 9, 10), None);
        assert_eq!(
            area_sound_special(60, 23, 30, 9, 10).unwrap().special_type,
            26
        );
    }

    #[test]
    fn dry_dungeon_and_underwater_tables_are_section_gated() {
        assert_eq!(
            area_sound_special(63, 12, 50, 1234, 9999)
                .unwrap()
                .special_type,
            40
        );
        assert_eq!(
            area_sound_special(114, 12, 20, 234, 6000)
                .unwrap()
                .special_type,
            48
        );
        assert_eq!(area_sound_special(1, 12, 10, 0, 0), None);
    }
}
