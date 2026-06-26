#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSection {
    pub id: u16,
    pub name: &'static str,
    pub level: u32,
}

#[derive(Debug, Clone, Copy)]
struct SectorRect {
    fx: usize,
    fy: usize,
    tx: usize,
    ty: usize,
    section_id: u16,
}

pub fn section_at(area_id: u16, x: usize, y: usize) -> Option<AreaSection> {
    let sectors = match area_id {
        1 => AREA1_SECTORS,
        _ => return None,
    };

    let sector = sectors
        .iter()
        .find(|sector| x >= sector.fx && x <= sector.tx && y >= sector.fy && y <= sector.ty)?;
    section_by_id(sector.section_id)
}

pub fn section_look_text(area_id: u16, x: usize, y: usize, character_level: u32) -> String {
    let Some(section) = section_at(area_id, x, y) else {
        return format!("({x},{y})");
    };

    if section.level == 0 {
        return format!("{}. ({x},{y})", section.name);
    }

    let diff = i64::from(section.level) - i64::from(character_level);
    let difficulty = match diff {
        i64::MIN..=-6 => "way too easy for you",
        -5 => "too easy for you",
        -4 => "easy for you",
        -3 => "fairly easy for you",
        -2 => "rather easy for you",
        -1 => "just about right for you",
        0 => "just right for you",
        1 => "slightly dangerous for you",
        2 => "a bit dangerous for you",
        3 => "somewhat dangerous for you",
        4 => "dangerous for you",
        5 => "very dangerous for you",
        6..=i64::MAX => "way too dangerous for you",
    };

    format!("{}. This area is {difficulty}. ({x},{y})", section.name)
}

fn section_by_id(id: u16) -> Option<AreaSection> {
    let (name, level) = match id {
        45 => ("Thieves House", 1),
        46 => ("Skellie I", 2),
        47 => ("Skellie II", 3),
        48 => ("Skellie III", 4),
        49 => ("Robbers Outpost", 5),
        50 => ("Skellie V", 5),
        51 => ("Skellie IV", 6),
        52 => ("Mad Mages", 7),
        53 => ("Mad Knights", 8),
        54 => ("Thieves Guild", 9),
        55 => ("Gwendylon's Tower", 0),
        56 => ("Fortress", 0),
        57 => ("Cameron", 0),
        58 => ("The Woods", 0),
        155 => ("Robber Hideout I", 8),
        156 => ("Robber Hideout II", 8),
        157 => ("Robber Hideout III", 8),
        158 => ("Robber Hideout IV", 8),
        159 => ("Bear Cave", 12),
        160 => ("Forest Dweller Cabin", 7),
        161 => ("Hermit Cabin", 0),
        _ => return None,
    };
    Some(AreaSection { id, name, level })
}

const AREA1_SECTORS: &[SectorRect] = &[
    SectorRect {
        fx: 73,
        fy: 174,
        tx: 91,
        ty: 192,
        section_id: 45,
    },
    SectorRect {
        fx: 146,
        fy: 115,
        tx: 174,
        ty: 169,
        section_id: 46,
    },
    SectorRect {
        fx: 171,
        fy: 164,
        tx: 178,
        ty: 169,
        section_id: 46,
    },
    SectorRect {
        fx: 170,
        fy: 126,
        tx: 174,
        ty: 151,
        section_id: 46,
    },
    SectorRect {
        fx: 174,
        fy: 115,
        tx: 178,
        ty: 129,
        section_id: 46,
    },
    SectorRect {
        fx: 35,
        fy: 229,
        tx: 72,
        ty: 252,
        section_id: 47,
    },
    SectorRect {
        fx: 182,
        fy: 224,
        tx: 196,
        ty: 254,
        section_id: 48,
    },
    SectorRect {
        fx: 175,
        fy: 130,
        tx: 187,
        ty: 163,
        section_id: 48,
    },
    SectorRect {
        fx: 3,
        fy: 229,
        tx: 33,
        ty: 251,
        section_id: 49,
    },
    SectorRect {
        fx: 3,
        fy: 186,
        tx: 38,
        ty: 223,
        section_id: 51,
    },
    SectorRect {
        fx: 72,
        fy: 209,
        tx: 103,
        ty: 254,
        section_id: 50,
    },
    SectorRect {
        fx: 169,
        fy: 84,
        tx: 205,
        ty: 114,
        section_id: 52,
    },
    SectorRect {
        fx: 149,
        fy: 103,
        tx: 169,
        ty: 114,
        section_id: 52,
    },
    SectorRect {
        fx: 147,
        fy: 58,
        tx: 169,
        ty: 88,
        section_id: 53,
    },
    SectorRect {
        fx: 147,
        fy: 88,
        tx: 153,
        ty: 94,
        section_id: 53,
    },
    SectorRect {
        fx: 151,
        fy: 210,
        tx: 181,
        ty: 254,
        section_id: 54,
    },
    SectorRect {
        fx: 96,
        fy: 109,
        tx: 116,
        ty: 135,
        section_id: 55,
    },
    SectorRect {
        fx: 110,
        fy: 95,
        tx: 114,
        ty: 110,
        section_id: 55,
    },
    SectorRect {
        fx: 104,
        fy: 135,
        tx: 115,
        ty: 146,
        section_id: 55,
    },
    SectorRect {
        fx: 114,
        fy: 168,
        tx: 134,
        ty: 176,
        section_id: 56,
    },
    SectorRect {
        fx: 114,
        fy: 176,
        tx: 145,
        ty: 191,
        section_id: 56,
    },
    SectorRect {
        fx: 108,
        fy: 76,
        tx: 169,
        ty: 168,
        section_id: 57,
    },
    SectorRect {
        fx: 197,
        fy: 226,
        tx: 224,
        ty: 254,
        section_id: 155,
    },
    SectorRect {
        fx: 179,
        fy: 119,
        tx: 186,
        ty: 125,
        section_id: 155,
    },
    SectorRect {
        fx: 179,
        fy: 115,
        tx: 183,
        ty: 119,
        section_id: 155,
    },
    SectorRect {
        fx: 126,
        fy: 209,
        tx: 150,
        ty: 254,
        section_id: 156,
    },
    SectorRect {
        fx: 227,
        fy: 209,
        tx: 237,
        ty: 215,
        section_id: 156,
    },
    SectorRect {
        fx: 104,
        fy: 209,
        tx: 125,
        ty: 254,
        section_id: 157,
    },
    SectorRect {
        fx: 237,
        fy: 4,
        tx: 245,
        ty: 10,
        section_id: 157,
    },
    SectorRect {
        fx: 225,
        fy: 226,
        tx: 254,
        ty: 254,
        section_id: 158,
    },
    SectorRect {
        fx: 239,
        fy: 74,
        tx: 244,
        ty: 83,
        section_id: 158,
    },
    SectorRect {
        fx: 4,
        fy: 114,
        tx: 51,
        ty: 135,
        section_id: 159,
    },
    SectorRect {
        fx: 5,
        fy: 135,
        tx: 51,
        ty: 139,
        section_id: 159,
    },
    SectorRect {
        fx: 8,
        fy: 139,
        tx: 51,
        ty: 143,
        section_id: 159,
    },
    SectorRect {
        fx: 22,
        fy: 143,
        tx: 51,
        ty: 151,
        section_id: 159,
    },
    SectorRect {
        fx: 99,
        fy: 56,
        tx: 110,
        ty: 65,
        section_id: 160,
    },
    SectorRect {
        fx: 167,
        fy: 4,
        tx: 178,
        ty: 11,
        section_id: 161,
    },
    SectorRect {
        fx: 1,
        fy: 1,
        tx: 254,
        ty: 254,
        section_id: 58,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area1_section_lookup_uses_legacy_order() {
        assert_eq!(section_at(1, 73, 174).unwrap().name, "Thieves House");
        assert_eq!(section_at(1, 250, 100).unwrap().name, "The Woods");
    }

    #[test]
    fn section_look_text_matches_legacy_difficulty_phrasing() {
        assert_eq!(
            section_look_text(1, 146, 115, 7),
            "Skellie I. This area is too easy for you. (146,115)"
        );
        assert_eq!(
            section_look_text(1, 146, 115, 2),
            "Skellie I. This area is just right for you. (146,115)"
        );
        assert_eq!(
            section_look_text(1, 146, 115, 1),
            "Skellie I. This area is slightly dangerous for you. (146,115)"
        );
    }

    #[test]
    fn unknown_area_falls_back_to_coordinates() {
        assert_eq!(section_look_text(99, 12, 13, 1), "(12,13)");
    }
}
