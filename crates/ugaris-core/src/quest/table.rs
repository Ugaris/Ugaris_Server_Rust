use super::*;

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
/// C `src/common/quest_exp.h`: per-encounter exp/money constants handed to
/// `give_exp`/`give_military_pts`/`create_money_item` by individual area
/// NPC drivers (not by `questlog.c` itself - none of these are read
/// anywhere in this file). Copied digit for digit; kept here since this is
/// the quest-adjacent home for quest reward constants until the P4 area
/// driver tasks that actually consume them land (`EXP_AREA15_HARDKILL`
/// and `EXP_AREA3_SHRINE` are the only two used in the C source today -
/// every other `EXP_AREA*` define is dead code in C too, and every
/// `MONEY_AREA*` one feeds a `create_money_item` call in an area driver;
/// `MONEY_AREA1_BEARTOOTH` is the first to actually land, in
/// `world::yoakin`'s bear-tooth reward - the rest still await their own
/// area driver ports).
pub mod quest_exp {
    pub const EXP_AREA1_SKULL1: i64 = 75;
    pub const EXP_AREA1_SKULL2: i64 = 150;
    pub const EXP_AREA1_SKULL3: i64 = 300;
    pub const EXP_AREA1_JESTER: i64 = 400;
    pub const EXP_AREA1_SKULL4: i64 = 800;
    pub const EXP_AREA1_BEARTOOTH: i64 = 600;
    pub const EXP_AREA1_MADMAGE1: i64 = 800;
    pub const EXP_AREA1_MADMAGE2: i64 = 900;
    pub const EXP_AREA1_MADKNIGHT: i64 = 1200;
    pub const EXP_AREA1_GUILD: i64 = 1250;

    pub const EXP_AREA3_SKULL1: i64 = 850;
    pub const EXP_AREA3_SKULL2: i64 = 1000;
    pub const EXP_AREA3_SKULL3: i64 = 1250;
    pub const EXP_AREA3_LOISAN: i64 = 1500;

    pub const EXP_AREA3_CREEPER: i64 = 1850;
    /// Per shrine, 3 total (C comment preserved verbatim).
    pub const EXP_AREA3_SHRINE: i64 = 1500;
    pub const EXP_AREA3_MOONIES: i64 = 5000;
    /// 50% bonus if no money (C comment preserved verbatim).
    pub const EXP_AREA2_VAMPIRE1: i64 = 5000;
    pub const EXP_AREA2_VAMPIRE2: i64 = 12000;
    pub const EXP_AREA3_REACHCLARA: i64 = 2500;
    pub const EXP_AREA15_HARDKILL: i64 = 7500;
    pub const EXP_AREA15_DIDKILL: i64 = 22500;
    pub const EXP_AREA16_BEARKILL: i64 = 12500;
    pub const EXP_AREA16_MANTIS: i64 = 15000;
    pub const EXP_AREA16_SPIDERKILL: i64 = 25000;

    pub const MONEY_AREA1_SKULL1: i64 = 125;
    pub const MONEY_AREA1_SKULL2: i64 = 250;
    pub const MONEY_AREA1_SKULL3: i64 = 400;
    pub const MONEY_AREA1_SKULL4: i64 = 600;
    pub const MONEY_AREA1_BEARTOOTH: i64 = 500;
    pub const MONEY_AREA1_MADMAGE1: i64 = 250;
    pub const MONEY_AREA1_MADMAGE2: i64 = 500;
    pub const MONEY_AREA1_MADKNIGHT: i64 = 550;
    pub const MONEY_AREA3_MOONIES: i64 = 2500;
    pub const MONEY_AREA3_VAMPIRE1: i64 = 2500;
}
