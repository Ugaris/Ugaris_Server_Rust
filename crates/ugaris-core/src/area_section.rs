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
        2 => AREA2_SECTORS,
        3 => AREA3_SECTORS,
        4 => AREA4_SECTORS,
        5 => AREA5_SECTORS,
        6 => AREA6_SECTORS,
        7 => AREA7_SECTORS,
        8 => AREA8_SECTORS,
        9 => AREA9_SECTORS,
        10 => AREA10_SECTORS,
        11 => AREA11_SECTORS,
        12 => AREA12_SECTORS,
        13 => AREA13_SECTORS,
        14 => AREA14_SECTORS,
        15 => AREA15_SECTORS,
        16 => AREA16_SECTORS,
        17 => AREA17_SECTORS,
        18 => AREA18_SECTORS,
        19 => AREA19_SECTORS,
        20 => AREA20_SECTORS,
        21 => AREA21_SECTORS,
        22 => AREA22_SECTORS,
        23 => AREA23_SECTORS,
        24 => AREA24_SECTORS,
        25 => AREA25_SECTORS,
        26 => AREA26_SECTORS,
        29 => AREA29_SECTORS,
        31 => AREA31_SECTORS,
        32 => AREA32_SECTORS,
        33 => AREA33_SECTORS,
        34 => AREA34_SECTORS,
        36 => AREA36_SECTORS,
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
        1 => ("Skellie I", 1),
        2 => ("Thieves Guild II", 8),
        3 => ("Skellie III Upstairs", 5),
        4 => ("Zombie I", 10),
        5 => ("Village", 0),
        6 => ("Fortress", 0),
        7 => ("Robber Outpost", 8),
        8 => ("Skellie II", 8),
        9 => ("Skellie III Downstairs", 6),
        10 => ("Skellie Showdown", 7),
        11 => ("Thieves Guild I", 0),
        12 => ("Mad Mages", 7),
        13 => ("Mad Knights", 8),
        14 => ("Cameron", 0),
        15 => ("Creeper Death Run", 14),
        16 => ("Creepers", 13),
        17 => ("Zombie Showdown", 13),
        18 => ("Zombie II", 13),
        19 => ("Zombie III", 13),
        20 => ("Palace", 11),
        21 => ("Underground Park I", 15),
        22 => ("Underground Park II", 16),
        23 => ("Underground Park III", 17),
        24 => ("Moonish Caverns", 18),
        25 => ("Lower Crypt", 20),
        26 => ("Crypt", 21),
        27 => ("Inner Crypt", 22),
        28 => ("Skellie III Downbelow", 7),
        29 => ("Sewers I", 8),
        30 => ("Sewers II", 10),
        31 => ("Sewers III", 12),
        32 => ("Sewers IV", 14),
        33 => ("Sewers V", 16),
        34 => ("Sewers VI", 18),
        35 => ("Sewers VII", 20),
        36 => ("Sewers VIII", 22),
        37 => ("Sewers IX", 24),
        38 => ("Sewers X", 26),
        39 => ("Sewers XI", 28),
        40 => ("Sewers XII", 30),
        41 => ("Sewers XIII", 32),
        42 => ("Sewers XIV", 34),
        43 => ("Sewers XV", 36),
        44 => ("Sewers XVI", 38),
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
        59 => ("Aston", 0),
        60 => ("Garden", 16),
        61 => ("Park", 0),
        62 => ("Graveyard", 0),
        63 => ("Pentagram Quest", 0),
        64 => ("Earth Underground", 0),
        65 => ("Fire Underground", 0),
        66 => ("Ice Underground", 0),
        67 => ("Ice Palace", 0),
        68 => ("Mine", 0),
        69 => ("Catacombs", 0),
        70 => ("Random Dungeons", 0),
        71 => ("Swamp", 0),
        72 => ("Forest", 0),
        73 => ("Robber's Lair", 36),
        74 => ("Dungeon of Bones", 36),
        75 => ("Skeleton Ruin", 36),
        76 => ("Robber Sector I", 36),
        77 => ("Robber Sector II", 39),
        78 => ("Robber Sector III", 42),
        79 => ("Robber Sector IV", 45),
        80 => ("Exkordon Sewers", 38),
        81 => ("Below the Library", 44),
        82 => ("Exkordon", 0),
        83 => ("Governor's Palace", 0),
        84 => ("Skeleton's Lair", 47),
        85 => ("Spider's Lair", 45),
        86 => ("Zombie's Lair", 48),
        87 => ("Depths of Bone, Level I", 50),
        88 => ("Depths of Bone, Level II", 51),
        89 => ("Depths of Bone, Level III", 52),
        90 => ("Depths of Bone, Level IV", 54),
        91 => ("Depths of Bone, Level V", 55),
        92 => ("Depths of Bone, Level VI", 57),
        93 => ("Depths of Bone, Level VII", 58),
        94 => ("Depths of Bone, Level VIII", 59),
        95 => ("Depths of Bone, Level IX", 61),
        96 => ("Depths of Bone, Hidden Level", 63),
        97 => ("Depths of Bone, Bottom", 63),
        98 => ("Depths of Bone, Towers", 0),
        99 => ("Tower of Ansuz", 50),
        100 => ("Tower of Berkano", 51),
        101 => ("Tower of Dagaz", 52),
        102 => ("Tower of Ehwaz", 54),
        103 => ("Tower of Fehu", 55),
        104 => ("Tower of Hagalaz", 57),
        105 => ("Tower of Isa", 58),
        106 => ("Tower of Ingwaz", 59),
        107 => ("Halls of Raidho", 61),
        108 => ("Nomad Plains", 0),
        109 => ("Live Quest Area", 0),
        110 => ("Test Pents", 0),
        111 => ("Labyrinth, Light&Dark", 20),
        112 => ("Live Quest Area, Entrance", 0),
        113 => ("Labyrinth, Undeads", 30),
        114 => ("Labyrinth, Underwater", 25),
        115 => ("Labyrinth, First Steps", 10),
        116 => ("Labyrinth, Hard Life", 15),
        117 => ("Ice Army Caves", 0),
        118 => ("More Ice Army Caves", 0),
        119 => ("Rodneys Warped World, Green", 0),
        120 => ("Rodneys Warped World, Orange", 0),
        121 => ("Rodneys Warped World, Red", 0),
        122 => ("Rodneys Warped World, Blue", 0),
        123 => ("Rodneys Warped World, White", 0),
        124 => ("Dragon's Breath", 30),
        125 => ("Tower Top", 0),
        126 => ("Tower I", 60),
        127 => ("Tower II", 60),
        128 => ("Tower III", 60),
        129 => ("Tower IV", 60),
        130 => ("Tover V", 60),
        131 => ("Tower VI", 60),
        132 => ("Tower VII", 60),
        133 => ("Tower VIII", 60),
        134 => ("Brannington, Bar", 0),
        135 => ("Brannington, Castle", 0),
        136 => ("Brannington", 0),
        137 => ("Grimroot", 0),
        138 => ("Jobbington", 0),
        139 => ("Long Tunnel", 0),
        140 => ("Teufelheim, Slums", 38),
        141 => ("Teufelheim, Worker District", 70),
        142 => ("Teufelheim, Noble District", 102),
        143 => ("Hell Pents", 0),
        144 => ("Caligar Forest", 60),
        145 => ("Caligar City", 0),
        146 => ("Dungeon of Blood", 60),
        147 => ("Dungeon of Bone", 60),
        148 => ("Dungeon of Flesh", 60),
        149 => ("Palace Level 1", 60),
        150 => ("Palace Level 2", 60),
        151 => ("Palace Level 3", 60),
        152 => ("Palace Level 4", 60),
        153 => ("Amazon Den", 60),
        154 => ("Underground Passage", 60),
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

pub fn section_name_by_id(id: u16) -> Option<&'static str> {
    section_by_id(id).map(|section| section.name)
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

const AREA2_SECTORS: &[SectorRect] = &[
    r(1, 1, 85, 70, 4),
    r(123, 2, 142, 57, 15),
    r(127, 58, 142, 102, 15),
    r(118, 102, 142, 124, 15),
    r(118, 124, 128, 183, 16),
    r(95, 125, 118, 183, 16),
    r(84, 137, 84, 183, 16),
    r(59, 148, 84, 183, 16),
    r(39, 141, 59, 183, 16),
    r(18, 124, 38, 183, 16),
    r(2, 112, 18, 183, 16),
    r(85, 1, 122, 62, 17),
    r(122, 58, 126, 62, 17),
    r(1, 70, 14, 84, 18),
    r(1, 84, 7, 100, 18),
    r(15, 69, 47, 88, 18),
    r(48, 74, 64, 74, 18),
    r(58, 70, 90, 97, 18),
    r(64, 98, 85, 101, 18),
    r(90, 63, 121, 82, 18),
    r(96, 83, 101, 90, 18),
    r(3, 101, 7, 110, 19),
    r(8, 86, 12, 90, 19),
    r(8, 90, 19, 111, 19),
    r(20, 90, 39, 123, 19),
    r(40, 92, 52, 140, 19),
    r(53, 97, 60, 140, 19),
    r(60, 97, 60, 148, 19),
    r(64, 102, 83, 147, 19),
    r(84, 124, 94, 136, 19),
    r(84, 113, 117, 113, 19),
    r(87, 101, 117, 112, 19),
    r(91, 83, 96, 100, 19),
    r(97, 90, 102, 100, 19),
    r(103, 95, 127, 100, 19),
    r(103, 85, 115, 94, 19),
    r(116, 83, 126, 89, 19),
    r(122, 83, 126, 63, 19),
    r(143, 1, 192, 85, 21),
    r(143, 85, 192, 170, 22),
    r(143, 170, 192, 254, 23),
    r(1, 184, 142, 254, 24),
    r(194, 208, 254, 254, 25),
    r(214, 197, 254, 207, 25),
    r(194, 197, 212, 207, 26),
    r(194, 153, 254, 197, 26),
    r(214, 116, 254, 153, 27),
];

const AREA3_SECTORS: &[SectorRect] = &[
    r(110, 100, 169, 162, 20),
    r(218, 209, 254, 254, 58),
    r(197, 163, 219, 189, 60),
    r(179, 101, 227, 160, 61),
    r(62, 173, 104, 213, 62),
    r(91, 93, 254, 254, 59),
];
const AREA4_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 63)];
const AREA5_SECTORS: &[SectorRect] = &[
    r(55, 70, 134, 119, 29),
    r(70, 1, 134, 70, 30),
    r(1, 55, 55, 119, 31),
    r(1, 1, 70, 55, 32),
    r(134, 55, 183, 134, 33),
    r(134, 1, 198, 55, 34),
    r(183, 70, 254, 134, 35),
    r(198, 1, 254, 70, 36),
    r(121, 136, 200, 185, 37),
    r(121, 185, 185, 254, 38),
    r(200, 136, 254, 200, 39),
    r(185, 200, 254, 254, 40),
    r(72, 121, 121, 200, 41),
    r(57, 200, 121, 254, 42),
    r(1, 121, 72, 185, 43),
    r(1, 185, 57, 254, 44),
];
const AREA6_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 64)];
const AREA7_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 63)];
const AREA8_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 65)];
const AREA9_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 63)];
const AREA10_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 66)];
const AREA11_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 67)];
const AREA12_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 68)];
const AREA13_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 69)];
const AREA14_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 70)];
const AREA15_SECTORS: &[SectorRect] = &[r(1, 1, 254, 254, 71)];
const AREA16_SECTORS: &[SectorRect] = &[
    r(92, 76, 116, 96, 73),
    r(42, 229, 86, 255, 74),
    r(114, 193, 134, 226, 75),
    r(1, 1, 254, 254, 72),
];
const AREA17_SECTORS: &[SectorRect] = &[
    r(110, 1, 129, 51, 76),
    r(130, 11, 133, 51, 76),
    r(134, 31, 137, 51, 76),
    r(138, 47, 148, 51, 76),
    r(113, 52, 133, 78, 77),
    r(101, 62, 123, 81, 77),
    r(134, 52, 170, 88, 78),
    r(149, 47, 153, 51, 78),
    r(132, 1, 175, 52, 79),
    r(195, 1, 254, 99, 80),
    r(1, 191, 37, 254, 81),
    r(1, 1, 59, 37, 83),
    r(38, 191, 82, 254, 84),
    r(83, 191, 160, 254, 85),
    r(161, 191, 208, 254, 86),
    r(209, 222, 225, 254, 99),
    r(1, 1, 194, 100, 82),
];
const AREA18_SECTORS: &[SectorRect] = &[
    r(1, 1, 85, 64, 87),
    r(85, 1, 170, 64, 88),
    r(170, 1, 255, 64, 89),
    r(1, 64, 85, 128, 90),
    r(85, 64, 170, 128, 91),
    r(170, 64, 255, 128, 92),
    r(1, 128, 85, 192, 93),
    r(85, 128, 170, 192, 94),
    r(170, 128, 255, 192, 95),
    r(1, 192, 85, 255, 96),
    r(85, 192, 170, 255, 97),
    r(172, 222, 188, 254, 100),
    r(188, 222, 204, 254, 101),
    r(204, 222, 220, 254, 102),
    r(220, 222, 236, 254, 103),
    r(236, 222, 252, 254, 104),
    r(172, 206, 204, 222, 105),
    r(204, 206, 236, 222, 106),
    r(172, 194, 236, 205, 107),
    r(237, 194, 254, 221, 107),
    r(170, 192, 255, 255, 98),
];
const AREA19_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 108)];
const AREA20_SECTORS: &[SectorRect] = &[r(230, 233, 255, 255, 112), r(1, 1, 255, 255, 109)];
const AREA21_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 110)];
const AREA22_SECTORS: &[SectorRect] = &[
    r(224, 199, 255, 255, 111),
    r(134, 141, 223, 254, 113),
    r(134, 1, 254, 140, 114),
    r(224, 140, 254, 198, 114),
    r(21, 171, 133, 254, 115),
    r(66, 1, 133, 114, 116),
];
const AREA23_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 117)];
const AREA24_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 118)];
const AREA25_SECTORS: &[SectorRect] = &[
    r(211, 132, 254, 254, 119),
    r(179, 45, 254, 131, 120),
    r(158, 14, 254, 44, 121),
    r(212, 1, 254, 44, 121),
    r(157, 1, 211, 13, 122),
    r(108, 1, 157, 44, 122),
    r(108, 45, 178, 131, 122),
    r(108, 132, 211, 254, 123),
];
const AREA26_SECTORS: &[SectorRect] = &[r(1, 1, 165, 55, 124)];
const AREA29_SECTORS: &[SectorRect] = &[
    r(37, 112, 71, 146, 125),
    r(37, 148, 71, 182, 126),
    r(37, 184, 71, 218, 127),
    r(37, 220, 71, 254, 128),
    r(1, 220, 35, 254, 129),
    r(1, 184, 35, 218, 130),
    r(1, 148, 35, 182, 131),
    r(1, 112, 35, 146, 132),
    r(1, 76, 35, 110, 133),
    r(1, 1, 24, 30, 134),
    r(1, 32, 26, 43, 135),
    r(182, 167, 207, 183, 135),
    r(172, 152, 243, 185, 136),
    r(136, 185, 243, 222, 136),
    r(181, 222, 220, 232, 136),
];
const AREA31_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 137)];
const AREA32_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 138)];
const AREA33_SECTORS: &[SectorRect] = &[r(1, 1, 255, 255, 139)];
const AREA34_SECTORS: &[SectorRect] = &[
    r(129, 193, 255, 255, 140),
    r(1, 193, 128, 255, 141),
    r(1, 1, 128, 192, 142),
    r(129, 1, 255, 192, 143),
];
const AREA36_SECTORS: &[SectorRect] = &[
    r(207, 167, 221, 194, 146),
    r(207, 138, 221, 165, 147),
    r(207, 109, 221, 136, 148),
    r(100, 175, 128, 227, 149),
    r(142, 176, 170, 228, 150),
    r(224, 109, 252, 161, 151),
    r(172, 176, 205, 196, 152),
    r(172, 196, 189, 202, 152),
    r(222, 181, 255, 255, 153),
    r(190, 197, 255, 255, 153),
    r(186, 203, 255, 255, 153),
    r(179, 203, 186, 240, 153),
    r(171, 203, 179, 228, 153),
    r(178, 241, 185, 250, 154),
    r(141, 229, 178, 253, 154),
    r(108, 239, 141, 253, 154),
    r(1, 1, 255, 108, 144),
    r(1, 109, 255, 255, 145),
];

const fn r(fx: usize, fy: usize, tx: usize, ty: usize, section_id: u16) -> SectorRect {
    SectorRect {
        fx,
        fy,
        tx,
        ty,
        section_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area1_section_lookup_uses_legacy_order() {
        assert_eq!(section_at(1, 73, 174).unwrap().name, "Thieves House");
        assert_eq!(section_at(1, 250, 100).unwrap().name, "The Woods");
    }

    #[test]
    fn non_area1_section_lookup_uses_legacy_tables() {
        assert_eq!(section_at(2, 1, 1).unwrap().name, "Zombie I");
        assert_eq!(section_at(3, 197, 163).unwrap().name, "Garden");
        assert_eq!(section_at(17, 209, 222).unwrap().name, "Tower of Ansuz");
        assert_eq!(
            section_at(34, 1, 193).unwrap().name,
            "Teufelheim, Worker District"
        );
        assert_eq!(section_at(36, 1, 1).unwrap().name, "Caligar Forest");
    }

    #[test]
    fn section_name_lookup_returns_legacy_section_name() {
        assert_eq!(section_name_by_id(1), Some("Skellie I"));
        assert_eq!(section_name_by_id(0), None);
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
        assert_eq!(
            section_look_text(34, 1, 193, 65),
            "Teufelheim, Worker District. This area is very dangerous for you. (1,193)"
        );
    }

    #[test]
    fn unknown_area_falls_back_to_coordinates() {
        assert_eq!(section_look_text(99, 12, 13, 1), "(12,13)");
    }
}
