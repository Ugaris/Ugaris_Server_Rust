use serde::{Deserialize, Serialize};

pub const MAX_QUESTS: usize = 100;
pub const QF_OPEN: u8 = 1;
pub const QF_DONE: u8 = 2;
pub const QLF_REPEATABLE: u8 = 1 << 0;
pub const QLF_XREPEAT: u8 = 1 << 1;

pub const QLOG_LYDIA: usize = 0;
pub const QLOG_GWENDY_FIRST_SKULL: usize = 1;
pub const QLOG_GWENDY_SECOND_SKULL: usize = 2;
pub const QLOG_GWENDY_THIRD_SKULL: usize = 3;
pub const QLOG_GWENDY_FOUL_MAGICIAN: usize = 4;
pub const QLOG_NOOK: usize = 6;
pub const QLOG_JESSICA_ROBBER_NOTE: usize = 79;
pub const QLOG_JIU: usize = 80;
pub const QLOG_BRITHILDIE: usize = 81;
pub const QLOG_HERMIT_QUEST1: usize = 82;
pub const QLOG_HERMIT_QUEST2: usize = 83;
pub const QLOG_JESSICA_KILL: usize = 84;

/// C `struct questlog` (`src/system/questlog.c:98-105`): the static quest
/// metadata table entry (name/level-range/giver/area/nominal exp/flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestMeta {
    pub name: &'static str,
    pub min_level: u8,
    pub max_level: u8,
    pub giver: &'static str,
    pub area: &'static str,
    /// Nominal exp reward (C `questlog[qnr].exp`). `0` marks quests whose
    /// exp is awarded ad hoc by the driver instead (documented per entry
    /// below, matching the C source comments).
    pub exp: i64,
    pub flags: u8,
}

/// C `struct questlog questlog[]` (`src/system/questlog.c:107-202`), copied
/// digit for digit including the two trailing-space quest names (`"The
/// Jewels of Brannington "`, `"A Thief's Loot "`) that exist verbatim in
/// the C source.
pub const QUEST_TABLE: [QuestMeta; 85] = [
    QuestMeta {
        name: "Lydia's Potion",
        min_level: 1,
        max_level: 2,
        giver: "James",
        area: "Cameron",
        exp: 15,
        flags: QLF_REPEATABLE,
    }, // 0
    QuestMeta {
        name: "Find the Magic Item",
        min_level: 2,
        max_level: 3,
        giver: "Gwendylon",
        area: "Cameron",
        exp: 75,
        flags: QLF_REPEATABLE,
    }, // 1
    QuestMeta {
        name: "The Second Skull",
        min_level: 3,
        max_level: 5,
        giver: "Gwendylon",
        area: "Cameron",
        exp: 150,
        flags: QLF_REPEATABLE,
    }, // 2
    QuestMeta {
        name: "The Third Skull",
        min_level: 5,
        max_level: 7,
        giver: "Gwendylon",
        area: "Cameron",
        exp: 300,
        flags: QLF_REPEATABLE,
    }, // 3
    QuestMeta {
        name: "Kill the Foul Magician",
        min_level: 6,
        max_level: 8,
        giver: "Gwendylon",
        area: "Cameron",
        exp: 800,
        flags: QLF_REPEATABLE,
    }, // 4
    QuestMeta {
        name: "Bear Hunt",
        min_level: 6,
        max_level: 8,
        giver: "Yoakin",
        area: "Cameron",
        exp: 600,
        flags: QLF_REPEATABLE,
    }, // 5
    QuestMeta {
        name: "A Fool's Request",
        min_level: 6,
        max_level: 8,
        giver: "Nook",
        area: "Cameron",
        exp: 400,
        flags: 0,
    }, // 6
    QuestMeta {
        name: "Mages Gone Berserk",
        min_level: 6,
        max_level: 9,
        giver: "Guiwynn",
        area: "Cameron",
        exp: 800,
        flags: QLF_REPEATABLE,
    }, // 7
    QuestMeta {
        name: "The Recipe for Happiness",
        min_level: 7,
        max_level: 10,
        giver: "Guiwynn",
        area: "Cameron",
        exp: 900,
        flags: QLF_REPEATABLE,
    }, // 8
    QuestMeta {
        name: "Knightly Troubles",
        min_level: 7,
        max_level: 10,
        giver: "Logain",
        area: "Cameron",
        exp: 1200,
        flags: QLF_REPEATABLE,
    }, // 9
    QuestMeta {
        name: "Loisan's House",
        min_level: 9,
        max_level: 12,
        giver: "Seymour",
        area: "Aston",
        exp: 850,
        flags: 0,
    }, // 10
    QuestMeta {
        name: "The Silver Skull",
        min_level: 10,
        max_level: 13,
        giver: "Seymour",
        area: "Aston",
        exp: 1000,
        flags: 0,
    }, // 11
    QuestMeta {
        name: "Find Loisan",
        min_level: 11,
        max_level: 15,
        giver: "Seymour",
        area: "Aston",
        exp: 1500,
        flags: QLF_REPEATABLE,
    }, // 12
    QuestMeta {
        name: "Jeepers Creepers",
        min_level: 12,
        max_level: 18,
        giver: "Kelly",
        area: "Aston",
        exp: 1850,
        flags: QLF_REPEATABLE,
    }, // 13
    QuestMeta {
        // C: "special case: exp awarded in driver, 4500 exp total"
        name: "Underground Park Shrines",
        min_level: 15,
        max_level: 20,
        giver: "Kelly",
        area: "Aston",
        exp: 0,
        flags: 0,
    }, // 14
    QuestMeta {
        name: "In Search of Clara",
        min_level: 20,
        max_level: 27,
        giver: "Kelly",
        area: "Aston",
        exp: 2500,
        flags: 0,
    }, // 15
    QuestMeta {
        name: "The Astronomer's Notes",
        min_level: 15,
        max_level: 20,
        giver: "Gerassimo",
        area: "Aston",
        exp: 5000,
        flags: QLF_REPEATABLE,
    }, // 16
    QuestMeta {
        name: "The Unwanted Tenants",
        min_level: 9,
        max_level: 12,
        giver: "Reskin",
        area: "Cameron",
        exp: 1250,
        flags: 0,
    }, // 17
    QuestMeta {
        name: "The Toughest Monster",
        min_level: 20,
        max_level: 25,
        giver: "Sir Jones",
        area: "Aston",
        exp: 7500,
        flags: 0,
    }, // 18
    QuestMeta {
        name: "The Toughestest Monster",
        min_level: 20,
        max_level: 26,
        giver: "Sir Jones",
        area: "Aston",
        exp: 12000,
        flags: 0,
    }, // 19
    QuestMeta {
        name: "Wanted: Occult Staff",
        min_level: 30,
        max_level: 36,
        giver: "Carlos",
        area: "Aston",
        exp: 40000,
        flags: QLF_REPEATABLE,
    }, // 20
    QuestMeta {
        name: "Slay the Swampbeast",
        min_level: 23,
        max_level: 30,
        giver: "Clara",
        area: "Swamp",
        exp: 22500,
        flags: 0,
    }, // 21
    QuestMeta {
        name: "Impish Bear Hunt",
        min_level: 20,
        max_level: 27,
        giver: "William/Imp",
        area: "Forest",
        exp: 12500,
        flags: 0,
    }, // 22
    QuestMeta {
        name: "Praying Mantis Stew",
        min_level: 20,
        max_level: 27,
        giver: "William",
        area: "Forest",
        exp: 15000,
        flags: 0,
    }, // 23
    QuestMeta {
        name: "The Spider Queen",
        min_level: 25,
        max_level: 30,
        giver: "Hermit",
        area: "Forest",
        exp: 25000,
        flags: 0,
    }, // 24
    QuestMeta {
        // C: "exp awarded in driver, amount depends on robbers killed. range: 5000 to 20000"
        name: "Earning the Lockpick",
        min_level: 25,
        max_level: 30,
        giver: "Guildmaster",
        area: "Exkordon",
        exp: 0,
        flags: QLF_XREPEAT,
    }, // 25
    QuestMeta {
        // C: "exp awarded in driver, 5000 or 10000"
        name: "Extortion",
        min_level: 25,
        max_level: 30,
        giver: "Guildmaster",
        area: "Exkordon",
        exp: 0,
        flags: QLF_XREPEAT,
    }, // 26
    QuestMeta {
        name: "Price Fix Exposed",
        min_level: 25,
        max_level: 30,
        giver: "Guildmaster",
        area: "Exkordon",
        exp: 15000,
        flags: QLF_XREPEAT,
    }, // 27
    QuestMeta {
        name: "The Golden Lockpick",
        min_level: 26,
        max_level: 33,
        giver: "Guildmaster",
        area: "Exkordon",
        exp: 15000,
        flags: QLF_XREPEAT,
    }, // 28
    QuestMeta {
        // C: "exp awarded in driver, 45000 total"
        name: "Dirty Hands",
        min_level: 26,
        max_level: 33,
        giver: "Sanwyn",
        area: "Exkordon",
        exp: 0,
        flags: 0,
    }, // 29
    QuestMeta {
        name: "The Old Governor's Cross",
        min_level: 33,
        max_level: 40,
        giver: "Skeleton",
        area: "Exkordon",
        exp: 30000,
        flags: QLF_REPEATABLE,
    }, // 30
    QuestMeta {
        name: "Spider Poison",
        min_level: 30,
        max_level: 40,
        giver: "Cervik",
        area: "Exkordon",
        exp: 30000,
        flags: QLF_REPEATABLE,
    }, // 31
    QuestMeta {
        name: "Join the Tribe",
        min_level: 63,
        max_level: 80,
        giver: "Kalanur",
        area: "Nomad Plains",
        exp: 10000,
        flags: 0,
    }, // 32
    QuestMeta {
        name: "Searching Sarkilar",
        min_level: 63,
        max_level: 80,
        giver: "Kir Laas",
        area: "Nomad Plains",
        exp: 450000,
        flags: 0,
    }, // 33
    QuestMeta {
        name: "A Golden Statue",
        min_level: 72,
        max_level: 90,
        giver: "Kir Garan",
        area: "Nomad Plains",
        exp: 280000,
        flags: 0,
    }, // 34
    QuestMeta {
        name: "Smuggler Book",
        min_level: 10,
        max_level: 15,
        giver: "Imp. Commander",
        area: "Below Aston 2",
        exp: 1000,
        flags: QLF_REPEATABLE,
    }, // 35
    QuestMeta {
        // C: "exp awarded in driver, 5000 total"
        name: "Contraband",
        min_level: 10,
        max_level: 15,
        giver: "Imp. Commander",
        area: "Below Aston 2",
        exp: 0,
        flags: 0,
    }, // 36
    QuestMeta {
        name: "Smuggler Leader",
        min_level: 10,
        max_level: 15,
        giver: "Imp. Commander",
        area: "Below Aston 2",
        exp: 2000,
        flags: QLF_REPEATABLE,
    }, // 37
    QuestMeta {
        name: "The Family Heirloom",
        min_level: 32,
        max_level: 40,
        giver: "Aristocrat",
        area: "Bran. Forest",
        exp: 40000,
        flags: QLF_REPEATABLE,
    }, // 38
    QuestMeta {
        name: "Bear Hunt - Again",
        min_level: 32,
        max_level: 36,
        giver: "Yoatin",
        area: "Bran. Forest",
        exp: 40000,
        flags: QLF_REPEATABLE,
    }, // 39
    QuestMeta {
        // C: "exp awarded in driver, 120k total"
        name: "The Jewels of Brannington ",
        min_level: 34,
        max_level: 40,
        giver: "Count B.",
        area: "Brannington",
        exp: 0,
        flags: QLF_REPEATABLE,
    }, // 40
    QuestMeta {
        name: "A Grolm's Spoils",
        min_level: 33,
        max_level: 42,
        giver: "Brenneth",
        area: "Brannington",
        exp: 15000,
        flags: QLF_REPEATABLE,
    }, // 41
    QuestMeta {
        name: "A Thief's Loot ",
        min_level: 33,
        max_level: 42,
        giver: "Brenneth",
        area: "Brannington",
        exp: 15000,
        flags: QLF_REPEATABLE,
    }, // 42
    QuestMeta {
        name: "A Necromancer's Notes",
        min_level: 33,
        max_level: 42,
        giver: "Brenneth",
        area: "Brannington",
        exp: 15000,
        flags: QLF_REPEATABLE,
    }, // 43
    QuestMeta {
        name: "A Rest Disturbed",
        min_level: 36,
        max_level: 43,
        giver: "Spirit",
        area: "Brannington",
        exp: 60000,
        flags: QLF_REPEATABLE,
    }, // 44
    QuestMeta {
        name: "Searching a Miner's Tool",
        min_level: 42,
        max_level: 48,
        giver: "Broklin",
        area: "Brannington",
        exp: 60000,
        flags: QLF_REPEATABLE,
    }, // 45
    QuestMeta {
        name: "A Miner's Vengeance",
        min_level: 44,
        max_level: 50,
        giver: "Broklin",
        area: "Brannington",
        exp: 60000,
        flags: 0,
    }, // 46
    QuestMeta {
        name: "A Miner's Misery",
        min_level: 85,
        max_level: 95,
        giver: "Dwarven Chief",
        area: "Grimroot",
        exp: 285000,
        flags: 0,
    }, // 47
    QuestMeta {
        name: "A Miner's Bane",
        min_level: 95,
        max_level: 105,
        giver: "Dwarven Chief",
        area: "Grimroot",
        exp: 395000,
        flags: 0,
    }, // 48
    QuestMeta {
        name: "A Miner's Anguish",
        min_level: 105,
        max_level: 115,
        giver: "Dwarven Chief",
        area: "Grimroot",
        exp: 525000,
        flags: 0,
    }, // 49
    QuestMeta {
        name: "A Miner Lost",
        min_level: 115,
        max_level: 125,
        giver: "Dwarven Chief",
        area: "Grimroot",
        exp: 680000,
        flags: 0,
    }, // 50
    QuestMeta {
        name: "Lizard's Teeth",
        min_level: 95,
        max_level: 105,
        giver: "Dwarven Shaman",
        area: "Grimroot",
        exp: 395000,
        flags: 0,
    }, // 51
    QuestMeta {
        name: "Collecting Berries",
        min_level: 100,
        max_level: 110,
        giver: "Dwarven Shaman",
        area: "Grimroot",
        exp: 455000,
        flags: 0,
    }, // 52
    QuestMeta {
        name: "Elitist Head",
        min_level: 105,
        max_level: 115,
        giver: "Dwarven Shaman",
        area: "Grimroot",
        exp: 525000,
        flags: 0,
    }, // 53
    QuestMeta {
        name: "Looking for Caligar",
        min_level: 55,
        max_level: 65,
        giver: "Kelly",
        area: "Aston",
        exp: 80000,
        flags: 0,
    }, // 54
    QuestMeta {
        name: "Fighting Styles",
        min_level: 55,
        max_level: 65,
        giver: "Glori",
        area: "Caligar",
        exp: 80000,
        flags: 0,
    }, // 55
    QuestMeta {
        name: "Obelisk Hunt",
        min_level: 55,
        max_level: 65,
        giver: "Glori",
        area: "Caligar",
        exp: 80000,
        flags: 0,
    }, // 56
    QuestMeta {
        name: "Find the Keyparts",
        min_level: 55,
        max_level: 65,
        giver: "Glori",
        area: "Caligar",
        exp: 80000,
        flags: 0,
    }, // 57
    QuestMeta {
        name: "Assemble the Key",
        min_level: 55,
        max_level: 65,
        giver: "Glori",
        area: "Caligar",
        exp: 80000,
        flags: 0,
    }, // 58
    QuestMeta {
        name: "Amazon Invaders",
        min_level: 55,
        max_level: 65,
        giver: "Homdem",
        area: "Caligar",
        exp: 80000,
        flags: 0,
    }, // 59
    QuestMeta {
        name: "The Emperor's Plaque",
        min_level: 55,
        max_level: 65,
        giver: "Kelly",
        area: "Aston",
        exp: 160000,
        flags: 0,
    }, // 60
    QuestMeta {
        name: "The Imperial Vault",
        min_level: 26,
        max_level: 28,
        giver: "Carlos",
        area: "Aston",
        exp: 20000,
        flags: 0,
    }, // 61
    QuestMeta {
        name: "Tunnel Magics",
        min_level: 26,
        max_level: 28,
        giver: "Rouven",
        area: "Imperial Vault",
        exp: 10000,
        flags: 0,
    }, // 62
    QuestMeta {
        name: "Chronicles of Seyan",
        min_level: 26,
        max_level: 28,
        giver: "Rouven",
        area: "Imperial Vault",
        exp: 10000,
        flags: 0,
    }, // 63
    QuestMeta {
        name: "Finding Arkhata",
        min_level: 47,
        max_level: 55,
        giver: "Guard",
        area: "Brannington",
        exp: 60000,
        flags: 0,
    }, // 64
    QuestMeta {
        name: "Rammy's Crown",
        min_level: 48,
        max_level: 58,
        giver: "Rammy",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 65
    QuestMeta {
        name: "Ishtar's Bracelet",
        min_level: 49,
        max_level: 59,
        giver: "Jaz",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 66
    QuestMeta {
        name: "Queen Fiona's Ring",
        min_level: 50,
        max_level: 60,
        giver: "Queen Fiona",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 67
    QuestMeta {
        name: "A Shopkeeper's Fright",
        min_level: 51,
        max_level: 61,
        giver: "Ramin",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 68
    QuestMeta {
        name: "The Monks' Request",
        min_level: 52,
        max_level: 62,
        giver: "Johnatan",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 69
    QuestMeta {
        name: "The Book Eater",
        min_level: 53,
        max_level: 63,
        giver: "Tracy",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 70
    QuestMeta {
        name: "Entrance Passes",
        min_level: 54,
        max_level: 64,
        giver: "Rammy",
        area: "Arkhata",
        exp: 90000,
        flags: 0,
    }, // 71
    QuestMeta {
        name: "The Source",
        min_level: 60,
        max_level: 70,
        giver: "Jada",
        area: "Arkhata",
        exp: 120000,
        flags: 0,
    }, // 72
    QuestMeta {
        name: "Ceremonial Pot",
        min_level: 48,
        max_level: 58,
        giver: "Pot Maker",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 73
    QuestMeta {
        name: "The Lost Secrets",
        min_level: 49,
        max_level: 59,
        giver: "Thai Pan",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 74
    QuestMeta {
        name: "A Kidnapped Student",
        min_level: 53,
        max_level: 63,
        giver: "Trainer",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 75
    QuestMeta {
        name: "The Traitors",
        min_level: 53,
        max_level: 63,
        giver: "Clerk",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 76
    QuestMeta {
        name: "The Blue Harpy",
        min_level: 58,
        max_level: 68,
        giver: "Hunter",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 77
    QuestMeta {
        name: "The Mysterious Language",
        min_level: 60,
        max_level: 65,
        giver: "Johnatan",
        area: "Arkhata",
        exp: 60000,
        flags: 0,
    }, // 78
    QuestMeta {
        name: "The Robber Operations",
        min_level: 6,
        max_level: 9,
        giver: "Jessica",
        area: "Cameron",
        exp: 750,
        flags: QLF_REPEATABLE,
    }, // 79
    QuestMeta {
        name: "Cleansing the Sanctuary",
        min_level: 39,
        max_level: 45,
        giver: "Jiu",
        area: "Cameron",
        exp: 50000,
        flags: 0,
    }, // 80
    QuestMeta {
        name: "The dying forest",
        min_level: 39,
        max_level: 45,
        giver: "Brithildie",
        area: "Cameron",
        exp: 50000,
        flags: 0,
    }, // 81
    QuestMeta {
        name: "Bear Control Hunt",
        min_level: 9,
        max_level: 15,
        giver: "Hermit",
        area: "Cameron",
        exp: 1000,
        flags: 0,
    }, // 82
    QuestMeta {
        name: "Bear Tooth Necklace",
        min_level: 9,
        max_level: 15,
        giver: "Hermit",
        area: "Cameron",
        exp: 1000,
        flags: QLF_REPEATABLE,
    }, // 83
    QuestMeta {
        name: "Defeating the Robber Leader",
        min_level: 6,
        max_level: 9,
        giver: "Jessica",
        area: "Cameron",
        exp: 750,
        flags: QLF_REPEATABLE,
    }, // 84
];

/// C `questlog[qnr]` lookup - `None` for `qnr >= QUEST_TABLE.len()` (indices
/// `85..MAX_QUESTS` have no metadata in C either; nothing in the ported
/// tree references them).
pub fn quest_meta(qnr: usize) -> Option<&'static QuestMeta> {
    QUEST_TABLE.get(qnr)
}

const QUESTLOG_FLAGS: [u8; MAX_QUESTS] = {
    let mut flags = [0u8; MAX_QUESTS];
    let mut i = 0;
    while i < QUEST_TABLE.len() {
        flags[i] = QUEST_TABLE[i].flags;
        i += 1;
    }
    flags
};

/// C `questlog_scale(cnt, ex)` (`src/system/questlog.c:240-265`): the
/// repeat-completion exp decay curve. `cnt` is the number of times the
/// quest had already been completed *before* this completion (C's
/// post-increment `quest[qnr].done++` read).
pub fn scale_exp(prior_completions: u8, base_exp: i64) -> i64 {
    match prior_completions {
        0 => base_exp,
        1 => base_exp * 82 / 100,
        2 => base_exp * 68 / 100,
        3 => base_exp * 56 / 100,
        4 => base_exp * 46 / 100,
        5 => base_exp * 38 / 100,
        6 => base_exp * 32 / 100,
        7 => base_exp * 26 / 100,
        8 => base_exp * 21 / 100,
        9 => base_exp * 18 / 100,
        _ => base_exp * 15 / 100,
    }
}

/// C `questlog_done`'s level-based taper (`src/system/questlog.c:286-295`):
/// "scale down by level for those rushing ahead". `level_value` must be the
/// caller's `ugaris_core::world::level_value(level)` result - this leaf
/// module intentionally takes it as a parameter instead of depending on
/// `world::exp` to avoid a `quest` -> `world` module dependency.
pub fn taper_exp_by_level(level: u32, level_value: u32, scaled_exp: i64) -> i64 {
    let level_value = i64::from(level_value);
    if level > 44 {
        scaled_exp.min(level_value / 6)
    } else if level > 19 {
        scaled_exp.min(level_value / 4)
    } else if level > 4 {
        scaled_exp.min(level_value / 2)
    } else {
        scaled_exp.min(level_value)
    }
}

/// Result of `QuestLog::complete_legacy`, mirroring the values C's
/// `questlog_done` (`src/system/questlog.c:267-305`) uses to call
/// `give_exp`/`dlog`/`sendquestlog` - all of which stay in the caller
/// (`World`/`PlayerRuntime` live in different structures, so this leaf
/// module cannot call `World::give_exp` directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestCompletion {
    /// `quest[qnr].done` *after* the increment (C's `cnt + 1` in the dlog
    /// text, and the function's `int` return value).
    pub times_done: u8,
    /// The exp value C passes to `give_exp(cn, val)` (already scaled by
    /// prior completions and tapered by level).
    pub granted_exp: i64,
    /// C's nominal `questlog[qnr].exp` - the `dlog` line is only emitted
    /// when this is `> 0`.
    pub nominal_exp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestReopenResult {
    Reopened,
    CannotOpenAgain,
    CannotOpenNow,
    InvalidQuest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestEntry {
    pub done: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestLog {
    quests: Vec<QuestEntry>,
}

impl Default for QuestLog {
    fn default() -> Self {
        Self {
            quests: vec![QuestEntry::default(); MAX_QUESTS],
        }
    }
}

impl QuestLog {
    pub fn entries(&self) -> &[QuestEntry] {
        &self.quests
    }

    /// C `questlog_open(cn, qnr)` (`src/system/questlog.c:204-219`): sets
    /// `flags` to exactly `QF_OPEN`, discarding any prior `QF_DONE` bit
    /// (C assigns, it doesn't OR). The caller is responsible for the C
    /// side effect of resending the quest log packet.
    pub fn open(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags = QF_OPEN;
        }
    }

    /// C `questlog_close(cn, qnr)` (`src/system/questlog.c:221-238`): only
    /// transitions `QF_OPEN` -> `QF_DONE` when `flags` is *exactly*
    /// `QF_OPEN` (C's `if (quest[qnr].flags == QF_OPEN)`); any other state
    /// (closed, already done) is left untouched.
    pub fn close(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            if entry.flags == QF_OPEN {
                entry.flags = QF_DONE;
            }
        }
    }

    /// C `questlog_done(cn, qnr)`'s bookkeeping half
    /// (`src/system/questlog.c:267-305`, minus the exp math and side
    /// effects - see `complete_legacy` for the full port). Kept as a
    /// simple flag/counter helper for callers that don't need the exp
    /// reward (e.g. test fixtures).
    pub fn mark_done(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags = (entry.flags | QF_DONE) & !QF_OPEN;
            entry.done = entry.done.saturating_add(1).min(0x3f);
        }
    }

    /// C `questlog_done(cn, qnr)` (`src/system/questlog.c:267-305`): full
    /// port including the exp reward computation. Returns `None` for an
    /// out-of-range quest number or one with no metadata row (C would read
    /// past the end of the 85-entry `questlog[]` table for indices
    /// `85..MAX_QUESTS`, which nothing in the ported tree ever does).
    ///
    /// The caller must still perform C's `give_exp(cn, val)` (using
    /// `QuestCompletion::granted_exp`), the `dlog` line (only when
    /// `nominal_exp > 0`), and `sendquestlog` resend - this leaf module
    /// has no access to `World`/`PlayerRuntime`.
    pub fn complete_legacy(
        &mut self,
        quest: usize,
        level: u32,
        level_value: u32,
    ) -> Option<QuestCompletion> {
        let meta = quest_meta(quest)?;
        let entry = self.quests.get_mut(quest)?;

        // C: `cnt = quest[qnr].done++;` (post-increment: `cnt` is the
        // count *before* this completion).
        let prior_completions = entry.done;
        entry.done = entry.done.saturating_add(1).min(0x3f);
        entry.flags = QF_DONE;

        let scaled = scale_exp(prior_completions, meta.exp);
        let granted_exp = taper_exp_by_level(level, level_value, scaled);

        Some(QuestCompletion {
            times_done: entry.done,
            granted_exp,
            nominal_exp: meta.exp,
        })
    }

    pub fn reopen(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags |= QF_OPEN;
            entry.flags &= !QF_DONE;
        }
    }

    pub fn try_reopen_legacy(&mut self, quest: usize) -> QuestReopenResult {
        let Some(entry) = self.quests.get_mut(quest) else {
            return QuestReopenResult::InvalidQuest;
        };
        if entry.done > 9 || (QUESTLOG_FLAGS[quest] & QLF_REPEATABLE) == 0 {
            return QuestReopenResult::CannotOpenAgain;
        }
        if (entry.flags & QF_DONE) == 0 {
            return QuestReopenResult::CannotOpenNow;
        }

        entry.flags = (entry.flags | QF_OPEN) & !QF_DONE;
        QuestReopenResult::Reopened
    }

    pub fn is_done(&self, quest: usize) -> bool {
        self.quests
            .get(quest)
            .is_some_and(|entry| (entry.flags & QF_DONE) != 0)
    }

    pub fn count(&self, quest: usize) -> u8 {
        self.quests.get(quest).map_or(0, |entry| entry.done)
    }

    /// Raw mutable access to a quest entry, for the `questlog_init_*`
    /// ports below which manipulate `quest[qnr].done`/`.flags` directly,
    /// exactly like the C `struct quest *quest` array they read
    /// (`src/system/questlog.c:828-1607`).
    fn entry_mut(&mut self, quest: usize) -> Option<&mut QuestEntry> {
        self.quests.get_mut(quest)
    }
}

/// C's repeated `if (!quest[qnr].done) { quest[qnr].done = 1; }
/// quest[qnr].flags = QF_DONE;` idiom used throughout `questlog_init_*`
/// (e.g. `src/system/questlog.c:836-839`): marks a quest done, seeding
/// `done` to `1` only the first time (never incrementing an existing
/// completion count).
fn mark_init_done(quests: &mut QuestLog, quest: usize) {
    if let Some(entry) = quests.entry_mut(quest) {
        if entry.done == 0 {
            entry.done = 1;
        }
        entry.flags = QF_DONE;
    }
}

fn set_flags(quests: &mut QuestLog, quest: usize, flags: u8) {
    if let Some(entry) = quests.entry_mut(quest) {
        entry.flags = flags;
    }
}

/// The `area1_ppd` fields consumed by `questlog_init_area1`
/// (`src/system/questlog.c:828-1039`); a snapshot built by
/// `PlayerRuntime::area1_quest_state` since this leaf module has no
/// access to `PlayerRuntime`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Area1QuestState {
    pub lydia_state: i32,
    pub gwendy_state: i32,
    pub yoakin_state: i32,
    pub nook_state: i32,
    pub guiwynn_state: i32,
    pub logain_state: i32,
    pub reskin_state: i32,
    pub jessica_state: i32,
    pub brithildie_state: i32,
    pub camhermit_state: i32,
}

// `struct gwendy_ppd`-family NPC state constants
// (`src/common/npc_states.h`), copied verbatim - only the values
// `questlog_init_area1` compares against are needed here.
const GWENDYLON_STATE_ENTRY: i32 = 0;
const GWENDYLON_STATE_FIRST_SKULL_DONE: i32 = 6;
const GWENDYLON_STATE_SECOND_SKULL_DONE: i32 = 10;
const GWENDYLON_STATE_THIRD_SKULL_DONE: i32 = 14;
const GWENDYLON_STATE_FOUL_MAGICIAN_DONE: i32 = 18;
const JESSICA_STATE_QUEST1_GIVE_1: i32 = 1;
const JESSICA_STATE_QUEST1_FINISH: i32 = 7;
const JESSICA_STATE_QUEST2_GIVE_1: i32 = 8;
const JESSICA_STATE_QUEST2_FINISH: i32 = 11;
const BRITHILDIE_STATE_NOMORETALES_QOPEN: i32 = 20;
const BRITHILDIE_STATE_NOMORETALES_QDONE: i32 = 21;
const CAMHERMIT_STATE_QUEST1DO: i32 = 5;
const CAMHERMIT_STATE_QUEST2WAIT: i32 = 6;
const CAMHERMIT_STATE_QUEST2DO: i32 = 11;
const CAMHERMIT_STATE_DONE: i32 = 13;

/// C `questlog_init_area1` (`src/system/questlog.c:828-1039`): derives
/// quest 0 (Lydia), 1-4 (Gwendylon's four skull quests), 5 (Yoakin), 6
/// (Nook), 7-8 (Guiwynn), 9 (Logain), 17 (Reskin), `QLOG_JESSICA_*`,
/// `QLOG_BRITHILDIE`, and `QLOG_HERMIT_QUEST1/2` flags from the matching
/// `area1_ppd` NPC-dialogue state machines. Called once per login via the
/// `questlog_init` dispatcher (not yet wired - no area1 NPC driver exists
/// in Rust to advance these states yet).
pub fn init_area1_quests(quests: &mut QuestLog, ppd: &Area1QuestState) {
    if ppd.lydia_state >= 6 {
        mark_init_done(quests, QLOG_LYDIA);
    } else if ppd.lydia_state > 0 {
        set_flags(quests, QLOG_LYDIA, QF_OPEN);
    } else {
        set_flags(quests, QLOG_LYDIA, 0);
    }

    if ppd.gwendy_state >= GWENDYLON_STATE_FOUL_MAGICIAN_DONE {
        mark_init_done(quests, QLOG_GWENDY_FIRST_SKULL);
        mark_init_done(quests, QLOG_GWENDY_SECOND_SKULL);
        mark_init_done(quests, QLOG_GWENDY_THIRD_SKULL);
        mark_init_done(quests, QLOG_GWENDY_FOUL_MAGICIAN);
    } else if ppd.gwendy_state >= GWENDYLON_STATE_THIRD_SKULL_DONE {
        mark_init_done(quests, QLOG_GWENDY_FIRST_SKULL);
        mark_init_done(quests, QLOG_GWENDY_SECOND_SKULL);
        mark_init_done(quests, QLOG_GWENDY_THIRD_SKULL);
        set_flags(quests, QLOG_GWENDY_FOUL_MAGICIAN, QF_OPEN);
    } else if ppd.gwendy_state >= GWENDYLON_STATE_SECOND_SKULL_DONE {
        mark_init_done(quests, QLOG_GWENDY_FIRST_SKULL);
        mark_init_done(quests, QLOG_GWENDY_SECOND_SKULL);
        set_flags(quests, QLOG_GWENDY_THIRD_SKULL, QF_OPEN);
        set_flags(quests, QLOG_GWENDY_FOUL_MAGICIAN, 0);
    } else if ppd.gwendy_state >= GWENDYLON_STATE_FIRST_SKULL_DONE {
        mark_init_done(quests, QLOG_GWENDY_FIRST_SKULL);
        set_flags(quests, QLOG_GWENDY_SECOND_SKULL, QF_OPEN);
        set_flags(quests, QLOG_GWENDY_THIRD_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_FOUL_MAGICIAN, 0);
    } else if ppd.gwendy_state > GWENDYLON_STATE_ENTRY {
        set_flags(quests, QLOG_GWENDY_FIRST_SKULL, QF_OPEN);
        set_flags(quests, QLOG_GWENDY_SECOND_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_THIRD_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_FOUL_MAGICIAN, 0);
    } else {
        set_flags(quests, QLOG_GWENDY_FIRST_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_SECOND_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_THIRD_SKULL, 0);
        set_flags(quests, QLOG_GWENDY_FOUL_MAGICIAN, 0);
    }

    if ppd.yoakin_state >= 5 {
        mark_init_done(quests, 5);
    } else if ppd.yoakin_state > 0 {
        set_flags(quests, 5, QF_OPEN);
    } else {
        set_flags(quests, 5, 0);
    }

    if ppd.nook_state >= 12 {
        mark_init_done(quests, QLOG_NOOK);
    } else if ppd.nook_state > 0 {
        set_flags(quests, QLOG_NOOK, QF_OPEN);
    } else {
        set_flags(quests, QLOG_NOOK, 0);
    }

    if ppd.guiwynn_state >= 9 {
        mark_init_done(quests, 7);
        mark_init_done(quests, 8);
    } else if ppd.guiwynn_state >= 6 {
        mark_init_done(quests, 7);
        set_flags(quests, 8, QF_OPEN);
    } else if ppd.guiwynn_state > 0 {
        set_flags(quests, 7, QF_OPEN);
        set_flags(quests, 8, 0);
    } else {
        set_flags(quests, 7, 0);
        set_flags(quests, 8, 0);
    }

    if ppd.logain_state >= 6 {
        mark_init_done(quests, 9);
    } else if ppd.logain_state > 0 {
        set_flags(quests, 9, QF_OPEN);
    } else {
        set_flags(quests, 9, 0);
    }

    if ppd.reskin_state >= 8 {
        mark_init_done(quests, 17);
    } else if ppd.reskin_state >= 4 {
        set_flags(quests, 17, QF_OPEN);
    } else {
        set_flags(quests, 17, 0);
    }

    if ppd.jessica_state >= JESSICA_STATE_QUEST1_FINISH {
        mark_init_done(quests, QLOG_JESSICA_ROBBER_NOTE);
    } else if ppd.jessica_state > JESSICA_STATE_QUEST1_GIVE_1 {
        set_flags(quests, QLOG_JESSICA_ROBBER_NOTE, QF_OPEN);
    } else {
        set_flags(quests, QLOG_JESSICA_ROBBER_NOTE, 0);
    }

    if ppd.jessica_state >= JESSICA_STATE_QUEST2_FINISH {
        mark_init_done(quests, QLOG_JESSICA_KILL);
    } else if ppd.jessica_state > JESSICA_STATE_QUEST2_GIVE_1 {
        set_flags(quests, QLOG_JESSICA_KILL, QF_OPEN);
    } else {
        set_flags(quests, QLOG_JESSICA_KILL, 0);
    }

    if ppd.brithildie_state == BRITHILDIE_STATE_NOMORETALES_QDONE {
        mark_init_done(quests, QLOG_BRITHILDIE);
    } else if ppd.brithildie_state == BRITHILDIE_STATE_NOMORETALES_QOPEN {
        set_flags(quests, QLOG_BRITHILDIE, QF_OPEN);
    } else {
        set_flags(quests, QLOG_BRITHILDIE, 0);
    }

    if ppd.camhermit_state >= CAMHERMIT_STATE_QUEST2WAIT {
        mark_init_done(quests, QLOG_HERMIT_QUEST1);
    } else if ppd.camhermit_state == CAMHERMIT_STATE_QUEST1DO {
        set_flags(quests, QLOG_HERMIT_QUEST1, QF_OPEN);
    } else {
        set_flags(quests, QLOG_HERMIT_QUEST1, 0);
    }

    if ppd.camhermit_state >= CAMHERMIT_STATE_DONE {
        mark_init_done(quests, QLOG_HERMIT_QUEST2);
    } else if ppd.camhermit_state == CAMHERMIT_STATE_QUEST2DO {
        set_flags(quests, QLOG_HERMIT_QUEST2, QF_OPEN);
    } else {
        set_flags(quests, QLOG_HERMIT_QUEST2, 0);
    }
}

/// The `nomad_ppd.nomad_state[]` array consumed by `questlog_init_nomad`
/// (`src/system/questlog.c:1571-1607`); a snapshot built by
/// `PlayerRuntime::nomad_quest_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NomadQuestState {
    pub nomad_state: [i32; 10],
}

/// C `questlog_init_nomad` (`src/system/questlog.c:1571-1607`): derives
/// quests 32-34 (Nomad Plains tribe quests) from `nomad_state[1]`,
/// `nomad_state[4]`, and `nomad_state[5]`.
pub fn init_nomad_quests(quests: &mut QuestLog, ppd: &NomadQuestState) {
    if ppd.nomad_state[1] >= 9 {
        mark_init_done(quests, 32);
    } else if ppd.nomad_state[1] > 0 {
        set_flags(quests, 32, QF_OPEN);
    } else {
        set_flags(quests, 32, 0);
    }

    if ppd.nomad_state[4] >= 4 {
        mark_init_done(quests, 33);
    } else if ppd.nomad_state[4] > 0 {
        set_flags(quests, 33, QF_OPEN);
    } else {
        set_flags(quests, 33, 0);
    }

    if ppd.nomad_state[5] >= 4 {
        mark_init_done(quests, 34);
    } else if ppd.nomad_state[5] > 0 {
        set_flags(quests, 34, QF_OPEN);
    } else {
        set_flags(quests, 34, 0);
    }
}

/// The `area3_ppd` fields consumed by `questlog_init_area3`
/// (`src/system/questlog.c:1040-1203`); a snapshot built by
/// `PlayerRuntime::area3_quest_state` since this leaf module has no
/// access to `PlayerRuntime`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Area3QuestState {
    pub seymour_state: i32,
    pub kelly_state: i32,
    pub astro2_state: i32,
    pub crypt_state: i32,
    pub clara_state: i32,
    pub william_state: i32,
    pub hermit_state: i32,
}

/// C `questlog_init_area3` (`src/system/questlog.c:1040-1203`): derives
/// quests 10-12 (Seymour), 13-15 (Kelly), 16 (astro2/Gerassimo), 18-19
/// (Sir Jones' crypt monster), 21 (Clara), 22-23 (William/Imp), and 24
/// (Hermit) from the matching `area3_ppd` NPC-dialogue state machines.
///
/// Faithfully reproduces the C `william_state` ladder's missing final
/// `else` (`src/system/questlog.c:1177-1191`): when `william_state <= 0`
/// quests 22/23 are left untouched instead of reset to `0`, unlike every
/// other ladder in this function.
pub fn init_area3_quests(quests: &mut QuestLog, ppd: &Area3QuestState) {
    if ppd.seymour_state >= 16 {
        mark_init_done(quests, 10);
        mark_init_done(quests, 11);
        mark_init_done(quests, 12);
    } else if ppd.seymour_state >= 12 {
        mark_init_done(quests, 10);
        mark_init_done(quests, 11);
        set_flags(quests, 12, QF_OPEN);
    } else if ppd.seymour_state >= 10 {
        mark_init_done(quests, 10);
        set_flags(quests, 11, QF_OPEN);
        set_flags(quests, 12, 0);
    } else if ppd.seymour_state > 0 {
        set_flags(quests, 10, QF_OPEN);
        set_flags(quests, 11, 0);
        set_flags(quests, 12, 0);
    } else {
        set_flags(quests, 10, 0);
        set_flags(quests, 11, 0);
        set_flags(quests, 12, 0);
    }

    if ppd.kelly_state >= 16 {
        mark_init_done(quests, 13);
        mark_init_done(quests, 14);
        mark_init_done(quests, 15);
    } else if ppd.kelly_state >= 14 {
        mark_init_done(quests, 13);
        mark_init_done(quests, 14);
        set_flags(quests, 15, QF_OPEN);
    } else if ppd.kelly_state >= 6 {
        mark_init_done(quests, 13);
        set_flags(quests, 14, QF_OPEN);
        set_flags(quests, 15, 0);
    } else if ppd.kelly_state >= 2 {
        set_flags(quests, 13, QF_OPEN);
        set_flags(quests, 14, 0);
        set_flags(quests, 15, 0);
    } else {
        set_flags(quests, 13, 0);
        set_flags(quests, 14, 0);
        set_flags(quests, 15, 0);
    }

    if ppd.astro2_state >= 5 {
        mark_init_done(quests, 16);
    } else if ppd.astro2_state > 0 {
        set_flags(quests, 16, QF_OPEN);
    } else {
        set_flags(quests, 16, 0);
    }

    if ppd.crypt_state >= 15 {
        mark_init_done(quests, 18);
        mark_init_done(quests, 19);
    } else if ppd.crypt_state >= 12 {
        mark_init_done(quests, 18);
        set_flags(quests, 19, QF_OPEN);
    } else if ppd.crypt_state > 0 {
        set_flags(quests, 18, QF_OPEN);
        set_flags(quests, 19, 0);
    } else {
        set_flags(quests, 18, 0);
        set_flags(quests, 19, 0);
    }

    if ppd.clara_state >= 15 {
        mark_init_done(quests, 21);
    } else if ppd.clara_state >= 6 {
        set_flags(quests, 21, QF_OPEN);
    } else {
        set_flags(quests, 21, 0);
    }

    // C has no final `else` here (`src/system/questlog.c:1177-1191`):
    // when `william_state <= 0` quests 22/23 keep whatever flags they
    // already had.
    if ppd.william_state >= 7 {
        mark_init_done(quests, 22);
        mark_init_done(quests, 23);
    } else if ppd.william_state >= 3 {
        mark_init_done(quests, 22);
        set_flags(quests, 23, QF_OPEN);
    } else if ppd.william_state > 0 {
        set_flags(quests, 22, QF_OPEN);
        set_flags(quests, 23, 0);
    }

    if ppd.hermit_state >= 5 {
        mark_init_done(quests, 24);
    } else if ppd.hermit_state > 0 {
        set_flags(quests, 24, QF_OPEN);
    } else {
        set_flags(quests, 24, 0);
    }
}

/// The `staffer_ppd` fields consumed by `questlog_init_staff`
/// (`src/system/questlog.c:1203-1394`); a snapshot built by
/// `PlayerRuntime::staff_quest_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StaffQuestState {
    pub carlos_state: i32,
    pub smugglecom_state: i32,
    pub aristocrat_state: i32,
    pub yoatin_state: i32,
    pub countbran_state: i32,
    pub countbran_bits: i32,
    pub brennethbran_state: i32,
    pub spiritbran_state: i32,
    pub broklin_state: i32,
    pub dwarfchief_state: i32,
    pub dwarfshaman_state: i32,
}

/// C `questlog_init_staff` (`src/system/questlog.c:1203-1394`): derives
/// quest 20 (Carlos), 35-37 (smuggler commander), 38 (Aristocrat), 39
/// (Yoatin), 40 (Count Brannington), 41-43 (Brenneth), 44 (Spirit), 45-46
/// (Broklin), and 47-53 (Dwarven Chief/Shaman) from the matching
/// `staffer_ppd` NPC-dialogue state machines.
///
/// Faithfully reproduces the C `yoatin_state` ladder's copy-paste bug
/// (`src/system/questlog.c:1284-1290`): the "open" branch tests
/// `ppd->aristocrat_state > 0`, not `ppd->yoatin_state > 0`.
pub fn init_staff_quests(quests: &mut QuestLog, ppd: &StaffQuestState) {
    if ppd.carlos_state >= 6 {
        mark_init_done(quests, 20);
    } else if ppd.carlos_state > 0 {
        set_flags(quests, 20, QF_OPEN);
    } else {
        set_flags(quests, 20, 0);
    }

    if ppd.smugglecom_state >= 10 {
        mark_init_done(quests, 35);
        mark_init_done(quests, 36);
        mark_init_done(quests, 37);
    } else if ppd.smugglecom_state >= 7 {
        mark_init_done(quests, 35);
        mark_init_done(quests, 36);
        set_flags(quests, 37, QF_OPEN);
    } else if ppd.smugglecom_state >= 5 {
        mark_init_done(quests, 35);
        set_flags(quests, 36, QF_OPEN);
        set_flags(quests, 37, 0);
    } else if ppd.smugglecom_state > 0 {
        set_flags(quests, 35, QF_OPEN);
        set_flags(quests, 36, 0);
        set_flags(quests, 37, 0);
    } else {
        set_flags(quests, 35, 0);
        set_flags(quests, 36, 0);
        set_flags(quests, 37, 0);
    }

    if ppd.aristocrat_state >= 8 {
        mark_init_done(quests, 38);
    } else if ppd.aristocrat_state > 0 {
        set_flags(quests, 38, QF_OPEN);
    } else {
        set_flags(quests, 38, 0);
    }

    // C bug preserved verbatim (`src/system/questlog.c:1284-1290`): this
    // "open" branch tests `aristocrat_state`, not `yoatin_state`.
    if ppd.yoatin_state >= 9 {
        mark_init_done(quests, 39);
    } else if ppd.aristocrat_state > 0 {
        set_flags(quests, 39, QF_OPEN);
    } else {
        set_flags(quests, 39, 0);
    }

    if (ppd.countbran_bits & (1 | 2 | 4)) == (1 | 2 | 4) {
        mark_init_done(quests, 40);
    } else if ppd.countbran_state > 0 {
        set_flags(quests, 40, QF_OPEN);
    } else {
        set_flags(quests, 40, 0);
    }

    if ppd.brennethbran_state >= 12 {
        mark_init_done(quests, 41);
        mark_init_done(quests, 42);
        mark_init_done(quests, 43);
    } else if ppd.brennethbran_state >= 9 {
        mark_init_done(quests, 41);
        mark_init_done(quests, 42);
        set_flags(quests, 43, QF_OPEN);
    } else if ppd.brennethbran_state >= 5 {
        mark_init_done(quests, 41);
        set_flags(quests, 42, QF_OPEN);
        set_flags(quests, 43, 0);
    } else if ppd.brennethbran_state > 0 {
        set_flags(quests, 41, QF_OPEN);
        set_flags(quests, 42, 0);
        set_flags(quests, 43, 0);
    } else {
        set_flags(quests, 41, 0);
        set_flags(quests, 42, 0);
        set_flags(quests, 43, 0);
    }

    if ppd.spiritbran_state >= 5 {
        mark_init_done(quests, 44);
    } else if ppd.spiritbran_state > 0 {
        set_flags(quests, 44, QF_OPEN);
    } else {
        set_flags(quests, 44, 0);
    }

    if ppd.broklin_state >= 11 {
        mark_init_done(quests, 45);
        mark_init_done(quests, 46);
    } else if ppd.broklin_state >= 5 {
        mark_init_done(quests, 45);
        set_flags(quests, 46, QF_OPEN);
    } else if ppd.broklin_state > 0 {
        set_flags(quests, 45, QF_OPEN);
        set_flags(quests, 46, 0);
    } else {
        set_flags(quests, 45, 0);
        set_flags(quests, 46, 0);
    }

    if ppd.dwarfchief_state >= 14 {
        mark_init_done(quests, 47);
        mark_init_done(quests, 48);
        mark_init_done(quests, 49);
        mark_init_done(quests, 50);
    } else if ppd.dwarfchief_state >= 11 {
        mark_init_done(quests, 47);
        mark_init_done(quests, 48);
        mark_init_done(quests, 49);
        set_flags(quests, 50, QF_OPEN);
    } else if ppd.dwarfchief_state >= 8 {
        mark_init_done(quests, 47);
        mark_init_done(quests, 48);
        set_flags(quests, 49, QF_OPEN);
        set_flags(quests, 50, 0);
    } else if ppd.dwarfchief_state >= 5 {
        mark_init_done(quests, 47);
        set_flags(quests, 48, QF_OPEN);
        set_flags(quests, 49, 0);
        set_flags(quests, 50, 0);
    } else if ppd.dwarfchief_state > 0 {
        set_flags(quests, 47, QF_OPEN);
        set_flags(quests, 48, 0);
        set_flags(quests, 49, 0);
        set_flags(quests, 50, 0);
    } else {
        set_flags(quests, 47, 0);
        set_flags(quests, 48, 0);
        set_flags(quests, 49, 0);
        set_flags(quests, 50, 0);
    }

    if ppd.dwarfshaman_state >= 9 {
        mark_init_done(quests, 51);
        mark_init_done(quests, 52);
        mark_init_done(quests, 53);
    } else if ppd.dwarfshaman_state >= 6 {
        mark_init_done(quests, 51);
        mark_init_done(quests, 52);
        set_flags(quests, 53, QF_OPEN);
    } else if ppd.dwarfshaman_state >= 3 {
        mark_init_done(quests, 51);
        set_flags(quests, 52, QF_OPEN);
        set_flags(quests, 53, 0);
    } else if ppd.dwarfshaman_state > 0 {
        set_flags(quests, 51, QF_OPEN);
        set_flags(quests, 52, 0);
        set_flags(quests, 53, 0);
    } else {
        set_flags(quests, 51, 0);
        set_flags(quests, 52, 0);
        set_flags(quests, 53, 0);
    }
}

/// The `twocity_ppd` fields consumed by `questlog_init_twocity`
/// (`src/system/questlog.c:1470-1546`); a snapshot built by
/// `PlayerRuntime::twocity_quest_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TwocityQuestState {
    pub thief_state: i32,
    pub sanwyn_state: i32,
    pub skelly_state: i32,
    pub alchemist_state: i32,
}

/// C `questlog_init_twocity` (`src/system/questlog.c:1470-1546`): derives
/// quests 25-28 (Guildmaster's thief chain), 29 (Sanwyn), 30 (Skelly),
/// and 31 (Alchemist) from the matching `twocity_ppd` NPC-dialogue state
/// machines.
pub fn init_twocity_quests(quests: &mut QuestLog, ppd: &TwocityQuestState) {
    if ppd.thief_state >= 20 {
        mark_init_done(quests, 25);
        mark_init_done(quests, 26);
        mark_init_done(quests, 27);
        mark_init_done(quests, 28);
    } else if ppd.thief_state >= 18 {
        mark_init_done(quests, 25);
        mark_init_done(quests, 26);
        mark_init_done(quests, 27);
        set_flags(quests, 28, QF_OPEN);
    } else if ppd.thief_state >= 14 {
        mark_init_done(quests, 25);
        mark_init_done(quests, 26);
        set_flags(quests, 27, QF_OPEN);
        set_flags(quests, 28, 0);
    } else if ppd.thief_state >= 10 {
        mark_init_done(quests, 25);
        set_flags(quests, 26, QF_OPEN);
        set_flags(quests, 27, 0);
        set_flags(quests, 28, 0);
    } else if ppd.thief_state >= 5 {
        set_flags(quests, 25, QF_OPEN);
        set_flags(quests, 26, 0);
        set_flags(quests, 27, 0);
        set_flags(quests, 28, 0);
    } else {
        set_flags(quests, 25, 0);
        set_flags(quests, 26, 0);
        set_flags(quests, 27, 0);
        set_flags(quests, 28, 0);
    }

    if ppd.sanwyn_state >= 8 {
        mark_init_done(quests, 29);
    } else if ppd.sanwyn_state > 0 {
        set_flags(quests, 29, QF_OPEN);
    } else {
        set_flags(quests, 29, 0);
    }

    if ppd.skelly_state >= 3 {
        mark_init_done(quests, 30);
    } else if ppd.skelly_state > 0 {
        set_flags(quests, 30, QF_OPEN);
    } else {
        set_flags(quests, 30, 0);
    }

    if ppd.alchemist_state >= 5 {
        mark_init_done(quests, 31);
    } else if ppd.alchemist_state > 0 {
        set_flags(quests, 31, QF_OPEN);
    } else {
        set_flags(quests, 31, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_constants_match_c_header() {
        assert_eq!(MAX_QUESTS, 100);
        assert_eq!(QF_OPEN, 1);
        assert_eq!(QF_DONE, 2);
        assert_eq!(QLF_REPEATABLE, 1);
        assert_eq!(QLOG_JESSICA_KILL, 84);
    }

    #[test]
    fn quest_done_count_is_six_bit_like_c_bitfield() {
        let mut log = QuestLog::default();
        for _ in 0..70 {
            log.mark_done(QLOG_LYDIA);
        }
        assert_eq!(log.count(QLOG_LYDIA), 0x3f);
        assert!(log.is_done(QLOG_LYDIA));
    }

    #[test]
    fn entries_expose_fixed_legacy_quest_count() {
        let log = QuestLog::default();

        assert_eq!(log.entries().len(), MAX_QUESTS);
    }

    #[test]
    fn reopen_legacy_allows_done_repeatable_quests() {
        let mut log = QuestLog::default();
        log.mark_done(QLOG_LYDIA);

        assert_eq!(
            log.try_reopen_legacy(QLOG_LYDIA),
            QuestReopenResult::Reopened
        );
        let entry = log.entries()[QLOG_LYDIA];
        assert_eq!(entry.done, 1);
        assert_eq!(entry.flags, QF_OPEN);
    }

    #[test]
    fn reopen_legacy_rejects_non_repeatable_and_not_done_quests() {
        let mut log = QuestLog::default();
        assert_eq!(
            log.try_reopen_legacy(QLOG_NOOK),
            QuestReopenResult::CannotOpenAgain
        );
        assert_eq!(
            log.try_reopen_legacy(QLOG_GWENDY_FIRST_SKULL),
            QuestReopenResult::CannotOpenNow
        );
    }

    #[test]
    fn reopen_legacy_rejects_after_ten_completions_like_c() {
        let mut log = QuestLog::default();
        for _ in 0..10 {
            log.mark_done(QLOG_LYDIA);
        }

        assert_eq!(
            log.try_reopen_legacy(QLOG_LYDIA),
            QuestReopenResult::CannotOpenAgain
        );
    }

    /// C `level_value(level)` (`src/system/tool.c:1282`), duplicated here
    /// only for test expectations (this leaf module doesn't depend on
    /// `world::exp` - see `taper_exp_by_level`'s doc comment).
    fn level_value(level: u32) -> u32 {
        let next = level + 1;
        next.pow(4) - level.pow(4)
    }

    #[test]
    fn quest_table_has_85_entries_matching_c_array() {
        assert_eq!(QUEST_TABLE.len(), 85);
        assert_eq!(quest_meta(85), None);
        assert_eq!(quest_meta(MAX_QUESTS - 1), None);
    }

    #[test]
    fn quest_table_entries_match_c_source_digit_for_digit() {
        let lydia = quest_meta(QLOG_LYDIA).unwrap();
        assert_eq!(lydia.name, "Lydia's Potion");
        assert_eq!(lydia.min_level, 1);
        assert_eq!(lydia.max_level, 2);
        assert_eq!(lydia.giver, "James");
        assert_eq!(lydia.area, "Cameron");
        assert_eq!(lydia.exp, 15);
        assert_eq!(lydia.flags, QLF_REPEATABLE);

        // Trailing-space quest names copied verbatim from the C table.
        assert_eq!(quest_meta(40).unwrap().name, "The Jewels of Brannington ");
        assert_eq!(quest_meta(42).unwrap().name, "A Thief's Loot ");

        // QLF_XREPEAT-only entries (not QLF_REPEATABLE).
        for qnr in [25, 26, 27, 28] {
            let meta = quest_meta(qnr).unwrap();
            assert_eq!(meta.flags, QLF_XREPEAT);
            assert_eq!(meta.flags & QLF_REPEATABLE, 0);
        }

        // Highest-value quest in the table.
        let sarkilar = quest_meta(33).unwrap();
        assert_eq!(sarkilar.name, "Searching Sarkilar");
        assert_eq!(sarkilar.exp, 450000);

        assert_eq!(
            quest_meta(QLOG_JESSICA_KILL).unwrap().name,
            "Defeating the Robber Leader"
        );
    }

    #[test]
    fn quest_table_flags_stay_in_sync_with_reopen_repeatability_table() {
        // Every quest previously hand-marked repeatable in QUESTLOG_FLAGS
        // must have QLF_REPEATABLE set in the ported metadata table too.
        let repeatable_indices = [
            0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 20, 30, 31, 35, 37, 38, 39, 40, 41, 42, 43, 44,
            45, 79, 83, 84,
        ];
        for qnr in 0..QUEST_TABLE.len() {
            let expects_repeatable = repeatable_indices.contains(&qnr);
            let is_repeatable = (QUEST_TABLE[qnr].flags & QLF_REPEATABLE) != 0;
            assert_eq!(
                is_repeatable, expects_repeatable,
                "quest {qnr} repeatability mismatch"
            );
        }
    }

    #[test]
    fn scale_exp_matches_c_questlog_scale_curve() {
        assert_eq!(scale_exp(0, 1000), 1000);
        assert_eq!(scale_exp(1, 1000), 820);
        assert_eq!(scale_exp(2, 1000), 680);
        assert_eq!(scale_exp(3, 1000), 560);
        assert_eq!(scale_exp(4, 1000), 460);
        assert_eq!(scale_exp(5, 1000), 380);
        assert_eq!(scale_exp(6, 1000), 320);
        assert_eq!(scale_exp(7, 1000), 260);
        assert_eq!(scale_exp(8, 1000), 210);
        assert_eq!(scale_exp(9, 1000), 180);
        assert_eq!(scale_exp(10, 1000), 150);
        assert_eq!(scale_exp(200, 1000), 150);
    }

    #[test]
    fn taper_exp_by_level_matches_c_bands() {
        // level <= 4: min(level_value(level), val)
        assert_eq!(
            taper_exp_by_level(1, level_value(1), 1_000_000),
            level_value(1) as i64
        );
        assert_eq!(taper_exp_by_level(1, level_value(1), 1), 1);

        // 4 < level <= 19: min(level_value(level)/2, val)
        assert_eq!(
            taper_exp_by_level(10, level_value(10), 1_000_000_000),
            (level_value(10) / 2) as i64
        );

        // 19 < level <= 44: min(level_value(level)/4, val)
        assert_eq!(
            taper_exp_by_level(30, level_value(30), 1_000_000_000),
            (level_value(30) / 4) as i64
        );

        // level > 44: min(level_value(level)/6, val)
        assert_eq!(
            taper_exp_by_level(50, level_value(50), 1_000_000_000),
            (level_value(50) / 6) as i64
        );
    }

    #[test]
    fn complete_legacy_ports_questlog_done_first_completion() {
        let mut log = QuestLog::default();
        log.open(QLOG_LYDIA);

        let result = log
            .complete_legacy(QLOG_LYDIA, 1, level_value(1))
            .expect("Lydia's Potion has metadata");

        assert_eq!(result.times_done, 1);
        assert_eq!(result.nominal_exp, 15);
        // scale_exp(0, 15) = 15, tapered by min(level_value(1), 15) = 15
        // (level_value(1) is far bigger than 15 for level 1).
        assert_eq!(result.granted_exp, 15);

        let entry = log.entries()[QLOG_LYDIA];
        assert_eq!(entry.done, 1);
        assert_eq!(entry.flags, QF_DONE);
    }

    #[test]
    fn complete_legacy_scales_repeat_completions_and_increments_done() {
        let mut log = QuestLog::default();
        // Complete Lydia's Potion (exp 15, repeatable) three times.
        for expected_prior in 0..3u8 {
            let result = log.complete_legacy(QLOG_LYDIA, 1, level_value(1)).unwrap();
            assert_eq!(result.times_done, expected_prior + 1);
        }
        assert_eq!(log.count(QLOG_LYDIA), 3);

        // Now complete a high-level, high-exp quest at a high level to
        // exercise the taper.
        let mut log2 = QuestLog::default();
        let result = log2
            .complete_legacy(20, 50, level_value(50))
            .expect("Wanted: Occult Staff has metadata");
        assert_eq!(result.nominal_exp, 40000);
        // level 50 > 44, so granted = min(level_value(50)/6, 40000)
        let expected = (level_value(50) as i64 / 6).min(40000);
        assert_eq!(result.granted_exp, expected);
    }

    #[test]
    fn complete_legacy_returns_none_for_indices_without_metadata() {
        let mut log = QuestLog::default();
        assert_eq!(log.complete_legacy(85, 1, level_value(1)), None);
        assert_eq!(log.complete_legacy(MAX_QUESTS, 1, level_value(1)), None);
    }

    #[test]
    fn open_matches_c_unconditional_assignment() {
        let mut log = QuestLog::default();
        log.mark_done(QLOG_LYDIA);
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);

        // C `questlog_open` assigns flags = QF_OPEN outright, clearing
        // QF_DONE, without touching `done`.
        log.open(QLOG_LYDIA);
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_LYDIA].done, 1);
    }

    #[test]
    fn close_only_transitions_from_exactly_open_like_c() {
        let mut log = QuestLog::default();

        // Closed (flags = 0): no-op.
        log.close(QLOG_LYDIA);
        assert_eq!(log.entries()[QLOG_LYDIA].flags, 0);

        // Open -> Done.
        log.open(QLOG_LYDIA);
        log.close(QLOG_LYDIA);
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);

        // Already done: closing again is a no-op (flags stay QF_DONE, not
        // reset to 0 or anything else).
        log.close(QLOG_LYDIA);
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);
    }

    #[test]
    fn init_area1_quests_lydia_branches_match_c() {
        let mut log = QuestLog::default();

        // done > 0, no flag transition into open until >=6.
        init_area1_quests(&mut log, &Area1QuestState::default());
        assert_eq!(log.entries()[QLOG_LYDIA].flags, 0);

        init_area1_quests(
            &mut log,
            &Area1QuestState {
                lydia_state: 3,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_OPEN);

        init_area1_quests(
            &mut log,
            &Area1QuestState {
                lydia_state: 6,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_LYDIA].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_LYDIA].done, 1);

        // Calling again with the same state must not bump `done` past 1
        // (C only seeds `done = 1` when it was previously 0).
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                lydia_state: 6,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_LYDIA].done, 1);
    }

    #[test]
    fn init_area1_quests_gwendy_series_matches_c_ladder() {
        let mut log = QuestLog::default();

        // Entry: all four closed.
        init_area1_quests(&mut log, &Area1QuestState::default());
        for quest in [
            QLOG_GWENDY_FIRST_SKULL,
            QLOG_GWENDY_SECOND_SKULL,
            QLOG_GWENDY_THIRD_SKULL,
            QLOG_GWENDY_FOUL_MAGICIAN,
        ] {
            assert_eq!(log.entries()[quest].flags, 0);
        }

        // In progress on first skull.
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                gwendy_state: 3,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, 0);

        // First skull done (>=6), second open.
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                gwendy_state: GWENDYLON_STATE_FIRST_SKULL_DONE,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, 0);

        // Second skull done (>=10), third open.
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                gwendy_state: GWENDYLON_STATE_SECOND_SKULL_DONE,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_GWENDY_FIRST_SKULL].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_GWENDY_SECOND_SKULL].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_GWENDY_FOUL_MAGICIAN].flags, 0);

        // Third skull done (>=14), foul magician open.
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                gwendy_state: GWENDYLON_STATE_THIRD_SKULL_DONE,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_GWENDY_THIRD_SKULL].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_GWENDY_FOUL_MAGICIAN].flags, QF_OPEN);

        // Foul magician done (>=18): whole series done, `done` seeded to 1.
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                gwendy_state: GWENDYLON_STATE_FOUL_MAGICIAN_DONE,
                ..Default::default()
            },
        );
        for quest in [
            QLOG_GWENDY_FIRST_SKULL,
            QLOG_GWENDY_SECOND_SKULL,
            QLOG_GWENDY_THIRD_SKULL,
            QLOG_GWENDY_FOUL_MAGICIAN,
        ] {
            assert_eq!(log.entries()[quest].flags, QF_DONE);
            assert_eq!(log.entries()[quest].done, 1);
        }
    }

    #[test]
    fn init_area1_quests_yoakin_nook_guiwynn_logain_reskin_match_c() {
        let mut log = QuestLog::default();
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                yoakin_state: 5,
                nook_state: 12,
                guiwynn_state: 9,
                logain_state: 6,
                reskin_state: 8,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[5].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_NOOK].flags, QF_DONE);
        assert_eq!(log.entries()[7].flags, QF_DONE);
        assert_eq!(log.entries()[8].flags, QF_DONE);
        assert_eq!(log.entries()[9].flags, QF_DONE);
        assert_eq!(log.entries()[17].flags, QF_DONE);

        let mut log = QuestLog::default();
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                yoakin_state: 2,
                nook_state: 4,
                guiwynn_state: 7,
                logain_state: 3,
                reskin_state: 5,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[5].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_NOOK].flags, QF_OPEN);
        assert_eq!(log.entries()[7].flags, QF_DONE);
        assert_eq!(log.entries()[8].flags, QF_OPEN);
        assert_eq!(log.entries()[9].flags, QF_OPEN);
        // reskin_state=5 is >=4 but <8: open, not done.
        assert_eq!(log.entries()[17].flags, QF_OPEN);
    }

    #[test]
    fn init_area1_quests_jessica_brithildie_camhermit_match_c() {
        let mut log = QuestLog::default();
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                jessica_state: JESSICA_STATE_QUEST1_FINISH,
                brithildie_state: BRITHILDIE_STATE_NOMORETALES_QOPEN,
                camhermit_state: CAMHERMIT_STATE_QUEST1DO,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_JESSICA_ROBBER_NOTE].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_JESSICA_KILL].flags, 0);
        assert_eq!(log.entries()[QLOG_BRITHILDIE].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_HERMIT_QUEST1].flags, QF_OPEN);
        assert_eq!(log.entries()[QLOG_HERMIT_QUEST2].flags, 0);

        let mut log = QuestLog::default();
        init_area1_quests(
            &mut log,
            &Area1QuestState {
                jessica_state: JESSICA_STATE_QUEST2_FINISH,
                brithildie_state: BRITHILDIE_STATE_NOMORETALES_QDONE,
                camhermit_state: CAMHERMIT_STATE_DONE,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[QLOG_JESSICA_ROBBER_NOTE].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_JESSICA_KILL].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_BRITHILDIE].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_HERMIT_QUEST1].flags, QF_DONE);
        assert_eq!(log.entries()[QLOG_HERMIT_QUEST2].flags, QF_DONE);
    }

    #[test]
    fn init_nomad_quests_matches_c_thresholds() {
        let mut log = QuestLog::default();
        init_nomad_quests(&mut log, &NomadQuestState::default());
        assert_eq!(log.entries()[32].flags, 0);
        assert_eq!(log.entries()[33].flags, 0);
        assert_eq!(log.entries()[34].flags, 0);

        let mut state = NomadQuestState::default();
        state.nomad_state[1] = 5;
        state.nomad_state[4] = 2;
        state.nomad_state[5] = 1;
        init_nomad_quests(&mut log, &state);
        assert_eq!(log.entries()[32].flags, QF_OPEN);
        assert_eq!(log.entries()[33].flags, QF_OPEN);
        assert_eq!(log.entries()[34].flags, QF_OPEN);

        let mut state = NomadQuestState::default();
        state.nomad_state[1] = 9;
        state.nomad_state[4] = 4;
        state.nomad_state[5] = 4;
        init_nomad_quests(&mut log, &state);
        assert_eq!(log.entries()[32].flags, QF_DONE);
        assert_eq!(log.entries()[32].done, 1);
        assert_eq!(log.entries()[33].flags, QF_DONE);
        assert_eq!(log.entries()[34].flags, QF_DONE);
    }

    #[test]
    fn init_area3_quests_seymour_and_kelly_ladders_match_c() {
        let mut log = QuestLog::default();
        init_area3_quests(&mut log, &Area3QuestState::default());
        for quest in [10, 11, 12, 13, 14, 15] {
            assert_eq!(log.entries()[quest].flags, 0);
        }

        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                seymour_state: 1,
                kelly_state: 2,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[10].flags, QF_OPEN);
        assert_eq!(log.entries()[11].flags, 0);
        assert_eq!(log.entries()[13].flags, QF_OPEN);
        assert_eq!(log.entries()[14].flags, 0);

        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                seymour_state: 12,
                kelly_state: 14,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[10].flags, QF_DONE);
        assert_eq!(log.entries()[11].flags, QF_DONE);
        assert_eq!(log.entries()[12].flags, QF_OPEN);
        assert_eq!(log.entries()[13].flags, QF_DONE);
        assert_eq!(log.entries()[14].flags, QF_DONE);
        assert_eq!(log.entries()[15].flags, QF_OPEN);

        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                seymour_state: 16,
                kelly_state: 16,
                ..Default::default()
            },
        );
        for quest in [10, 11, 12, 13, 14, 15] {
            assert_eq!(log.entries()[quest].flags, QF_DONE);
            assert_eq!(log.entries()[quest].done, 1);
        }
    }

    #[test]
    fn init_area3_quests_astro2_crypt_clara_hermit_match_c() {
        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                astro2_state: 5,
                crypt_state: 12,
                clara_state: 15,
                hermit_state: 5,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[16].flags, QF_DONE);
        assert_eq!(log.entries()[18].flags, QF_DONE);
        assert_eq!(log.entries()[19].flags, QF_OPEN);
        assert_eq!(log.entries()[21].flags, QF_DONE);
        assert_eq!(log.entries()[24].flags, QF_DONE);

        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                astro2_state: 1,
                crypt_state: 1,
                clara_state: 6,
                hermit_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[16].flags, QF_OPEN);
        assert_eq!(log.entries()[18].flags, QF_OPEN);
        assert_eq!(log.entries()[19].flags, 0);
        assert_eq!(log.entries()[21].flags, QF_OPEN);
        assert_eq!(log.entries()[24].flags, QF_OPEN);
    }

    #[test]
    fn init_area3_quests_william_ladder_has_no_final_else_like_c() {
        // Prime quests 22/23 to a non-zero flag, then confirm
        // `william_state <= 0` leaves them untouched (C has no final
        // `else` branch in this ladder, unlike every other one).
        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                william_state: 7,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[22].flags, QF_DONE);
        assert_eq!(log.entries()[23].flags, QF_DONE);

        init_area3_quests(&mut log, &Area3QuestState::default());
        assert_eq!(log.entries()[22].flags, QF_DONE);
        assert_eq!(log.entries()[23].flags, QF_DONE);

        // A fresh log with william_state=1 opens quest 22 only.
        let mut log = QuestLog::default();
        init_area3_quests(
            &mut log,
            &Area3QuestState {
                william_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[22].flags, QF_OPEN);
        assert_eq!(log.entries()[23].flags, 0);
    }

    #[test]
    fn init_staff_quests_carlos_smugglecom_countbran_match_c() {
        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                carlos_state: 6,
                smugglecom_state: 10,
                countbran_bits: 1 | 2 | 4,
                countbran_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[20].flags, QF_DONE);
        assert_eq!(log.entries()[35].flags, QF_DONE);
        assert_eq!(log.entries()[36].flags, QF_DONE);
        assert_eq!(log.entries()[37].flags, QF_DONE);
        assert_eq!(log.entries()[40].flags, QF_DONE);

        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                carlos_state: 1,
                smugglecom_state: 5,
                countbran_bits: 1 | 2,
                countbran_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[20].flags, QF_OPEN);
        assert_eq!(log.entries()[35].flags, QF_DONE);
        assert_eq!(log.entries()[36].flags, QF_OPEN);
        assert_eq!(log.entries()[37].flags, 0);
        // Missing bit 4: not all bits set, but state>0 so open.
        assert_eq!(log.entries()[40].flags, QF_OPEN);
    }

    #[test]
    fn init_staff_quests_yoatin_ladder_reproduces_c_copy_paste_bug() {
        // C bug: the "open" branch for quest 39 tests `aristocrat_state`,
        // not `yoatin_state` (`src/system/questlog.c:1284-1290`). With
        // yoatin_state=0 but aristocrat_state>0, quest 39 still opens.
        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                yoatin_state: 0,
                aristocrat_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[39].flags, QF_OPEN);

        // With both zero, quest 39 stays closed.
        let mut log = QuestLog::default();
        init_staff_quests(&mut log, &StaffQuestState::default());
        assert_eq!(log.entries()[39].flags, 0);

        // yoatin_state alone (aristocrat_state=0) does NOT open it either,
        // per the same bug.
        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                yoatin_state: 3,
                aristocrat_state: 0,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[39].flags, 0);

        // yoatin_state>=9 always marks it done regardless of aristocrat.
        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                yoatin_state: 9,
                aristocrat_state: 0,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[39].flags, QF_DONE);
    }

    #[test]
    fn init_staff_quests_brenneth_broklin_dwarf_ladders_match_c() {
        let mut log = QuestLog::default();
        init_staff_quests(
            &mut log,
            &StaffQuestState {
                brennethbran_state: 9,
                spiritbran_state: 5,
                broklin_state: 11,
                dwarfchief_state: 11,
                dwarfshaman_state: 6,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[41].flags, QF_DONE);
        assert_eq!(log.entries()[42].flags, QF_DONE);
        assert_eq!(log.entries()[43].flags, QF_OPEN);
        assert_eq!(log.entries()[44].flags, QF_DONE);
        assert_eq!(log.entries()[45].flags, QF_DONE);
        assert_eq!(log.entries()[46].flags, QF_DONE);
        assert_eq!(log.entries()[47].flags, QF_DONE);
        assert_eq!(log.entries()[48].flags, QF_DONE);
        assert_eq!(log.entries()[49].flags, QF_DONE);
        assert_eq!(log.entries()[50].flags, QF_OPEN);
        assert_eq!(log.entries()[51].flags, QF_DONE);
        assert_eq!(log.entries()[52].flags, QF_DONE);
        assert_eq!(log.entries()[53].flags, QF_OPEN);

        let mut log = QuestLog::default();
        init_staff_quests(&mut log, &StaffQuestState::default());
        for quest in [41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53] {
            assert_eq!(log.entries()[quest].flags, 0);
        }
    }

    #[test]
    fn init_twocity_quests_thief_ladder_matches_c() {
        let mut log = QuestLog::default();
        init_twocity_quests(&mut log, &TwocityQuestState::default());
        for quest in [25, 26, 27, 28] {
            assert_eq!(log.entries()[quest].flags, 0);
        }

        let mut log = QuestLog::default();
        init_twocity_quests(
            &mut log,
            &TwocityQuestState {
                thief_state: 18,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[25].flags, QF_DONE);
        assert_eq!(log.entries()[26].flags, QF_DONE);
        assert_eq!(log.entries()[27].flags, QF_DONE);
        assert_eq!(log.entries()[28].flags, QF_OPEN);

        let mut log = QuestLog::default();
        init_twocity_quests(
            &mut log,
            &TwocityQuestState {
                thief_state: 20,
                ..Default::default()
            },
        );
        for quest in [25, 26, 27, 28] {
            assert_eq!(log.entries()[quest].flags, QF_DONE);
            assert_eq!(log.entries()[quest].done, 1);
        }
    }

    #[test]
    fn init_twocity_quests_sanwyn_skelly_alchemist_match_c() {
        let mut log = QuestLog::default();
        init_twocity_quests(
            &mut log,
            &TwocityQuestState {
                sanwyn_state: 8,
                skelly_state: 3,
                alchemist_state: 5,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[29].flags, QF_DONE);
        assert_eq!(log.entries()[30].flags, QF_DONE);
        assert_eq!(log.entries()[31].flags, QF_DONE);

        let mut log = QuestLog::default();
        init_twocity_quests(
            &mut log,
            &TwocityQuestState {
                sanwyn_state: 1,
                skelly_state: 1,
                alchemist_state: 1,
                ..Default::default()
            },
        );
        assert_eq!(log.entries()[29].flags, QF_OPEN);
        assert_eq!(log.entries()[30].flags, QF_OPEN);
        assert_eq!(log.entries()[31].flags, QF_OPEN);
    }
}
